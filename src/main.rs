mod config;

use serenity::{
    prelude::*,
    model::prelude::*,
    Client,
};
struct Handler;
impl EventHandler for Handler {
    fn message(&self, context: Context, msg: Message) {
        unimplemented!();
    }
}

fn main() {
    let mut client = Client::new(config::TOKEN, Handler).expect("couldn't create the new client!");
}
