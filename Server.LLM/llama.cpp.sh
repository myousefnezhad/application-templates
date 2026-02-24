# Download model
# mkdir ~/model
# wget https://huggingface.co/ggml-org/gpt-oss-20b-GGUF/resolve/main/gpt-oss-20b-mxfp4.gguf -O ~/model/gpt-oss-20b-mxfp4.gguf

# We test on ghcr.io/ggml-org/llama.cpp@sha256:617ab960d6adcdaf521408d4d5fe591bcdfbf799a5822ec751a07565ac3c812f
docker pull ghcr.io/ggml-org/llama.cpp:server-cuda
docker rm -f gpt-oss-20b
docker run -d --name gpt-oss-20b \
  --gpus '"device=0"' \
  -p 8000:8000 \
  -v ~/model:/model \
  --restart unless-stopped \
  ghcr.io/ggml-org/llama.cpp:server-cuda \
  --host 0.0.0.0 --port 8000 --jinja \
  --model /model/gpt-oss-20b-mxfp4.gguf \
  --ctx-size 4096 --flash-attn on \
  --embeddings --pooling mean \
  --api-key <TOKEN GOES HERE>

# Other option (not working well with tools) 
#  --repeat-penalty 1.5 --n-gpu-layers 999 \
#  --embeddings --pooling mean \ to provide embeddings and RAG feature
