use actix_web::{get, Responder, web};
use askama::Template;
use serde::Deserialize;

use crate::templates::template;

#[derive(Template)]
#[template(path = "search.html")]
struct SearchTemplate {
    results: Vec<GeniusResult>,
}

#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    q: String,
}

#[get("/search")]
pub async fn search(info: web::Query<SearchQuery>) -> impl Responder {
    let body = reqwest::Client::new()
        .get(format!("https://api.genius.com/search?q={}", info.q))
        .header("Authorization", format!("Bearer {}", std::env::var("AUTH_TOKEN").unwrap()))
        .send()
        .await.unwrap().text_with_charset("utf-8")
        .await.unwrap();
    let deserialized: GeniusSearch = serde_json::from_str(&body).unwrap();

    template(SearchTemplate {
        results: deserialized.response.hits.into_iter().map(|x| x.result).collect(),
    })
}

// region Genius Response
#[derive(Deserialize)]
struct GeniusSearch {
    response: GeniusResponse,
}

#[derive(Deserialize)]
struct GeniusResponse {
    hits: Vec<GeniusHit>,
}

#[derive(Deserialize)]
struct GeniusHit {
    result: GeniusResult,
}

#[allow(dead_code)]
#[derive(Deserialize)]
struct GeniusResult {
    title: String,
    artist_names: String,
    path: String,
    stats: GeniusStats,
}

#[allow(dead_code)]
#[derive(Deserialize)]
struct GeniusStats {
    pageviews: Option<i32>,
}
// endregion
