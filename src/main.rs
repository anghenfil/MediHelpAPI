#![feature(duration_constructors_lite)]

use std::sync::Arc;
use std::time::{Duration};
use rocket::{get, launch, routes, tokio};
use rocket::tokio::sync::RwLock;
use rocket::tokio::time::Instant;
use crate::lieferengpaesse::Lieferengpass;

pub mod lieferengpaesse;

#[derive(Default)]
pub struct TempStorage{
    storage: RwLock<InnerStorage>
}

#[derive(Default)]
pub struct InnerStorage{
    pub lieferengpaesse: Vec<Lieferengpass>,
    pub reqwest_client: reqwest::Client,
}

pub async fn refresh_worker(storage: Arc<TempStorage>){
    tokio::task::spawn(async move {
        let mut last_refresh = Instant::now();

        loop{
            last_refresh = Instant::now();

            // Refresh lieferengpässe
            lieferengpaesse::refresh_lieferengpaesse(storage.clone()).await;

            tokio::time::sleep_until(last_refresh + Duration::from_mins(2)).await;
        }
    });
}

#[rocket::main]
async fn main() -> Result<(), rocket::Error> {
    let storage = Arc::new(TempStorage::default());

    // Start refresh worker
    refresh_worker(storage.clone()).await;

    let _rocket = rocket::build()
        .mount("/lieferengpaesse", routes![lieferengpaesse::lieferengpaesse])
        .manage(storage)
        .launch()
        .await?;

    Ok(())
}