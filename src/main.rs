#![allow(dead_code)]
#![allow(unused_variables)]

mod commands;
mod auction;
mod gamestate;

use log::{debug, info};
use std::sync::Arc;
use tokio_postgres::{NoTls};
use serenity::{
    prelude::*,
    async_trait,
    
    framework::standard::{
        StandardFramework,
        macros::{group, hook},
    },
    
    model::{
        channel::{Message},
        gateway::Ready,
        id::ChannelId,
		id::UserId,
    },
};
use chrono::{NaiveDateTime,Local};
use tokio::sync::RwLock;
use commands::*;
use tokio::time::Duration;
use crate::auction::auction;
use gamestate::GameState;
use envconfig::Envconfig;

#[derive(Envconfig)]
pub struct Config {
	#[envconfig(from = "DISCORD_TOKEN")]
	pub token: String,
	#[envconfig(from = "DATABASE_USER")]
	pub dbuser: String,
    #[envconfig(from = "DATABASE_HOST")]
    pub dbhost: String,
    #[envconfig(from = "DATABASE_PW")]
    pub dbpw: String,
}

fn uid_to_u64(id : i64) -> u64{
    id as u64
}

async fn spamperson(ctx: &Context, userid: u64,oldday: i16, points: i32, newstate: GameState){
	let data = ctx.data.read().await;
	let arcdb = data.get::<DbClientContainer>().expect("expected db client in sharemap");
		
	let user = UserId(userid);
	if let Ok(channel) = user.create_dm_channel(&ctx).await{
		let mut s = format!("Auctions for day {} were resolved.\n", oldday);
		
		s.push_str(&get_wins(&arcdb,Some(userid as i64),Some(oldday)).await); 
		
		s.push_str(&match newstate {
			GameState::Finished => "\n Game is finished.\n".to_string(),
			GameState::Auction{day,deadline,rate} => {
				format!("auctions for day {} are open. {}", day, pretty_print_deadline(deadline))
			},
			_ => "".to_string()
		});
		
		s.push_str(&format!("\nYou have {} points remaining.",points));
		
		if let Err(err) = channel.say(&ctx.http,s).await {
			dbg!(err);
		}
	} else {
		dbg!("failed to create channel for {}\n", userid);
	}
}

async fn spamchat(ctx: &Context, oldday: i16, channel: u64, newstate: GameState){
	let data = ctx.data.read().await;
	let arcdb = data.get::<DbClientContainer>().expect("expected db client in sharemap");
	
	let chanid = ChannelId(channel);
	
	let mut s = format!("Auctions for day {} were resolved.\n", oldday);
	s.push_str(&get_wins(&arcdb,None,Some(oldday)).await);
	
	s.push_str(&match newstate {
			GameState::Finished => "\n Game is finished.\n".to_string(),
			GameState::Auction{day,deadline,rate} => {
				format!("auctions for day {} are open. {}", day, pretty_print_deadline(deadline))
			},
			_ => "".to_string()
		});
	
	
	if let Err(err) = chanid.say(&ctx.http,s).await {
		dbg!(err);
	};
}

async fn spam(ctx : Context, newstate: GameState, oldstate: GameState){
    let data = ctx.data.read().await;
    let arcdb = data.get::<DbClientContainer>().expect("expected db client in sharemap");
    
	let (oldday, olddeadline, oldrate) = if let GameState::Auction {day,deadline,rate } = oldstate {
		(day, deadline, rate)
	} else { panic!("auction during non-auction day?") };

	match arcdb.query_opt("SELECT id FROM channel",&[]).await.expect("dberror") {
        Some(row) => { 
            let channel : i64 = row.get(0);
            spamchat(&ctx,oldday,channel as u64,newstate).await;
        },
        None => ()
    }
    for row in arcdb.query("SELECT id,points FROM users",&[]).await.expect("dberror") {
        let uid = uid_to_u64(row.get(0));
        let points : i32 = row.get(1);
        if uid >12 {
            spamperson(&ctx,uid,oldday,points,newstate).await;
        }
    }
}

