use crate::db;
use chrono::{DateTime, Duration, Utc};
use std::collections::HashMap;
use std::fs;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::sleep;

#[derive(Clone)]
pub struct DeletionService {
    requests: Arc<Mutex<HashMap<String, chrono::DateTime<Utc>>>>,
}

impl DeletionService {
    pub fn new() -> Self {
        Self {
            requests: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn from(requests: HashMap<String, DateTime<Utc>>) -> Self {
        Self {
            requests: Arc::new(Mutex::new(requests)),
        }
    }

    pub async fn delete(&self, user_id: &String) -> Result<(), String> {
        let mut requests = self.requests.lock().await;

        if requests.contains_key(user_id) {
            return Err(format!("User already exists: {}", user_id));
        }

        requests.insert(user_id.clone(), Utc::now() + Duration::days(30)); // Duration::days(30)

        println!("{:?}", requests);

        Ok(())
    }

    pub async fn cancel(&self, user_id: &String) -> Result<(), String> {
        let mut requests = self.requests.lock().await;

        if requests.contains_key(user_id) {
            requests.remove(user_id);
        }
        Ok(())
    }

    async fn delete_account(&self, user_id: &String) -> Result<(), String> {
        let posts = db::surrealdb::post_get_all(user_id)
            .await
            .expect("err -> db::surrealdb::post_get_all");
        for post in posts.0 {
            match fs::remove_file(format!("images/{}", post.image)) {
                Ok(_) => println!("deleted image: {}", post.image),
                Err(e) => return Err(format!("{}", e)),
            }
        }
        db::surrealdb::user_delete(user_id)
            .await
            .expect("err -> db::surrealdb::user_delete");
        db::surrealdb::friend_delete(user_id)
            .await
            .expect("err -> db::surrealdb::friend_delete");

        Ok(())
    }

    async fn process(&self) {
        let mut deleting = Vec::<String>::new();
        let mut requests = self.requests.lock().await;

        requests.retain(|user_id, request| {
            if *request > Utc::now() {
                deleting.push(user_id.clone());
                return false;
            }
            true
        });

        for user_id in deleting {
            self.delete_account(&user_id)
                .await
                .expect("delete account error");
        }
    }

    pub async fn start(self) {
        let this = self.clone();
        tokio::spawn(async move {
            loop {
                sleep(tokio::time::Duration::from_secs(12*60*60)).await;
                this.process().await;
            }
        });
    }

    pub async fn get_requests(&self) -> Arc<Mutex<HashMap<String, DateTime<Utc>>>> {
        self.requests.clone()
    }
}
