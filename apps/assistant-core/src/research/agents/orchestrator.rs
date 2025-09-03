use super::super::types::{ReportBundle, PaperMini};
use super::worker::{produce_notes_deterministic, WorkerOptions};
use super::judge::{rank_and_select, JudgeSelection};

#[derive(Debug, Clone)]
pub struct OrchestratorOptions {
    pub shards: usize,
    pub per_worker_tokens: usize,
    pub top_k: usize,
}

impl Default for OrchestratorOptions {
    fn default() -> Self { Self { shards: 2, per_worker_tokens: 512, top_k: 5 } }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AggregatedOutcome {
    pub selected_ids: Vec<String>,
    pub highlights: Vec<String>,
}

/// Deterministic sharding + worker notes + judge selection.
pub fn orchestrate(bundle: &ReportBundle, opts: &OrchestratorOptions) -> AggregatedOutcome {
    let mut shards: Vec<Vec<PaperMini>> = vec![vec![]; opts.shards.max(1)];
    let n = shards.len();
    for (i, p) in bundle.sources.iter().cloned().enumerate() {
        let idx = i % n;
        if let Some(bucket) = shards.get_mut(idx) { bucket.push(p); }
    }
    // Run workers (deterministically) and collect notes
    let wopts = WorkerOptions { max_tokens: opts.per_worker_tokens };
    let mut all_notes = vec![];
    for shard in shards.into_iter() {
        let notes = produce_notes_deterministic(&shard, &wopts);
        all_notes.extend(notes);
    }
    // Judge selection
    let sel: JudgeSelection = rank_and_select(&all_notes, opts.top_k);
    AggregatedOutcome { selected_ids: sel.top_ids, highlights: sel.highlights }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn orchestrates_deterministically() {
        let mk = |i: usize| PaperMini { id: format!("id{}", i), title: format!("paper {} with longish title", i), authors: vec![], updated: "2025-01-01".into(), html_url: None, pdf_url: None, summary: Some("summary text".into()) };
        let b = ReportBundle { kind: "k".into(), topic: "t".into(), generated_at: "now".into(), sources: (0..10).map(mk).collect() };
        let out = orchestrate(&b, &OrchestratorOptions { shards: 3, per_worker_tokens: 128, top_k: 4 });
        assert_eq!(out.selected_ids.len(), 4);
        // Order stable
        let out2 = orchestrate(&b, &OrchestratorOptions { shards: 3, per_worker_tokens: 128, top_k: 4 });
        assert_eq!(out.selected_ids, out2.selected_ids);
    }
}
