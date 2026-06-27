import re
import json
import time
import uuid
from fastapi import FastAPI, Request
from fastapi.responses import StreamingResponse
from config import Config
from inference import TokenGenerator, parse_tool_call, parse_tool_calls
from schemas import ChatCompletionRequest

app = FastAPI()
generator = TokenGenerator()

MODEL_ID = Config.model_id


# Gemma 4 control tokens (see convert_gemma4_weights.py special tokens).
_CONTROL_TOKEN_RE = re.compile(
    r"<\|turn>|<turn\|>|<\|channel>|<channel\|>|<\|think\|>|"
    r"<\|tool_response>|<tool_response\|>|<\|tool_call>|<tool_call\|>|"
    r"<\|tool>|<tool\|>|<\|image>|<image\|>|<\|audio>|<audio\|>|<\|audio\|>|"
    r'<\|"\|>|<bos>|<eos>|<pad>|<end_of_turn>|<start_of_turn>\w*'
)
# Reasoning lives in the "thought" channel; tool calls in their own block. Both
# are surfaced separately (or hidden) and must not leak into assistant content.
_THOUGHT_RE = re.compile(r"<\|channel>thought.*?<channel\|>", re.DOTALL)
_TOOLCALL_RE = re.compile(r"<\|tool_call>.*?<tool_call\|>", re.DOTALL)
_TURN_PREFIX_RE = re.compile(r"^\s*<\|turn>model\s*")


def clean_model_output(text: str) -> str:
    """Final, user-visible assistant content: drop reasoning + tool-call blocks
    and all Gemma 4 control tokens."""
    text = _THOUGHT_RE.sub("", text)
    text = _TOOLCALL_RE.sub("", text)
    text = _TURN_PREFIX_RE.sub("", text)
    text = _CONTROL_TOKEN_RE.sub("", text)
    return text.strip()


def _visible_content(text: str) -> str:
    """Streaming-safe view of the visible content produced so far.

    Strips COMPLETE reasoning/tool-call blocks, and additionally hides anything
    from a still-open (unclosed) `<|channel>`/`<|tool_call>` opener onward, so
    partial reasoning or tool-call syntax is never streamed as content. No
    trailing strip, so the running prefix stays stable for suffix diffing.
    """
    text = _THOUGHT_RE.sub("", text)
    text = _TOOLCALL_RE.sub("", text)
    for opener in ("<|channel>", "<|tool_call>"):
        idx = text.rfind(opener)
        if idx != -1:
            text = text[:idx]
    text = _TURN_PREFIX_RE.sub("", text)
    text = _CONTROL_TOKEN_RE.sub("", text)
    return text


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


def _coerce_to_id_list(res, tok) -> list[int]:
    """Normalise apply_chat_template / encode output into a flat list[int].
    Handles plain lists, nested [[...]] batches, and Mapping-like returns
    (dict, BatchEncoding, UserDict) — the latter are NOT `dict` subclasses, so
    we test the mapping protocol via `keys`, not `isinstance(res, dict)`."""
    if hasattr(res, "keys") and "input_ids" in res:
        res = res["input_ids"]
    elif isinstance(res, str):
        res = tok.encode(res)
    if len(res) > 0 and isinstance(res[0], (list, tuple)):
        res = res[0]
    return [int(t) for t in res]


def build_prompt_tokens(req: ChatCompletionRequest) -> list[int]:
    """
    Use the tokenizer's built-in Gemma 4 chat template. It natively understands
    system/user/assistant/tool roles, tool schemas, and thinking mode. The
    `tools` are what let the model do tool selection, so the template path
    (not the plain-text fallback) must succeed when tools are present.
    """
    tok = generator.tokenizer
    msgs = _messages_to_dicts(req.messages)
    kwargs = dict(add_generation_prompt=True, tokenize=True, return_dict=False)
    if req.tools:
        kwargs["tools"] = req.tools
    try:
        res = tok.apply_chat_template(msgs, **kwargs)
        return _coerce_to_id_list(res, tok)
    except Exception as e:
        # NOTE: this fallback drops tool schemas entirely, so tool selection
        # won't work if we land here. It exists only so plain chat still responds.
        print(f"[WARN] apply_chat_template failed ({e}); falling back to plain encode.")
        if req.tools:
            print("[WARN] tools were provided but the template failed -> the model "
                  "will NOT see the tool definitions and cannot select a tool.")
        text = "\n".join(f"{m['role']}: {m.get('content','')}" for m in msgs)
        return _coerce_to_id_list(tok.encode(text), tok)

def _tool_to_openai(tool, request_id_suffix: str, index: int = 0) -> dict:
    return {
        "index": index,
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
        prev_vis = ""
        for token in generator.generate(
            prompt_tokens=tokens,
            temperature=req.temperature,
            max_tokens=req.max_tokens,
            return_logprobs=False,
        ):
            generated.append(token)
            text = tok.decode(generated, skip_special_tokens=False)

            # Emit only newly produced *visible* content. Reasoning ("thought"
            # channel) and tool-call blocks are withheld here — including while
            # a block is still open — so tool-call syntax never leaks as content.
            vis = _visible_content(text)
            piece = vis[len(prev_vis):]
            prev_vis = vis
            if piece:
                chunk = {
                    "id": request_id, "object": "chat.completion.chunk", "created": created,
                    "model": MODEL_ID,
                    "choices": [{"index": 0, "delta": {"content": piece}, "finish_reason": None}],
                }
                yield f"data: {json.dumps(chunk, ensure_ascii=False)}\n\n"

        # After generation: surface tool call(s) if any were produced.
        full_text = tok.decode(generated, skip_special_tokens=False)
        tools = parse_tool_calls(full_text)
        if tools:
            tool_calls = [_tool_to_openai(t, uuid.uuid4().hex[:8], index=i) for i, t in enumerate(tools)]
            chunk = {
                "id": request_id, "object": "chat.completion.chunk", "created": created,
                "model": MODEL_ID,
                "choices": [{"index": 0, "delta": {"tool_calls": tool_calls}, "finish_reason": "tool_calls"}],
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
    tools = parse_tool_calls(text)

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

    if tools:
        tool_calls = [_tool_to_openai(t, uuid.uuid4().hex[:8], index=i) for i, t in enumerate(tools)]
        base["choices"] = [{
            "index": 0,
            "message": {"role": "assistant", "content": None, "tool_calls": tool_calls},
            "finish_reason": "tool_calls",
        }]
    else:
        base["choices"] = [{
            "index": 0,
            "message": {"role": "assistant", "content": clean_model_output(text)},
            "finish_reason": "stop",
        }]
    return base
