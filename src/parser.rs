use std::collections::HashSet;
use std::time::Duration;

use scraper::{Html, Selector};
use surf::{Client, Config, Url};
use urlencoding::encode;

use crate::{
    constants::{BASE_URL, END_OF_TONONKIRA_1, END_OF_TONONKIRA_2},
    types::{Lyrics, Options},
};

pub struct Parser {
    client: Client,
}

impl Default for Parser {
    fn default() -> Self {
        Self::new()
    }
}

impl Parser {
    pub fn new() -> Self {
        let client: Client = Config::new()
            .set_base_url(Url::parse(BASE_URL).unwrap())
            .set_timeout(Some(Duration::from_secs(10)))
            .try_into()
            .unwrap();
        Self { client }
    }

    pub async fn search(
        &self,
        keywords: &str,
        options: Options,
    ) -> Result<Vec<Lyrics>, surf::Error> {
        let mut paths: Vec<String> = Vec::new();

        let has_artist = options.is_artist_search && options.artist.as_deref().map(|s| !s.trim().is_empty()).unwrap_or(false);
        let has_title = options.is_title_search && options.title.as_deref().map(|s| !s.trim().is_empty()).unwrap_or(false);
        let has_lyrics = options.is_lyrics_search && options.lyrics.as_deref().map(|s| !s.trim().is_empty()).unwrap_or(false);

        if has_artist && has_title {
            paths.push(format!(
                "/hira?lohateny={}&mpihira={}",
                encode(options.title.as_deref().unwrap_or("")),
                encode(options.artist.as_deref().unwrap_or(""))
            ));
        } else if has_artist {
            paths.push(format!(
                "/hira?mpihira={}",
                encode(options.artist.as_deref().unwrap_or(""))
            ));
        } else if has_title {
            paths.push(format!(
                "/hira?lohateny={}",
                encode(options.title.as_deref().unwrap_or(""))
            ));
        } else if has_lyrics {
            paths.push(format!(
                "/hira?tononkira={}",
                encode(options.lyrics.as_deref().unwrap_or(""))
            ));
        } else if !keywords.trim().is_empty() {
            // General keywords: search both by song title and by artist
            paths.push(format!("/hira?lohateny={}", encode(keywords.trim())));
            paths.push(format!("/hira?mpihira={}", encode(keywords.trim())));
        } else {
            paths.push("/hira".to_string());
        }

        let mut all_artists: Vec<String> = Vec::new();
        let mut all_titles: Vec<String> = Vec::new();
        let mut all_artist_urls: Vec<String> = Vec::new();
        let mut all_title_urls: Vec<String> = Vec::new();
        let mut seen_urls: HashSet<String> = HashSet::new();

        for path in paths {
            if let Ok(page) = self.client.get(&path).recv_string().await {
                let document = Html::parse_document(&page);
                if let Ok((artists, titles, artist_urls, title_urls)) =
                    self.parse_artist_and_title(&document).await
                {
                    for (i, t_url) in title_urls.into_iter().enumerate() {
                        if seen_urls.insert(t_url.clone()) {
                            all_titles.push(titles[i].clone());
                            all_artists.push(artists[i].clone());
                            all_artist_urls.push(artist_urls[i].clone());
                            all_title_urls.push(t_url);
                        }
                    }
                }
            }
        }

        let mut lyrics: Vec<Lyrics> = Vec::new();

        for (i, artist) in all_artists.iter().enumerate() {
            let mut lines: Vec<String> = Vec::new();
            if all_artists.len() < 5 {
                if let Ok(parsed_lines) = self.parse_lyrics(&all_title_urls[i]).await {
                    lines = parsed_lines;
                }
            }
            lyrics.push(Lyrics {
                artist: artist.to_string(),
                title: all_titles[i].to_string(),
                artist_url: all_artist_urls[i].to_string(),
                title_url: all_title_urls[i].to_string(),
                lines,
            });
        }

        Ok(lyrics)
    }

