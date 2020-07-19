mod config;

mod commands;

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
use chrono::NaiveDateTime;
use tokio::sync::RwLock;
use tokio::sync::Mutex;
use commands::*;

struct Handler;
#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, _ctx: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
    }
    async fn message(&self, _ctx: Context, msg: Message) {
    
    info!("[{}]: {}",msg.author.name, msg.content);
    }
}


#[group]
#[commands(help,listwins,register,hello2)]
struct General;

#[group]
#[commands(status,bid,unregister)]
struct Auction;

#[group]
#[commands(astatus,setstate,getstate,kick)]
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

    let (dbclient, connection) = tokio_postgres::connect("host='/var/run/postgresql/' user=antuunai", NoTls).await.unwrap();
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

    let mut client = Client::new(config::TOKEN)
        .event_handler(Handler)
        .add_intent(GatewayIntents::DIRECT_MESSAGES)
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
