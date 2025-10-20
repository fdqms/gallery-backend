use actix_http::body::{BoxBody};
use actix_web::{
    dev::{ServiceRequest, ServiceResponse},
    middleware::{ Next},
    Error, HttpMessage
};
use actix_web::error::ErrorBadRequest;
use bytes::Bytes;
use futures::TryStreamExt;
use regex::Regex;
use serde_json::Value;

pub async fn check_inputs(
    mut req: ServiceRequest,
    next: Next<BoxBody>
) -> Result<ServiceResponse<BoxBody>, Error> {

    if sanitize_query_params(&mut req).is_err() {
        return Err(ErrorBadRequest("query err"));
    }

    let payload = req.take_payload();
    let stream = payload.map_ok(|chunk| {
        if let Ok(chunk_str) = std::str::from_utf8(&chunk) {
            if let Ok(mut json_value) = serde_json::from_str::<Value>(chunk_str) {
                sanitize_json(&mut json_value);
                let sanitized_json = json_value.to_string();
                Bytes::from(sanitized_json)
            } else {
                let sanitized_chunk = sanitize_input(chunk_str);
                Bytes::from(sanitized_chunk)
            }
        } else {
            chunk
        }
    });

    req.set_payload(stream.into_inner());

    next.call(req).await
}

fn sanitize_json(value: &mut Value) {
    match value {
        Value::Object(obj) => {
            for (_key, val) in obj.iter_mut() {
                sanitize_json(val);
            }
        },
        Value::String(s) => {
            *s = sanitize_input(s);
        },
        Value::Array(arr) => {
            for val in arr.iter_mut() {
                sanitize_json(val);
            }
        },
        _ => {}
    }
}

fn sanitize_input(input: &str) -> String {
    let injection_patterns = vec![
        r"<script",
        r"javascript:",
        r"\.\.",
        r"(/\.\.|\\\.\.)",
        r"(?i)(?:^|\s)(SELECT|DELETE|UPDATE|INSERT|DROP|UNION|ALTER)",
        r"(\d+\s*[=<>]\s*\d+)",
        r";\s*[a-zA-Z]",
        r#""\s*OR\s*"|'\\s*OR\\s*'"#,
    ];

    let mut sanitized = input.to_string();

    for pattern in injection_patterns {
        let re = Regex::new(pattern).unwrap_or_else(|_| Regex::new("").unwrap());
        sanitized = re.replace_all(&sanitized, "").to_string();
    }

    sanitized
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}

fn sanitize_query_params(req: &mut ServiceRequest) -> Result<(), Error> {
    let query = req.query_string();
    let path = req.path();

    if !query.is_empty() {
        let sanitized_query = sanitize_input(query);

        if sanitized_query != query {
            return Err(ErrorBadRequest("query err"));
        }
    }
    
    if !path.is_empty() {
        let sanitized_path = sanitize_input(path);
        if sanitized_path != path {
            return Err(ErrorBadRequest("path err"));
        }
    }
    
    Ok(())
}
