use hmac::{Hmac, Mac};
use jwt::{Error, SignWithKey, VerifyWithKey};
use sha2::Sha384;
use std::collections::BTreeMap;
use regex::Regex;

pub fn check_mail_invalid(input: &String) -> bool {
    let re = Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$").unwrap();

    if re.is_match(&input) {
        return false;
    }

    return true;
}

pub fn check_characters_invalid(inputs: Vec<&String>) -> bool {
    let re = Regex::new(r"^[a-zA-Z0-9]*$").unwrap();

    for input in inputs {
        if !re.is_match(&input) {
            return true;
        }
    }

    return false;
}

pub fn sign(key: &str, value: &String) -> String {
    let secret_key = std::env::var("SECRET_KEY").expect("env err -> SECRET_KEY");
    let sign_key: Hmac<Sha384> = Hmac::new_from_slice(&*secret_key.into_bytes()).unwrap();
    let mut claims = BTreeMap::new();
    claims.insert(key, value);
    let token_str = claims.sign_with_key(&sign_key).unwrap();

    return token_str;
}

pub fn verify(token_str: &str, key: &str) -> Result<String, Error> {
    let secret_key = std::env::var("SECRET_KEY").expect("env err -> SECRET_KEY");
    let sign_key: Hmac<Sha384> = Hmac::new_from_slice(&*secret_key.into_bytes()).unwrap();

    let verified_claims: BTreeMap<String, String> = token_str.verify_with_key(&sign_key)?;

    return Ok(verified_claims[key].to_string());
}