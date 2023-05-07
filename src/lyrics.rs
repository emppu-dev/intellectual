use actix_web::{get, web, Responder, Result};
use askama::Template;
use futures::future;
use once_cell::sync::Lazy;

use scraper::{Html, Node, Selector};
use serde::Deserialize;

use crate::genius::GeniusSong;
use crate::genius::{self, GeniusApi};
use crate::templates::template;
use crate::utils;

static SONG_ID_SELECTOR: Lazy<Selector> =
    Lazy::new(|| Selector::parse("meta[property='twitter:app:url:iphone']").unwrap());
static LYRIC_SELECTOR: Lazy<Selector> =
    Lazy::new(|| Selector::parse("div[data-lyrics-container=true]").unwrap());

struct Verse {
    title: String,
    lyrics: Vec<String>,
}

#[derive(Template)]
#[template(path = "lyrics.html")]
struct LyricsTemplate {
    verses: Vec<Verse>,
    query: LyricsQuery,
    song: GeniusSong,
}

#[derive(Debug, Deserialize)]
pub struct LyricsQuery {
    id: Option<u32>,
    path: String,
}

#[get("/lyrics")]
pub async fn lyrics(info: web::Query<LyricsQuery>) -> Result<impl Responder> {
    let document: Html;
    let song: GeniusSong;

    if let Some(id) = info.id {
        let responses = future::join(
            GeniusApi::global().get_text(genius::SubDomain::Root, &info.path, None),
            GeniusApi::global().get_song(id),
        )
        .await;
        document = Html::parse_document(&responses.0?);
        song = responses.1?;
    } else {
        let lyric_page = GeniusApi::global()
            .get_text(genius::SubDomain::Root, &info.path, None)
            .await?;
        document = Html::parse_document(&lyric_page);
        let id = get_song_id(&document)?;
        song = GeniusApi::global().get_song(id).await?;
    }

    let verses = scrape_lyrics(&document);

    Ok(template(LyricsTemplate {
        verses,
        query: info.0,
        song,
    }))
}

fn get_song_id(document: &Html) -> crate::Result<u32> {
    Ok(document
        .select(&SONG_ID_SELECTOR)
        .next()
        .ok_or("Failed to find meta tag with song ID")?
        .value()
        .attr("content")
        .and_then(|content| content.strip_prefix("genius://songs/"))
        .ok_or("Failed to find content attribute")?
        .parse::<u32>()?)
}

fn scrape_lyrics(document: &Html) -> Vec<Verse> {
    let mut verses = Vec::new();
    let mut current_verse: Option<Verse> = None;
    let mut new_line = false;

    for child in document
        .select(&LYRIC_SELECTOR)
        .flat_map(|e| e.descendants())
    {
        match child.value() {
            Node::Element(e) if e.name() == "br" => {
                new_line = true;
            }
            Node::Text(text) => {
                let text: &str = text;
                let is_title = text.starts_with('[') && text.ends_with(']');
                if is_title {
                    if let Some(curr) = current_verse {
                        verses.push(curr);
                    }
                    current_verse = Some(Verse {
                        title: text.to_string(),
                        lyrics: Vec::new(),
                    });
                } else if let Some(curr) = current_verse.as_mut() {
                    let last = curr.lyrics.last_mut();
                    if new_line || last.is_none() {
                        curr.lyrics.push(text.to_owned());
                        new_line = false;
                    } else if let Some(lyric) = last {
                        lyric.push_str(text);
                    }
                }
            }
            _ => {}
        }
    }

    if let Some(curr) = current_verse {
        verses.push(curr);
    } else {
        verses.push(Verse {
            title: String::new(),
            lyrics: vec!["This song has no lyrics.".to_owned()],
        })
    }

    verses
}
