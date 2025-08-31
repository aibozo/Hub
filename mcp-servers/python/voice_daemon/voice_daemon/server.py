import asyncio
import json
import threading
from http.server import BaseHTTPRequestHandler, HTTPServer
from urllib.parse import urlparse, parse_qs
import os

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

try:
    import numpy as np
except Exception:
    np = None

try:
    import whisper
    HAVE_WHISPER = True
except Exception:
    whisper = None
    HAVE_WHISPER = False
try:
    import torch
except Exception:
    torch = None

_stt_model = None
_stt_device = None
_stt_loading = False

def get_stt_model():
    global _stt_model
    global _stt_device
    global _stt_loading
    if _stt_model is None and HAVE_WHISPER:
        model_name = os.environ.get("WHISPER_MODEL", "base.en")
        # Select device: env override or auto-detect
        device = os.environ.get("WHISPER_DEVICE")
        if device is None:
            if torch is not None and hasattr(torch, 'cuda') and torch.cuda.is_available():
                device = 'cuda'
            elif torch is not None and hasattr(torch.backends, 'mps') and torch.backends.mps.is_available():
                device = 'mps'
            else:
                device = 'cpu'
        try:
            _stt_loading = True
            _stt_model = whisper.load_model(model_name, device=device)
            _stt_device = device
            msg = f"voice-daemon: loaded whisper model '{model_name}' on device '{device}'"
            if device == 'cuda' and torch is not None:
                try:
                    name = torch.cuda.get_device_name(0)
                    msg += f" (GPU: {name})"
                except Exception:
                    pass
            print(msg)
        except Exception as e:
            print(f"voice-daemon: failed to load whisper model '{model_name}': {e}")
            _stt_model = None
            _stt_device = None
        finally:
            _stt_loading = False
    return _stt_model


async def run_server(host: str, port: int):
    engine = select_engine()

    if HAVE_AIOHTTP:
        app = web.Application()

        async def health_handler(request: web.Request):
            h = Health(ok=True, engine=engine.name, sample_rate=SAMPLE_RATE, channels=CHANNELS, tts_ready=True)
            return web.json_response(h.__dict__)

        async def stt_health_handler(request: web.Request):
            # Do not block on model load here; just report current state
            ready = HAVE_WHISPER and (_stt_model is not None) and (np is not None)
            return web.json_response({
                "ok": True,
                "stt_ready": bool(ready),
                "have_whisper": bool(HAVE_WHISPER),
                "have_numpy": bool(np is not None),
                "model": os.environ.get("WHISPER_MODEL", "base.en"),
                "device": _stt_device,
                "loading": _stt_loading,
            })

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

        async def stt_transcribe_handler(request: web.Request):
            if not HAVE_WHISPER:
                return web.json_response({"error": "openai-whisper not installed"}, status=503)
            if np is None:
                return web.json_response({"error": "numpy not installed"}, status=503)
            model = get_stt_model()
            if model is None:
                return web.json_response({"error": "STT model failed to load"}, status=500)
            data = await request.read()
            if not data:
                return web.json_response({"error": "empty body"}, status=400)
            try:
                # Decode PCM16 mono little-endian to float32 -1..1
                arr = np.frombuffer(data, dtype=np.int16).astype(np.float32) / 32768.0
                # Transcribe
                lang = request.query.get("language") or os.environ.get("WHISPER_LANG")
                opts = {"language": lang} if lang else {}
                # Use fp16 on CUDA only for speed; CPU/MPS prefer fp32
                if _stt_device in (None, 'cpu', 'mps'):
                    opts["fp16"] = False
                res = model.transcribe(arr, **opts)
                text = res.get("text", "")
                return web.json_response({"text": text})
            except Exception as e:
                return web.json_response({"error": str(e)}, status=500)

        app.router.add_get("/v1/tts/health", health_handler)
        app.router.add_get("/v1/tts/stream", ws_handler)
        app.router.add_get("/v1/stt/health", stt_health_handler)
        app.router.add_post("/v1/stt/transcribe", stt_transcribe_handler)
        # Optionally preload model in background
        if HAVE_WHISPER and os.environ.get("WHISPER_PRELOAD", "1") not in ("0","false","False"):
            async def _preload():
                loop = asyncio.get_running_loop()
                await loop.run_in_executor(None, get_stt_model)
            asyncio.create_task(_preload())
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
                elif self.path.startswith("/v1/stt/health"):
                    ready = HAVE_WHISPER and (_stt_model is not None) and np is not None
                    body = json.dumps({
                        "ok": True,
                        "stt_ready": bool(ready),
                        "have_whisper": bool(HAVE_WHISPER),
                        "have_numpy": bool(np is not None),
                        "model": os.environ.get("WHISPER_MODEL", "base.en"),
                        "device": _stt_device,
                        "loading": _stt_loading,
                    }).encode("utf-8")
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

            def do_POST(self):
                if self.path.startswith("/v1/stt/transcribe"):
                    if not HAVE_WHISPER or np is None:
                        body = json.dumps({"error": "STT not available"}).encode("utf-8")
                        self.send_response(503)
                        self.send_header("Content-Type", "application/json")
                        self.send_header("Content-Length", str(len(body)))
                        self.end_headers()
                        self.wfile.write(body)
                        return
                    model = get_stt_model()
                    if model is None:
                        body = json.dumps({"error": "STT model failed to load"}).encode("utf-8")
                        self.send_response(500)
                        self.send_header("Content-Type", "application/json")
                        self.send_header("Content-Length", str(len(body)))
                        self.end_headers()
                        self.wfile.write(body)
                        return
                    try:
                        length = int(self.headers.get('Content-Length') or '0')
                        data = self.rfile.read(length)
                        arr = np.frombuffer(data, dtype=np.int16).astype(np.float32) / 32768.0
                        lang = None
                        opts = {"language": lang} if lang else {}
                        if _stt_device in (None, 'cpu', 'mps'):
                            opts["fp16"] = False
                        res = model.transcribe(arr, **opts)
                        text = res.get("text", "")
                        body = json.dumps({"text": text}).encode("utf-8")
                        self.send_response(200)
                        self.send_header("Content-Type", "application/json")
                        self.send_header("Content-Length", str(len(body)))
                        self.end_headers()
                        self.wfile.write(body)
                    except Exception as e:
                        body = json.dumps({"error": str(e)}).encode("utf-8")
                        self.send_response(500)
                        self.send_header("Content-Type", "application/json")
                        self.send_header("Content-Length", str(len(body)))
                        self.end_headers()
                        self.wfile.write(body)

        httpd = HTTPServer((host, port), Handler)

        loop = asyncio.get_running_loop()
        await loop.run_in_executor(None, httpd.serve_forever)
