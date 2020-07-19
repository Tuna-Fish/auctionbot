
use log::{debug, error, info};
use std::sync::Arc;
use tokio_postgres::{NoTls, Error};
use serenity::{
    prelude::*,
    async_trait,
    client::bridge::gateway::GatewayIntents,
    
    framework::standard::{
        Args, CheckResult, CommandOptions, CommandResult, CommandGroup,
        DispatchError, HelpOptions, help_commands, StandardFramework,
        macros::{command, group, help, check, hook},
    },
    http::Http,
    
    model::{
        channel::{Message, Channel},
        gateway::Ready,
        id::UserId,
        permissions::Permissions
    },
};
use tokio::sync::Mutex;
use crate::DbClientContainer;
use crate::GameStateContainer;
use crate::GameState;
use chrono::NaiveDateTime;
use crate::auction::auction;
//general

// Discord userids are u64s. Postgres does not natively support that data type. Since we just pass
// them through, we are treating them as i64's.
fn getuid_i64(id: UserId) -> i64 {
        (*id.as_u64()) as i64
}


async fn isadmin(arcdb : &Arc<tokio_postgres::Client>, id: &UserId) -> bool {
    let uid = getuid_i64(*id);
    let mayberow = &arcdb.query_opt("SELECT FROM admins name WHERE id = $1;",&[&uid]).await.expect("database error while checking permissions");
    mayberow.is_some()

}
//returns option(uid (i64))
async fn get_player_uid_and_points(arcdb : &Arc<tokio_postgres::Client>, id: &UserId) -> Option<(i64,i32)> {
    let uid = getuid_i64(*id);
    let mayberow = &arcdb.query_opt("SELECT points FROM users WHERE id = $1;",&[&uid]).await.expect("database error while checking permissions");
    match mayberow {
        Some(row) => {
            let points : i32 = row.get(0);
            Some((uid,points))
        },
        None => None
    }
}

async fn get_wins(arcdb : &Arc<tokio_postgres::Client>, userid: Option<i64>, day: Option<i16>) -> String {
    let rows = match (userid, day) {
(None,None)         => arcdb.query("SELECT users.name,wins.item,wins.day,wins.cost FROM wins INNER JOIN users ON wins.userid = users.id ORDER BY wins.day",&[]).await,
(None,Some(d))      => arcdb.query("SELECT users.name,wins.item,wins.day,wins.cost FROM wins INNER JOIN users ON wins.userid = users.id WHERE wins.day = $1 ORDER BY wins.day",&[&d]).await,
(Some(uid), None)   => arcdb.query("SELECT users.name,wins.item,wins.day,wins.cost FROM wins INNER JOIN users ON wins.userid = users.id WHERE wins.userid = $1 ORDER BY wins.day",&[&uid]).await,
(Some(uid),Some(d)) => arcdb.query("SELECT users.name,wins.item,wins.day,wins.cost FROM wins INNER JOIN users ON wins.userid = users.id WHERE wins.userid = $1 AND wins.day = $2 ORDER BY WINS.DAY",&[&uid,&d]).await,
    }.expect("db error fetching wins");
    let mut s = String::from("```day|cost|             item             |winner\n");
    s.reserve(70*rows.len());
    for row in rows {
        let user_name   : String = row.get(0);
        let item_name   : String = row.get(1);
        let costint     : i32    = row.get(3);
        let cost        : String = costint.to_string();
        let dayint      : i16    = row.get(2);
        let day         : String = dayint.to_string();

       s.push_str(&format!("{:>3}|{:>4}|{:>30}|{}\n",day,cost,item_name,user_name));
    }
    s.push_str("```");
    s
}

async fn get_bids(arcdb: &Arc<tokio_postgres::Client>, userid: i64, day: i16) -> String {
    let mut listing = String::from("```cost|reserve|item\n");
    match day {
        1 => {
            let rows = arcdb.query("SELECT racename, bid FROM racebids WHERE userid = $1",&[&userid]).await.expect("dberror");
            for row in rows {
                let price : i32 = row.get(1);
                let item : String = row.get(0);
                listing.push_str(&format!("{:>4}|{:>7}|{}\n",&price,&0,&item));
            }
        },
        i@2..=3 => {
            let priority :i16 = i-1;
            let rows = arcdb.query("SELECT pathname,bid FROM pathbids WHERE userid = $1 AND priority = $2",&[&userid,&priority]).await.expect("dberror");
            for row in rows {
                let price : i32 = row.get(1);
                let pathname : String = row.get(0);
                if price != 0 {
                    listing.push_str(&format!("{:>4}|{:>7}|{}_{}\n",&price,&0,&pathname,&["PRIMARY","SECONDARY"][(priority as usize)-1]));
                }
            }
        },
        i => {
            let rows = arcdb.query("SELECT perkbids.perkname,perkbids.bid,perkbids.reserve FROM perkbids INNER JOIN perks ON (perkbids.perkname = perks.name) WHERE perks.day = $1 AND perkbids.userid = $2",&[&day,&userid]).await.expect("dberror");
            for row in rows {
                let price :i32 = row.get(1);
                let reserve : i32 = row.get(2);
                let perkname : String = row.get(0);
                listing.push_str(&format!("{:>4}|{:>7}|{}\n",&price,&reserve,&perkname));
            }
        },

    }
    listing.push_str("```");
    listing               
}

