import sys
import json
import uuid

PLANS = {}


def mk_plan(pkg, manager=None):
    mgr = manager or ("apt" if sys.platform.startswith("linux") else "cargo")
    cmds = {
        "apt": ["sudo apt-get update", f"sudo apt-get install -y {pkg}"],
        "snap": [f"sudo snap install {pkg}"],
        "flatpak": [f"flatpak install -y {pkg}"],
        "pip": [f"pip install {pkg}"],
        "cargo": [f"cargo install {pkg}"],
    }.get(mgr, [f"echo install {pkg} via {mgr}"])
    plan_id = str(uuid.uuid4())
    plan = {"plan_id": plan_id, "manager": mgr, "pkg": pkg, "commands": cmds}
    PLANS[plan_id] = plan
    return plan


def handle(req):
    tool = req.get("tool")
    params = req.get("params", {})
    if tool == "plan_install":
        pkg = params.get("pkg")
        if not pkg:
            return {"ok": False, "error": "pkg required"}
        return {"ok": True, "result": mk_plan(pkg, params.get("manager"))}
    if tool == "explain_install":
        pid = params.get("plan_id")
        p = PLANS.get(pid)
        if not p:
            return {"ok": False, "error": "unknown plan_id"}
        return {"ok": True, "result": {"plan_id": pid, "explain": f"Install {p['pkg']} via {p['manager']}.", "commands": p["commands"]}}
    if tool == "dry_run":
        pid = params.get("plan_id")
        p = PLANS.get(pid)
        if not p:
            return {"ok": False, "error": "unknown plan_id"}
        return {"ok": True, "result": {"plan_id": pid, "dry_run": True, "commands": p["commands"]}}
    if tool == "apply_install":
        pid = params.get("plan_id")
        p = PLANS.get(pid)
        if not p:
            return {"ok": False, "error": "unknown plan_id"}
        return {"ok": True, "result": {"plan_id": pid, "applied": False, "note": "execution gated by core approval"}}
    return {"ok": False, "error": "unknown tool"}


def main():
    line = sys.stdin.readline()
    try:
        req = json.loads(line)
    except Exception as e:
        print(json.dumps({"ok": False, "error": str(e)}))
        return
    resp = handle(req)
    print(json.dumps(resp))


if __name__ == "__main__":
    main()