    pub async fn parse_artist_and_title(
        &self,
        document: &Html,
    ) -> Result<(Vec<String>, Vec<String>, Vec<String>, Vec<String>), ()> {
        let card_selector = Selector::parse(".border.p-2.mb-3, td, .list-group-item").unwrap();
        let a_selector = Selector::parse("a").unwrap();

        let mut artists: Vec<String> = Vec::new();
        let mut titles: Vec<String> = Vec::new();
        let mut artist_urls: Vec<String> = Vec::new();
        let mut title_urls: Vec<String> = Vec::new();

        for card in document.select(&card_selector) {
            let mut card_title = None;
            let mut card_title_url = None;
            let mut card_artist = None;
            let mut card_artist_url = None;

            for a in card.select(&a_selector) {
                if let Some(href) = a.value().attr("href") {
                    if href.contains("/ankafizo")
                        || href.contains("/ankafiziko")
                        || href.contains("/nampidiriko")
                        || href.contains("tononkira/fitadiavana")
                        || href.contains("/ahitsio")
                    {
                        continue;
                    }

                    let text: String = a.text().collect::<Vec<_>>().join(" ").trim().to_string();
                    if text.is_empty() {
                        continue;
                    }

                    if href.contains("/hira/") {
                        card_title = Some(text);
                        card_title_url = Some(href.to_string());
                    } else if href.contains("/mpihira/") {
                        card_artist = Some(text);
                        card_artist_url = Some(href.to_string());
                    }
                }
            }

            if let (Some(title), Some(t_url), Some(artist), Some(a_url)) =
                (card_title, card_title_url, card_artist, card_artist_url)
            {
                titles.push(title);
                title_urls.push(t_url);
                artists.push(artist);
                artist_urls.push(a_url);
            }
        }

        // Fallback: look for /hira/ links if cards were not matched
        if titles.is_empty() {
            for a in document.select(&a_selector) {
                if let Some(href) = a.value().attr("href") {
                    if href.contains("/hira/")
                        && !href.contains("/ankafizo")
                        && !href.contains("/ankafiziko")
                        && !href.contains("/nampidiriko")
                        && !href.contains("/ahitsio")
                        && !href.contains("hira/rehetra")
                    {
                        let text = a.text().collect::<Vec<_>>().join(" ").trim().to_string();
                        if !text.is_empty() && !text.starts_with("Hira") {
                            titles.push(text);
                            title_urls.push(href.to_string());
                            artists.push("Mpihira".to_string());
                            artist_urls.push(String::new());
                        }
                    }
                }
            }
        }

        Ok((artists, titles, artist_urls, title_urls))
    }

    pub async fn parse_lyrics(&self, link: &str) -> Result<Vec<String>, surf::Error> {
        let clean_link = if let Some(stripped) = link.strip_prefix("https://tononkira.serasera.org") {
            stripped
        } else if let Some(stripped) = link.strip_prefix("http://tononkira.serasera.org") {
            stripped
        } else {
            link
        };

        let path = if clean_link.starts_with('/') {
            clean_link.to_string()
        } else {
            format!("/{}", clean_link)
        };

        let page = self.client.get(&path).recv_string().await?;
        let document = Html::parse_document(&page);

        let col_selector = Selector::parse(".col-md-8").unwrap();
        let div_selector = Selector::parse("div").unwrap();

        let mut lyrics_div_text = None;

        if let Some(col) = document.select(&col_selector).next() {
            for div in col.select(&div_selector) {
                let classes = div.value().attr("class").unwrap_or("");
                let id = div.value().attr("id").unwrap_or("");

                if classes.contains("text-end")
                    || classes.contains("print")
                    || classes.contains("no-print")
                    || classes.contains("mw-100")
                    || classes.contains("tooltiptext")
                    || id == "permalink"
                    || id == "copylink"
                {
                    continue;
                }

                let text = div.text().collect::<Vec<_>>().join("");
                if text.trim().len() > 50 {
                    lyrics_div_text = Some(text);
                    break;
                }
            }
        }

        if lyrics_div_text.is_none() {
            for div in document.select(&div_selector) {
                let classes = div.value().attr("class").unwrap_or("");
                let id = div.value().attr("id").unwrap_or("");
                if classes.contains("container")
                    || classes.contains("row")
                    || classes.contains("no-print")
                    || id == "header"
                {
                    continue;
                }
                let text = div.text().collect::<Vec<_>>().join("");
                if text.trim().len() > 100 {
                    lyrics_div_text = Some(text);
                    break;
                }
            }
        }

        let mut lyrics = Vec::new();
        if let Some(raw) = lyrics_div_text {
            for line in raw.lines() {
                let cleaned = line.replace("tononkira.serasera.org", "");
                let trimmed = cleaned.trim();
                if trimmed.is_empty() {
                    continue;
                }
                if trimmed.contains(END_OF_TONONKIRA_1)
                    || trimmed.contains(END_OF_TONONKIRA_2)
                    || trimmed.starts_with("Rohy:")
                    || trimmed.starts_with("Adikao")
                {
                    break;
                }
                lyrics.push(format!("{}\n", trimmed));
            }
        }

        Ok(lyrics)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_parse_artist_and_title() {
        let parser = Parser::new();
        let sample_html = r#"
            <div class="border p-2 mb-3">
                1. <a href="https://tononkira.serasera.org/hira/mahaleo/antanambao">Antanambao</a> -
                <a href="https://tononkira.serasera.org/mpihira/mahaleo">Mahaleo</a>
                <a href="https://tononkira.serasera.org/hira/mahaleo/antanambao/ankafizo">Like</a>
            </div>
        "#;
        let document = Html::parse_document(sample_html);
        let (artists, titles, _artist_urls, title_urls) =
            parser.parse_artist_and_title(&document).await.unwrap();

        assert_eq!(artists.len(), 1);
        assert_eq!(artists[0], "Mahaleo");
        assert_eq!(titles[0], "Antanambao");
        assert_eq!(title_urls[0], "https://tononkira.serasera.org/hira/mahaleo/antanambao");
    }
}
