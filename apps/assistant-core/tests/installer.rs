use assistant_core::{api, app, config, gatekeeper::ProposedAction};
use axum::{http::Request, body::{Body, to_bytes}};
use tower::ServiceExt;

#[tokio::test]
async fn installer_apply_requires_approval() {
    let state = app::AppState::new(config::Config::default()).await;
    let app_router = api::build_router(state.clone());

    // Create an approval for an installer action and approve it to get a token
    let action = serde_json::json!({"command":"install","writes":true,"paths":["/usr/bin"],"intent":"pkg install"});
    let resp = app_router.clone().oneshot(
        Request::builder().method("POST").uri("/api/approvals").header("content-type","application/json").body(Body::from(action.to_string())).unwrap()
    ).await.unwrap();
    assert!(resp.status().is_success());
    let b = to_bytes(resp.into_body(), 1024*1024).await.unwrap();
    let mut approval: serde_json::Value = serde_json::from_slice(&b).unwrap();
    let id = approval.get("id").and_then(|v| v.as_str()).unwrap().to_string();
    let _ = app_router.clone().oneshot(
        Request::builder().method("POST").uri(format!("/api/approvals/{}/approve", id)).body(Body::empty()).unwrap()
    ).await.unwrap();

    let resp = app_router.clone().oneshot(
        Request::builder().method("GET").uri("/api/approvals").body(Body::empty()).unwrap()
    ).await.unwrap();
    let b = to_bytes(resp.into_body(), 1024*1024).await.unwrap();
    let list: serde_json::Value = serde_json::from_slice(&b).unwrap();
    let token = list.as_array().unwrap()[0].get("token").and_then(|v| v.as_str()).unwrap().to_string();

    // Plan an install
    let plan_req = serde_json::json!({"params": {"pkg": "ripgrep", "manager": "apt"}});
    let resp = app_router.clone().oneshot(
        Request::builder().method("POST").uri("/api/tools/installer/plan_install").header("content-type","application/json").body(Body::from(plan_req.to_string())).unwrap()
    ).await.unwrap();
    assert!(resp.status().is_success());
    let b = to_bytes(resp.into_body(), 1024*1024).await.unwrap();
    let plan: serde_json::Value = serde_json::from_slice(&b).unwrap();
    let plan_id = plan.get("plan_id").and_then(|v| v.as_str()).unwrap().to_string();

    // Apply without token → 409 (ephemeral approval prompt)
    let bad_apply = serde_json::json!({"params": {"plan_id": plan_id}});
    let resp = app_router.clone().oneshot(
        Request::builder().method("POST").uri("/api/tools/installer/apply_install").header("content-type","application/json").body(Body::from(bad_apply.to_string())).unwrap()
    ).await.unwrap();
    assert_eq!(resp.status(), 409);

    // Apply with token → 200
    let good_apply = serde_json::json!({"params": {"plan_id": plan.get("plan_id").unwrap(), "approval_id": id, "approve_token": token}});
    let resp = app_router.clone().oneshot(
        Request::builder().method("POST").uri("/api/tools/installer/apply_install").header("content-type","application/json").body(Body::from(good_apply.to_string())).unwrap()
    ).await.unwrap();
    assert!(resp.status().is_success());
}
