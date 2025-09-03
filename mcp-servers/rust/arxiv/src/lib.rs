use anyhow::Result;
use chrono::Local;
use chrono::Datelike;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value as JsonValue};

pub mod arxiv;

// --- Public tool handlers ---

pub async fn search(params: &JsonValue) -> Result<JsonValue> {
    let query = params.get("query").and_then(|v| v.as_str()).unwrap_or("");
    let cats: Vec<String> = params
        .get("categories")
        .and_then(|v| v.as_array())
        .map(|a| a.iter().filter_map(|x| x.as_str()).map(|s| s.to_string()).collect())
        .unwrap_or_default();
    let max = params.get("max_results").and_then(|v| v.as_u64()).unwrap_or(25) as usize;
    let from = params.get("from").and_then(|v| v.as_str());
    let client = arxiv::client::ArxivClient::default();
    let res = client.search(query, &cats, from, max).await?;
    Ok(json!({"results": res}))
}

pub async fn top(params: &JsonValue) -> Result<JsonValue> {
    // Return first N papers updated within the given month, using a broad ML-centric category filter to avoid
    // overly broad all:* queries that some mirrors throttle.
    let month = params
        .get("month")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| Local::now().format("%Y-%m").to_string());
    let n = params.get("n").and_then(|v| v.as_u64()).unwrap_or(5) as usize;
    let from = format!("{}-01T00:00:00Z", month);
    // Compute exclusive upper bound (next month)
    let (y, m) = {
        let mut it = month.split('-');
        let y = it.next().and_then(|s| s.parse::<i32>().ok()).unwrap_or(Local::now().year());
        let m = it.next().and_then(|s| s.parse::<u32>().ok()).unwrap_or(Local::now().month());
        (y, m)
    };
    let (ny, nm) = if m >= 12 { (y + 1, 1u32) } else { (y, m + 1) };
    let to = format!("{:04}-{:02}-01T00:00:00Z", ny, nm);

    // Categories: allow caller override; else use a broad default
    let cats: Vec<String> = params
        .get("categories")
        .and_then(|v| v.as_array())
        .map(|a| a.iter().filter_map(|x| x.as_str()).map(|s| s.to_string()).collect())
        .unwrap_or_else(|| vec!["cs.AI","cs.LG","cs.CL","cs.IR","cs.CV","stat.ML"].into_iter().map(|s| s.to_string()).collect());

    let client = arxiv::client::ArxivClient::default();
    // Page through results until we collect N within [from,to), or reach a hard cap
    let mut items: Vec<serde_json::Value> = vec![];
    let mut start: usize = 0;
    let page_size: usize = 50;
    let hard_cap: usize = 1000; // scan up to 1000 entries
    while items.len() < n && start < hard_cap {
        let page = client.search_page("", &cats, start, page_size).await?;
        if page.is_empty() { break; }
        for c in page.iter() {
            let upd = c.updated.as_str();
            if upd >= from.as_str() && upd < to.as_str() {
                items.push(c.compact());
                if items.len() >= n { break; }
            } else if upd < from.as_str() {
                // Since results are sorted desc by updated, once we dip below the window on a page, we may break entire loop
                // but be conservative: continue scanning page and then decide.
            }
        }
        // Early stop: if the last entry on the page is older than from, next pages will be even older
        if let Some(last) = page.last() {
            if last.updated.as_str() < from.as_str() { break; }
        }
        start += page_size;
    }
    Ok(json!({"month": month, "items": items }))
}

pub async fn summarize(_params: &JsonValue) -> Result<JsonValue> {
    // Not implemented yet; return an explicit error instead of a stub.
    anyhow::bail!("summarize not implemented")
}

pub async fn fetch_pdf(params: &JsonValue) -> Result<JsonValue> {
    let id = params.get("id").and_then(|v| v.as_str()).unwrap_or("arXiv:unknown");
    let client = arxiv::client::ArxivClient::default();
    let path = client.download_pdf(id).await?;
    Ok(json!({"path": path}))
}
