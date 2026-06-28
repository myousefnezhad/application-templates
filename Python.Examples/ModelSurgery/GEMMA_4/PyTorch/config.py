import torch
from dataclasses import dataclass


@dataclass()
class Config:
    """
    Centralised runtime configuration for the token generator.

    NOTE: This is the *serving* config (device, sampling, paths). The actual
    model hyper-parameters live in `model.ModelConfigs` and are read from the
    checkpoint's `config.json` at load time.
    """
    debug_mode: bool = True

    # Point this at a local download of `google/gemma-4-26B-A4B-it`
    # (the official bf16 instruction-tuned checkpoint), e.g. via:
    #   huggingface-cli download google/gemma-4-26B-A4B-it --local-dir ./gemma-4-26B-A4B-it
    checkpoint_path: str = "./gemma-4-26B-A4B-it"

    # Reported by /v1/models and echoed back in responses.
    model_id: str = "gemma-4-12b-it"

    device: torch.device = torch.device(
        "cuda" if torch.cuda.is_available()
        else "mps" if torch.backends.mps.is_available()
        else "cpu"
    )

    # Gemma 4 instruction-tuned models are tuned for sampling; 1.0 is the
    # reference default. Lower it for more deterministic output.
    temperature: float = 0.1
    top_p: float = 0.95

    # 0 -> use the model's full context window (max_position_embeddings).
    max_tokens: int = 4096
