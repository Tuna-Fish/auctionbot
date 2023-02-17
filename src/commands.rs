
use log::{info};
use std::sync::{Arc};
use serenity::{
    prelude::*,
    framework::standard::{
        Args, CommandResult,
        macros::{command},
    },
    
    model::{
        channel::{Message},
		id::ChannelId,
        id::UserId,
    },
};
use crate::DbClientContainer;
use crate::GameStateContainer;
use crate::gamestate::GameState;
use chrono::{NaiveDateTime,Local,Duration};
use crate::auction::auction;
//general

// Discord userids are u64s. Postgres does not natively support that data type. Since we just pass
// them through, we are treating them as i64's.
fn getuid_i64(id: UserId) -> i64 {
        (*id.as_u64()) as i64
}


async fn isadmin(arcdb : &Arc<tokio_postgres::Client>, id: &UserId) -> bool {
    let uid = getuid_i64(*id);
    let mayberow = &arcdb.query_opt("SELECT FROM admin name WHERE id = $1;",&[&uid]).await.expect("database error while checking permissions");
    mayberow.is_some()

}
//returns option(uid (i64))
async fn get_player_uid_and_points(arcdb : &Arc<tokio_postgres::Client>, id: &UserId) -> Option<(i64,i32)> {
    let uid = getuid_i64(*id);
    let mayberow = &arcdb.query_opt("SELECT points FROM discorduser WHERE id = $1;",&[&uid]).await.expect("database error while checking permissions");
    match mayberow {
        Some(row) => {
            let points : i32 = row.get(0);
            Some((uid,points))
        },
        None => None
    }
}

