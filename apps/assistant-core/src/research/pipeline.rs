use super::pack::pack_bundle;
use super::types::{ReportBundle, ResearchBudgets, ResearchTaskParams};
use crate::tools::ToolsManager;
use serde_json::json;

pub async fn run_pipeline(tm: &ToolsManager, p: &ResearchTaskParams) -> anyhow::Result<ReportBundle> {
    let res = tm.invoke("arxiv", "search", json!({
        "query": p.query,
        "categories": p.categories,
        "max_results": p.limit.max(1).min(50)
    })).await?;
    let items = res.get("results").and_then(|v| v.as_array()).cloned().unwrap_or_default();
    let budgets = if p.budgets.max_papers == 0 { ResearchBudgets::default() } else { p.budgets.clone() };
    Ok(pack_bundle(&p.query, &items, &budgets))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn pipeline_packs_results() {
        // Use ToolsManager fallback stub (no stdio server required)
        let tm = crate::tools::ToolsManager::load_from_dir(std::path::Path::new("config/tools.d"));
        let p = ResearchTaskParams { query: "test".into(), categories: vec![], window_days: 0, limit: 5, budgets: ResearchBudgets { max_papers: 3, max_title_chars: 120, max_summary_chars: 400 } };
        let b = run_pipeline(&tm, &p).await.expect("run");
        assert_eq!(b.sources.len(), 3);
    }
}

