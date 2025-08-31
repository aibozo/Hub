import sys
import json
from datetime import datetime


def handle(req: dict) -> dict:
    tool = req.get("tool")
    params = req.get("params", {})
    if tool == "search":
        q = params.get("query", "")
        results = [
            {"id": f"arXiv:{datetime.utcnow().strftime('%Y.%m')}{i:04}", "title": f"{q} — result {i}"}
            for i in range(1, 6)
        ]
        return {"ok": True, "result": {"results": results}}
    if tool == "top":
        month = params.get("month") or datetime.utcnow().strftime("%Y-%m")
        n = int(params.get("n", 5))
        items = [{"id": f"arXiv:{month.replace('-', '')}.{i:04}", "title": f"Top {month} paper #{i}"} for i in range(1, n + 1)]
        return {"ok": True, "result": {"month": month, "items": items}}
    if tool == "summarize":
        id_ = params.get("id", "unknown")
        return {"ok": True, "result": {"summary": f"Summary for {id_} (stub)"}}
    if tool == "fetch_pdf":
        id_ = params.get("id", "arXiv:unknown").replace(":", "_")
        path = f"storage/arxiv_cache/{id_}.pdf"
        try:
            import os
            os.makedirs("storage/arxiv_cache", exist_ok=True)
            with open(path, "wb") as f:
                f.write(b"%PDF-1.4\n% Stub PDF\n")
        except Exception:
            pass
        return {"ok": True, "result": {"path": path}}
    return {"ok": False, "error": "unknown tool"}


def main() -> None:
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
