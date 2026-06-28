conda create -n transformers python=3.12 -y
conda activate transformers
pip install torch torchvision torchaudio transformers \
	accelerate datasets peft trl sentencepiece safetensors huggingface_hub \
    einops jupyter matplotlib ipywidgets scipy scikit-learn notebook tiktoken \
    openai-harmony fastapi uvicorn 