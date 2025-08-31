use serde::Serialize;

#[derive(Debug, Serialize, Clone)]
pub struct Card {
    pub atom_id: i64,
    pub text: String,
    pub tokens_est: usize,
    pub importance: i32,
    pub pinned: bool,
}

#[derive(Debug, Serialize, Clone, Default)]
pub struct Dropped { pub cards: usize, pub expansions: usize }

#[derive(Debug, Serialize)]
pub struct ContextPack {
    pub system_digest: String,
    pub task_digest: Option<String>,
    pub cards: Vec<Card>,
    pub expansions: Vec<String>,
    pub dropped: Dropped,
}

pub fn build_pack(system_digest: &str, task_digest: Option<&str>, mut cards: Vec<Card>, budget_tokens: usize, expansions: Vec<String>) -> ContextPack {
    // naive budget: sum card tokens; drop from end if exceeds
    let mut used = 0usize;
    let mut keep: Vec<Card> = vec![];
    let mut total = 0usize;
    for c in cards.drain(..) {
        total += 1;
        if used + c.tokens_est <= budget_tokens { used += c.tokens_est; keep.push(c); } else { break; }
    }
    let dropped = Dropped { cards: total.saturating_sub(keep.len()), expansions: 0 };
    ContextPack {
        system_digest: system_digest.to_string(),
        task_digest: task_digest.map(|s| s.to_string()),
        cards: keep,
        expansions,
        dropped,
    }
}
