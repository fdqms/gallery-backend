use std::sync::{Arc, Mutex};

use crate::AiModel;

pub struct AppData {
    pub ai_model: AiModel,
    pub user_id: Arc<Mutex<String>>,
}