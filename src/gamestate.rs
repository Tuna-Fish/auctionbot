use std::sync::Arc;
use chrono::{NaiveDateTime,Duration};


#[derive(Copy,Clone,PartialEq)]
pub enum AuctionType {
	Race,
	PrimaryPath,
	SecondaryPath,
	Perk(i16),
}

pub async fn get_auctiontype_for_day(arcdb : &Arc<tokio_postgres::Client>, day: i16) -> Option<AuctionType> {
	let maybegamerulerow = &arcdb.query_opt("SELECT daytype,perksonday FROM gamerules WHERE day = $1;",&[&day]).await.expect("database failure fetching game rules");
	
	match maybegamerulerow {
		None => None,
		Some(row) => {
						let daytype: i16 = row.get(0);
						match daytype {
							1 => Some(AuctionType::Race),
							2 => Some(AuctionType::PrimaryPath),
							3 => Some(AuctionType::SecondaryPath),
							4 => Some(AuctionType::Perk(row.get(1))),
							_ => unreachable!()
						}
		}
						
	}
}

#[derive(Copy,Clone)]
pub enum GameState {
    Closed,
    Registration,
    Auction{day: i16, auctiontype: AuctionType, deadline: NaiveDateTime, rate: i32},
    Finished,
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
						let auctiontype: AuctionType = get_auctiontype_for_day(&arcdb, i).await.expect("confusion regarding game rules? Gamestate thought it was auction day, yet game rules disagreed.");
                        GameState::Auction {
                            day : i, auctiontype, deadline, rate 
						}
                    }
                }
            }
        }
	}
	pub async fn advance(self: GameState, arcdb: &Arc<tokio_postgres::Client> ) -> GameState {
		
		match self {
			GameState::Auction{day, auctiontype,deadline,rate} => {
			let newdeadline =  deadline + Duration::minutes(rate as i64);
			let newday = day+1;
			let maybeauctiontype = get_auctiontype_for_day(&arcdb,newday).await;
		
			match maybeauctiontype {
				None => {
						let _rows = &arcdb.query("DELETE FROM gamestate",&[]).await.expect("database failure");
						let _rows = &arcdb.query("INSERT INTO gamestate (phase) VALUES ($1);",&[&-1i16]).await.expect("database failure");    
						GameState::Finished 
						},
				Some(auctiontype) => {
						let _rows = &arcdb.query("DELETE FROM gamestate",&[]).await.expect("database failure");
						let _rows = &arcdb.query("INSERT INTO gamestate (phase,deadline,rate) VALUES ($1,$2,$3);",&[&newday,&newdeadline,&(rate)]).await.expect("database failure");
						
						GameState::Auction{day: newday, auctiontype, deadline: newdeadline, rate: rate}
					}
				}	

			},
			_ => { panic!("advance called when auction was not open") }
		}
		
		
	}

}
