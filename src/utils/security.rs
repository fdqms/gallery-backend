use hmac::{Hmac, Mac};
use jwt::{Error, SignWithKey, VerifyWithKey};
use sha2::Sha384;
use std::collections::BTreeMap;
use actix_web::body::{BoxBody};
use actix_web::dev::{ServiceRequest, ServiceResponse};
use actix_web::http::header;
use actix_web::middleware::Next;
use regex::Regex;

pub fn check_mail_invalid(input: &String) -> bool {
    let re = Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$").unwrap();

    if re.is_match(input) {
        return false;
    }

    true
}

pub async fn check_request(req: ServiceRequest, next: Next<BoxBody>) -> Result<ServiceResponse<BoxBody>, actix_web::Error> {
    if req.headers().contains_key("Transfer-Encoding") && req.headers().contains_key("Content-Length") {
        Err(actix_web::error::ErrorBadRequest("Ambiguous request detected."))
    } else {
        let res = next.call(req).await?;

        Ok(res)
    }
}

pub async fn add_csp(req: ServiceRequest, next: Next<BoxBody>) -> Result<ServiceResponse<BoxBody>, actix_web::Error> {
    let re = Regex::new(r"^/file/([^/]+)\.(jpe?g|png)$").unwrap();

    let path = req.path().to_owned();
    let mut res = next.call(req).await?;


    if re.is_match(&path) {
        res.headers_mut().insert(
            header::CONTENT_SECURITY_POLICY,
            header::HeaderValue::from_static("default-src 'self'; img-src 'self'; script-src 'none'; style-src 'self' 'unsafe-inline';"),
        );
    } else {
        res.headers_mut().insert(
            header::CONTENT_SECURITY_POLICY,
            header::HeaderValue::from_static("default-src 'self' data:; img-src 'self' data: blob:; style-src 'self'; script-src 'self';"),
        );
    }

    Ok(res)
}

pub fn compare_string(val1: &String, val2: &String) -> bool {
    val1 == val2
}

pub fn check_characters_invalid(inputs: Vec<&String>) -> bool {
    let re = Regex::new(r"^[a-zA-Z0-9]*$").unwrap();

    for input in inputs {
        if !re.is_match(input) {
            return true;
        }
    }

    false
}

pub fn sign(key: &str, value: &String) -> String {
    let secret_key = std::env::var("SECRET_KEY").expect("env err -> SECRET_KEY");
    let sign_key: Hmac<Sha384> = Hmac::new_from_slice(&secret_key.into_bytes()).unwrap();
    let mut claims = BTreeMap::new();
    claims.insert(key, value);


    claims.sign_with_key(&sign_key).unwrap()
}

pub fn verify(token_str: &str, key: &str) -> Result<String, Error> {
    let secret_key = std::env::var("SECRET_KEY").expect("env err -> SECRET_KEY");
    let sign_key: Hmac<Sha384> = Hmac::new_from_slice(&secret_key.into_bytes()).unwrap();

    let verified_claims: BTreeMap<String, String> = token_str.verify_with_key(&sign_key)?;

    Ok(verified_claims[key].to_string())
}