pub async fn get_wins(arcdb : &Arc<tokio_postgres::Client>, userid: Option<i64>, day: Option<i16>) -> String {
    let rows = match (userid, day) {
(None,None)         => arcdb.query("SELECT discorduser.name,win.item,win.day,win.cost FROM win INNER JOIN discorduser ON win.userid = discorduser.id ORDER BY win.day",&[]).await,
(None,Some(d))      => arcdb.query("SELECT discorduser.name,win.item,win.day,win.cost FROM win INNER JOIN discorduser ON win.userid = discorduser.id WHERE win.day = $1 ORDER BY win.day",&[&d]).await,
(Some(uid), None)   => arcdb.query("SELECT discorduser.name,win.item,win.day,win.cost FROM win INNER JOIN discorduser ON win.userid = user.id WHERE win.userid = $1 ORDER BY win.day",&[&uid]).await,
(Some(uid),Some(d)) => arcdb.query("SELECT discorduser.name,win.item,win.day,win.cost FROM win INNER JOIN users ON win.userid = discorduser.id WHERE win.userid = $1 AND win.day = $2 ORDER BY win.day",&[&uid,&d]).await,
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

    let rows = arcdb.query("SELECT bid.itemname,bid.bid,bid.reserve FROM bid INNER JOIN item ON (bid.itemname = item.name) WHERE item.day = $1 AND bid.userid = $2 ORDER BY item.nr",&[&day,&userid]).await.expect("dberror");
    for row in rows {
        let price :i32 = row.get(1);
        let reserve : i32 = row.get(2);
        let perkname : String = row.get(0);
        if price != 0 {
            listing.push_str(&format!("{:>4}|{:>7}|{}\n",&price,&reserve,&perkname));
        }
    }
    let rows = arcdb.query("SELECT points FROM discorduser WHERE discorduser.id = $1",&[&userid]).await.expect("dberror");
    let cash : i32= rows[0].get(0);
    listing.push_str(&format!("points remaining : {}```",cash));
    listing               
}

#[command]
async fn help(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let arg1 = if args.len() == 0 { "all".to_string() } else {
        args.single::<String>().unwrap()
    };
    let response= match &arg1[..] {
        "register" => "adds you to a game, works during registration only",
        "unregister" => "removes you from the game, works during registration only",
        "bids" => "lists what bids you have made today, and how much points you have left.\nusage:\n bids : lists bids\n",
        "wins" => "lists what auction results have been decided.\nusage:\n wins : lists what you have won\n wins <day> : lists what everyone won on a given day\n wins all : lists what everyone has won so far",
        "bid" => "places a bid on an item.\n usage:\n bid <ITEM> <price> <reserve> : places a bid on an item, with a reserve set\n bid <ITEM> <price> : places a bid on an item without reserve",
        "status" => "displays game state and the time until deadline\nusage:\nstatus",
        "users" => "displays all users and their points\nusage:\nusers",
        _ => "commands:\nbids wins bid status users register unregister \ntry !help <command>",
    
    };
    let _ = msg.channel_id.say(&ctx.http, response).await;
    Ok(())
}

#[command]
async fn wins(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let data = ctx.data.read().await;
    let arcdb = data.get::<DbClientContainer>().expect("expected db client in sharemap");
    let userid = getuid_i64(msg.author.id);
    let s = match args.len() {
        0 => get_wins(&arcdb, Some(userid),None).await,
        1 => match &args.single::<String>().unwrap()[..] {
            "help" => "usage:\n to list bids you've won: listbids\nto list bids anyone has won: listbids all\n to list bids anyone won on a given day: listbids <day>\n".to_string(),
            "all" => get_wins(&arcdb, None, None).await,
            arg  => if let Ok(i) = arg.parse::<i16>() {
                get_wins(&arcdb, None, Some(i)).await
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
	
	let bids = if msg.is_private() {
		get_bids(arcdb, userid, day).await
	} else {
		"Maybe don't do this in a public channel?".to_string()
	};
    let _ = msg.channel_id.say(&ctx.http, bids).await;
    Ok(())
}

#[command]
async fn users(ctx: &Context, msg: &Message, _args: Args) -> CommandResult {
    let data = ctx.data.read().await;
    let arcdb = data.get::<DbClientContainer>().expect("expected db client in sharemap");
   
    let rows = arcdb.query("SELECT name,points FROM discorduser",&[]).await.expect("dberror");
    let mut s = String::with_capacity(1000);
    s.push_str("```points|name\n");
    for row in rows {
        let points : i32 = row.get(1);
        let name : String = row.get(0);
        s.push_str(&format!("{:>6}|{}\n",points,name));
    }
    s.push_str("```");

    let _ =msg.channel_id.say(&ctx.http, &s).await;
    Ok(())
}
pub fn pretty_print_deadline(deadline: NaiveDateTime) -> String {
    let time = Local::now().naive_local();
    let duration = deadline - time;
    match duration {
        i if i <= Duration::zero() =>  "deadline has passed".to_string(),
        remaining => {
            let secs = remaining.num_seconds() % 60;
            let mins = remaining.num_minutes() %60;
            let hours = remaining.num_hours();
            let secs_s = if secs == 0 { "".to_string() } else {
                format!("{} seconds ",secs)
            };
            let mins_s = if mins == 0 { "".to_string() } else {
                format!("{} minutes ",mins)
            };
            let hours_s = if hours == 0 { "".to_string() } else {
                format!("{} hours ",hours)
            };
                format!("there are {}{}{}remaining.",hours_s,mins_s,secs_s)
        }
    }
}
#[command]
async fn status(ctx: &Context, msg: &Message, _args: Args) -> CommandResult {
    let data = ctx.data.read().await;
    let arcdb = data.get::<DbClientContainer>().expect("expected db client in sharemap");
   
    let gamestatearc = data.get::<GameStateContainer>().expect("expected gamestate in sharemap");
    let gamestatereadguard = (&gamestatearc).read().await;
    let s : String = match *gamestatereadguard {
        GameState::Closed => "game is closed".to_string(),
        GameState::Registration => "game is accepting registrations".to_string(),
        GameState::Auction{day,deadline,rate} => {
            let time_remaining = pretty_print_deadline(deadline);
            format!("Auctions for day {} are open, and {}",day,time_remaining)
        },
        GameState::Finished => "finished".to_string()
        
    };
    let _ =msg.channel_id.say(&ctx.http, s).await;
    Ok(())
}

#[command]
async fn bid(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let data = ctx.data.read().await;

    let gamestatearc = data.get::<GameStateContainer>().expect("expected gamestate in sharemap");
    let gamestatereadguard = (&gamestatearc).read().await;
    let (day) = match *gamestatereadguard {
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

    let item : String   = args.single::<String>().unwrap().to_ascii_uppercase();
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
	
    let itemopt = arcdb.query_opt("SELECT day FROM item WHERE (name = $1)",&[&item]).await.expect("db failure");

    match itemopt {
        Some(row) => {
            let pday : i16 = row.get(0);
            if pday  != day {
                let _ = msg.channel_id.say(&ctx.http, "The item you specified is not up for auction today").await;
                return Ok(());
            }
            arcdb.query_opt(
                "INSERT INTO bid (userid,itemname,bid,reserve) VALUES ($1,$2,$3,$4) ON CONFLICT ON CONSTRAINT pepk DO UPDATE SET bid=EXCLUDED.bid, reserve=EXCLUDED.reserve",
                &[&playerid,&item,&price,&reserve])
                .await.expect("failed to insert bid");
        },
        None => {
            let _ = msg.channel_id.say(&ctx.http, "The item you specified was not found").await;
            return Ok(());
        }
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

    let rows = &arcdb.query("INSERT INTO discorduser (id,name) VALUES ($1,$2);",&[&authorid,authorname]).await;
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

    let rows = &arcdb.query_opt("DELETE FROM discorduser WHERE id = $1 RETURNING *;",&[&authorid]).await;
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
    
    auction(ctx,false).await;
    
    Ok(())
}

#[command]
async fn kick(_ctx: &Context, _msg: &Message, _args: Args) -> CommandResult {
    Ok(())
}


#[command]
async fn setstate(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
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
        i => {
                let rate : i32 = args.single::<i32>().unwrap(); 
                args.quoted();
                let deadlinestring = args.single::<String>().unwrap();
                let deadline : NaiveDateTime = NaiveDateTime::parse_from_str(&deadlinestring,"%Y-%m-%d %H:%M").expect("date parsing failure");
                let _rows = &arcdb.query("DELETE FROM gamestate",&[]).await.expect("database failure");
                let _rows = &arcdb.query("INSERT INTO gamestate (phase,deadline,rate) VALUES ($1,$2,$3);",&[&i,&deadline,&rate]).await.expect("database failure");

                GameState::Auction{day : i, deadline, rate}
            },
    };
	info!("applied changes");
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

    let row = &arcdb.query_one("SELECT channel from gamestate",&[]).await.expect("database failure");
    let dbvalue : i64 = row.get(0);
	let adjusted : u64 = dbvalue as u64;
    
	let channel_id = ChannelId::from(adjusted);
	
    let message = args.message();

    if let Err(why) = channel_id.say(&ctx.http, "test").await {
        println!("error: {:?}", why);
    }

    arcdb.query("UPDATE test SET foo = $1 WHERE id = '1'",&[&message]).await.expect("database update failure");

    Ok(())
}
