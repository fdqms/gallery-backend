use hmac::{Hmac, Mac};
use jwt::{Error, SignWithKey, VerifyWithKey};
use sha2::Sha384;
use std::collections::BTreeMap;
use actix_web::body::{BoxBody};
use actix_web::dev::{ServiceRequest, ServiceResponse};
use actix_web::http::header::{HeaderName, HeaderValue};
use actix_web::middleware::Next;
use regex::Regex;

pub fn check_mail_invalid(input: &str) -> bool {
    let re = Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$").unwrap();

    if re.is_match(input) {
        return false;
    }

    true
}


// pub async fn add_csp(req: ServiceRequest, next: Next<BoxBody>) -> Result<ServiceResponse<BoxBody>, actix_web::Error> {
//     let re = Regex::new(r"^/file/([^/]+)\.(jpe?g|png)$").unwrap();
//
//     let path = req.path().to_owned();
//     let mut res = next.call(req).await?;
//
//
//     if re.is_match(&path) {
//         res.headers_mut().insert(
//             header::CONTENT_SECURITY_POLICY,
//             header::HeaderValue::from_static("default-src 'self'; img-src 'self'; script-src 'none'; style-src 'self' 'unsafe-inline';"),
//         );
//     } else {
//         res.headers_mut().insert(
//             header::CONTENT_SECURITY_POLICY,
//             header::HeaderValue::from_static("default-src 'self' data:; img-src 'self' data: blob:; style-src 'self'; script-src 'self';"),
//         );
//     }
//
//     Ok(res)
// }

pub async fn add_csp(
    req: ServiceRequest, next: Next<BoxBody>
) -> Result<ServiceResponse<BoxBody>, actix_web::Error> {
    let mut response = next.call(req).await?;

    let csp_policy = [
        "default-src 'self'",
        "script-src 'self'",
        "style-src 'self' 'unsafe-inline'",
        "img-src 'self' data: blob: https:",
        "connect-src 'self' ws: wss: https:",
        "font-src 'self' data: https:",
        "frame-src 'none'",
        "object-src 'none'",
        "base-uri 'self'",
        "form-action 'self'",
        "media-src 'self' blob:; ",
        "worker-src 'self' blob:; ",
        "manifest-src 'self'"
    ];
    
    response.headers_mut().insert(
        HeaderName::from_static("content-security-policy"),
        HeaderValue::from_str(&csp_policy.join("; "))?
    );
    
    response.headers_mut().insert(
        HeaderName::from_static("x-content-type-options"),
        HeaderValue::from_static("nosniff")
    );

    response.headers_mut().insert(
        HeaderName::from_static("x-frame-options"),
        HeaderValue::from_static("DENY")
    );

    response.headers_mut().insert(
        HeaderName::from_static("referrer-policy"),
        HeaderValue::from_static("strict-origin-when-cross-origin")
    );

    Ok(response)
}

pub async fn add_cors(
    req: ServiceRequest, next: Next<BoxBody>
) -> Result<ServiceResponse<BoxBody>, actix_web::Error> {
    let mut response = next.call(req).await?;

    response.headers_mut().insert(
        HeaderName::from_static("access-control-allow-origin"),
        HeaderValue::from_static("https://xn--hatradefteri-34b.com")
    );

    // // CORS için ek başlıklar ekleyebilirsiniz:
    // response.headers_mut().insert(
    //     HeaderName::from_static("access-control-allow-methods"),
    //     HeaderValue::from_static("GET, POST, PUT, DELETE, OPTIONS")
    // );
    //
    // response.headers_mut().insert(
    //     HeaderName::from_static("access-control-allow-headers"),
    //     HeaderValue::from_static("Content-Type, Authorization")
    // );

    Ok(response)
}

pub fn compare_string(val1: &String, val2: &String) -> bool {
    val1 == val2
}

pub fn check_injection(inputs: Vec<&String>) -> bool {
    let re = Regex::new(r"(?i)(--|#|/\*|\*/|\b(select|insert|update|delete|drop|union|alter|exec|create|table)\b)").unwrap();

    for input in inputs {
        if re.is_match(input) {
            return true;
        }
    }

    false
}

pub fn check_xss(inputs: Vec<&String>) -> bool {
    let re = Regex::new(r"(<script.*?>.*?</script>|<.*?>)").unwrap();

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