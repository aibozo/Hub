import sys
import json


def handle(req: dict) -> dict:
    tool = req.get("tool")
    params = req.get("params", {})
    if tool == "sources":
        return {"ok": True, "result": {"sources": ["Reuters", "AP", "BBC", "HN"]}}
    if tool == "latest":
        limit = int(params.get("limit", 5))
        items = [{"title": f"Headline {i}", "url": f"https://example.com/{i}"} for i in range(1, limit + 1)]
        return {"ok": True, "result": {"items": items}}
    if tool == "daily_brief":
        cats = params.get("categories", ["world", "tech"])
        md = f"# News Brief\n\n- Categories: {', '.join(cats)}\n- Stub summary.\n"
        return {"ok": True, "result": {"markdown": md}}
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

