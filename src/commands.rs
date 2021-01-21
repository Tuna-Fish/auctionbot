
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
use crate::gamestate::get_auctiontype_for_day;
use crate::gamestate::AuctionType;
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
            let rows = arcdb.query("SELECT racename, bid FROM racebids WHERE userid = $1 ORDER BY bid DESC",&[&userid]).await.expect("dberror");
            for row in rows {
                let price : i32 = row.get(1);
                let item : String = row.get(0);
                if price != 0 {
                    listing.push_str(&format!("{:>4}|{:>7}|{}\n",&price,&0,&item));
                }
            }
        },
/*        i@2..=3 => {
            let priority :i16 = i-1;
            let rows = arcdb.query("SELECT pathname,bid FROM pathbids WHERE userid = $1 AND priority = $2 ORDER BY bid DESC",&[&userid,&priority]).await.expect("dberror");
            for row in rows {
                let price : i32 = row.get(1);
                let pathname : String = row.get(0);
                if price != 0 {
                    listing.push_str(&format!("{:>4}|{:>7}|{}_{}\n",&price,&0,&pathname,&["PRIMARY","SECONDARY"][(priority as usize)-1]));
                }
            }
        },*/
        i => {
            let rows = arcdb.query("SELECT perkbids.perkname,perkbids.bid,perkbids.reserve FROM perkbids INNER JOIN perks ON (perkbids.perkname = perks.name) WHERE perks.day = $1 AND perkbids.userid = $2 ORDER BY perks.nr",&[&day,&userid]).await.expect("dberror");
            for row in rows {
                let price :i32 = row.get(1);
                let reserve : i32 = row.get(2);
                let perkname : String = row.get(0);
                if price != 0 {
                    listing.push_str(&format!("{:>4}|{:>7}|{}\n",&price,&reserve,&perkname));
                }
            }
        },

    }
    let rows = arcdb.query("SELECT points FROM users WHERE users.id = $1",&[&userid]).await.expect("dberror");
    let cash : i32= rows[0].get(0);
    listing.push_str(&format!("points remaining : {}```",cash));
    listing               
}

async fn get_items(arcdb: &Arc<tokio_postgres::Client>, day: Option<i16>) -> String {
    let mut listing = String::new();
    if day == None {
        listing.reserve(10000);
    } else {
    listing.reserve(2000);
    }
    //race items
    if day == Some(1) {
        /*
		listing.push_str("```          name          | description \n");
        let rows = arcdb.query("SELECT name,descr FROM races",&[]).await.expect("dberror");
        for row in rows {
            let name : String = row.get(0);
            let descr: String = row.get(1);
            listing.push_str(&format!("{:>12}|{}\n",&name,&descr));
        }
        listing.push_str("```\n\n");
		*/
		listing.push_str("https://rentry.co/natgen2nations\n");
    }
/*    //paths
    if day == Some(2) || day == Some(3) || day == None {
        listing.push_str("```name|longname\n");
        let rows = arcdb.query("SELECT name,longname FROM paths",&[]).await.expect("dberror");
        for row in rows {
            let name : String = row.get(0);
            let descr: String = row.get(1);
            listing.push_str(&format!("{:>3} |{}\n",&name,&descr));
        }
        listing.push_str("```\n\n");
    }*/
    //perks if you ask for all of them
    if day == None {
        /*listing.push_str("```day|             name             |longname\n");
        let rows = arcdb.query("SELECT day,name,descr FROM perks ORDER BY (day,nr)",&[]).await.expect("dberror");
        for row in rows {
            let name : String = row.get(1);
            let descr: String = row.get(2);
            let day  : i16 = row.get(0);
            listing.push_str(&format!("{:>3}|{:>30}|{}\n",&day,&name,&descr));
        }
        listing.push_str("```\n\n");
		*/
		listing.push_str("https://rentry.co/natgenauction2perksbyday");
    }

    let d = day.unwrap_or(0);
    if d > 1 {
		/*
        listing.push_str("```day|             name             |longname\n");
        let rows = arcdb.query("SELECT day,name,descr FROM perks WHERE day = $1 ORDER BY (day,nr)",&[&d]).await.expect("dberror");
        for row in rows {
            let name : String = row.get(1);
            let descr: String = row.get(2);
            let day  : i16 = row.get(0);
            listing.push_str(&format!("{:>3}|{:>30}|{}\n",&d,&name,&descr));
        }
        listing.push_str("```\n\n");
    } else if d == 0 { 
        listing.push_str("```day|             name             |longname\n");
        let rows = arcdb.query("SELECT day,name,descr FROM perks ORDER BY (day,nr)",&[]).await.expect("dberror");
        for row in rows {
            let name : String = row.get(1);
            let descr: String = row.get(2);
            let day  : i16 = row.get(0);
            listing.push_str(&format!("{:>3}|{:>30}|{}\n",&d,&name,&descr));
        }
        listing.push_str("```\n\n");
		*/
		let s = format!("https://rentry.co/natgenauction2perksbyday#day-{}", &d);
		listing.push_str(&s);
    }
    listing
}


