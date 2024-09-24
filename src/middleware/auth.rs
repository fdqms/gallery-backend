use actix_web::body::BoxBody;
use actix_web::dev::{ServiceRequest, ServiceResponse};
use actix_web::{web, Error};
use actix_web::http::Method;
use actix_web::middleware::Next;
use crate::model::app::AppData;
use crate::utils::security::verify;

pub async fn auth_middleware(req: ServiceRequest, srv: Next<BoxBody>) -> Result<ServiceResponse<BoxBody>, Error> {
    let (req_parts, body) = req.into_parts();

    if req_parts.path() == "/login" || req_parts.path() == "/register" || req_parts.path() == "/logout" {
        return srv.call(ServiceRequest::from_parts(req_parts, body)).await;
    }

    if req_parts.method() == Method::POST || req_parts.path() == "/post" || req_parts.path() == "/profile" {
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
            return Err(actix_web::error::ErrorUnauthorized("Token not found"));
            // return Ok(req.into_response(HttpResponse::Unauthorized().body("Token not found")))
        }
    }

    srv.call(ServiceRequest::from_parts(req_parts, body)).await

    // return srv.call(req).await;
}