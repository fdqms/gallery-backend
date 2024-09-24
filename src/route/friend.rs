use actix_web::{get, post, web, Error, HttpResponse};
use crate::db;
use crate::model::app::AppData;

#[post("/follow/{friend_id}")]
pub async fn follow(app_data: web::Data<AppData>, f: web::Path<String>) -> Result<HttpResponse, Error> {
    let database = &app_data.database;
    let friend_id = f.into_inner();
    let user_id = {
        let uid = app_data.user_id.lock().unwrap();
        uid.clone()
    };

    db::surrealdb::follow(database, &user_id, &friend_id).await.expect("err -> db::surrealdb::follow");

    Ok(HttpResponse::Ok().body(""))
}

#[post("/unfollow")]
pub async fn unfollow(app_data: web::Data<AppData>, body: String) -> Result<HttpResponse, Error> {
    let database = &app_data.database;
    let user_id = {
        let uid = app_data.user_id.lock().unwrap();
        uid.clone()
    };

    db::surrealdb::unfollow(database, &user_id, &body).await.expect("err -> db::surrealdb::follow");

    Ok(HttpResponse::Ok().body(""))
}

#[post("/follow/accept")]
pub async fn follow_accept(app_data: web::Data<AppData>, body: String) -> Result<HttpResponse, Error> {
    let database = &app_data.database;
    let user_id = {
        let uid = app_data.user_id.lock().unwrap();
        uid.clone()
    };

    db::surrealdb::follow_accept(database, &user_id, &body).await.expect("err -> db::surrealdb::follow");

    Ok(HttpResponse::Ok().body(""))
}

#[post("/follow/reject")]
pub async fn follow_reject(app_data: web::Data<AppData>, body: String) -> Result<HttpResponse, Error> {
    let database = &app_data.database;
    let user_id = {
        let uid = app_data.user_id.lock().unwrap();
        uid.clone()
    };

    db::surrealdb::follow_reject(database, &user_id, &body).await.expect("err -> db::surrealdb::follow");

    Ok(HttpResponse::Ok().body(""))
}

#[get("/follow/pendings")]
pub async fn follow_pendings(app_data: web::Data<AppData>) -> Result<HttpResponse, Error> {
    let database = &app_data.database;
    let user_id = {
        let uid = app_data.user_id.lock().unwrap();
        uid.clone()
    };
    let pendings = db::surrealdb::follow_pendings(database, &user_id).await.expect("err -> db::surrealdb::follow_pendings");

    Ok(HttpResponse::Ok()
        .content_type("application/json")
        .json(pendings))
}

#[get("/follow/requests")]
pub async fn follow_requests(app_data: web::Data<AppData>) -> Result<HttpResponse, Error> {
    let database = &app_data.database;
    let user_id = {
        let uid = app_data.user_id.lock().unwrap();
        uid.clone()
    };

    let requests = db::surrealdb::follow_requests(database, &user_id).await.expect("err -> db::surrealdb::follow_requests");

    Ok(HttpResponse::Ok()
        .content_type("application/json")
        .json(requests))
}

#[get("/friends")]
pub async fn friends(app_data: web::Data<AppData>) -> Result<HttpResponse, Error> {
    let database = &app_data.database;
    let user_id = {
        let uid = app_data.user_id.lock().unwrap();
        uid.clone()
    };

    let friends = db::surrealdb::friends(database, &user_id).await.expect("err -> db::surrealdb::friends");

    Ok(HttpResponse::Ok()
        .content_type("application/json")
        .json(friends))
}

#[get("/friend/{friend_id}/post")]
pub async fn friend_posts(app_data: web::Data<AppData>, path: web::Path<String>) -> Result<HttpResponse, Error> {
    let friend_id = path.into_inner();
    let database = &app_data.database;
    let user_id = {
        let uid = app_data.user_id.lock().unwrap();
        uid.clone()
    };
    let f_posts = db::surrealdb::friend_post(database, &user_id, &friend_id).await.expect("err -> db::surrealdb::friend_posts");

    Ok(HttpResponse::Ok()
        .content_type("application/json")
        .json(f_posts))
}