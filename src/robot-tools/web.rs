use super::{LIMIT, Result, credential};
use serde::Serialize;
use serde_json::Value;
use std::time::Duration;

#[derive(Debug, Serialize, PartialEq)]
pub struct Source {
    pub title: String,
    pub url: String,
    pub description: String,
}

// Provider-independent search; the key lives in orbit-robots / brave-search.
pub fn search(query: &str) -> Result<Vec<Source>> {
    if query.trim().is_empty() || query.len() > 4096 {
        return Err("Enter a web search query of at most 4096 bytes.".into());
    }
    let key = credential("brave-search")?;
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(30)))
        .max_redirects(0)
        .build()
        .into();
    let mut response = agent
        .get("https://api.search.brave.com/res/v1/web/search")
        .query("q", query)
        .query("count", "10")
        .header("Accept", "application/json")
        .header("X-Subscription-Token", &key)
        .call()
        .map_err(|_| "Web search failed. Check the Brave Search credential and connection.")?;
    let value = response
        .body_mut()
        .with_config()
        .limit(LIMIT as u64)
        .read_json::<Value>()
        .map_err(|_| "Invalid or oversized web search response.")?;
    sources(&value)
}

fn sources(value: &Value) -> Result<Vec<Source>> {
    if value.get("web").is_none() && value.get("query").is_some() {
        return Ok(Vec::new());
    }
    let results = value["web"]["results"]
        .as_array()
        .ok_or("Invalid web search results.")?;
    results
        .iter()
        .take(10)
        .map(|result| {
            let text = |field| result[field].as_str().ok_or("Invalid web search source.");
            let url = text("url")?;
            let parsed = url
                .parse::<ureq::http::Uri>()
                .map_err(|_| "Invalid source URL.")?;
            if !matches!(parsed.scheme_str(), Some("https" | "http")) || parsed.host().is_none() {
                return Err("Invalid source URL.".into());
            }
            Ok(Source {
                title: text("title")?.into(),
                url: url.into(),
                description: result["description"].as_str().unwrap_or_default().into(),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn keeps_source_links_and_rejects_non_web_urls() {
        let value = json!({"web":{"results":[{"title":"مصدر", "url":"https://example.org/a?q=1", "description":"Evidence"}]}});
        let result = sources(&value).unwrap();
        assert_eq!(result[0].url, "https://example.org/a?q=1");
        assert_eq!(result[0].title, "مصدر");
        assert!(
            sources(&json!({"web":{"results":[{"title":"x", "url":"file:///etc/passwd"}]}}))
                .is_err()
        );
        assert!(sources(&json!({"error":"failed"})).is_err());
        assert!(
            sources(&json!({"query":{"original":"nothing"}}))
                .unwrap()
                .is_empty()
        );
    }
}
