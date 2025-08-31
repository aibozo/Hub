from ..engine_base import TTSEngine
from ..audio import sine_wave_pcm


class NullTTSEngine(TTSEngine):
    name = "null"

    def synthesize_pcm(self, text: str) -> bytes:
        return sine_wave_pcm(text, duration_s=min(2.0, max(0.2, len(text) / 20.0)))

