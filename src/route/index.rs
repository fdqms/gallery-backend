use std::path::PathBuf;
use actix_files::NamedFile;
use actix_web::{get, Error, HttpRequest, HttpResponse};

#[get("/")]
async fn index(req: HttpRequest) -> Result<HttpResponse, Error> {
    let referer = req.headers().get("referer").map(|r| r.to_str().unwrap_or(""));

    let path: PathBuf = match referer {
        Some(r) if r.contains(&std::env::var("DOMAIN").expect("env err -> DOMAIN")) => "../gallery-frontend/index.html".parse()?,
        _ => "../gallery-frontend/default.html".parse()?,
    };

    let file = NamedFile::open_async(path).await?;
    let mut res = file.into_response(&req);
    res.headers_mut().insert(actix_web::http::header::CACHE_CONTROL, "no-store, no-cache, must-revalidate, proxy-revalidate, max-age=0, pre-check=0, post-check=0".parse()?);

    Ok(res)
}