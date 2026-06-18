import re
import json
import time
import uuid
from fastapi import FastAPI, Request
from fastapi.responses import StreamingResponse
from config import Config
from inference import TokenGenerator, parse_tool_call
from schemas import ChatCompletionRequest

app = FastAPI()
generator = TokenGenerator()

MODEL_ID = Config.model_id


def clean_model_output(text: str) -> str:
    # Strip Gemma turn/control markers that may leak into decoded text.
    text = re.sub(r"<end_of_turn>|<eos>|<start_of_turn>\w*", "", text)
    text = re.sub(r"```(?:json)?", "", text, flags=re.IGNORECASE).replace("```", "")
    return text.strip()


# ------------------------------------------------------------------ #
# Prompt building via the Gemma 4 chat template
# ------------------------------------------------------------------ #
def _messages_to_dicts(messages) -> list[dict]:
    out = []
    for m in messages:
        d: dict = {"role": m.role}
        if m.content is not None:
            d["content"] = m.content
        if m.tool_calls:
            d["tool_calls"] = m.tool_calls
        if m.tool_call_id:
            d["tool_call_id"] = m.tool_call_id
        out.append(d)
    return out


def build_prompt_tokens(req: ChatCompletionRequest) -> list[int]:
    """
    Use the tokenizer's built-in Gemma 4 chat template. It natively understands
    system/user/assistant/tool roles, tool schemas, and thinking mode.
    """
    tok = generator.tokenizer
    msgs = _messages_to_dicts(req.messages)
    kwargs = dict(add_generation_prompt=True, tokenize=True)
    if req.tools:
        kwargs["tools"] = req.tools
    try:
        res = tok.apply_chat_template(msgs, **kwargs)
        # FIX: Unpack input_ids if the tokenizer returns a dictionary mapping
        if isinstance(res, dict):
            res = res.get("input_ids", [])
        elif isinstance(res, str):
            res = tok.encode(res)
        return [int(t) for t in res]
    except Exception as e:
        print(f"[WARN] apply_chat_template failed ({e}); falling back to plain encode.")
        text = "\n".join(f"{m['role']}: {m.get('content','')}" for m in msgs)
        res = tok.encode(text)
        if isinstance(res, dict):
            res = res.get("input_ids", [])
        return [int(t) for t in res]

# def build_prompt_tokens(req: ChatCompletionRequest) -> list[int]:
#     tok = generator.tokenizer
#     msgs = _messages_to_dicts(req.messages)
#     kwargs = dict(add_generation_prompt=True, tokenize=True)
#     if req.tools:
#         kwargs["tools"] = req.tools
#     try:
#         return tok.apply_chat_template(msgs, **kwargs)
#     except Exception as e:
#         print(f"[WARN] apply_chat_template failed ({e}); falling back to plain encode.")
#         text = "\n".join(f"{m['role']}: {m.get('content','')}" for m in msgs)
#         return tok.encode(text)


def _tool_to_openai(tool, request_id_suffix: str) -> dict:
    return {
        "id": f"call_{request_id_suffix}",
        "type": "function",
        "function": {"name": tool.function.name, "arguments": tool.function.arguments},
    }


@app.get("/health")
def health():
    return {"status": "ok"}


@app.get("/v1/models")
def models():
    return {
        "object": "list",
        "data": [{"id": MODEL_ID, "object": "model", "created": 0, "owned_by": "google"}],
    }


# ------------------------------------------------------------------ #
# Streaming
# ------------------------------------------------------------------ #
def stream_response(req: ChatCompletionRequest):
    request_id = f"chatcmpl-{uuid.uuid4()}"
    created = int(time.time())
    tokens = build_prompt_tokens(req)
    tok = generator.tokenizer

    yield f"data: {json.dumps({'id': request_id, 'object': 'chat.completion.chunk', 'created': created, 'model': MODEL_ID, 'choices': [{'index': 0, 'delta': {'role': 'assistant'}, 'finish_reason': None}]})}\n\n"

    try:
        generated: list[int] = []
        prev_text = ""
        for token in generator.generate(
            prompt_tokens=tokens,
            temperature=req.temperature,
            max_tokens=req.max_tokens,
            return_logprobs=False,
        ):
            generated.append(token)
            text = tok.decode(generated, skip_special_tokens=False)

            # Emit any newly decoded, cleaned text as a content delta.
            clean = clean_model_output(text)
            piece = clean[len(clean_model_output(prev_text)):]
            prev_text = text
            if piece:
                chunk = {
                    "id": request_id, "object": "chat.completion.chunk", "created": created,
                    "model": MODEL_ID,
                    "choices": [{"index": 0, "delta": {"content": piece}, "finish_reason": None}],
                }
                yield f"data: {json.dumps(chunk, ensure_ascii=False)}\n\n"

        # After generation: surface a tool call if one was produced.
        full_text = tok.decode(generated, skip_special_tokens=False)
        tool = parse_tool_call(full_text)
        if tool:
            chunk = {
                "id": request_id, "object": "chat.completion.chunk", "created": created,
                "model": MODEL_ID,
                "choices": [{"index": 0, "delta": {"tool_calls": [_tool_to_openai(tool, uuid.uuid4().hex[:8])]}, "finish_reason": "tool_calls"}],
            }
            yield f"data: {json.dumps(chunk)}\n\n"
        else:
            yield f"data: {json.dumps({'id': request_id, 'object': 'chat.completion.chunk', 'created': created, 'model': MODEL_ID, 'choices': [{'index': 0, 'delta': {}, 'finish_reason': 'stop'}]})}\n\n"
        yield "data: [DONE]\n\n"

    except Exception as e:
        err = {
            "id": request_id, "object": "chat.completion.chunk", "created": created, "model": MODEL_ID,
            "choices": [{"index": 0, "delta": {"content": f"\n[SERVER ERROR] {e}"}, "finish_reason": "stop"}],
        }
        yield f"data: {json.dumps(err, ensure_ascii=False)}\n\n"
        yield "data: [DONE]\n\n"


# ------------------------------------------------------------------ #
# Chat completions
# ------------------------------------------------------------------ #
@app.post("/v1/chat/completions")
async def chat(request: Request):
    body = await request.json()
    req = ChatCompletionRequest(**body)

    if req.stream:
        return StreamingResponse(stream_response(req), media_type="text/event-stream")

    tokens = build_prompt_tokens(req)
    tok = generator.tokenizer

    out = [t for t in generator.generate(
        prompt_tokens=tokens,
        temperature=req.temperature,
        max_tokens=req.max_tokens,
        return_logprobs=False,
    )]
    text = tok.decode(out, skip_special_tokens=False)
    tool = parse_tool_call(text)

    base = {
        "id": f"chatcmpl-{uuid.uuid4()}",
        "object": "chat.completion",
        "created": int(time.time()),
        "model": MODEL_ID,
        "usage": {
            "prompt_tokens": len(tokens),
            "completion_tokens": len(out),
            "total_tokens": len(tokens) + len(out),
        },
    }

    if tool:
        base["choices"] = [{
            "index": 0,
            "message": {"role": "assistant", "content": None, "tool_calls": [_tool_to_openai(tool, uuid.uuid4().hex[:8])]},
            "finish_reason": "tool_calls",
        }]
    else:
        base["choices"] = [{
            "index": 0,
            "message": {"role": "assistant", "content": clean_model_output(text)},
            "finish_reason": "stop",
        }]
    return base
