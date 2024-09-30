use std::path::{Path};
use std::sync::{Arc, Mutex};
use actix_cors::Cors;
use actix_web::{web, App, HttpServer};
use actix_web::middleware::{from_fn, TrailingSlash};
use actix_files::{Files};

use dotenv::dotenv;
use surrealdb::engine::local::RocksDb;
use tract_onnx::onnx;
use tract_onnx::prelude::{Datum, Framework, InferenceFact, InferenceModelExt, tvec};

use gallery_backend::{middleware, route, model::app::AppData};
use gallery_backend::db::surrealdb::DB;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();

    let db_path = Path::new("database");

    let db_init = !(db_path.exists() && db_path.is_dir());

    DB.connect::<RocksDb>("database").await.unwrap();
    DB.use_ns("fdqms").await.expect("namespace err");
    DB.use_db("gallery").await.expect("db err");

    if db_init {
        DB.query(r#"
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

    let model = onnx()
        .model_for_path("model.onnx").unwrap()
        .with_input_fact(0, InferenceFact::dt_shape(f32::datum_type(), tvec![1, 3, 224, 224])).unwrap()
        .into_optimized().unwrap()
        .into_runnable().unwrap();

    let app_data = web::Data::new(AppData {
        ai_model: model,
        user_id: Arc::new(Mutex::new("".to_string())),
    });

    HttpServer::new(move || {
        let cors = Cors::permissive();

        App::new()
            // .wrap(Logger::default())
            .wrap(actix_web::middleware::DefaultHeaders::new()
                .add(("Content-Security-Policy", "frame-ancestors 'none';"))
            )
            // .wrap(from_fn(add_csp))
            .wrap(from_fn(middleware::security::check_request))
            .wrap(from_fn(middleware::auth::auth_middleware))
            .wrap(actix_web::middleware::NormalizePath::new(TrailingSlash::Trim))
            .wrap(cors)
            .app_data(app_data.clone())
            .service(route::user::logout)
            .service(route::user::login)
            .service(route::user::register)
            .service(route::user::users)
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
            .service(route::index::index)
            .service(Files::new("/", "../gallery-frontend"))
    })
        .bind(("0.0.0.0", 8080))?
        .run()
        .await
}