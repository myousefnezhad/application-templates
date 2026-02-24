# This run a docker on your local for vLLM, load OpenAI GPT OSS 20B
# Considering the following DIR on host to cache the model
# mkdir -p ~/vllm
sudo docker pull vllm/vllm-openai:latest
sudo docker rm -f vllm
sudo docker run -itd --runtime nvidia --gpus all \
	-v ~/vllm:/root/.cache/huggingface \
  --restart always \
  --name vllm \
  --env "HUGGING_FACE_HUB_TOKEN=${HF_TOKEN}" \
 	-p 0.0.0.0:8000:8000 \
  --ipc=host \
  vllm/vllm-openai:latest \
	--model openai/gpt-oss-20b \
	--gpu-memory-utilization 0.8 \
	--tensor-parallel-size 1 \
  --pipeline-parallel-size 1 \
  --api-key <TOKEN GOES HERE> \
	--enable-auto-tool-choice \
  --tool-call-parser openai

# Other Options
#  --model mistralai/Mistral-Small-3.1-24B-Instruct-2503 \
#  --tool-call-parser mistral
#  --model meta-llama/Llama-3.2-11B-Vision-Instruct \
#  --tool-call-parser llama3_json
#  --model Qwen/Qwen3-30B-A3B-Instruct-2507 \
#  --tool-call-parser qwen3_xml
#  --max-model-len 1024 \
#  --max-model-len 4096 \
