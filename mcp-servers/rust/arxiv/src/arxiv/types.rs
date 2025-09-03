use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PaperLink {
    pub html_url: Option<String>,
    pub pdf_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PaperCard {
    pub id: String,            // canonical arXiv id (e.g., 2501.01234)
    pub title: String,
    pub authors: Vec<String>,
    pub primary_category: Option<String>,
    pub updated: String,       // ISO date
    pub summary: Option<String>,
    pub links: PaperLink,
}

impl PaperCard {
    pub fn compact(&self) -> serde_json::Value {
        serde_json::json!({
            "id": self.id,
            "title": self.title,
            "authors": self.authors,
            "primary_category": self.primary_category,
            "updated": self.updated,
            "html_url": self.links.html_url,
            "pdf_url": self.links.pdf_url,
            "summary": self.summary,
        })
    }
}

