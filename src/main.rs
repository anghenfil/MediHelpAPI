#![feature(duration_constructors_lite)]

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration};
use rocket::{routes, tokio};
use rocket::tokio::sync::RwLock;
use rocket::tokio::time::Instant;
use crate::lieferengpaesse::Lieferengpass;
use crate::rote_hand_briefe::{crawl_bfarm, crawl_pei, Brief};

pub mod lieferengpaesse;
pub mod rote_hand_briefe;
mod api;

#[derive(Default)]
pub struct TempStorage{
    storage: RwLock<InnerStorage>
}

#[derive(Default)]
pub struct InnerStorage{
    pub lieferengpaesse: Vec<Lieferengpass>,
    pub briefe: HashMap<String, Brief>,
    pub reqwest_client: reqwest::Client,
}

pub async fn refresh_worker(storage: Arc<TempStorage>){
    tokio::task::spawn(async move {
        let mut last_refresh = Instant::now();

        loop{
            last_refresh = Instant::now();

            println!("Starting refresh!");

            println!("Refreshing pei letters...");
            if let Err(e) = crawl_pei(storage.clone()).await{
                eprintln!("Failed to crawl pei: {}", e);                
            }
            
            println!("Refreshing bfarm letters...");
            if let Err(e) = crawl_bfarm(storage.clone()).await{
                eprintln!("Failed to crawl bfarm: {}", e);
            }
            
            println!("Refreshing lieferengpässe...");
            // Refresh lieferengpässe
            if let Err(e) = lieferengpaesse::refresh_lieferengpaesse(storage.clone()).await{
                eprintln!("Reqwest Error: {:?}. Trying again in 5 seconds.", e);
                tokio::time::sleep(Duration::from_secs(5)).await;
                continue;
            };

            println!("Refresh finished. We have {} Lieferengpässe and {} letters listed. Waiting for next refresh interval.", storage.storage.read().await.lieferengpaesse.len(), storage.storage.read().await.briefe.len());
            tokio::time::sleep_until(last_refresh + Duration::from_mins(15)).await;
        }
    });
}

#[rocket::main]
async fn main() -> Result<(), rocket::Error> {
    let storage = Arc::new(TempStorage::default());

    // Start refresh worker
    refresh_worker(storage.clone()).await;

    let _rocket = rocket::build()
        .mount("/api", routes![api::lieferengpaesse, api::briefe])
        .manage(storage)
        .launch()
        .await?;

    Ok(())
}