from .engines.null_tts import NullTTSEngine


def select_engine(config: dict | None = None):
    # TODO: use config and license gating to choose engines
    return NullTTSEngine()

