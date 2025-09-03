use super::super::types::{PaperMini};

#[derive(Debug, Clone)]
pub struct WorkerOptions {
    pub max_tokens: usize,
}

impl Default for WorkerOptions {
    fn default() -> Self { Self { max_tokens: 512 } }
}

#[derive(Debug, Clone, PartialEq)]
pub struct WorkerNote {
    pub id: String,
    pub claims: String,
    pub methods: String,
    pub caveats: String,
}

/// Deterministic, offline-friendly worker that produces compact notes.
/// In a future PR this can call the model with strict token limits.
pub fn produce_notes_deterministic(papers: &[PaperMini], opts: &WorkerOptions) -> Vec<WorkerNote> {
    let mut out: Vec<WorkerNote> = vec![];
    for p in papers.iter() {
        let take = |s: &str, n: usize| -> String { if s.len() <= n { s.into() } else { format!("{}…", &s[..n.saturating_sub(1)]) } };
        let head = opts.max_tokens.saturating_div(8).max(32);
        let claims = take(&p.title, head);
        let methods = take(p.summary.as_deref().unwrap_or(""), head);
        let caveats = if let Some(cat) = p.html_url.as_ref() { take(cat, 24) } else { String::new() };
        out.push(WorkerNote { id: p.id.clone(), claims, methods, caveats });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn deterministic_notes() {
        let p1 = PaperMini { id: "1".into(), title: "A study of graphs".into(), authors: vec![], updated: "2025-01-01".into(), html_url: Some("http://example/a".into()), pdf_url: None, summary: Some("We explore".into()) };
        let notes = produce_notes_deterministic(&[p1], &WorkerOptions { max_tokens: 64 });
        assert_eq!(notes.len(), 1);
        assert!(notes[0].claims.starts_with("A study"));
    }
}

