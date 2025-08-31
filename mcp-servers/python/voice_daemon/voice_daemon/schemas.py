from dataclasses import dataclass


@dataclass
class Health:
    ok: bool
    engine: str
    sample_rate: int
    channels: int
    tts_ready: bool

