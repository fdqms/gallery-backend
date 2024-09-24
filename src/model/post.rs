use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct Post {
    pub id: String,
    pub image: String,
    pub ratio: String,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct UploadForm {
    pub ratio: String,
}
