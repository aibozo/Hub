use super::types::{PaperMini, ReportBundle, ResearchBudgets};

fn clamp_str(s: &str, max: usize) -> String {
    if s.len() <= max { s.to_string() } else { format!("{}…", &s[..max.saturating_sub(1)]) }
}

pub fn pack_bundle(topic: &str, items: &[serde_json::Value], budgets: &ResearchBudgets) -> ReportBundle {
    let mut minis: Vec<PaperMini> = vec![];
    for it in items.iter().take(budgets.max_papers) {
        let id = it.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let title = clamp_str(it.get("title").and_then(|v| v.as_str()).unwrap_or(""), budgets.max_title_chars);
        let authors = it.get("authors").and_then(|v| v.as_array()).map(|a| a.iter().filter_map(|x| x.as_str()).map(|s| s.to_string()).collect()).unwrap_or_else(|| vec![]);
        let updated = it.get("updated").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let html_url = it.get("html_url").and_then(|v| v.as_str()).map(|s| s.to_string());
        let pdf_url = it.get("pdf_url").and_then(|v| v.as_str()).map(|s| s.to_string());
        let summary = it.get("summary").and_then(|v| v.as_str()).map(|s| clamp_str(s, budgets.max_summary_chars));
        minis.push(PaperMini { id, title, authors, updated, html_url, pdf_url, summary });
    }
    ReportBundle { kind: "research_report/v1".into(), topic: topic.to_string(), generated_at: chrono::Utc::now().to_rfc3339(), sources: minis }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn trims_and_limits() {
        let budgets = ResearchBudgets { max_papers: 2, max_title_chars: 10, max_summary_chars: 20 };
        let items = vec![
            serde_json::json!({"id":"1","title":"abcdefghijk","authors":["A"],"updated":"2025-01-01","summary":"012345678901234567890","html_url":"u","pdf_url":"p"}),
            serde_json::json!({"id":"2","title":"short","authors":[],"updated":"2025-01-02"}),
            serde_json::json!({"id":"3","title":"extra","authors":[],"updated":"2025-01-03"}),
        ];
        let bundle = pack_bundle("topic", &items, &budgets);
        assert_eq!(bundle.sources.len(), 2);
        assert_eq!(bundle.sources[0].title, "abcdefghi…");
        assert_eq!(bundle.sources[0].summary.as_deref(), Some("0123456789012345678…"));
    }
}

