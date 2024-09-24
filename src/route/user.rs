use actix_web::{get, post, web, HttpResponse};
use actix_web::cookie::{Cookie, SameSite};
use actix_web::cookie::time::Duration;
use actix_web::http::Error;
use crate::db;
use crate::model::app::AppData;
use crate::model::user::{LoginForm, RegisterForm};
use crate::utils::security::{check_characters_invalid, check_mail_invalid, sign};

#[get("/logout")]
pub async fn logout() -> HttpResponse {
    let cookie = Cookie::build("token", "").max_age(Duration::ZERO).finish();
    HttpResponse::Ok().cookie(cookie).body("exit successful")
}

#[post("/login")]
pub async fn login(app_data: web::Data<AppData>, form: web::Json<LoginForm>) -> HttpResponse {
    let characters_invalid = check_characters_invalid(vec![&form.username, &form.password]);

    if characters_invalid {
        return HttpResponse::Ok().body("invalid character");
    }

    let database = &app_data.database;

    let user_id = db::surrealdb::login(database, &form.username, &form.password).await.unwrap();

    if user_id == "-1" {
        HttpResponse::Unauthorized().body("Login failed")
    } else {
        let token = sign("token", &user_id);

        let logged_cookie = Cookie::build("logged", "1").domain(std::env::var("DOMAIN").expect("env err -> DOMAIN")).finish();
        let token_cookie = Cookie::build("token", token).domain(std::env::var("DOMAIN").expect("env err -> DOMAIN")).http_only(true).same_site(SameSite::Strict).finish();
        HttpResponse::Ok().cookie(token_cookie).cookie(logged_cookie).body("Login successful")
    }
}

#[post("/register")]
pub async fn register(app_data: web::Data<AppData>, form: web::Json<RegisterForm>) -> HttpResponse {
    let characters_invalid = check_characters_invalid(vec![&form.username, &form.password]);
    let mail_invalid = check_mail_invalid(&form.email);

    if characters_invalid || mail_invalid {
        return HttpResponse::Ok().body("Invalid character");
    }

    let database = &app_data.database;

    let user_id = db::surrealdb::register(database, &String::from(&form.username), &String::from(&form.email), &String::from(&form.password)).await.expect("err -> db::surrealdb::register");
    let token = sign("token", &user_id);

    let logged_cookie = Cookie::build("logged", "1").domain(std::env::var("DOMAIN").expect("env err -> DOMAIN")).finish();
    let token_cookie = Cookie::build("token", token).domain(std::env::var("DOMAIN").expect("env err -> DOMAIN")).http_only(true).same_site(SameSite::Strict).finish();

    HttpResponse::Ok().cookie(token_cookie).cookie(logged_cookie).body("Register successful")
}

#[post("/users")]
pub async fn users(app_data: web::Data<AppData>, body: String) -> Result<HttpResponse, Error> {
    let database = &app_data.database;
    let user_id = {
        let uid = app_data.user_id.lock().unwrap();
        uid.clone()
    };

    let users = db::surrealdb::user_search(database, &user_id, &body).await.expect("user err");

    Ok(HttpResponse::Ok()
        .content_type("application/json")
        .json(users))
}