use crate::db;
use crate::model::app::AppData;
use crate::model::user::{ChangePasswordForm, LoginForm, RegisterForm};
use crate::utils::security::{   sign};
use actix_web::cookie::time::{Duration, OffsetDateTime};
use actix_web::cookie::{Cookie, SameSite};
use actix_web::{get, post, web, Error, HttpResponse};
use std::str::FromStr;
use web3::types::{Address, BlockId, H256, U64};

#[post("/delete")]
pub async fn delete(app_data: web::Data<AppData>) -> Result<HttpResponse, Error> {
    let user_id = {
        let uid = app_data.user_id.lock().unwrap();
        uid.clone()
    };

    app_data
        .deletion_service
        .delete(&user_id)
        .await
        .expect("deletion service err -> ");

    let cookie = Cookie::build("token", "")
        .max_age(Duration::ZERO)
        .domain(std::env::var("DOMAIN").expect("env err -> DOMAIN"))
        .finish();
    let logged_cookie = Cookie::build("logged", "")
        .max_age(Duration::ZERO)
        .domain(std::env::var("DOMAIN").expect("env err -> DOMAIN"))
        .finish();
    let premium_cookie = Cookie::build("premium", "")
        .max_age(Duration::ZERO)
        .domain(std::env::var("DOMAIN").expect("env err -> DOMAIN"))
        .finish();

    Ok(HttpResponse::Ok()
        .cookie(cookie)
        .cookie(logged_cookie)
        .cookie(premium_cookie)
        .body("Delete successful"))
}

#[get("/logout")]
pub async fn logout() -> HttpResponse {
    let cookie = Cookie::build("token", "")
        .max_age(Duration::ZERO)
        .domain(std::env::var("DOMAIN").expect("env err -> DOMAIN"))
        .finish();
    let logged_cookie = Cookie::build("logged", "")
        .max_age(Duration::ZERO)
        .domain(std::env::var("DOMAIN").expect("env err -> DOMAIN"))
        .finish();
    let premium_cookie = Cookie::build("premium", "")
        .max_age(Duration::ZERO)
        .domain(std::env::var("DOMAIN").expect("env err -> DOMAIN"))
        .finish();
    HttpResponse::Ok()
        .cookie(cookie)
        .cookie(logged_cookie)
        .cookie(premium_cookie)
        .body("exit successful")
}

#[post("/login")]
pub async fn login(form: web::Json<LoginForm>, app_data: web::Data<AppData>) -> HttpResponse {

    let user_id = db::surrealdb::login(&form.username, &form.password)
        .await
        .unwrap();

    if user_id == "-1" {
        HttpResponse::Unauthorized().body("login failed")
    } else {
        let token = sign("token", &user_id);

        app_data
            .deletion_service
            .cancel(&user_id)
            .await
            .expect("deletion service cancel err ->");

        let mut uid = app_data.user_id.lock().unwrap();
        *uid = user_id;

        let logged_cookie = Cookie::build("logged", "1")
            .domain(std::env::var("DOMAIN").expect("env err -> DOMAIN"))
            .finish();
        let token_cookie = Cookie::build("token", token)
            .domain(std::env::var("DOMAIN").expect("env err -> DOMAIN"))
            .http_only(true)
            .same_site(SameSite::Strict)
            .finish();
        HttpResponse::Ok()
            .cookie(token_cookie)
            .cookie(logged_cookie)
            .body("login successful")
    }
}

#[post("/change_password")]
pub async fn change_password(
    form: web::Json<ChangePasswordForm>,
    app_data: web::Data<AppData>,
) -> HttpResponse {
    let user_id = {
        let uid = app_data.user_id.lock().unwrap();
        uid.clone()
    };

    db::surrealdb::change_password(&user_id, &form.old, &form.new)
        .await
        .expect("change password err");

    HttpResponse::Ok().body("password changed")
}

#[get("/profile")]
pub async fn profile(app_data: web::Data<AppData>) -> Result<HttpResponse, Error> {
    let user_id = {
        let uid = app_data.user_id.lock().unwrap();
        uid.clone()
    };

    let profile = db::surrealdb::profile(&user_id).await.expect("profile err");

    Ok(HttpResponse::Ok()
        .content_type("application/json")
        .json(profile))
}

#[post("/register")]
pub async fn register(form: web::Json<RegisterForm>, app_data: web::Data<AppData>) -> HttpResponse {
    let user_id = db::surrealdb::register(
        &String::from(&form.username),
        &String::from(&form.email),
        &String::from(&form.password),
    )
    .await
    .expect("err -> db::surrealdb::register");
    let token = sign("token", &user_id);

    let mut uid = app_data.user_id.lock().unwrap();
    *uid = user_id;

    let logged_cookie = Cookie::build("logged", "1")
        .domain(std::env::var("DOMAIN").expect("env err -> DOMAIN"))
        .finish();
    let token_cookie = Cookie::build("token", token)
        .domain(std::env::var("DOMAIN").expect("env err -> DOMAIN"))
        .http_only(true)
        .same_site(SameSite::Strict)
        .finish();

    HttpResponse::Ok()
        .cookie(token_cookie)
        .cookie(logged_cookie)
        .body("register successful")
}

#[post("/users")]
pub async fn users(app_data: web::Data<AppData>, body: String) -> Result<HttpResponse, Error> {
    let user_id = {
        let uid = app_data.user_id.lock().unwrap();
        uid.clone()
    };

    let users = db::surrealdb::user_search(&user_id, &body)
        .await
        .expect("user err");

    Ok(HttpResponse::Ok()
        .content_type("application/json")
        .json(users))
}

