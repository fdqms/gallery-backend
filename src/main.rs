use std::path::Path;
use std::sync::{Arc, Mutex};
use actix_cors::Cors;
use actix_web::{get, post, web, App, HttpResponse, HttpServer, cookie::{Cookie, time::Duration}, Error, middleware};
use actix_web::body::{BoxBody, MessageBody};
use actix_web::dev::{ServiceRequest, ServiceResponse};
use actix_web::middleware::{Next, from_fn};
use actix_files::Files;

use dotenv::dotenv;
use serde::{Deserialize, Serialize};
use surrealdb::engine::local::{Db}; // Mem
use surrealdb::Surreal;

use futures::{stream, StreamExt};

use actix_multipart::Multipart;
use actix_web::cookie::SameSite;
use actix_web::http::Method;
use bytes::Bytes;
use sha2::{Sha512, Digest};
use tokio::fs::{File};
use tokio::io::{self, AsyncReadExt, AsyncWriteExt};

use gallery_backend::AiModel;
use gallery_backend::utils::ai::check_safety;
use gallery_backend::utils::security::{sign, verify, check_characters_invalid, check_mail_invalid, compare_string, check_request};
use gallery_backend::utils::db;

// modelin yüklenmesi çok sürdüğünden dolayı debug_assertions eklendi
// daha sonra onlar silinecek

