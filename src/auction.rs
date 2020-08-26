use std::cmp::Ordering;
use log::{debug, error, info};
use std::sync::Arc;
use serenity::prelude::*;
use tokio::sync::Mutex;
use crate::DbClientContainer;
use crate::GameStateContainer;
use crate::GameState;
use chrono::{NaiveDateTime,Duration};
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

pub async fn auction(ctx: &Context, advance: bool) -> Option<NaiveDateTime> {
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
    
    let mut outer : Vec<Vec<Bid>> = match day {
        1 => {
            let mut outer= Vec::new();
            let mut bids = Vec::new();
            let rows = arcdb.query("SELECT userid,racename,bid FROM racebids",&[]).await.expect("error loading all racebids");
            for row in rows {
                let userid : i64 = row.get(0);
                let bid : i32 = row.get(2);
                bids.push(Bid {
                        item: row.get(1),
                        user: userid,
                        price:  bid,
                        reserve: 0,
                });

            }
            outer.push(bids);
            outer
        },
        i @2..=3 => {
            let mut outer = Vec::new();
            let mut bids = Vec::new();
            let priority = i-1;
            let rows = arcdb.query("SELECT userid,pathname,bid FROM pathbids WHERE priority = $1",&[&priority]).await.expect("error loading all pathbids");
            for row in rows {
                let path : String = row.get(1);
                bids.push(Bid {
                        item: format!("{}_{}", path,["PRIMARY","SECONDARY"][(priority as usize)-1]),
                        user: row.get(0),
                        price:  row.get(2),
                        reserve: 0,
                });

            }
            outer.push(bids);
            outer
        },
        i @4..=8 => {
            let mut outer = Vec::new();
            for j in 1..=12 {
                let mut bids = Vec::new();
                let day = i as i16;
                let nr = j as i16;
                let rows = arcdb.query("SELECT perkbids.userid,perkbids.perkname, perkbids.bid, perkbids.reserve FROM perkbids INNER JOIN perks ON (perkbids.perkname = perks.name) WHERE  (perks.day = $1) AND (perks.nr = $2)",&[&day,&nr]).await.expect("error loading all perkbids");
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
        },
        _ => unreachable!()
    };
    
    let userrows = arcdb.query("SELECT id, name, points FROM users",&[]).await.expect("muh users");
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



    let mut pathbasket = HashMap::new();
    pathbasket.insert('A',2);
    pathbasket.insert('E',2);
    pathbasket.insert('F',2);
    pathbasket.insert('W',2);
    pathbasket.insert('S',2);
    pathbasket.insert('D',2);
    pathbasket.insert('N',2);
    pathbasket.insert('B',2);
    

    let mut rng = StdRng::from_entropy();

    for mut bids in outer {
        (&mut bids[..]).shuffle(&mut rng);
        let mut wins_this_auction = 0;
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
            
            //minimum bid of 10 for perk auctions
            if day > 3 && winner.price < 10 {
                break;
            }
            //prices of 0 are dead bids
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
            if day > 3 && cost < 10 {
                cost = 10;
            }
            wins_this_auction += 1;
            users.get_mut(&winner.user).unwrap().points -= cost;
            info!(" user: {} won bid for {} with {} points", users.get(&winner.user).unwrap().name, winner.item, cost);            
            arcdb.query("INSERT INTO wins (userid,item,day,cost) VALUES($1,$2,$3,$4)",&[&winner.user,&winner.item,&day,&cost]).await.expect("failed to store winning bid");
            arcdb.query("UPDATE users SET points = $1 WHERE id = $2;",&[&users.get(&winner.user).unwrap().points,&users.get(&winner.user).unwrap().id]).await.expect("failed to store winner's points");
            

            //remove invalid bids
            //first, remove all remaining bids by the same bidder
            bids.retain(|bid|bid.user != winner.user);

            //second, remove bids on items that are no longer available
            
            match (day, wins_this_auction) {
                //for days 1, 4..=8, remove all bids on the same thing
                (1,_) => bids.retain(|bid|bid.item != winner.item),
                (4..=8,_) => bids.retain(|bid|bid.item != winner.item),
                // for primary paths, first 4 winners get uniques
                (2, 1..=4 ) => {
                    bids.retain(|bid|bid.item != winner.item);
                    *pathbasket.get_mut(&first_char(&winner.item)).unwrap() = 0;
                },
                // for secondary paths and last 8 primary paths, there are two winners per path
                (2..=3, _) => {
                    let mut picks_left = pathbasket[&first_char(&winner.item)];
                    picks_left -= 1;
                    *pathbasket.get_mut(&first_char(&winner.item)).unwrap() = picks_left;
                    if picks_left == 0 {    
                        bids.retain(|bid|bid.item != winner.item);
                    }
                },
                _ => unreachable!()
            }
        }
    }
    let mut ret = None;
    if advance {
        let (day, deadline, rate) = match *gamestatewriteguard {
            GameState::Auction{day, deadline, rate} => (day, deadline, rate),
            _ => { error!("called auction on non-auction day");
                return None;
            }      
        };
        let newdeadline =  deadline + Duration::minutes(rate as i64);
        let gamestate = if day == 8 {
            let _rows = &arcdb.query("DELETE FROM gamestate",&[]).await.expect("database failure");
            let _rows = &arcdb.query("INSERT INTO gamestate (phase) VALUES ($1);",&[&-1i16]).await.expect("database failure");      
            GameState::Finished
        } else {
            let newday = day+1;
            let _rows = &arcdb.query("DELETE FROM gamestate",&[]).await.expect("database failure");
            let _rows = &arcdb.query("INSERT INTO gamestate (phase,deadline,rate) VALUES ($1,$2,$3);",&[&newday,&newdeadline,&rate]).await.expect("database failure");

            GameState::Auction{
                day: newday,
                deadline: newdeadline,
                rate: rate,
            }
        };
        *gamestatewriteguard = gamestate;
        
        
        ret = Some(newdeadline);
    }
    ret

}