#[get("/premium")]
pub async fn check_premium(app_data: web::Data<AppData>) -> HttpResponse {
    let user_id = {
        let uid = app_data.user_id.lock().unwrap();
        uid.clone()
    };

    if user_id.is_empty() {
        return HttpResponse::Unauthorized().body(format!("{}", false));
    }

    let last_date = db::surrealdb::check_premium(&user_id)
        .await
        .expect("err -> db::user::check_premium");

    if last_date == 0 {
        HttpResponse::Unauthorized().body("Premium not found")
    } else {
        let premium_cookie = Cookie::build("premium", "1")
            .expires(OffsetDateTime::from_unix_timestamp(last_date / 1000).unwrap())
            .domain(std::env::var("DOMAIN").expect("env err -> DOMAIN"))
            .finish();

        HttpResponse::Ok()
            .cookie(premium_cookie)
            .body(format!("{}", last_date))
    }
}

#[post("/payment")]
pub async fn payment(app_data: web::Data<AppData>, body: String) -> Result<HttpResponse, Error> {
    let user_id = {
        let uid = app_data.user_id.lock().unwrap();
        uid.clone()
    };

    if user_id.is_empty() {
        return Err(actix_web::error::ErrorBadRequest(format!("{}", body)));
    }

    if db::surrealdb::check_transaction(&body)
        .await
        .expect("err -> db::surrealdb::check_transaction")
    {
        return Err(actix_web::error::ErrorBadRequest(
            "transaction already exist",
        ));
    }

    db::surrealdb::add_transaction(&user_id, &body)
        .await
        .expect("err -> db::surrealdb::add_transaction");

    let crypto_network = app_data.crypto_network.clone();

    let hash_str = body.trim_start_matches("0x");
    let tx_hash: H256 = match hex::decode(hash_str) {
        Ok(h) => {
            if h.len() != 32 {
                return Err(actix_web::error::ErrorBadRequest("invalid hash length"));
            }
            H256::from_slice(&h)
        }
        Err(_) => return Err(actix_web::error::ErrorBadRequest("invalid hash format")),
    };

    let mut receipt = None;
    let max_attempts = 6;
    let mut attempts = 0;

    while receipt.is_none() {
        if attempts >= max_attempts {
            return Err(actix_web::error::ErrorInternalServerError(
                "avax servers are busy".to_string(),
            ));
        }

        receipt = match crypto_network.eth().transaction_receipt(tx_hash).await {
            Ok(r) => r,
            Err(e) => {
                return Err(actix_web::error::ErrorInternalServerError(format!(
                    "err -> tx {}",
                    e
                )))
            }
        };

        if receipt.is_none() {
            tokio::time::sleep(core::time::Duration::from_secs(10)).await;
            attempts += 1;
        }
    }

    let receipt = receipt.unwrap();

    if receipt.status != Some(U64::from(1)) {
        return Err(actix_web::error::ErrorBadRequest("validator not accepted"));
    }

    let tx = match crypto_network.eth().transaction(tx_hash.into()).await {
        Ok(Some(tx)) => tx,
        Ok(None) => return Err(actix_web::error::ErrorNotFound("transaction not found")),
        Err(e) => {
            return Err(actix_web::error::ErrorBadRequest(format!(
                "err -> tx {}",
                e
            )))
        }
    };

    let block = match crypto_network
        .eth()
        .block(BlockId::Hash(tx.block_hash.unwrap()))
        .await
    {
        Ok(Some(block)) => block,
        Ok(None) => return Err(actix_web::error::ErrorNotFound("block not found")),
        Err(e) => {
            return Err(actix_web::error::ErrorBadRequest(format!(
                "err -> block {}",
                e
            )))
        }
    };

    let transaction_date = block.timestamp.as_u64() + 10800; // 10800 -> utc +3

    let my_address = Address::from_str(&std::env::var("WALLET").expect("env err -> WALLET"))
        .expect("wrong format -> target");

    if let Some(to_address) = tx.to {
        if format!("{:?}", to_address).to_lowercase() != format!("{:?}", my_address).to_lowercase()
        {
            return Err(actix_web::error::ErrorBadRequest("wrong address"));
        }
    } else {
        return Err(actix_web::error::ErrorBadRequest("invalid address"));
    }

    let amount = tx.value.as_u128() as f64 / 1e18;

    if amount >= 0.05 {
        db::surrealdb::add_premium(&user_id, &body, &transaction_date)
            .await
            .expect("err -> db::user::add_premium");
        let premium_cookie = Cookie::build("premium", "1")
            .expires(OffsetDateTime::from_unix_timestamp(transaction_date as i64).unwrap())
            .domain(std::env::var("DOMAIN").expect("env err -> DOMAIN"))
            .finish();
        Ok(HttpResponse::Ok()
            .cookie(premium_cookie)
            .body(format!("{}", transaction_date)))
    } else {
        Err(actix_web::error::ErrorBadRequest("invalid money"))
    }
}

#[get("/upload_limit")]
pub async fn upload_limit(app_data: web::Data<AppData>) -> HttpResponse {
    let user_id = {
        let uid = app_data.user_id.lock().unwrap();
        uid.clone()
    };

    if user_id.is_empty() {
        HttpResponse::Unauthorized().body(format!("{}", false));
    }
    
    let limit = db::surrealdb::upload_limit(&user_id).await.expect("err -> db::surrealdb::upload_limit");

    HttpResponse::Ok().body(format!("{}", limit))
}