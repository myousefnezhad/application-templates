import torch
from transformers import AutoTokenizer, AutoModelForCausalLM
# huggingface-cli download Qwen/Qwen3.5-35B-A3B --local-dir ../Qwen3.5-35B-A3B
# https://huggingface.co/Qwen/Qwen3.5-35B-A3B
MODEL_PATH = "../Qwen3.5-35B-A3B"
TEXT = "What is the meaning of life?"

def main():
    device = "mps" if torch.backends.mps.is_available() else "cpu"
    tokenizer = AutoTokenizer.from_pretrained(
        MODEL_PATH, 
        trust_remote_code=True
    )
    model = AutoModelForCausalLM.from_pretrained(
        MODEL_PATH, 
        dtype=torch.bfloat16,
        trust_remote_code=True
    )
    model.to(device)
    for k, v in model.state_dict().items():
        print(k, v.shape)

    # Inference
    inputs = tokenizer(TEXT, return_tensors="pt").to(device)
    outputs = model.generate(**inputs, max_new_tokens=50)
    print(tokenizer.decode(outputs[0], skip_special_tokens=True))

if __name__ == "__main__":
    main()
