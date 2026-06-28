import torch
from dataclasses import dataclass

@dataclass()
class Config:
    """
    Centralised configuration for the token generator.
    """
    debug_mode: bool = True
    checkpoint_path: str = "./gpt-oss-20b/original"
    device: torch.device = torch.device(
        "cuda" if torch.cuda.is_available()
        else "mps" if torch.backends.mps.is_available()
        else "cpu"
    )
    temperature: float = 0.1
    max_tokens: int = 4096  # if set to 0 -> full max context length
