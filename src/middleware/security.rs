use actix_web::body::BoxBody;
use actix_web::dev::{ServiceRequest, ServiceResponse};
use actix_web::middleware::Next;

pub async fn check_request(req: ServiceRequest, next: Next<BoxBody>) -> Result<ServiceResponse<BoxBody>, actix_web::Error> {
    if req.headers().contains_key("Transfer-Encoding") && req.headers().contains_key("Content-Length") {
        Err(actix_web::error::ErrorBadRequest("Ambiguous request detected."))
    } else {
        let res = next.call(req).await?;

        Ok(res)
    }
}