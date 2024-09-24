use std::sync::{Arc, Mutex};
use surrealdb::engine::local::Db;
use surrealdb::Surreal;
use crate::AiModel;

pub struct AppData {
    pub ai_model: AiModel,
    pub user_id: Arc<Mutex<String>>,
    pub database: Surreal<Db>,
}