import math
from typing import Generator


SAMPLE_RATE = 24000
CHANNELS = 1


def sine_wave_pcm(text: str, duration_s: float = 1.0) -> bytes:
    # Generate a short sine wave; pitch varies with text hash
    freq = 440 + (hash(text) % 400)
    n_samples = int(SAMPLE_RATE * duration_s)
    frames = bytearray()
    for i in range(n_samples):
        t = i / SAMPLE_RATE
        amp = int(0.2 * 32767 * math.sin(2 * math.pi * freq * t))
        frames += int(amp).to_bytes(2, byteorder="little", signed=True)
    return bytes(frames)


def pcm_chunks(pcm: bytes, chunk_ms: int = 50) -> Generator[bytes, None, None]:
    samples_per_chunk = (SAMPLE_RATE * chunk_ms) // 1000
    bytes_per_chunk = samples_per_chunk * 2 * CHANNELS
    for i in range(0, len(pcm), bytes_per_chunk):
        yield pcm[i : i + bytes_per_chunk]

