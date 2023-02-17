use std::sync::Arc;
use chrono::{NaiveDateTime,Duration};

#[derive(Copy,Clone)]
pub enum GameState {
    Closed,
    Registration,
    Auction{day: i16, deadline: NaiveDateTime, rate: i32},
    Finished,
}

pub async fn auctions_per_day(arcdb : &Arc<tokio_postgres::Client>, day: i16) -> i32 {
    let row = arcdb.query_one("SELECT COUNT(DISTINCT nr) from item where day = $1 ;",&[&day]).await.expect("database failure counting auction days");
    row.get(0)
}

pub async fn items_per_auction(arcdb : &Arc<tokio_postgres::Client>, day: i16, nr: i16) -> i32 {
    let row = arcdb.query_one("SELECT COUNT(name) from item where day = $1 and nr = $2 ;",&[&day,&nr]).await.expect("database failure counting auctions per day");
    row.get(0)
}

impl GameState {
	pub async fn fromdb(arcdb : &Arc<tokio_postgres::Client>) -> GameState {
		let maybegamestaterow = &arcdb.query_opt("SELECT phase,deadline,rate FROM gamestate;",&[]).await.expect("database failure fetching gamestate");
		match maybegamestaterow {
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
                            day : i, deadline, rate
						}
                    }
                }
            }
        }
	}
	pub async fn advance(self: GameState, arcdb: &Arc<tokio_postgres::Client> ) -> GameState {
		
		match self {
			GameState::Auction{day, deadline,rate} => {
			let newdeadline =  deadline + Duration::minutes(rate as i64);
			let newday = day+1;
			let auctions_on_newday = auctions_per_day(&arcdb, newday).await;
            match auctions_on_newday{
                0 => {
                    let _rows = &arcdb.query("DELETE FROM gamestate", &[]).await.expect("database failure");
                    let _rows = &arcdb.query("INSERT INTO gamestate (phase) VALUES ($1);", &[&-1i16]).await.expect("database failure");
                    GameState::Finished
                },
				_ => {
						let _rows = &arcdb.query("DELETE FROM gamestate",&[]).await.expect("database failure");
						let _rows = &arcdb.query("INSERT INTO gamestate (phase,deadline,rate) VALUES ($1,$2,$3);",&[&newday,&newdeadline,&(rate)]).await.expect("database failure");
						
						GameState::Auction{day: newday, deadline: newdeadline, rate: rate}
					}
				}

			},
			_ => { panic!("advance called when auction was not open") }
		}
		
		
	}

}
