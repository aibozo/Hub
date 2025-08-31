import asyncio
import json
import threading
from http.server import BaseHTTPRequestHandler, HTTPServer
from urllib.parse import urlparse, parse_qs

try:
    import aiohttp
    from aiohttp import web
    HAVE_AIOHTTP = True
except Exception:
    HAVE_AIOHTTP = False

try:
    import websockets
    HAVE_WS = True
except Exception:
    HAVE_WS = False

from .audio import SAMPLE_RATE, CHANNELS, pcm_chunks
from .router import select_engine
from .schemas import Health


async def run_server(host: str, port: int):
    engine = select_engine()

    if HAVE_AIOHTTP:
        app = web.Application()

        async def health_handler(request: web.Request):
            h = Health(ok=True, engine=engine.name, sample_rate=SAMPLE_RATE, channels=CHANNELS, tts_ready=True)
            return web.json_response(h.__dict__)

        async def ws_handler(request: web.Request):
            ws = web.WebSocketResponse()
            await ws.prepare(request)
            # Expect a JSON message with { text }
            async for msg in ws:
                if msg.type == aiohttp.WSMsgType.TEXT:
                    try:
                        data = json.loads(msg.data)
                        text = data.get("text", "Hello from voice-daemon")
                        pcm = engine.synthesize_pcm(text)
                        for chunk in pcm_chunks(pcm):
                            await ws.send_bytes(chunk)
                        await ws.close()
                    except Exception as e:
                        await ws.send_json({"error": str(e)})
                        await ws.close()
                        break
            return ws

        app.router.add_get("/v1/tts/health", health_handler)
        app.router.add_get("/v1/tts/stream", ws_handler)
        runner = web.AppRunner(app)
        await runner.setup()
        site = web.TCPSite(runner, host, port)
        print(f"voice-daemon (aiohttp) listening on http://{host}:{port}")
        await site.start()
        while True:
            await asyncio.sleep(3600)

    else:
        # Fallback: start a minimal HTTP server for health only
        class Handler(BaseHTTPRequestHandler):
            def do_GET(self):
                if self.path.startswith("/v1/tts/health"):
                    h = Health(ok=True, engine=engine.name, sample_rate=SAMPLE_RATE, channels=CHANNELS, tts_ready=True)
                    body = json.dumps(h.__dict__).encode("utf-8")
                    self.send_response(200)
                    self.send_header("Content-Type", "application/json")
                    self.send_header("Content-Length", str(len(body)))
                    self.end_headers()
                    self.wfile.write(body)
                elif self.path.startswith("/v1/tts/stream"):
                    # HTTP chunked streaming fallback: text via query string
                    qs = parse_qs(urlparse(self.path).query)
                    text = (qs.get("text") or ["Hello from voice-daemon"])[0]
                    pcm = engine.synthesize_pcm(text)
                    self.send_response(200)
                    self.send_header("Content-Type", "application/octet-stream")
                    self.send_header("Transfer-Encoding", "chunked")
                    self.end_headers()
                    for chunk in pcm_chunks(pcm):
                        size = f"{len(chunk):X}\r\n".encode("ascii")
                        self.wfile.write(size)
                        self.wfile.write(chunk)
                        self.wfile.write(b"\r\n")
                    self.wfile.write(b"0\r\n\r\n")
                else:
                    self.send_response(404)
                    self.end_headers()

        httpd = HTTPServer((host, port), Handler)

        loop = asyncio.get_running_loop()
        await loop.run_in_executor(None, httpd.serve_forever)

