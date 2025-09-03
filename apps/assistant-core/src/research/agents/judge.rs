use super::worker::WorkerNote;

#[derive(Debug, Clone, PartialEq)]
pub struct JudgeSelection {
    pub top_ids: Vec<String>,
    pub highlights: Vec<String>,
}

/// Deterministic aggregator: rank by simple score (title length proxy via claims length),
/// tie-break by id lexicographically.
pub fn rank_and_select(notes: &[WorkerNote], k: usize) -> JudgeSelection {
    let mut scored: Vec<(&WorkerNote, usize)> = notes.iter().map(|n| (n, n.claims.len() + n.methods.len()/2)).collect();
    scored.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.id.cmp(&b.0.id)));
    let mut top_ids: Vec<String> = vec![];
    let mut highlights: Vec<String> = vec![];
    for (note, _) in scored.into_iter().take(k) {
        top_ids.push(note.id.clone());
        let hl = format!("{} — {}", note.id, truncate(&note.claims, 80));
        highlights.push(hl);
    }
    JudgeSelection { top_ids, highlights }
}

fn truncate(s: &str, n: usize) -> String { if s.len() > n { format!("{}…", &s[..n]) } else { s.to_string() } }

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ranks_stably() {
        let mk = |id: &str, claims: &str| WorkerNote { id: id.into(), claims: claims.into(), methods: String::new(), caveats: String::new() };
        let notes = vec![mk("a","short"), mk("b","a very very long claim"), mk("c","middling length")];
        let sel = rank_and_select(&notes, 2);
        assert_eq!(sel.top_ids, vec!["b","c"]);
        assert!(sel.highlights[0].contains("b — a very very"));
    }
}