// repeatedly called
async fn tick(ctx : Context){
    let data = ctx.data.read().await;
    debug!("tick");
    let now = Local::now().naive_local();
	let mut oldday = 0;
    let shouldauction = {
        let gamestatearc = data.get::<GameStateContainer>().expect("expected gamestate in sharemap");
        let gamestatereadguard = (&gamestatearc).read().await;
        match *gamestatereadguard {
            GameState::Auction{day,deadline,rate} => {
                oldday = day;
				deadline < now
				},
            _ => false
        }
    };
    if shouldauction {
        if let (newgamestate, oldgamestate) = auction(&ctx,true).await.expect("auction failed") {
			let ctx2 = ctx.clone();
			tokio::spawn( async move {
				spam(ctx2,newgamestate, oldgamestate).await;
			});
		}
    }

}
async fn ticker(ctx: Context)
{
    debug!("ticker called");
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(120)).await;
            tick(ctx.clone()).await;
        }
    }); //tokio::spawn returns a joinhandle, which is detached on drop
}

struct Handler;
#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {

        if msg.is_own(&ctx.cache) {
			debug!("[{}]: {}",msg.author.name, msg.content);
        } else {
			if msg.is_private() {
				info!("[{}]: {}",msg.author.name, msg.content);
			}
        }
    }
    async fn ready(&self, ctx: Context, ready: Ready) {
        info!("{} is connected!", ready.user.name);
        ticker(ctx).await;
    }
}





#[group]
#[commands(users,help,wins,register,hello2)]
struct General;

#[group]
#[commands(status,bids,bid,unregister)]
struct Auction;

#[group]
#[commands(runauction,setstate,getstate,kick)]
struct Admin;

struct GameStateContainer;

impl TypeMapKey for GameStateContainer {
    type Value = Arc<RwLock<GameState>>;
}

struct DbClientContainer;

impl TypeMapKey for DbClientContainer {
    type Value = Arc<tokio_postgres::Client>;
}

#[hook]
async fn before(ctx: &Context, msg: &Message, command_name: &str) -> bool {

    debug!("[{}]: {} -> {}",msg.author.name, msg.content, command_name);
	if !msg.is_private() 
	{
		info!("[{}]: {}",msg.author.name, msg.content);
	}
    true
}

#[tokio::main]
async fn main() {
    
    env_logger::init();

	let config = Config::init_from_env().unwrap();

	let dbstring = format!("host='{}' user='{}' password='{}'",config.dbhost,config.dbuser,config.dbpw);

	info!("{}",&dbstring);

    let (dbclient, connection) = tokio_postgres::connect(&dbstring, NoTls).await.unwrap();
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("connection error: {}", e);
        }
    });
    let arcdb = Arc::new(dbclient);



    let gamestate = GameState::fromdb(&arcdb).await;
    let rwlarcstate = Arc::new(RwLock::new(gamestate.clone()));
    
    let framework = StandardFramework::new()
        .configure(|c| c
            .with_whitespace(true)
            .prefix("!")
            .delimiters(vec![", ", ","," "]))
        
        .before(before)
        .group(&GENERAL_GROUP)
        .group(&ADMIN_GROUP)
        .group(&AUCTION_GROUP);


    let mut client = Client::builder(config.token, GatewayIntents::DIRECT_MESSAGES | GatewayIntents::GUILD_MESSAGES )
        .event_handler(Handler)
        .framework(framework)
        .await
        .expect("couldn't create the new client!");
    {
        let mut data = client.data.write().await;
        data.insert::<DbClientContainer>(arcdb);
        data.insert::<GameStateContainer>(rwlarcstate);
    }
    if let Err(why) = client.start().await {
        println!("An error occurred while running the client: {:?}", why);
    }

}
