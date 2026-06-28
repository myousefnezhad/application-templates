import time
from collections.abc import Mapping
from config import Config
from inference import TokenGenerator, debug_print


def _to_id_list(encoded):
    """Normalize whatever apply_chat_template/encode returns into a flat list[int]."""
    # BatchEncoding (subclasses UserDict, NOT dict) / dict / any Mapping -> input_ids
    if isinstance(encoded, Mapping):
        encoded = encoded["input_ids"]
    # torch tensor / numpy -> python list
    if hasattr(encoded, "tolist"):
        encoded = encoded.tolist()
    # Unwrap a single batch row: [[...]] -> [...]
    if isinstance(encoded, (list, tuple)) and len(encoded) > 0 and isinstance(encoded[0], (list, tuple)):
        encoded = encoded[0]
    return [int(x) for x in encoded]


def build_prompt(tokenizer, prompt: str):
    """
    Turn a raw user string into chat-formatted token ids.

    Prefer the tokenizer's own chat template; fall back to a manual layout if no
    template is attached. The return value is always a flat list of ints.
    """
    messages = [{"role": "user", "content": prompt}]
    try:
        if getattr(tokenizer, "chat_template", None):
            encoded = tokenizer.apply_chat_template(messages, add_generation_prompt=True, tokenize=True)
            return _to_id_list(encoded)
    except Exception as e:
        debug_print(f"apply_chat_template failed ({e}); using manual template")

    # Fallback for the multimodal variant's token scheme.
    text = f"<|user|>{prompt}<|end|><|assistant|>"
    return _to_id_list(tokenizer.encode(text))


def main():
    config = Config
    prompt = input("Enter your prompt: ")
    max_tokens_input = input(f"Enter max tokens to generate (default {Config.max_tokens}): ")
    max_tokens = config.max_tokens if config.max_tokens else 50
    try:
        max_tokens = int(max_tokens_input) if max_tokens_input else config.max_tokens if config.max_tokens else 50
    except Exception:
        pass

    print("=" * 60)
    print("PHI-4-MULTIMODAL GENERATOR (text backbone)")
    print(f"DEBUG MODE IS {'ON' if config.debug_mode else 'OFF'}")
    print("=" * 60)

    checkpoint_path = config.checkpoint_path
    device = config.device
    temperature = config.temperature
    print(f"\n[CONFIG]")
    print(f"  Checkpoint : {checkpoint_path}")
    print(f"  Device     : {device}")
    print(f"  Prompt     : {prompt}")
    print(f"  Temperature: {temperature}")
    print(f"  Max tokens : {max_tokens}")
    print()

    print("=" * 60)
    print("INITIALISATION")
    print("=" * 60)
    init_start = time.time()
    generator = TokenGenerator(checkpoint=checkpoint_path, device=device)
    print(f"\n[TIMING] TokenGenerator init complete: {time.time() - init_start:.2f}s\n")

    print("=" * 60)
    print("TOKENISATION")
    print("=" * 60)
    tokens = build_prompt(generator.tokenizer, prompt)
    debug_print(f"Encoded prompt: {tokens}")
    print(f"Prompt length: {len(tokens)} tokens\n")

    print("=" * 60)
    print("GENERATION")
    print("=" * 60)
    gen_start = time.time()
    generated_tokens = []
    for token, _logprob in generator.generate(
        prompt_tokens=tokens,
        stop_tokens=generator.stop_tokens,
        temperature=temperature,
        max_tokens=max_tokens,
        return_logprobs=True,
    ):
        generated_tokens.append(token)
    print(f"\n[TIMING] Total generation: {time.time() - gen_start:.2f}s\n")

    print("=" * 60)
    print("FINAL OUTPUT")
    print("=" * 60)
    generated_text = generator.tokenizer.decode(generated_tokens, skip_special_tokens=True)
    print(f"Generated text  : {generated_text}")
    print(f"Tokens generated: {len(generated_tokens)}")


if __name__ == "__main__":
    main()
