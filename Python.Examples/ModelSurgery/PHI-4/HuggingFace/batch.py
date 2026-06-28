import torch
from transformers import AutoTokenizer, AutoModelForCausalLM
# huggingface-cli download microsoft/phi-4 --local-dir phi-4
# https://huggingface.co/microsoft/phi-4
MODEL_PATH = "../PyTorch/phi-4"
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
