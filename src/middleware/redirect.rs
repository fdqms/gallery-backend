use actix_web::body::BoxBody;
use actix_web::dev::{ServiceRequest, ServiceResponse};
use actix_web::HttpResponse;
use actix_web::middleware::Next;

pub async fn redirect_https(req: ServiceRequest, next: Next<BoxBody>) -> Result<ServiceResponse<BoxBody>, actix_web::Error> {
    if req.connection_info().scheme() == "http" {
        let https_url = format!("https://{}{}", req.headers().get("Host").unwrap().to_str().unwrap(), req.uri());
        let response = req.into_response(HttpResponse::PermanentRedirect().append_header(("Location", https_url)).finish());
        return Ok(response);
    }
    
    next.call(req).await
}