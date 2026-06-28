import time
from config import Config
from inference import TokenGenerator, debug_print

def main():
    config = Config
    prompt = input("Enter your prompt: ")
    max_tokens_input = input(f"Enter max tokens to generate (default {Config.max_tokens}): ")
    max_tokens = config.max_tokens if config.max_tokens else 50    
    try:
        max_tokens = int(max_tokens_input) if max_tokens_input else config.max_tokens if config.max_tokens else 50
    except:
        pass
    print("=" * 60)
    print("GPT-OSS-20B GENERATOR")
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
    tokens = generator.tokenizer.encode(prompt, allowed_special="all")
    debug_print(f"Encoded prompt: {tokens}")
    print(f"Prompt length: {len(tokens)} tokens\n")
    print("=" * 60)
    print("GENERATION")
    print("=" * 60)
    gen_start = time.time()
    generated_tokens = []
    for token, _logprob in generator.generate(
        prompt_tokens=tokens,
        stop_tokens=[generator.eot_token],
        temperature=temperature,
        max_tokens=max_tokens,
        return_logprobs=True,
    ):
        generated_tokens.append(token)
    print(f"\n[TIMING] Total generation: {time.time() - gen_start:.2f}s\n")
    print("=" * 60)
    print("FINAL OUTPUT")
    print("=" * 60)
    generated_text = generator.tokenizer.decode(generated_tokens)
    print(f"Generated text  : {generated_text}")
    print(f"Tokens generated: {len(generated_tokens)}")


if __name__ == "__main__":
    main()
