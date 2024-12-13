use actix_files::Files;
use actix_web::middleware::{from_fn, Logger, TrailingSlash};
use actix_web::{web, App, HttpServer};
use chrono::{DateTime, Utc};
use dotenv::dotenv;
use serde_json::{from_reader, to_writer};
use std::collections::HashMap;
use std::fs::File;
use std::path::Path;
use std::sync::{Arc, Mutex};
use openssl::ssl::{SslAcceptor, SslFiletype, SslMethod};
use gallery_backend::db::surrealdb::DB;
use gallery_backend::service::deletion_service::DeletionService;
use gallery_backend::{middleware, model::app::AppData, route};
use surrealdb::engine::local::RocksDb;
use tokio::signal::unix::{signal, SignalKind};
use tract_onnx::onnx;
use tract_onnx::prelude::{tvec, Datum, Framework, InferenceFact, InferenceModelExt};
use web3::Web3;
use gallery_backend::middleware::redirect::redirect_https;
use gallery_backend::utils::security::{add_cors, add_csp};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let mut builder = SslAcceptor::mozilla_intermediate(SslMethod::tls())?;
    builder.set_private_key_file("privkey.pem", SslFiletype::PEM)?;
    builder.set_certificate_chain_file("fullchain.pem")?;

    dotenv().ok();

    let db_path = Path::new("database");

    let db_init = !(db_path.exists() && db_path.is_dir());

    DB.connect::<RocksDb>("database").await.unwrap();
    DB.use_ns("fdqms").await.expect("namespace err");
    DB.use_db("gallery").await.expect("db err");

    if db_init {
        DB.query(
            r#"
            DEFINE TABLE user SCHEMAFULL;
            DEFINE FIELD upload_limit ON user TYPE int DEFAULT 0;
            DEFINE FIELD transaction ON user TYPE option<string>;
            DEFINE FIELD transaction_date ON user TYPE option<datetime>;
            DEFINE FIELD username ON TABLE user TYPE string;
            DEFINE FIELD password ON TABLE user TYPE string;
            DEFINE FIELD email ON TABLE user TYPE string ASSERT string::is::email($value);
            DEFINE FIELD created_at ON TABLE user TYPE datetime DEFAULT time::now();
            DEFINE FIELD posts ON TABLE user FLEXIBLE TYPE array<object>;
            DEFINE INDEX uniq_email ON TABLE user COLUMNS email UNIQUE;
            DEFINE INDEX uniq_username ON TABLE user COLUMNS username UNIQUE;
            DEFINE INDEX uniq_transaction ON TABLE user COLUMNS transaction UNIQUE;
            DEFINE TABLE friend TYPE RELATION IN user OUT user;
            DEFINE FIELD accepted ON TABLE friend TYPE bool;
            DEFINE INDEX uniq_friend ON TABLE friend COLUMNS in, out UNIQUE;
        "#,
        )
        .await
        .expect("err -> db::init");
    }

    let model = onnx()
        .model_for_path("model.onnx")
        .unwrap()
        .with_input_fact(
            0,
            InferenceFact::dt_shape(f32::datum_type(), tvec![1, 3, 224, 224]),
        )
        .unwrap()
        .into_optimized()
        .unwrap()
        .into_runnable()
        .unwrap();

    // https://api.avax.network/ext/bc/C/rpc
    // https://api.avax-test.network/ext/bc/C/rpc
    let transport = web3::transports::Http::new("https://api.avax.network/ext/bc/C/rpc").expect("transport err");

    let web3 = Web3::new(transport);

    let deletion_service = match File::open("requests.json") {
        Ok(f) => {
            DeletionService::from(from_reader(f)?)
        }
        Err(_) => {
            DeletionService::new()
        }
    };

    deletion_service.clone().start().await;

    let app_data = web::Data::new(AppData {
        ai_model: model,
        user_id: Arc::new(Mutex::new("".to_string())),
        crypto_network: web3,
        deletion_service: deletion_service.clone(),
    });
    let server_http = HttpServer::new(|| {
        App::new()
            .app_data(web::PayloadConfig::new(4 * 1920 * 1080))
            .wrap(from_fn(redirect_https))
            .service(route::index::index_http)
    }).bind(("0.0.0.0", 80))?.run();

    let server_https = HttpServer::new(move || {
        App::new()
            .wrap(from_fn(add_cors))
            .wrap(from_fn(add_csp))
            .wrap(from_fn(middleware::security::check_inputs))
            .wrap(from_fn(middleware::auth::auth_middleware))
            .wrap(actix_web::middleware::NormalizePath::new(
                TrailingSlash::Trim,
            ))
            .wrap(Logger::default())
            .app_data(app_data.clone())
            .service(route::user::profile)
            .service(route::user::logout)
            .service(route::user::login)
            .service(route::user::register)
            .service(route::user::users)
            .service(route::user::check_premium)
            .service(route::user::payment)
            .service(route::user::delete)
            .service(route::user::change_password)
            .service(route::user::upload_limit)
            .service(route::friend::follow_requests)
            .service(route::friend::follow_pendings)
            .service(route::friend::follow_accept)
            .service(route::friend::follow_reject)
            .service(route::friend::follow)
            .service(route::friend::unfollow)
            .service(route::friend::friends)
            .service(route::friend::friend_posts)
            .service(route::post::upload)
            .service(route::post::post_delete)
            .service(route::post::posts)
            .service(route::post::get_file)
            .service(route::index::word)
            .service(route::index::index)
            .service(Files::new("/", "../gallery-frontend"))
    })
        // .bind(("0.0.0.0", 5000))?
    .bind_openssl(("0.0.0.0", 443), builder)?
    .run();

    tokio::select! {
        result = server_http => {
            println!("actix web ssl server terminated");
            result
        }
        result = server_https => {
            println!("actix web server terminated");
            result
        }
        _ = tokio::signal::ctrl_c() => {
            let requests: HashMap<String, DateTime<Utc>> = deletion_service.get_requests().await.lock().await.clone();
            to_writer(File::create("requests.json")?, &requests)?;
            Ok(())
        }
        _ = async {
            let mut sigterm = signal(SignalKind::terminate()).expect("sigterm err");
            sigterm.recv().await;
        } => {
            let requests: HashMap<String, DateTime<Utc>> = deletion_service.get_requests().await.lock().await.clone();
            to_writer(File::create("requests.json")?, &requests)?;
            Ok(())
        }
    }
}