async fn get_items(arcdb: &Arc<tokio_postgres::Client>, userid: i64, day: i16) -> String {
    String::new()
}


#[command]
async fn help(_ctx: &Context, _msg: &Message, _args: Args) -> CommandResult {
    Ok(())
}

#[command]
async fn listitems(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    Ok(())
}

#[command]
async fn wins(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let data = ctx.data.read().await;
    let arcdb = data.get::<DbClientContainer>().expect("expected db client in sharemap");
    let userid = getuid_i64(msg.author.id);
    let s = match args.len() {
        0 => wins(&arcdb, Some(userid),None).await,
        1 => match &args.single::<String>().unwrap()[..] {
            "help" => "usage:\n to list bids you've won: listbids\nto list bids anyone has won: listbids all\n to list bids anyone won on a given day: listbids <day>\n".to_string(),
            "all" => wins(&arcdb, None, None).await,
            arg  => if let Ok(i) = arg.parse::<i16>() {
                wins(&arcdb, None, Some(i)).await
            } else {"unrecognized parameter".to_string()},
        }
        _ => "too many parameters".to_string()
    };

    let _ = msg.channel_id.say(&ctx.http, &s).await;
    Ok(())
}
#[command]
async fn bids(ctx: &Context, msg: &Message, mut _args: Args) -> CommandResult {
    let data = ctx.data.read().await;
    let arcdb = data.get::<DbClientContainer>().expect("expected db client in sharemap");
    let userid = getuid_i64(msg.author.id);

    let gamestatearc = data.get::<GameStateContainer>().expect("expected gamestate in sharemap");
    let gamestatereadguard = (&gamestatearc).read().await;
    let day = match *gamestatereadguard {
        GameState::Auction{day, ..} => day,
        _ => {
            let _ =msg.channel_id.say(&ctx.http, "Bidding is not open").await;
            return Ok(())
        }
    };

    let bids = get_bids(arcdb, userid, day).await;
    let _ = msg.channel_id.say(&ctx.http, bids).await;
    Ok(())
}

