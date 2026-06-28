import time
from config import Config
from inference import TokenGenerator, debug_print


def main():
    config = Config
    prompt = input("Enter your prompt: ")
    max_tokens_input = input(f"Enter max tokens to generate (default {Config.max_tokens}): ")
    max_tokens = config.max_tokens if config.max_tokens else 50
    try:
        max_tokens = int(max_tokens_input) if max_tokens_input else (config.max_tokens or 50)
    except ValueError:
        pass

    print("=" * 60)
    print("QWEN3.5 GENERATOR")
    print(f"DEBUG MODE IS {'ON' if config.debug_mode else 'OFF'}")
    print("=" * 60)

    print("\n[CONFIG]")
    print(f"  Checkpoint   : {config.checkpoint_path}")
    print(f"  Device       : {config.device}")
    print(f"  Prompt       : {prompt}")
    print(f"  Temperature  : {config.temperature}")
    print(f"  Top-p / Top-k: {config.top_p} / {config.top_k}")
    print(f"  Max tokens   : {max_tokens}")
    print(f"  Chat template: {config.use_chat_template}")
    print()

    print("=" * 60)
    print("INITIALISATION")
    print("=" * 60)
    init_start = time.time()
    generator = TokenGenerator(checkpoint=config.checkpoint_path, device=config.device)
    print(f"\n[TIMING] TokenGenerator init complete: {time.time() - init_start:.2f}s\n")

    print("=" * 60)
    print("TOKENISATION")
    print("=" * 60)
    if config.use_chat_template:
        tokens = generator.apply_chat_template(prompt)
    else:
        tokens = generator.encode(prompt)
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
        temperature=config.temperature,
        top_p=config.top_p,
        top_k=config.top_k,
        max_tokens=max_tokens,
        return_logprobs=True,
    ):
        generated_tokens.append(token)
    print(f"\n[TIMING] Total generation: {time.time() - gen_start:.2f}s\n")

    print("=" * 60)
    print("FINAL OUTPUT")
    print("=" * 60)
    # Drop a trailing stop token before decoding for cleaner output.
    display = generated_tokens
    if display and display[-1] in generator.stop_tokens:
        display = display[:-1]
    generated_text = generator.tokenizer.decode(display)
    print(f"Generated text  : {generated_text}")
    print(f"Tokens generated: {len(generated_tokens)}")


if __name__ == "__main__":
    main()