#[command]
async fn help(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let arg1 = if args.len() == 0 { "all".to_string() } else {
        args.single::<String>().unwrap()
    };
    let response= match &arg1[..] {
        "minorpaths" => "sets or views your minor path choices, usage:\n minorpaths : shows what you have chosen\n minorpaths <path1> <path2> : selects paths",
        "register" => "adds you to a game, works during registration only",
        "unregister" => "removes you from the game, works during registration only",
        "items" => "lists things available for auction.\nUsage:\n items :lists what items are available today\n items <day> : list things available on a given day\n",
        "bids" => "lists what bids you have made today, and how much points you have left.\nusage:\n bids : lists bids\n",
        "wins" => "lists what auction results have been decided.\nusage:\n wins : lists what you have won\n wins <day> : lists what everyone won on a given day\n wins all : lists what everyone has won so far",
        "bid" => "places a bid on an item.\n usage:\n bid <ITEM> <price> <reserve> : places a bid on an item, with a reserve set\n bid <ITEM> <price> : places a bid on an item without reserve",
        "status" => "displays game state and the time until deadline\nusage:\nstatus",
        "users" => "displays all users and their points\nusage:\nusers",
        _ => "commands:\nitems bids wins bid status users register unregister minorpaths\ntry help <command>",
    
    };
    let _ = msg.channel_id.say(&ctx.http, response).await;
    Ok(())
}

#[command]
async fn items(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let data = ctx.data.read().await;
    let arcdb = data.get::<DbClientContainer>().expect("expected db client in sharemap");
   
    let gamestatearc = data.get::<GameStateContainer>().expect("expected gamestate in sharemap");
    let gamestatereadguard = (&gamestatearc).read().await;
    let mut day : Option<i16> = match *gamestatereadguard {
        GameState::Auction{day, ..} => Some(day),
        _ => None
    };

    if day == None {
        day = Some(1);
    }
    let s = match args.len() {
        0 => get_items(&arcdb, day).await,
        1 => match &args.single::<String>().unwrap()[..] {
            "help" => "usage:\n to list items available today: items\n to list items on a given day: items <day>\n".to_string(),
            "all" => get_items(&arcdb, None).await,
            arg  => if let Ok(i) = arg.parse::<i16>() {
                get_items(&arcdb, Some(i)).await
            } else {"unrecognized parameter".to_string()},
        }
        _ => "too many parameters".to_string()
    };

    let _x = msg.channel_id.say(&ctx.http, &s).await;
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

    let bids = get_bids(arcdb, userid, day).await;
    let _ = msg.channel_id.say(&ctx.http, bids).await;
    Ok(())
}

#[command]
async fn users(ctx: &Context, msg: &Message, _args: Args) -> CommandResult {
    let data = ctx.data.read().await;
    let arcdb = data.get::<DbClientContainer>().expect("expected db client in sharemap");
   
    let rows = arcdb.query("SELECT name,points FROM users",&[]).await.expect("dberror");
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
	dbg!("in status");
    let data = ctx.data.read().await;
    let arcdb = data.get::<DbClientContainer>().expect("expected db client in sharemap");
   
    let gamestatearc = data.get::<GameStateContainer>().expect("expected gamestate in sharemap");
    let gamestatereadguard = (&gamestatearc).read().await;
    let s : String = match *gamestatereadguard {
        GameState::Closed => "game is closed".to_string(),
        GameState::Registration => "game is accepting registrations".to_string(),
        GameState::Auction{day,auctiontype,deadline,rate} => {
            let time_remaining = pretty_print_deadline(deadline);
            format!("Auctions for day {} are open, and {}",day,time_remaining)
        },
        GameState::Finished => "finished".to_string()
        
    };
    let _ =msg.channel_id.say(&ctx.http, s).await;
    Ok(())
}

