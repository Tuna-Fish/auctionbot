use std::cmp::Ordering;
use log::{error, info};
use serenity::prelude::*;
use crate::DbClientContainer;
use crate::GameStateContainer;
use crate::gamestate::GameState;
use crate::gamestate::{auctions_per_day, items_per_auction };
use rand::prelude::*;
use std::collections::HashMap;
use std::cmp;

struct User {
    name : String,
    id : i64,
    points : i32
}

#[derive(Eq,Clone)]
struct Bid {
    item : String,
    user : i64,
    price : i32,
    reserve : i32
}

impl Ord for Bid {
    fn cmp(&self, other :&Self) -> Ordering {
        self.price.cmp(&other.price)
    }
}

impl PartialOrd for Bid {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for Bid {
    fn eq(&self, other: &Self) -> bool {
        self.price == other.price
    }
}

pub fn first_char(s : &str) -> char {
    s.chars().next().unwrap()
}

pub async fn auction(ctx: &Context, advance: bool) -> Option<(GameState,GameState)> {
    let data = ctx.data.read().await;

    let gamestatearc = data.get::<GameStateContainer>().unwrap();
    let mut gamestatewriteguard = (&gamestatearc).write().await;
    let (day, rate) = match *gamestatewriteguard {
        GameState::Auction{day, deadline, rate} => (day, rate),
        _ => { error!("called auction on non-auction day");
                return None;
        }   
    };

    let arcdb = data.get::<DbClientContainer>().expect("where is my db?");

    let nr_auctions = auctions_per_day(&arcdb,day).await;

    let mut outer : Vec<Vec<Bid>> = {
        let mut outer = Vec::new();
        for j in 1..=nr_auctions {
            let mut bids = Vec::new();
            let nr = j as i16;
            let rows = arcdb.query("SELECT bid.userid,bid.itemname, bid.bid, bid.reserve FROM bid INNER JOIN item ON (bid.itemname = item.name) WHERE (item.day = $1) AND (item.nr = $2)",&[&day,&nr]).await.expect("error loading all bids");
            for row in rows {
                bids.push(Bid {
                    item: row.get(1),
                    user: row.get(0),
                    price:  row.get(2),
                    reserve: row.get(3),
                });
            }
            outer.push(bids);
        }
        outer
    };
    
    let userrows = arcdb.query("SELECT id, name, points FROM discorduser",&[]).await.expect("muh users");
    let mut users = HashMap::new();
    for row in userrows {
        let user = User {
            name : row.get(1),
            id : row.get(0),
            points : row.get(2)
        };
        info!("prior to auction on day {}, user {}/{} had {} points",day,user.name,user.id,user.points);
        users.insert(user.id,user);
    }

    let mut rng = StdRng::from_entropy();

    for mut bids in outer {
        (&mut bids[..]).shuffle(&mut rng);
        loop {
            //no more bids, we are done
            if bids.len() == 0 {
                break;
            }
            //capping bids to cash/reserves
            for bid in bids.iter_mut() {
                let userid = bid.user;
                let points = users.get(&userid).unwrap().points;
                bid.price = cmp::max(0,cmp::min(bid.price,points-bid.reserve));
            
            }
            //finding highest bid
            bids.sort();
            let winner = bids.pop().expect("no highest bid in a list with >0 entries");

            if winner.price == 0 {
                break;
            }
            
            //congratulations
            

            //figure out cost
            let mut cost = 0;
            for bid in bids.iter().rev(){
                if bid.item == winner.item {
                        cost = bid.price;
                        break;
                    }
            }

            users.get_mut(&winner.user).unwrap().points -= cost;
            info!(" user: {} won bid for {} with {} points", users.get(&winner.user).unwrap().name, winner.item, cost);            
            arcdb.query("INSERT INTO win (userid,item,day,cost) VALUES($1,$2,$3,$4)",&[&winner.user,&winner.item,&day,&cost]).await.expect("failed to store winning bid");
            arcdb.query("UPDATE discorduser SET points = $1 WHERE id = $2;",&[&users.get(&winner.user).unwrap().points,&users.get(&winner.user).unwrap().id]).await.expect("failed to store winner's points");
            

            //remove invalid bids
            //first, remove all remaining bids by the same bidder
            bids.retain(|bid|bid.user != winner.user);

            //second, remove bids on items that are no longer available
            bids.retain(|bid|bid.item != winner.item);
        }
    }
	
    return if advance {
        let gamestate =  *gamestatewriteguard;
        let newgamestate = gamestate.advance(&arcdb).await;
		*gamestatewriteguard = newgamestate;
		Some((newgamestate, gamestate))
    } else { None };
	
}
