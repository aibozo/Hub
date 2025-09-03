use super::types::PaperCard;
use anyhow::{anyhow, Result};
use quick_xml::events::Event;
use quick_xml::Reader;
use reqwest::Client;
use std::time::Duration;
// urlencoding no longer used for building the query string; keep import if used elsewhere

#[derive(Clone)]
pub struct ArxivClient {
    http: Client,
    base: String,
}

impl Default for ArxivClient {
    fn default() -> Self {
        let http = Client::builder()
            .user_agent("foreman-mcp-arxiv/0.1")
            .timeout(Duration::from_secs(15))
            .build()
            .expect("reqwest client");
        Self { http, base: "https://export.arxiv.org/api/query".into() }
    }
}

impl ArxivClient {
    fn build_query(q: &str, cats: &[String]) -> String {
        let mut parts: Vec<String> = vec![];
        let q = q.trim();
        if !q.is_empty() {
            let toks: Vec<&str> = q.split_whitespace().collect();
            if toks.len() <= 1 {
                parts.push(format!("all:{}", toks.get(0).copied().unwrap_or(q)));
            } else {
                let anded = toks.into_iter().map(|t| format!("all:{}", t)).collect::<Vec<_>>().join(" AND ");
                parts.push(anded);
            }
        }
        let nonempty: Vec<String> = cats.iter().filter(|c| !c.trim().is_empty()).cloned().collect();
        if !nonempty.is_empty() {
            if nonempty.len() == 1 {
                parts.push(format!("cat:{}", nonempty[0]));
            } else {
                let orcats = nonempty.into_iter().map(|c| format!("cat:{}", c)).collect::<Vec<_>>().join(" OR ");
                parts.push(format!("({})", orcats));
            }
        }
        if parts.is_empty() { "all:*".into() } else { parts.join(" AND ") }
    }

    pub async fn search_page(&self, query: &str, categories: &[String], start: usize, page_size: usize) -> Result<Vec<PaperCard>> {
        use reqwest::header::{ACCEPT, CONTENT_TYPE};
        let search_query = Self::build_query(query, categories);
        let resp = self.http
            .get(&self.base)
            .query(&[("search_query", search_query.as_str())])
            .query(&[("start", start), ("max_results", page_size)])
            .query(&[("sortBy", "lastUpdatedDate"), ("sortOrder", "descending")])
            .header(ACCEPT, "application/atom+xml, application/xml;q=0.9, text/xml;q=0.8")
            .send().await?;
        let status = resp.status();
        let ctype: String = resp.headers().get(CONTENT_TYPE).and_then(|v| v.to_str().ok()).map(|s| s.to_string()).unwrap_or_default();
        if !status.is_success() { return Err(anyhow!("arXiv API error: HTTP {}", status)); }
        if !(ctype.contains("xml") || ctype.contains("atom")) {
            let snip = resp.text().await.unwrap_or_default();
            let mut preview = snip.trim().to_string();
            if preview.len() > 200 { preview.truncate(200); preview.push('…'); }
            return Err(anyhow!("arXiv API unexpected content-type: {} body: {}", ctype, preview));
        }
        let text = resp.text().await?;
        let cards = parse_atom_feed(&text)?;
        Ok(cards)
    }
    pub fn with_base(base: &str) -> Self {
        let mut s = Self::default();
        s.base = base.to_string();
        s
    }

