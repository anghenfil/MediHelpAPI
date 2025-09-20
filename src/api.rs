use std::sync::Arc;
use rocket::{get, State};
use rocket::serde::json::Json;
use serde::Serialize;
use crate::lieferengpaesse::Lieferengpass;
use crate::rote_hand_briefe::Brief;
use crate::TempStorage;

#[derive(Serialize, Clone)]
pub enum ApiResponse<T>{
    NotReady,
    Success(T),
}

#[get("/lieferengpaesse")]
pub async fn lieferengpaesse(storage: &State<Arc<TempStorage>>) -> Json<ApiResponse<Vec<Lieferengpass>>> {
    if !storage.storage.read().await.lieferengpaesse_loaded_initially{
        return Json(ApiResponse::NotReady)
    }
    let data = storage.storage.read().await.lieferengpaesse.clone();

    Json(ApiResponse::Success(data))
}

#[get("/briefe")]
pub async fn briefe(storage: &State<Arc<TempStorage>>) -> Json<ApiResponse<Vec<Brief>>> {
    if !storage.storage.read().await.briefe_loaded_initially{
        return Json(ApiResponse::NotReady)
    }
    
    let data = storage.storage.read().await.briefe.clone().into_values().collect::<Vec<Brief>>();
    
    Json(ApiResponse::Success(data))
}