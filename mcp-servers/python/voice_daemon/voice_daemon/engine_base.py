from abc import ABC, abstractmethod


class TTSEngine(ABC):
    name = "null"

    @abstractmethod
    def synthesize_pcm(self, text: str) -> bytes:
        ...