struct AppData {
    #[cfg(not(debug_assertions))]
    ai_model: AiModel,
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
async fn login(app_data: web::Data<AppData>, form: web::Json<LoginForm>) -> HttpResponse {
    let characters_invalid = check_characters_invalid(vec![&form.username, &form.password]);

    if characters_invalid {
        return HttpResponse::Ok().body("invalid character");
    }

    let database = &app_data.database;

    let user_id = db::login(database, &form.username, &form.password).await.unwrap();

    return if user_id == "-1" {
        HttpResponse::Unauthorized().body("Login failed")
    } else {
        let token = sign("token", &user_id);

        let logged_cookie = Cookie::build("logged", "1").domain(std::env::var("DOMAIN").expect("env err -> DOMAIN")).finish();
        let token_cookie = Cookie::build("token", token).domain(std::env::var("DOMAIN").expect("env err -> DOMAIN")).http_only(true).same_site(SameSite::Strict).finish();
        HttpResponse::Ok().cookie(token_cookie).cookie(logged_cookie).body("Login successful")
    };
}

#[post("/register")]
async fn register(app_data: web::Data<AppData>, form: web::Json<RegisterForm>) -> HttpResponse {
    let characters_invalid = check_characters_invalid(vec![&form.username, &form.password]);
    let mail_invalid = check_mail_invalid(&form.email);

    if characters_invalid || mail_invalid {
        return HttpResponse::Ok().body("Invalid character");
    }

    let database = &app_data.database;

    let user_id = db::register(database, &String::from(&form.username), &String::from(&form.email), &String::from(&form.password)).await.expect("err -> db::register");
    let token = sign("token", &user_id);

    let logged_cookie = Cookie::build("logged", "1").domain(std::env::var("DOMAIN").expect("env err -> DOMAIN")).finish();
    let token_cookie = Cookie::build("token", token).domain(std::env::var("DOMAIN").expect("env err -> DOMAIN")).http_only(true).same_site(SameSite::Strict).finish();

    HttpResponse::Ok().cookie(token_cookie).cookie(logged_cookie).body("Register successful")
}

#[get("/file/{file}")]
async fn get_file(app_data: web::Data<AppData>, path: web::Path<String>) -> Result<HttpResponse, Error> {
    // let database = &app_data.database;
    // kullanıcı kontrolü yapılacak

    // düzenlenecek path saldırısına karşı
    let file_name = path.into_inner();

    let mut image = match File::open(format!("images/{}", file_name)).await {
        Ok(f) => f,
        Err(_) => {
            return Ok(HttpResponse::BadRequest().body("File not found"));
        }
    };

    let mut buffer = Vec::new();

    image.read_to_end(&mut buffer).await.expect("");

    return Ok(HttpResponse::Ok().content_type("image/jpeg").body(buffer));
}

#[post("/delete")]
async fn post_delete(mut body: web::Payload, app_data: web::Data<AppData>) -> Result<HttpResponse, Error> {
    let mut bytes = web::BytesMut::new();
    while let Some(item) = body.next().await {
        let item = item?;
        bytes.extend_from_slice(&item);
    }

    let database = &app_data.database;
    let user_id = &app_data.user_id.lock().unwrap();
    let post_id = String::from_utf8_lossy(&bytes).to_string();

    let image_name = db::post_delete(&database, &user_id, &post_id).await.expect("err -> db::post_delete");

    match tokio::fs::remove_file(format!("images/{}", image_name)).await {
        Ok(_) => return Ok(HttpResponse::Ok().body("silindi")),
        Err(e) => {
            println!("err -> delete file -> {}", e);
            return Ok(HttpResponse::Ok().body("silinemedi"));
        }
    }
}

#[post("/users/{username}")]
async fn users(app_data: web::Data<AppData>, u: web::Path<String>) -> Result<HttpResponse, Error> {
    let database = &app_data.database;
    let username = u.into_inner();

    let users = db::user_search(database, &username).await.expect("user err");

    Ok(HttpResponse::Ok()
        .content_type("application/json")
        .json(users))
}

#[post("/friend/post")]
async fn friend_post(app_data: web::Data<AppData>) -> Result<HttpResponse, Error> {
    let data = vec!["Veri 1", "Veri 2", "Veri 3", "Veri 4"];

    let data_stream = stream::iter(data.into_iter().map(|item| {
        Ok::<_, io::Error>(Bytes::from(item))
    }));

    Ok(HttpResponse::Ok()
        .content_type("text/plain")
        .streaming(data_stream))
}

/*
#[post("/friend/post")]
async fn friend_post(app_data: web::Data<AppData>) -> Result<HttpResponse, Error> {
    let data = vec!["Veri 1", "Veri 2", "Veri 3", "Veri 4"];

    let data_stream = stream::iter(data.into_iter().map(|item| {
        Ok::<_, io::Error>(Bytes::from(item))
    }));

    Ok(HttpResponse::Ok()
        .content_type("text/plain")
        .streaming(data_stream))
}
*/

#[post("/friend/add")]
async fn friend_add(app_data: web::Data<AppData>, data: String) -> Result<HttpResponse, Error> {
    let database = &app_data.database;
    let user_id = &app_data.user_id.lock().unwrap();

    let val: String;

    if data == String::from("5") {
        val = "3".to_string()
    }

    let friend_id = if compare_string(&data, &*user_id) {
        return Ok(HttpResponse::Ok().body(""));
    } else {
        data
    };

    db::friend_add(database, &*user_id, &friend_id);

    Ok(HttpResponse::Ok().body(""))
}

#[post("/friend/accept")]
async fn friend_accept(app_data: web::Data<AppData>) -> Result<HttpResponse, Error> {
    let database = &app_data.database;
    let user_id = &app_data.user_id.lock().unwrap();
    let friend_id = String::from("");

    let result = db::friend_accept(database, &*user_id, &friend_id);

    Ok(HttpResponse::Ok().body(""))
}

#[post("/friend/delete")]
async fn friend_delete(app_data: web::Data<AppData>) -> Result<HttpResponse, Error> {
    let database = &app_data.database;
    let user_id = &app_data.user_id.lock().unwrap();
    let friend_id = String::from("");

    let result = db::friend_delete(database, &*user_id, &friend_id);

    Ok(HttpResponse::Ok().body(""))
}

#[post("/friends")]
async fn friend_list(app_data: web::Data<AppData>) -> Result<HttpResponse, Error> {
    let database = &app_data.database;
    let user_id = &app_data.user_id.lock().unwrap();

    let result = db::friend_list(database, &*user_id);

    Ok(HttpResponse::Ok().body(""))
}

#[get("/post")]
async fn posts(app_data: web::Data<AppData>) -> Result<HttpResponse, Error> {
    let database = &app_data.database;

    let user_id = &app_data.user_id.lock().unwrap();

    let result = db::post_get_all(database, &*user_id).await.expect("err -> db::post_get_all");

    Ok(HttpResponse::Ok().content_type("application/json").body(result))
}

#[post("/upload")]
async fn upload(mut payload: Multipart, app_data: web::Data<AppData>) -> Result<HttpResponse, Error> {
    #[cfg(not(debug_assertions))]{
        let ai_model = &app_data.ai_model;
    }

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
                        body.extend_from_slice(&*data); //
                        hasher.update(data);
                    }

                    let hash = hex::encode(hasher.finalize());

                    file_name = format!("{}.{}", hash, extension);
                    file_path = format!("images/{}.{}", hash, extension);

                    if Path::new(&file_path).exists() {
                        return Ok(HttpResponse::NotAcceptable().body("File already exist"));
                    } else {
                        #[cfg(not(debug_assertions))]
                        {
                            let safety = check_safety(ai_model, &body).await.expect("ai err -> check_safety");

                            if !safety {
                                return Ok(HttpResponse::NotAcceptable().body("NSFW content"));
                            }
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
    }

    fields.pop();

    fields.push('}');

    let user_data: Option<UploadForm> = match serde_json::from_str(&*fields) {
        Ok(_user_data) => {
            _user_data
        }
        Err(_) => {
            return Ok(HttpResponse::BadRequest().body("Invalid field"));
        }
    };

    return match user_data {
        Some(user_data) => {
            let user_id = &app_data.user_id.lock().unwrap();
            let post_id = db::post_add(database, user_data.ratio, file_name, user_id).await.expect("db::err -> post_add");

            let mut file = File::create(&file_path).await?;
            file.write_all(&body).await?;

            Ok(HttpResponse::Ok().body(post_id))
        }
        None => {
            Ok(HttpResponse::BadRequest().body("Data not found"))
        }
    };
}