#[command]
async fn status(_ctx: &Context, _msg: &Message, _args: Args) -> CommandResult {
    Ok(())
}
#[command]
async fn bid(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let data = ctx.data.read().await;

    let gamestatearc = data.get::<GameStateContainer>().expect("expected gamestate in sharemap");
    let gamestatereadguard = (&gamestatearc).read().await;
    let day = match *gamestatereadguard {
        GameState::Auction{day, ..} => day,
        _ => {
            let _ =msg.channel_id.say(&ctx.http, "Bidding is not open").await;
            return Ok(())
        }
    };
    let arcdb = data.get::<DbClientContainer>().expect("expected db client in sharemap");

    let (playerid, points) = match get_player_uid_and_points(&arcdb,&msg.author.id).await {
            Some((uid,points)) => (uid,points),
            None => {
                let _ = msg.channel_id.say(&ctx.http, "You are not registered to bid").await;
                return Ok(());
            }
    };
    
    if args.len() > 3 || args.len() < 2 {
            let _ = msg.channel_id.say(&ctx.http, "Your bid must have a target, price and may optionally have reserve, as in\n bid MAGEPRIESTS 200 400").await;
            return Ok(());
    }

    let item : String   = args.single::<String>().unwrap();
    if item.len() > 30 {
            let _ = msg.channel_id.say(&ctx.http, "the item you selected to bid for is not valid (too long)").await;
            return Ok(());
    }
    let price : i32      = match args.single::<i32>() {
        Ok(i) => i,
        _ => {
            let _ = msg.channel_id.say(&ctx.http, "the second argument to a bid must be the price you are willing to pay.").await;
            return Ok(());
        }
    };
    
    let reserve = match (args.len(),args.single::<i32>()) {
            (3, Ok(i)) => i,
            (2, _) => 0,
            _ => {
                let _ = msg.channel_id.say(&ctx.http, "the third argument must be the amount you wish to reserve").await;
                return Ok(());
            }
        
    };

    if price < 0 { 
                let _ = msg.channel_id.say(&ctx.http, "your bid may not be negative").await;
                return Ok(());
    }

    if reserve < 0 { 
                let _ = msg.channel_id.say(&ctx.http, "your reserve may not be negative").await;
                return Ok(());
    }

    if reserve + price > points {
        let _ = msg.channel_id.say(&ctx.http, "the sum of your bid and reserve may not be more than your remaining points").await;
        return Ok(())
    }

    match day {
        1 => { // race bid day
            let raceopt = arcdb.query_opt("SELECT name FROM races WHERE name = $1",&[&item]).await.expect("db failure");
            match raceopt {
                Some(_) => {
                    arcdb.query_opt(
    "INSERT INTO racebids (userid,racename,bid) VALUES ($1,$2,$3) ON CONFLICT ON CONSTRAINT rapk DO UPDATE SET bid=EXCLUDED.bid",
                        &[&playerid,&item,&price])
                        .await.expect("failed to insert bid");
                },
                None => { 
                    let _ = msg.channel_id.say(&ctx.http, "The race you specified was not found").await;
                    return Ok(());
                }
            }
        },
        i @ 2..=3 => { //magic path day 
            let pathopt = arcdb.query_opt("SELECT name FROM paths WHERE name = $1",&[&item]).await.expect("db failure");
            match pathopt { 
                Some(_) => {
                    let priority = i-1;
                    arcdb.query_opt(
        "INSERT INTO pathbids (userid,pathname,bid,priority) VALUES ($1,$2,$3,$4) ON CONFLICT ON CONSTRAINT papk DO UPDATE SET bid = EXCLUDED.bid",
                        &[&playerid,&item,&price,&priority])
                        .await.expect("failed to insert bid");
                },
                None => { 
                    let _ = msg.channel_id.say(&ctx.http, "The path you specified was not found").await;
                    return Ok(());
                }
            }
        },
        i @4..=8 => { //perk day
            let perkopt = arcdb.query_opt("SELECT day FROM perks WHERE (name = $1)",&[&item]).await.expect("db failure");
            match perkopt {
                Some(row) => {
                    let pday : i32 = row.get(0);
                    if pday as i16 != i {
                        let _ = msg.channel_id.say(&ctx.http, "The perk you specified is not up for auction today").await;
                        return Ok(());
                    }
                    arcdb.query_opt(
    "INSERT INTO perkbids (userid,perkname,bid,reserve) VALUES ($1,$2,$3,$4) ON CONFLICT ON CONSTRAINT pepk DO UPDATE SET bid=EXCLUDED.bid, reserve=EXCLUDED.reserve",
                        &[&playerid,&item,&price,&reserve])
                        .await.expect("failed to insert bid");
                },
                None => { 
                    let _ = msg.channel_id.say(&ctx.http, "The perk you specified was not found").await;
                    return Ok(());
                }
            }
        }
        _ => unreachable!()
    }
    if price == 0 { 
        let _ = msg.channel_id.say(&ctx.http, "successfully removed bid").await;
    } else {
        let _ = msg.channel_id.say(&ctx.http, "successfully inserted bid").await;
    }
    Ok(())
}

#[command]
async fn register(ctx: &Context, msg: &Message, _args: Args) -> CommandResult {
    let data = ctx.data.read().await;

    let gamestatearc = data.get::<GameStateContainer>().expect("expected gamestate in sharemap");
    let gamestatereadguard = (&gamestatearc).read().await;
    match *gamestatereadguard {
        GameState::Registration => (),
        _ => {
            let _ = msg.channel_id.say(&ctx.http, "Registration is closed!").await;
            return Ok(())
        }
    }

    let arcdb = data.get::<DbClientContainer>().expect("expected db client in sharemap");
    let authorid = getuid_i64(msg.author.id);
    let authorname = &msg.author.name;  

    let rows = &arcdb.query("INSERT INTO users (id,name) VALUES ($1,$2);",&[&authorid,authorname]).await;
    let _res = match rows {
        Ok(_) =>    msg.channel_id.say(&ctx.http, "Successfully registered!").await,
        Err(_) =>   msg.channel_id.say(&ctx.http, "Failed to register. Are you already registered?").await
        
    };

    Ok(())
}

//auction

#[command]
async fn unregister(ctx: &Context, msg: &Message, _args: Args) -> CommandResult {
    
    let data = ctx.data.read().await;

    let gamestatearc = data.get::<GameStateContainer>().expect("expected gamestate in sharemap");
    let gamestatereadguard = (&gamestatearc).read().await;
    match *gamestatereadguard {
        GameState::Registration => (),
        _ => {
            let _ = msg.channel_id.say(&ctx.http, "Registration is closed!").await;
            return Ok(())
        }
    }


    let arcdb = data.get::<DbClientContainer>().expect("expected db client in sharemap");
    let authorid = getuid_i64(msg.author.id);

    let rows = &arcdb.query_opt("DELETE FROM users WHERE id = $1 RETURNING *;",&[&authorid]).await;
    match rows {
        Ok(None)  => {
            let _ = msg.channel_id.say(&ctx.http, "Failed to find you. Were you even registered?").await;
            ()
        }

        Err(_) =>    {
            let _ = msg.channel_id.say(&ctx.http, "Database error!").await;
            ()
        },
        Ok(Some(_)) =>   {
            let _ = msg.channel_id.say(&ctx.http, "Successfully unregistered!").await;
            ()
        }
        
    };

    Ok(())
}

