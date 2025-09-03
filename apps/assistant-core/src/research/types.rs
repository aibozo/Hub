use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchBudgets {
    pub max_papers: usize,
    pub max_title_chars: usize,
    pub max_summary_chars: usize,
}

impl Default for ResearchBudgets {
    fn default() -> Self {
        Self { max_papers: 30, max_title_chars: 160, max_summary_chars: 800 }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchTaskParams {
    pub query: String,
    #[serde(default)]
    pub categories: Vec<String>,
    #[serde(default)]
    pub window_days: usize,
    #[serde(default)]
    pub limit: usize,
    #[serde(default)]
    pub budgets: ResearchBudgets,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaperMini {
    pub id: String,
    pub title: String,
    pub authors: Vec<String>,
    pub updated: String,
    pub html_url: Option<String>,
    pub pdf_url: Option<String>,
    pub summary: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportBundle {
    pub kind: String,
    pub topic: String,
    pub generated_at: String,
    pub sources: Vec<PaperMini>,
}

