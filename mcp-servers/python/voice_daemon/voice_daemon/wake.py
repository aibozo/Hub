import os
import threading
import time

try:
    import pvporcupine  # type: ignore
    HAVE_PV = True
except Exception:
    pvporcupine = None
    HAVE_PV = False

try:
    import sounddevice as sd  # type: ignore
    HAVE_SD = True
except Exception:
    sd = None
    HAVE_SD = False

import requests  # type: ignore


class WakeThread:
    def __init__(self, base_url: str, refractory_ms: int = 3000):
        self._stop = threading.Event()
        self._thr = None
        self.base_url = base_url.rstrip('/')
        self.refractory_ms = refractory_ms

    def start(self):
        if self._thr and self._thr.is_alive():
            return
        self._thr = threading.Thread(target=self._run, name="porcupine-wake", daemon=True)
        self._thr.start()

    def stop(self):
        self._stop.set()
        if self._thr:
            self._thr.join(timeout=1.0)

    def _run(self):
        if not HAVE_PV or not HAVE_SD:
            print("voice-daemon: wake disabled (pvporcupine or sounddevice not available)")
            return
        access_key = os.environ.get(os.environ.get("PORCUPINE_ACCESS_KEY_ENV", "PICOVOICE_ACCESS_KEY")) or os.environ.get("PICOVOICE_ACCESS_KEY")
        if not access_key:
            print("voice-daemon: wake disabled (PICOVOICE_ACCESS_KEY not set)")
            return
        keyword_path = os.environ.get("PORCUPINE_KEYWORD_PATH")
        if not keyword_path:
            # try directory scan
            d = os.environ.get("PORCUPINE_KEYWORD_DIR", "")
            if d and os.path.isdir(d):
                for name in os.listdir(d):
                    if name.lower().endswith(".ppn"):
                        keyword_path = os.path.join(d, name)
                        break
        if not keyword_path:
            print("voice-daemon: wake disabled (no keyword .ppn found)")
            return
        model_path = os.environ.get("PORCUPINE_MODEL_PATH")
        sensitivity = float(os.environ.get("PORCUPINE_SENSITIVITY", "0.5"))
        try:
            porcupine = pvporcupine.create(access_key=access_key, keyword_paths=[keyword_path], model_path=model_path, sensitivities=[sensitivity])
        except Exception as e:
            print(f"voice-daemon: Porcupine init error: {e}")
            return
        print(f"voice-daemon: Porcupine loaded (sr={porcupine.sample_rate}Hz, frame={porcupine.frame_length}, keyword={os.path.basename(keyword_path)})")
        last = 0.0
        try:
            with sd.RawInputStream(samplerate=porcupine.sample_rate, blocksize=porcupine.frame_length, dtype='int16', channels=1) as stream:
                while not self._stop.is_set():
                    try:
                        frame, _ = stream.read(porcupine.frame_length)
                        if not frame:
                            continue
                        res = porcupine.process(memoryview(frame).cast('h'))
                        if res >= 0:
                            now = time.time()
                            if (now - last) * 1000.0 >= self.refractory_ms:
                                print("voice-daemon: wake detected → calling core /api/realtime/start")
                                last = now
                                try:
                                    body = {
                                        "voice": os.environ.get("FOREMAN_REALTIME_VOICE", "alloy"),
                                        "audio": {"in_sr": 16000, "out_format": "pcm16"}
                                    }
                                    requests.post(f"{self.base_url}/api/realtime/start", json=body, timeout=2)
                                except Exception as e:
                                    print(f"voice-daemon: wake notify error: {e}")
                    except Exception as e:
                        print(f"voice-daemon: wake stream error: {e}")
                        time.sleep(0.1)
        finally:
            try:
                porcupine.delete()
            except Exception:
                pass

