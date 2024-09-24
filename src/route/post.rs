use std::path::Path;
use actix_files::NamedFile;
use actix_multipart::Multipart;
use actix_web::{get, post, web, Error, HttpRequest, HttpResponse};
use futures::StreamExt;
use sha2::{Digest, Sha512};
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use crate::ai::image_classification::check_safety;
use crate::db;
use crate::model::app::AppData;
use crate::model::post::UploadForm;

#[get("/file/{file}")]
async fn get_file(req: HttpRequest, path: web::Path<String>) -> Result<HttpResponse, Error> {
    let file_name = path.into_inner();

    let file = NamedFile::open_async(format!("images/{}", file_name)).await?;

    Ok(file.into_response(&req))
}

#[post("/post/delete")]
async fn post_delete(body: String, app_data: web::Data<AppData>) -> Result<HttpResponse, Error> {
    let database = &app_data.database;
    let user_id = {
        let uid = app_data.user_id.lock().unwrap();
        uid.clone()
    };

    let image_name = db::surrealdb::post_delete(database, &user_id, &body).await.expect("err -> db::surrealdb::post_delete");

    match image_name {
        Some(image) => {
            match tokio::fs::remove_file(format!("images/{}", image)).await {
                Ok(_) => Ok(HttpResponse::Ok().body("silindi")),
                Err(_) => {
                    Ok(HttpResponse::NotModified().body("silinemedi"))
                }
            }
        }
        None => {
            Ok(HttpResponse::NotModified().body("silinemedi"))
        }
    }
}

#[get("/post")]
async fn posts(app_data: web::Data<AppData>) -> Result<HttpResponse, Error> {
    let database = &app_data.database;

    let user_id = {
        let uid = app_data.user_id.lock().unwrap();
        uid.clone()
    };

    let result = db::surrealdb::post_get_all(database, &user_id).await.expect("err -> db::surrealdb::post_get_all");

    Ok(HttpResponse::Ok().content_type("application/json").json(result))
}

#[post("/upload")]
async fn upload(mut payload: Multipart, app_data: web::Data<AppData>) -> Result<HttpResponse, Error> {
    let ai_model = &app_data.ai_model;

    let database = &app_data.database;


    let mut file_name = String::new();
    let mut file_path = String::new();
    let mut body = web::BytesMut::new();

    let mut fields = String::from("{");

    while let Some(item) = payload.next().await {
        let mut field = item?;

        match field.content_disposition() {
            Some(cd) => {
                let filename = cd.get_filename().map(ToString::to_string);

                if let Some(filename) = filename {
                    let (_, extension) = filename.rsplit_once(".").unwrap();

                    let mut hasher = Sha512::new();

                    while let Some(chunk) = field.next().await {
                        let data = chunk?;
                        body.extend_from_slice(&data); //
                        hasher.update(data);
                    }

                    let hash = hex::encode(hasher.finalize());

                    file_name = format!("{}.{}", hash, extension);
                    file_path = format!("images/{}.{}", hash, extension);

                    if Path::new(&file_path).exists() {
                        return Ok(HttpResponse::NotModified().body("File already exist"));
                    } else {
                        let safety = check_safety(ai_model, &body).await.expect("ai err -> check_safety");

                        if !safety {
                            return Ok(HttpResponse::NotAcceptable().body("NSFW content"));
                        }
                    }
                } else {
                    let area_name = cd.get_name().unwrap_or("");

                    if area_name.is_empty() {
                        return Ok(HttpResponse::BadRequest().body("Fill in the required fields"));
                    }

                    fields.push('"');
                    fields.push_str(area_name);
                    fields.push('"');
                    fields.push(':');
                    fields.push('"');

                    while let Some(chunk) = field.next().await {
                        let data = chunk?;
                        fields.push_str(&String::from_utf8_lossy(&data));
                    }

                    fields.push('"');
                    fields.push(',');
                }
            }
            None => {
                return Err(actix_web::error::ErrorBadRequest("Content disposition not found"))
            }
        }
    };

    fields.pop();

    fields.push('}');

    let user_data: Option<UploadForm> = match serde_json::from_str(&fields) {
        Ok(_user_data) => {
            _user_data
        }
        Err(_) => {
            return Ok(HttpResponse::BadRequest().body("Invalid field"));
        }
    };

    return match user_data {
        Some(user_data) => {
            let user_id = {
                let uid = app_data.user_id.lock().unwrap();
                uid.clone()
            };
            let post_id = db::surrealdb::post_add(database, user_data.ratio, file_name, &user_id).await.expect("db::surrealdb::err -> post_add");

            let mut file = File::create(&file_path).await?;
            file.write_all(&body).await?;

            Ok(HttpResponse::Ok().body(post_id))
        }
        None => {
            Ok(HttpResponse::BadRequest().body("Data not found"))
        }
    };
}