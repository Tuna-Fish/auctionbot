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
    client::bridge::gateway::GatewayIntents,
    
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
	pub dbuser: String
}

fn uid_to_u64(id : i64) -> u64{
    id as u64
}

async fn spamperson(ctx: &Context, userid: u64,newday: i16, deadline: &str,points: i32){
	let userid = UserId(userid);
	if let Ok(channel) = userid.create_dm_channel(&ctx).await{
		
		
		
		if let Err(err) = channel.say(&ctx.http,"test").await {
			dbg!(err);
		}
	} else {
		println!("failed to create channel for {}\n", userid);
	}
}

async fn spamchat(ctx: &Context, newday: i16, channel: u64, deadline: &str){
	let chanid = ChannelId(channel);
	dbg!(chanid);
	if let Err(err) = chanid.say(&ctx.http,"test").await {
		dbg!(err);
	};
}

async fn spam(ctx : &Context, state: GameState){
    let data = ctx.data.read().await;
    let arcdb = data.get::<DbClientContainer>().expect("expected db client in sharemap");
    
	let (day, auctiontype, deadline, rate) = if let GameState::Auction {day,auctiontype,deadline,rate } = state {
		(day, auctiontype, deadline, rate)
	} else { return };
	
	let ds = dbg!(pretty_print_deadline(deadline));
	match arcdb.query_opt("SELECT id FROM channel",&[]).await.expect("dberror") {
        Some(row) => { 
            let uid : i64 = row.get(0);
            spamchat(&ctx,day,uid_to_u64(uid),&ds).await;
        },
        None => ()
    }
    for row in arcdb.query("SELECT id,points FROM users",&[]).await.expect("dberror") {
        let uid = uid_to_u64(row.get(0));
        let points : i32 = row.get(1);
        if uid >12 {
            spamperson(&ctx,uid,day,&ds,points).await;
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
            GameState::Auction{day,auctiontype,deadline,rate} => {
                oldday = day;
				deadline < now
				},
            _ => false
        }
    };
    if shouldauction {
        let gamestate = auction(&ctx,true).await.expect("auction failed");
        
		spam(&ctx,gamestate).await;
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
    async fn ready(&self, ctx: Context, ready: Ready) {
        //info!("{} is connected!", ready.user.name);
        ticker(ctx).await; 
    }
    async fn message(&self, _ctx: Context, msg: Message) {
    
        //Am I the bot, super hacky
        //if msg.author.id == 732540354468773908 {
        //    debug!("[{}]: {}",msg.author.name, msg.content);
        //} else {
        //    info!("[{}]: {}",msg.author.name, msg.content);
        //}
    }
    //async fn cache_ready(&self, ctx: Context, _guilds: Vec<GuildId>)
}





#[group]
#[commands(users,help,wins,items,register,hello2)]
struct General;

#[group]
#[commands(status,bids,bid,unregister,minorpaths)]
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

    info!("[{}]: {} -> {}",msg.author.name, msg.content, command_name);

    true
}

#[tokio::main]
async fn main() {
    
    env_logger::init();

	let config = Config::init_from_env().unwrap();

	let dbstring = format!("host='/var/run/postgresql/' user={}",config.dbuser);

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


    let mut client = Client::builder(config.token)
        .event_handler(Handler)
        .intents(GatewayIntents::DIRECT_MESSAGES | GatewayIntents::GUILD_MESSAGES )
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
