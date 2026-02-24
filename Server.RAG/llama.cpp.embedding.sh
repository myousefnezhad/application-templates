# Download model
# https://huggingface.co/gpustack/bge-m3-GGUF

docker pull ghcr.io/ggml-org/llama.cpp:server-cuda
docker rm -f bge-m3
docker run -d --name bge-m3 \
  --gpus '"device=0"' \
  -p 8000:8000 \
  -v ./model:/model \
  --restart unless-stopped \
  ghcr.io/ggml-org/llama.cpp:server-cuda \
  --host 0.0.0.0 --port 8000 \
  --model /model/bge-m3-FP16.gguf \
  --ctx-size 4096 --flash-attn on \
  --embeddings --pooling mean \
  --api-key <TOKEN>
