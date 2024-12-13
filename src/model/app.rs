use crate::service::deletion_service::DeletionService;
use crate::AiModel;
use std::sync::{Arc, Mutex};
use web3::Web3;

pub struct AppData {
    pub ai_model: AiModel,
    pub user_id: Arc<Mutex<String>>,
    pub crypto_network: Web3<web3::transports::http::Http>,
    pub deletion_service: DeletionService,
}
