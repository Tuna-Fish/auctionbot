mod config;

mod commands;
mod auction;

use log::{debug, error, info};
use std::sync::Arc;
use tokio_postgres::{NoTls, Error};
use serenity::{
    prelude::*,
    async_trait,
    client::bridge::gateway::GatewayIntents,
    
    framework::standard::{
        Args, CommandOptions, CommandResult, CommandGroup,
        DispatchError, HelpOptions, help_commands, StandardFramework,
        macros::{command, group, help, check, hook},
    },
    http::Http,
    
    model::{
        channel::{Message, Channel},
        gateway::Ready,
        id::UserId,
        id::GuildId,
        id::ChannelId,
        permissions::Permissions
    },
};
use chrono::{NaiveDateTime,Local};
use tokio::sync::RwLock;
use tokio::sync::Mutex;
use commands::*;
use tokio::time::Duration;
use crate::auction::auction;

fn uid_to_u64(id : i64) -> u64{
    id as u64
}

async fn spamperson(ctx: &Context, userid: u64,newday: i16, deadline: &str,points: i32){
let chanid = ChannelId(userid);
let _ = chanid.say(&ctx,"test");
debug!("spampersontest {}",userid);
}

async fn spamchat(ctx: &Context, newday: i16, channel: u64, deadline: &str){
let chanid = ChannelId(channel);
let _ = chanid.say(&ctx,"test");
debug!("spamchattest {}",channel);
}

async fn spam(ctx : &Context, newday : i16, deadline : NaiveDateTime){
    let data = ctx.data.read().await;
    let arcdb = data.get::<DbClientContainer>().expect("expected db client in sharemap");
    let ds = pretty_print_deadline(deadline);
    match arcdb.query_opt("SELECT id FROM channel",&[]).await.expect("dberror") {
        Some(row) => { 
            let uid : i64 = row.get(0);
            spamchat(&ctx,newday,uid_to_u64(uid),&ds).await;
        },
        None => ()
    }
    for row in arcdb.query("SELECT id,points FROM users",&[]).await.expect("dberror") {
        let uid = uid_to_u64(row.get(0));
        let points : i32 = row.get(1);
        if uid >12 {
            spamperson(&ctx,uid,newday,&ds,points).await;
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
                if deadline < now {
                    true
                } else {
                    false
                }
            },
            _ => false
        }
    };
    if shouldauction {
        let deadline = auction(&ctx,true).await.unwrap();
        //spam(&ctx,oldday+1, deadline).await;
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
        if msg.author.id == 732540354468773908 {
            debug!("[{}]: {}",msg.author.name, msg.content);
        } else {
            info!("[{}]: {}",msg.author.name, msg.content);
        }
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

#[derive(Copy,Clone)]
enum GameState {
    Closed,
    Registration,
    Auction{day: i16, deadline: NaiveDateTime, rate: i32},
    Finished,
}

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

    true
}

#[tokio::main]
async fn main() {
    
    env_logger::init();

    let (dbclient, connection) = tokio_postgres::connect("host='/var/run/postgresql/' user=auctionbot", NoTls).await.unwrap();
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("connection error: {}", e);
        }
    });
    let arcdb = Arc::new(dbclient);



    let gamestate = {
        
        let mayberow = &arcdb.query_opt("SELECT phase,deadline,rate FROM gamestate;",&[]).await.expect("database failure fetching gamestate");
        
        match mayberow {
            None => GameState::Closed,
            Some(row) => {
                let state : i16 = row.get(0);
                match state {
                    0 => GameState::Registration,
                    -1 => GameState::Finished,
                    i => {
                        let deadline : NaiveDateTime = row.get(1);
                        let rate : i32 = row.get(2);
                        GameState::Auction {
                            day : i, deadline, rate }
                    }
                }
            }
        }
    };

    let rwlarcstate = Arc::new(RwLock::new(gamestate.clone()));
    
    let framework = StandardFramework::new()
        .configure(|c| c
            .with_whitespace(true)
            .prefix("")
            .delimiters(vec![", ", ","," "]))
        
        .before(before)
        .group(&GENERAL_GROUP)
        .group(&ADMIN_GROUP)
        .group(&AUCTION_GROUP);

    let mut client = Client::builder(config::TOKEN)
        .event_handler(Handler)
        .intents(GatewayIntents::DIRECT_MESSAGES | GatewayIntents::GUILD_MESSAGES)
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