async fn auth_middleware(req: ServiceRequest, srv: Next<BoxBody>) -> Result<ServiceResponse<BoxBody>, actix_web::Error> {
    let (req_parts, body) = req.into_parts();

    if req_parts.path() == "/login" || req_parts.path() == "/register" || req_parts.path() == "/logout" {
        return srv.call(ServiceRequest::from_parts(req_parts, body)).await;
    }

    if req_parts.method() == Method::POST || req_parts.path() == "/post" {
        if let Some(cookie) = req_parts.cookie("token") {
            match verify(cookie.value(), "token") {
                Ok(_user_id) => {
                    let app_data = req_parts.app_data::<web::Data<AppData>>().unwrap();
                    let mut user_id = app_data.user_id.lock().unwrap();
                    *user_id = _user_id;
                }
                _ => return Err(actix_web::error::ErrorUnauthorized("Token invalid"))
            }
        } else {
            return Err(actix_web::error::ErrorUnauthorized("Token not found"))
            // return Ok(req.into_response(HttpResponse::Unauthorized().body("Token not found")))
        }
    }

    srv.call(ServiceRequest::from_parts(req_parts, body)).await

    // return srv.call(req).await;
}

#[actix_web::main]
async fn main() -> io::Result<()> {
    dotenv().ok();

    let db_path = Path::new("database");

    let db_init = !(db_path.exists() && db_path.is_dir());

    let db = Surreal::new::<surrealdb::engine::local::File>("database").await.unwrap();

    db.use_ns("fdqms").await.expect("namespace err");
    db.use_db("gallery").await.expect("db err");

    if db_init {
        db.query(r#"
            DEFINE TABLE user SCHEMALESS;
            DEFINE FIELD username ON TABLE user TYPE string;
            DEFINE FIELD email ON TABLE user TYPE string ASSERT string::is::email($value);
            DEFINE FIELD created_at ON TABLE USER TYPE datetime DEFAULT time::now();
            DEFINE INDEX uniq_email ON TABLE user COLUMNS email UNIQUE;
            DEFINE INDEX uniq_username ON TABLE user COLUMNS username UNIQUE;
            DEFINE TABLE friend TYPE RELATION IN user OUT user;
            DEFINE FIELD accepted ON TABLE friend TYPE bool;
            DEFINE INDEX uniq_friend ON TABLE friend COLUMNS in, out UNIQUE;
        "#).await.expect("err -> db::init");
    }

    let app_data;

    #[cfg(debug_assertions)]
    {
        app_data = web::Data::new(AppData {
            user_id: Arc::new(Mutex::new("".to_string())),
            database: db,
        });
    }

    #[cfg(not(debug_assertions))]
    {
        let model = onnx()
            .model_for_path("model.onnx").unwrap()
            .with_input_fact(0, InferenceFact::dt_shape(f32::datum_type(), tvec![1, 3, 224, 224])).unwrap()
            .into_optimized().unwrap()
            .into_runnable().unwrap();

        app_data = web::Data::new(AppData {
            ai_model: model,
            user_id: Arc::new(Mutex::new("".to_string())),
            database: db,
        });
    }

    HttpServer::new(move || {
        let cors = Cors::permissive();

        App::new()
            // .wrap(Logger::default())
            .wrap(middleware::DefaultHeaders::new()
                .add(("X-Frame-Options", "DENY"))
                .add(("Content-Security-Policy", "default-src 'self'; style-src 'self'; img-src 'self'; script-src 'self' 'unsafe-eval'"))
            )
            .wrap(from_fn(check_request))
            .wrap(from_fn(auth_middleware))
            .wrap(cors)
            .app_data(app_data.clone())
            .service(logout)
            .service(login)
            .service(register)
            .service(users)
            .service(upload)
            .service(post_delete)
            .service(posts)
            .service(friend_post)
            .service(friend_add)
            .service(get_file)
            .service(Files::new("/", "../gallery-frontend").index_file("index.html"))
    })
        .bind(("0.0.0.0", 8080))?
        .run()
        .await
}