import torch
from dataclasses import dataclass

# Resolve the device once, at import time, so that `Config.device` is a real
# torch.device when accessed at the class level (the rest of the code uses
# `Config.device` without instantiating Config).
if torch.cuda.is_available():
    _DEVICE = torch.device("cuda")
elif torch.backends.mps.is_available():
    _DEVICE = torch.device("mps")
else:
    _DEVICE = torch.device("cpu")


@dataclass()
class Config:
    """Runtime / generation configuration for the Qwen3.5 token generator."""
    debug_mode: bool = True

    # Local checkpoint directory: config.json + *.safetensors + tokenizer files.
    checkpoint_path: str = "../Qwen3.5-35B-A3B"

    device: torch.device = _DEVICE

    # Sampling
    temperature: float = 0.7
    top_p: float = 0.8
    top_k: int = 20
    max_tokens: int = 1024  # if set to 0 -> use the model's max_position_embeddings

    # Wrap the prompt with the Qwen chat template (<|im_start|> ...).
    use_chat_template: bool = True