    pub async fn search(
        &self,
        query: &str,
        categories: &[String],
        from_date: Option<&str>,
        max_results: usize,
    ) -> Result<Vec<serde_json::Value>> {
        let search_query = Self::build_query(query, categories);
        let mut collected: Vec<serde_json::Value> = vec![];
        let mut start: usize = 0;
        // If we apply a client-side from_date filter, request a bit extra headroom (up to 3x)
        let max_fetch = (max_results.saturating_mul(3)).clamp(1, 150);
        use reqwest::header::{ACCEPT, CONTENT_TYPE};
        loop {
            let page_size = (max_fetch - start).min(50);
            if page_size == 0 { break; }
            let resp = self.http
                .get(&self.base)
                .query(&[("search_query", search_query.as_str())])
                .query(&[("start", start), ("max_results", page_size)])
                .query(&[("sortBy", "lastUpdatedDate"), ("sortOrder", "descending")])
                .header(ACCEPT, "application/atom+xml, application/xml;q=0.9, text/xml;q=0.8")
                .send().await?;
            let status = resp.status();
            let ctype: String = resp.headers().get(CONTENT_TYPE).and_then(|v| v.to_str().ok()).map(|s| s.to_string()).unwrap_or_default();
            if !status.is_success() {
                return Err(anyhow!("arXiv API error: HTTP {}", status));
            }
            // arXiv should return some XML content type; if clearly HTML/plain, surface an error early.
            if !(ctype.contains("xml") || ctype.contains("atom")) {
                let snip = resp.text().await.unwrap_or_default();
                let mut preview = snip.trim().to_string();
                if preview.len() > 200 { preview.truncate(200); preview.push('…'); }
                return Err(anyhow!("arXiv API unexpected content-type: {} body: {}", ctype, preview));
            }
            let text = resp.text().await?;
            let mut cards = parse_atom_feed(&text)?;
            if let Some(from) = from_date {
                cards.retain(|c| c.updated.as_str() >= from);
            }
            for c in cards.into_iter() {
                collected.push(c.compact());
                if collected.len() >= max_results { return Ok(collected); }
            }
            // If the page came back empty (no more results), stop
            if page_size < 50 { break; }
            start = start.saturating_add(page_size);
            if start >= max_fetch { break; }
        }
        Ok(collected)
    }

    pub async fn download_pdf(&self, id: &str) -> Result<String> {
        let id_core = normalize_id(id).ok_or_else(|| anyhow!("invalid arXiv id"))?;
        let url = format!("https://arxiv.org/pdf/{}.pdf", id_core);
        let resp = self.http.get(&url).send().await?;
        let status = resp.status();
        if !status.is_success() { return Err(anyhow!("arXiv PDF fetch error: HTTP {}", status)); }
        let bytes = resp.bytes().await?;
        let base = std::path::PathBuf::from("storage/artifacts/papers/arxiv").join(&id_core);
        tokio::fs::create_dir_all(&base).await?;
        let path = base.join(format!("{}.pdf", id_core));
        tokio::fs::write(&path, &bytes).await?;
        // meta.json (minimal)
        let meta = serde_json::json!({"id": id_core, "source": "arxiv", "pdf": path.to_string_lossy()});
        tokio::fs::write(base.join("meta.json"), serde_json::to_vec_pretty(&meta)?).await?;
        Ok(path.to_string_lossy().to_string())
    }
}

fn normalize_id(id: &str) -> Option<String> {
    // Accept forms: "arXiv:YYMM.NNNNN", "YYMM.NNNNN", with optional version; strip version
    let s = id.trim().strip_prefix("arXiv:").unwrap_or(id).trim();
    let core = s.split('v').next().unwrap_or(s);
    // Basic validation: digits '.' digits
    if core.contains('.') && core.chars().all(|ch| ch.is_ascii_digit() || ch == '.') {
        Some(core.to_string())
    } else {
        None
    }
}

