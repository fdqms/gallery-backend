use hmac::{Hmac, Mac};
use jwt::{Error, SignWithKey, VerifyWithKey};
use sha2::Sha384;
use std::collections::BTreeMap;
use actix_web::body::{BoxBody, MessageBody};
use actix_web::dev::{Service, ServiceRequest, ServiceResponse};
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