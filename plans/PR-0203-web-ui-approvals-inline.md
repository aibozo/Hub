# PR-0203 — Web UI Approvals Inline (Gatekeeper Flow)

Summary: Surface approval prompts inline during chat streams and in the Activity drawer. Allow Approve/Deny to unblock core actions without leaving the chat context.

Dependencies: PR-0202 (Chat MVP), PR-0003 (policy/approvals in core).

Deliverables

- Approval prompt poller: `GET /api/approval/prompt` at a modest interval with backoff.
- Inline `ApprovalBanner` component: shows concise summary of requested action (e.g., shell/git/fs), with Approve/Deny buttons.
- Actions call `/api/approval/answer` (POST) with the ephemeral approval token; show success/failure feedback.
- Activity drawer: approvals history view using `/api/approvals` (if present) or local session event log fallback.
- Error handling: if no prompt available, banner hides; if Denied, stream reports meaningful message.

Wiring Checklist

- Approval banners do not block rendering; they appear between messages and in the drawer.
- Approve/Deny correctly resume or stop the in‑flight agent/tool action.
- Respect policy limits; do not attempt to bypass the gatekeeper.

Tests

- Mock approval prompt → answer → prompt clears; validate banner state transitions.
- Deny path test: stream continues with an error/result message visible to user.

Acceptance Criteria

- When core requests an approval, the banner appears promptly; Approve or Deny reflects in the run and unblocks/halts actions accordingly.
- Approvals log is visible in the Activity drawer.

Out of Scope

- Persisted approvals management UI beyond a basic history view.

Rollback Plan

- Disable the poller; hide the banner and the approvals panel.

References

- docs/POLICY.md (Approvals model)
- docs/WEBUI_SPEC.md (Approvals)