fn parse_atom_feed(xml: &str) -> Result<Vec<PaperCard>> {
    let mut reader = Reader::from_str(xml);
    reader.trim_text(true);
    let mut buf = Vec::new();
    let mut out: Vec<PaperCard> = vec![];

    let mut in_entry = false;
    let mut cur_id = String::new();
    let mut cur_title = String::new();
    let mut cur_updated = String::new();
    let mut cur_summary: Option<String> = None;
    let mut cur_authors: Vec<String> = vec![];
    let mut cur_primary: Option<String> = None;
    let mut cur_html: Option<String> = None;
    let mut cur_pdf: Option<String> = None;
    let mut text_target: Option<&'static str> = None;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let name_buf: Vec<u8> = e.name().as_ref().to_vec();
                let raw = name_buf.as_slice();
                let colon = raw.iter().position(|b| *b == b':');
                let name = match colon { Some(ix) => &raw[ix+1..], None => raw };
                match name {
                    b"entry" => {
                        in_entry = true;
                        cur_id.clear(); cur_title.clear(); cur_updated.clear(); cur_summary = None; cur_authors.clear(); cur_primary = None; cur_html = None; cur_pdf = None; text_target = None;
                    }
                    b"id" if in_entry => { text_target = Some("id"); }
                    b"title" if in_entry => { text_target = Some("title"); }
                    b"updated" if in_entry => { text_target = Some("updated"); }
                    b"summary" if in_entry => { text_target = Some("summary"); }
                    b"name" if in_entry => { text_target = Some("author"); }
                    b"primary_category" if in_entry => {
                        for a in e.attributes().flatten() {
                            let k = a.key.as_ref();
                            if k.ends_with(b"term") { cur_primary = Some(String::from_utf8_lossy(&a.value).to_string()); }
                        }
                    }
                    b"link" if in_entry => {
                        let mut rel: Option<String> = None;
                        let mut href: Option<String> = None;
                        let mut typ: Option<String> = None;
                        let mut title_attr: Option<String> = None;
                        for a in e.attributes().flatten() {
                            let k = a.key.as_ref();
                            let v = String::from_utf8_lossy(&a.value).to_string();
                            match k {
                                b"rel" => rel = Some(v),
                                b"href" => href = Some(v),
                                b"type" => typ = Some(v),
                                b"title" => title_attr = Some(v),
                                _ => {}
                            }
                        }
                        if let Some(h) = href {
                            if rel.as_deref() == Some("alternate") && cur_html.is_none() {
                                cur_html = Some(h);
                            } else if typ.as_deref().unwrap_or("").contains("pdf") && cur_pdf.is_none() {
                                cur_pdf = Some(h);
                            } else if title_attr.as_deref().map(|s| s.eq_ignore_ascii_case("pdf")).unwrap_or(false) && cur_pdf.is_none() {
                                cur_pdf = Some(h);
                            }
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::Text(t)) => {
                if let Some(tag) = text_target.take() {
                    let txt = t.unescape().unwrap_or_default().to_string();
                    match tag {
                        "id" => cur_id = txt,
                        "title" => cur_title = txt,
                        "updated" => cur_updated = txt,
                        "summary" => cur_summary = Some(txt),
                        "author" => cur_authors.push(txt),
                        _ => {}
                    }
                }
            }
            Ok(Event::End(e)) => {
                let end_name_buf: Vec<u8> = e.name().as_ref().to_vec();
                let raw = end_name_buf.as_slice();
                let colon = raw.iter().position(|b| *b == b':');
                let name = match colon { Some(ix) => &raw[ix+1..], None => raw };
                if name == b"entry" && in_entry {
                    in_entry = false;
                    // Normalize id
                    let id_norm = cur_id
                        .rsplit('/')
                        .next()
                        .unwrap_or(&cur_id)
                        .trim()
                        .trim_start_matches("abs/")
                        .to_string();
                    let id_core = id_norm.strip_prefix("arXiv:").unwrap_or(&id_norm).split('v').next().unwrap_or(&id_norm).to_string();
                    let card = PaperCard {
                        id: id_core,
                        title: cur_title.clone(),
                        authors: cur_authors.clone(),
                        primary_category: cur_primary.clone(),
                        updated: cur_updated.clone(),
                        summary: cur_summary.clone(),
                        links: super::types::PaperLink { html_url: cur_html.clone(), pdf_url: cur_pdf.clone() },
                    };
                    out.push(card);
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(anyhow!("XML parse error: {}", e)),
            _ => {}
        }
        buf.clear();
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    const SAMPLE: &str = r#"<?xml version=\"1.0\" encoding=\"UTF-8\"?>
<feed xmlns=\"http://www.w3.org/2005/Atom\">
  <entry>
    <id>http://arxiv.org/abs/2501.01234v1</id>
    <updated>2025-01-15T12:00:00Z</updated>
    <title>Mixture-of-Experts routing</title>
    <summary>We study...</summary>
    <author><name>Doe, J.</name></author>
    <author><name>Smith, A.</name></author>
    <link rel=\"alternate\" type=\"text/html\" href=\"https://arxiv.org/abs/2501.01234\"/>
    <link title=\"pdf\" href=\"https://arxiv.org/pdf/2501.01234.pdf\"/>
    <arxiv:primary_category xmlns:arxiv=\"http://arxiv.org/schemas/atom\" term=\"cs.LG\"/>
  </entry>
</feed>
"#;

    #[test]
    fn parse_basic() {
        let cards = parse_atom_feed(SAMPLE).expect("parse");
        assert_eq!(cards.len(), 1);
        let c = &cards[0];
        assert_eq!(c.id, "2501.01234");
        assert_eq!(c.title, "Mixture-of-Experts routing");
        assert_eq!(c.authors, vec!["Doe, J.", "Smith, A."]);
        assert_eq!(c.primary_category.as_deref(), Some("cs.LG"));
        assert_eq!(c.updated, "2025-01-15T12:00:00Z");
        assert_eq!(c.links.html_url.as_deref(), Some("https://arxiv.org/abs/2501.01234"));
        assert_eq!(c.links.pdf_url.as_deref(), Some("https://arxiv.org/pdf/2501.01234.pdf"));
    }
}
