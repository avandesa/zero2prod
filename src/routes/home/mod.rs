mod login;

pub use login::*;

use actix_web::{http::header::ContentType, HttpResponse};

pub async fn home() -> HttpResponse {
    HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(include_str!("home.html"))
}