//admin

#[command]
async fn runauction(ctx: &Context, msg: &Message, _args: Args) -> CommandResult {
    let data = ctx.data.read().await;

    let arcdb = data.get::<DbClientContainer>().expect("expected db client in sharemap");
    
    if !isadmin(&arcdb, &msg.author.id).await{
            let _ = msg.channel_id.say(&ctx.http, "You are not in admin list").await;
            return Ok(());
    }
    
    auction(ctx).await;
    
    Ok(())
}

#[command]
async fn kick(_ctx: &Context, _msg: &Message, _args: Args) -> CommandResult {
    Ok(())
}


#[command]
async fn setstate(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    info!("{:?}",&args);
    
    let data = ctx.data.read().await;

    let arcdb = data.get::<DbClientContainer>().expect("expected db client in sharemap");
    
    if !isadmin(&arcdb, &msg.author.id).await{
            let _ = msg.channel_id.say(&ctx.http, "You are not in admin list").await;
            return Ok(());
    }

    let gamestatearc = data.get::<GameStateContainer>().expect("expected gamestate in sharemap");
    let mut gamestatewriteguard = (&gamestatearc).write().await;
    

    let state = args.single::<i16>().unwrap();

     *gamestatewriteguard = match state {
        -2 => {
                let _rows = &arcdb.query("DELETE FROM gamestate",&[]).await.expect("database failure");
                GameState::Closed
        },
        i@ -1 => {
                let _rows = &arcdb.query("DELETE FROM gamestate",&[]).await.expect("database failure");
                let _rows = &arcdb.query("INSERT INTO gamestate (phase) VALUES ($1);",&[&i]).await.expect("database failure");
                GameState::Finished
        },
        i@ 0 => {
                
                let _rows = &arcdb.query("DELETE FROM gamestate",&[]).await.expect("database failure");
                let _rows = &arcdb.query("INSERT INTO gamestate (phase) VALUES ($1);",&[&i]).await.expect("database failure");
                GameState::Registration
            },
        i@ 1..=8 => {
                let rate : i32 = args.single::<i32>().unwrap(); 
                args.quoted();
                let deadlinestring = args.single::<String>().unwrap();
                let deadline : NaiveDateTime = NaiveDateTime::parse_from_str(&deadlinestring,"%Y-%m-%d %H:%M").expect("date parsing failure");
                let _rows = &arcdb.query("DELETE FROM gamestate",&[]).await.expect("database failure");
                let _rows = &arcdb.query("INSERT INTO gamestate (phase,deadline,rate) VALUES ($1,$2,$3);",&[&i,&deadline,&rate]).await.expect("database failure");

                GameState::Auction{day : i, deadline: deadline, rate: rate}
            },
        _ => { unreachable!(); }
    };
    Ok(())
}


#[command]
async fn getstate(ctx: &Context, msg: &Message, _args: Args) -> CommandResult { 
    let data = ctx.data.read().await;

    let gamestatearc = data.get::<GameStateContainer>().expect("expected gamestate in sharemap");
    let gamestatereadguard = (&gamestatearc).read().await;
    let _ = match *gamestatereadguard {
        GameState::Registration =>  msg.channel_id.say(&ctx.http, "Game is in registration!").await,
        GameState::Closed => msg.channel_id.say(&ctx.http, "Game is closed!").await,
        GameState::Finished => msg.channel_id.say(&ctx.http, "Game is finished!").await,
        GameState::Auction {day,deadline,rate} => msg.channel_id.say(&ctx.http, format!("Auctions are open. It is day: {}, current deadline is {}, and rate is {}",day,deadline,rate)).await
    };
    
    Ok(())
}

#[command]
async fn hello2(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    
    let data = ctx.data.read().await;
    let arcdb = data.get::<DbClientContainer>().expect("expected db client in sharemap");

    let rows = &arcdb.query("SELECT * FROM test",&[]).await.expect("database failure");
    let dbvalue : &str = rows[0].get(1);
    
    let message = args.message();

    if let Err(why) = msg.channel_id.say(&ctx.http, dbvalue).await {
        println!("error: {:?}", why);
    }

    &arcdb.query("UPDATE test SET foo = $1 WHERE id = '1'",&[&message]).await.expect("database update failure");

    Ok(())
}
