use std::path::Path;
use std::sync::{Arc, Mutex};
use actix_web::{get, post, web, App, HttpResponse, HttpServer, Responder, cookie::{Cookie, time::Duration}, Error};
use actix_web::body::MessageBody;
use actix_web::dev::{ServiceRequest, ServiceResponse};
use actix_web::middleware::{Next, from_fn};

use dotenv::dotenv;
use serde::{Deserialize, Serialize};
use surrealdb::engine::local::{Db, Mem};
use surrealdb::Surreal;

use futures::{StreamExt};

use actix_multipart::Multipart;
use sha2::{Sha512, Digest};
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use gallery_backend::utils::security::{sign, verify, check_characters_invalid, check_mail_invalid};
use gallery_backend::utils::db;


struct AppData {
    user_id: Arc<Mutex<String>>,
    database: Surreal<Db>,
}

#[derive(Deserialize)]
struct LoginForm {
    username: String,
    password: String,
}

#[derive(Deserialize)]
struct RegisterForm {
    username: String,
    email: String,
    password: String,
}

#[derive(Deserialize, Serialize, Debug)]
struct UploadForm {
    ratio: String,
}

#[get("/logout")]
async fn logout() -> HttpResponse {
    let cookie = Cookie::build("token", "").max_age(Duration::ZERO).finish();
    return HttpResponse::Ok().cookie(cookie).body("exit successful");
}

#[post("/login")]
async fn login(app_data: web::Data<AppData>, form: web::Form<LoginForm>) -> HttpResponse {
    let characters_invalid = check_characters_invalid(vec![&form.username, &form.password]);

    if characters_invalid {
        return HttpResponse::Ok().body("invalid character");
    }

    let database = &app_data.database;

    let user_id = db::login(database, &form.username, &form.password).await.unwrap();

    return if user_id == "-1" {
        HttpResponse::Ok().body("login failed")
    } else {
        let token = sign("token", &user_id);
        let cookie = Cookie::build("token", token).domain(std::env::var("DOMAIN").expect("env err -> DOMAIN")).http_only(true).finish();
        HttpResponse::Ok().cookie(cookie).body("login successful")
    };
}

#[post("/register")]
async fn register(app_data: web::Data<AppData>, form: web::Form<RegisterForm>) -> HttpResponse {
    let characters_invalid = check_characters_invalid(vec![&form.username, &form.password]);
    let mail_invalid = check_mail_invalid(&form.email);

    if characters_invalid || mail_invalid {
        return HttpResponse::Ok().body("invalid character");
    }

    let database = &app_data.database;

    let user_id = db::register(database, &String::from(&form.username), &String::from(&form.email), &String::from(&form.password)).await.expect("err -> db::register");
    let token = sign("token", &user_id);

    let cookie = Cookie::build("token", token).domain(std::env::var("DOMAIN").expect("env err -> DOMAIN")).http_only(true).finish();

    return HttpResponse::Ok().cookie(cookie).body("register successful");
}

#[post("/upload")]
async fn upload(mut payload: Multipart) -> Result<HttpResponse, Error> {
    let mut user_data: Option<UploadForm> = None;
    let mut file_exist = false;

    while let Some(item) = payload.next().await {
        let mut field = item?;

        match field.content_disposition() {
            Some(cd) => {
                let filename = cd.get_filename().map(ToString::to_string);
                if let Some(filename) = filename {
                    let (_, extension) = filename.rsplit_once(".").unwrap();

                    let mut body = web::BytesMut::new();
                    let mut hasher = Sha512::new();

                    while let Some(chunk) = field.next().await {
                        let data = chunk?;
                        body.extend_from_slice(&*data);
                        hasher.update(data);
                    }

                    let hash = hex::encode(hasher.finalize());

                    let file_path = format!("images/{}.{}", hash, extension);

                    if Path::new(&file_path).exists() {
                        file_exist = true;
                    } else {
                        let mut file = File::create(&file_path).await?;
                        let n = file.write_all(&body).await?;
                    }
                } else {
                    let mut s = String::new();
                    while let Some(chunk) = field.next().await {
                        let data = chunk?;
                        s.push_str(&String::from_utf8_lossy(&data));
                    }


                    user_data = match serde_json::from_str(&*s) {
                        Ok(user_data) => {
                            user_data
                        },
                        _ => return Ok(HttpResponse::BadRequest().body("Data not found"))
                    };
                }
            }
            None => {
                return Err(actix_web::error::ErrorBadRequest("Content disposition not found"))
            }
        }
    }

    if file_exist {
        return Ok(HttpResponse::NotAcceptable().body("File already exist"));
    } else {

        match user_data {
            Some(user_data) => {
                return Ok(HttpResponse::BadRequest().body("Upload successful"));
            }
            None => {
                return Ok(HttpResponse::BadRequest().body("Data not found"));
            }
        }
    }
}

async fn hi(app_data: web::Data<AppData>) -> impl Responder {
    // let user_id = app_data.user_id.lock().unwrap();
    HttpResponse::Ok().body("Hey there!")
}

async fn auth_middleware(
    req: ServiceRequest,
    srv: Next<actix_web::body::BoxBody>,
) -> Result<ServiceResponse<impl MessageBody>, Error> {
    let (req_parts, body) = req.into_parts();

    if req_parts.path() != "/login" && req_parts.path() != "/register" && req_parts.path() != "/logout" {
        if let Some(cookie) = req_parts.cookie("token") {
            match verify(cookie.value(), "token") {
                Ok(_user_id) => {
                    let app_data = req_parts.app_data::<web::Data<AppData>>().unwrap();
                    let mut user_id = app_data.user_id.lock().unwrap();
                    *user_id = _user_id;
                }
                _ => return Ok(ServiceResponse::new(req_parts, HttpResponse::Unauthorized().body("Token invalid")))
            }
        } else {
            return Ok(ServiceResponse::new(req_parts, HttpResponse::Unauthorized().body("Token not found")));
            // return Ok(req.into_response(HttpResponse::Unauthorized().body("Token not found")))
        }
    }

    return srv.call(ServiceRequest::from_parts(req_parts, body)).await;

    // return srv.call(req).await;
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();

    let db = Surreal::new::<Mem>(()).await.unwrap();
    db.use_ns("fdqms").await.expect("namespace err");
    db.use_db("gallery").await.expect("db err");

    let app_data = web::Data::new(AppData {
        user_id: Arc::new(Mutex::new("".to_string())),
        database: db,
    });

    HttpServer::new(move || {
        App::new()
            // .wrap(Logger::default())
            .wrap(from_fn(auth_middleware))
            .app_data(app_data.clone())
            .service(logout)
            .service(login)
            .service(register)
            .service(upload)
            .route("/", web::get().to(hi))
    })
        .bind(("0.0.0.0", 8080))?
        .run()
        .await
}