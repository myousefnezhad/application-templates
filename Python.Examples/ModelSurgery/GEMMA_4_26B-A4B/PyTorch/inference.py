import time
import json
import torch
from config import Config
from model import Transformer
from schemas import ToolCall, ToolFunction, GenerationResult
from typing import Optional, List, Any, Generator, Union, Tuple
from transformers import AutoTokenizer


def debug_print(*args: Any, **kwargs: Any) -> None:
    if Config.debug_mode:
        print("[DEBUG]", *args, **kwargs)


def get_tokenizer(checkpoint: str = Config.checkpoint_path):
    """
    Load the Gemma 4 tokenizer (SentencePiece + chat template) straight from
    the checkpoint directory. This replaces the GPT-OSS Harmony encoding.
    """
    return AutoTokenizer.from_pretrained(checkpoint)


def parse_tool_call(text: str) -> Optional[ToolCall]:
    """
    Parse a Gemma 4 tool call. The instruction-tuned models emit calls in a
    dedicated channel, roughly:
        <|tool_call>call:tool_name{arg:value,...}<tool_call|>
    We do a best-effort extraction of the name and a JSON-ish argument blob.

    VERIFY: the exact tool-call token spelling/format depends on the
    tokenizer's chat_template; adjust the markers below if your template
    differs (some builds use `<|tool_call>` / `<tool_call|>`, others vary).
    """
    import re
    m = re.search(r"call:([a-zA-Z0-9_\-]+)\s*\{(.*?)\}", text, re.DOTALL)
    if m is None:
        return None
    name = m.group(1)
    body = m.group(2).strip()
    args: dict = {}
    # Try strict JSON first, then a permissive key:value parse.
    try:
        args = json.loads("{" + body + "}")
    except json.JSONDecodeError:
        for pair in body.split(","):
            if ":" in pair:
                k, v = pair.split(":", 1)
                args[k.strip().strip('"')] = v.strip().strip('"')
    return ToolCall(
        id=None,
        type="function",
        function=ToolFunction(name=name, arguments=json.dumps(args)),
    )


class TokenGenerator:
    _model: Optional[Transformer] = None
    _tokenizer = None

    def __init__(self, checkpoint: str = Config.checkpoint_path, device: torch.device = Config.device):
        self.device = device

        if TokenGenerator._model is None:
            debug_print(f"Loading Gemma 4 weights from {checkpoint}...")
            start = time.time()
            TokenGenerator._model = Transformer.from_checkpoint(checkpoint, device=device)
            print(f"\u2713 Model weights loaded in {time.time() - start:.2f}s")
        else:
            print("Model weights already loaded. Reusing existing instance.")
        self.model: Transformer = TokenGenerator._model

        if TokenGenerator._tokenizer is None:
            print("Loading tokenizer...")
            TokenGenerator._tokenizer = get_tokenizer(checkpoint)
        self.tokenizer = TokenGenerator._tokenizer

        # Gemma stop tokens: <end_of_turn> (106) ends an assistant turn; <eos> (1).
        self.eot_token = self._tok_id("<end_of_turn>", default=106)
        self.eos_token = self.tokenizer.eos_token_id or 1
        self.stop_tokens = sorted({t for t in (self.eot_token, self.eos_token) if t is not None})
        debug_print(f"stop tokens: {self.stop_tokens}")

    def _tok_id(self, piece: str, default: Optional[int] = None) -> Optional[int]:
        tid = self.tokenizer.convert_tokens_to_ids(piece)
        unk = getattr(self.tokenizer, "unk_token_id", None)
        if tid is None or tid == unk:
            return default
        return tid

    @torch.inference_mode()
    def generate(
        self,
        prompt_tokens: List[int],
        stop_tokens: Optional[List[int]] = None,
        temperature: float = Config.temperature,
        top_p: float = Config.top_p,
        max_tokens: int = Config.max_tokens,
        return_logprobs: bool = False,
    ) -> Generator[Union[int, Tuple[int, float]], None, None]:

        cfg = self.model.configs
        stop_tokens = stop_tokens if stop_tokens is not None else self.stop_tokens

        max_gen = max_tokens if max_tokens > 0 else cfg.max_position_embeddings
        cache_size = min(len(prompt_tokens) + max_gen, cfg.max_position_embeddings)

        print("Initialising KV caches...")
        print(f"  - Cache size : {cache_size} tokens")
        print(f"  - Layers     : {cfg.num_hidden_layers} (global: {sum(cfg.is_global(i) for i in range(cfg.num_hidden_layers))})")
        cache_start = time.time()
        caches = self.model.build_caches(batch_size=1, max_len=cache_size, device=self.device)
        print(f"\u2713 Caches initialised in {time.time() - cache_start:.2f}s")

        tokens = list(prompt_tokens)
        print(f"Prompt length: {len(tokens)} tokens")

        # ---- Prefill ----
        prefill_start = time.time()
        input_tensor = torch.as_tensor([tokens], dtype=torch.long, device=self.device)
        logits = self.model(input_tensor, caches=caches)[:, -1, :].squeeze(0)
        print(f"\u2713 Prefill complete in {time.time() - prefill_start:.2f}s")

        # ---- Decode ----
        num_generated = 0
        predicted = None
        while max_tokens == 0 or num_generated < max_tokens:
            if num_generated > 0:
                input_tensor = torch.as_tensor([[predicted]], dtype=torch.long, device=self.device)
                logits = self.model(input_tensor, caches=caches)[:, -1, :].squeeze(0)

            predicted = self._sample(logits, temperature, top_p)
            tokens.append(predicted)
            num_generated += 1

            if return_logprobs:
                logprob = torch.log_softmax(logits, dim=-1)[predicted].item()
                yield predicted, logprob
            else:
                yield predicted

            if predicted in stop_tokens:
                print("Stop token encountered, ending generation")
                break

    @staticmethod
    def _sample(logits: torch.Tensor, temperature: float, top_p: float) -> int:
        if temperature == 0.0:
            return int(torch.argmax(logits, dim=-1).item())
        probs = torch.softmax(logits.float() / temperature, dim=-1)
        if 0.0 < top_p < 1.0:
            sorted_probs, sorted_idx = torch.sort(probs, descending=True)
            cumulative = torch.cumsum(sorted_probs, dim=-1)
            keep = cumulative - sorted_probs <= top_p
            sorted_probs = sorted_probs * keep
            sorted_probs = sorted_probs / sorted_probs.sum()
            choice = torch.multinomial(sorted_probs, num_samples=1)
            return int(sorted_idx[choice].item())
        return int(torch.multinomial(probs, num_samples=1).item())

    @torch.inference_mode()
    def generate_text(
        self,
        prompt_tokens: List[int],
        stop_tokens: Optional[List[int]] = None,
        temperature: float = Config.temperature,
        max_tokens: int = Config.max_tokens,
    ) -> GenerationResult:
        out = [t for t in self.generate(
            prompt_tokens=prompt_tokens,
            stop_tokens=stop_tokens,
            temperature=temperature,
            max_tokens=max_tokens,
            return_logprobs=False,
        )]
        text = self.tokenizer.decode(out, skip_special_tokens=False)
        return GenerationResult(text=text, tool_call=parse_tool_call(text))