#[command]
async fn minorpaths(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
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
    
    if !( args.len() == 0 || args.len() == 2) {
            let _ = msg.channel_id.say(&ctx.http, "You must either select two minor paths, or use without arguments to show paths you chose.").await;
            return Ok(());
    }

    if args.len() == 0 {
        match arcdb.query_opt("SELECT path3,path4 FROM minorpaths WHERE userid = $1",&[&playerid]).await.expect("dberror") {
            None => {
                let _ = msg.channel_id.say(&ctx.http, "you have not yet selected minor paths").await;
                return Ok(());
            },
            Some(row) => {
                let minor3: String = row.get(0);
                let minor4: String = row.get(1);
                let _ = msg.channel_id.say(&ctx.http, format!("Your minor paths are {} and {}", &minor3, &minor4)).await;
                return Ok(());
            }
        }
    }
    let arg1: String = args.single::<String>().unwrap().to_ascii_uppercase();
    let arg2: String = args.single::<String>().unwrap().to_ascii_uppercase();
    let paths = "AEFWSNDB";


    if arg1.len() != 1 ||  !paths.contains(&arg1) {
        let _ = msg.channel_id.say(&ctx.http, "did not recognize first argument").await;
        return Ok(());
    }

    if arg2.len() != 1 || !paths.contains(&arg2) {
        let _ = msg.channel_id.say(&ctx.http, "did not recognize second argument").await;
        return Ok(());
    }
   
    arcdb.query_opt(
        "INSERT INTO minorpaths (userid,path3,path4) VALUES ($1,$2,$3) ON CONFLICT (userid) DO UPDATE SET path3=EXCLUDED.path3, path4 = EXCLUDED.path4",
        &[&playerid,&arg1,&arg2]
    ).await.expect("failed to insert bid");
    
    let _ = msg.channel_id.say(&ctx.http, "successfully updated minor paths").await;
    return Ok(());
}



#[command]
async fn bid(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let data = ctx.data.read().await;

    let gamestatearc = data.get::<GameStateContainer>().expect("expected gamestate in sharemap");
    let gamestatereadguard = (&gamestatearc).read().await;
    let (day, auctiontype) = match *gamestatereadguard {
        GameState::Auction{day, auctiontype, ..} => (day, auctiontype),
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
	
    match auctiontype {
        AuctionType::Race => { // race bid day
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
        }, /*
        AuctionType::PrimaryPath | AuctionType::SecondaryPath => { //magic path day 
            let pathopt = arcdb.query_opt("SELECT name FROM paths WHERE name = $1",&[&item]).await.expect("db failure");
            match pathopt { 
                Some(_) => {
                    if (day == 3) {
                        let primarystring = format!("{}_PRIMARY",&item);
                        match arcdb.query_opt("SELECT cost FROM wins WHERE userid = $1 AND item = $2",&[&playerid,&primarystring]).await.expect("db failure") {
                            None => (),
                            Some(_) => {
                                let _ = msg.channel_id.say(&ctx.http, "You cannot pick the same secondary as your primary").await;
                                return Ok(());
                            }
                        }
                    }
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
        }, */
        AuctionType::Perk(..) => { //perk day
            let perkopt = arcdb.query_opt("SELECT day FROM perks WHERE (name = $1)",&[&item]).await.expect("db failure");
            if price < 10 && price != 0 {
                let _ = msg.channel_id.say(&ctx.http, "The mimimum bid on Perk days is 10.").await;
                return Ok(());
            }
            match perkopt {
                Some(row) => {
                    let pday : i16 = row.get(0);
                    if pday  != day {
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
        },
		_ => { unreachable!() }
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

	let maybeauctiontype = get_auctiontype_for_day(&arcdb,state).await;

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
				let auctiontype = maybeauctiontype.expect("set day to one without an auction");
                let rate : i32 = args.single::<i32>().unwrap(); 
                args.quoted();
                let deadlinestring = args.single::<String>().unwrap();
                let deadline : NaiveDateTime = NaiveDateTime::parse_from_str(&deadlinestring,"%Y-%m-%d %H:%M").expect("date parsing failure");
                let _rows = &arcdb.query("DELETE FROM gamestate",&[]).await.expect("database failure");
                let _rows = &arcdb.query("INSERT INTO gamestate (phase,deadline,rate) VALUES ($1,$2,$3);",&[&i,&deadline,&rate]).await.expect("database failure");

                GameState::Auction{day : i, auctiontype, deadline, rate}
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
        GameState::Auction {day,auctiontype,deadline,rate} => msg.channel_id.say(&ctx.http, format!("Auctions are open. It is day: {}, current deadline is {}, and rate is {}",day,deadline,rate)).await
    };
    
    Ok(())
}

#[command]
async fn hello2(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    
    let data = ctx.data.read().await;
    let arcdb = data.get::<DbClientContainer>().expect("expected db client in sharemap");
	
    let row = &arcdb.query_one("SELECT id FROM channel",&[]).await.expect("database failure");
    let dbvalue : i64 = row.get(0);
	let adjusted : u64 = dbvalue as u64;
    
	let channel_id = ChannelId::from(adjusted);
	
    let message = args.message();

    if let Err(why) = channel_id.say(&ctx.http, "test").await {
        println!("error: {:?}", why);
    }

    &arcdb.query("UPDATE test SET foo = $1 WHERE id = '1'",&[&message]).await.expect("database update failure");

    Ok(())
}
