use actix_web::body::BoxBody;
use actix_web::dev::{ServiceRequest, ServiceResponse};
use actix_web::{web, Error};
use actix_web::http::Method;
use actix_web::middleware::Next;
use crate::model::app::AppData;
use crate::utils::security::verify;

pub async fn auth_middleware(req: ServiceRequest, srv: Next<BoxBody>) -> Result<ServiceResponse<BoxBody>, Error> {
    if req.path() == "/login" || req.path() == "/register" || req.path() == "/logout" {
        return srv.call(req).await;
    }

    if req.method() == Method::POST || req.path() == "/post" || req.path() == "/profile" {
        if let Some(cookie) = req.cookie("token") {
            match verify(cookie.value(), "token") {
                Ok(_user_id) => {
                    if let Some(app_data) = req.app_data::<web::Data<AppData>>() {
                        let mut user_id = app_data.user_id.lock().unwrap();
                        *user_id = _user_id;
                    }
                }
                _ => return Err(actix_web::error::ErrorUnauthorized("Token invalid"))
            }
        } else {
            return Err(actix_web::error::ErrorUnauthorized("Token not found"));
        }
    }

    srv.call(req).await
}