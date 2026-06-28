import re
import json
import time
import uuid

from fastapi import FastAPI, Request
from fastapi.responses import StreamingResponse

from config import Config
from inference import TokenGenerator, _coerce_ids
from schemas import ChatCompletionRequest

app = FastAPI()
generator = TokenGenerator()

MODEL_ID = "qwen3.5-35b-a3b"


# ------------------------------------------------------------------ #
# Decoding / cleanup helpers                                          #
# ------------------------------------------------------------------ #
def safe_decode(token_ids: list, skip_special: bool = True) -> str:
    """Decode token ids, tolerating both HF and tokenizers backends."""
    tok = generator.tokenizer
    try:
        return tok.decode(token_ids, skip_special_tokens=skip_special)
    except TypeError:
        # tokenizers.Tokenizer.decode without the kwarg
        try:
            return tok.decode(token_ids)
        except Exception:
            return ""
    except Exception:
        # Fall back to per-token decode, skipping anything that errors.
        parts = []
        for tid in token_ids:
            try:
                parts.append(tok.decode([tid]))
            except Exception:
                pass
        return "".join(parts)


def strip_specials(text: str) -> str:
    """Remove Qwen chat-control markers from a text fragment."""
    return re.sub(r"<\|[^|]*\|>", "", text)


def clean_model_output(text: str) -> str:
    text = re.sub(r"<tool_call>.*?</tool_call>", "", text, flags=re.DOTALL)
    text = strip_specials(text)
    return text.strip()


# ------------------------------------------------------------------ #
# Qwen tool-call parsing                                              #
# ------------------------------------------------------------------ #
# Qwen emits tool calls inline in the assistant turn as:
#     <tool_call>
#     {"name": "fn", "arguments": {...}}
#     </tool_call>
# (one or more blocks).
# ------------------------------------------------------------------ #
def parse_qwen_tool_calls(text: str) -> list[dict]:
    calls = []
    for m in re.finditer(r"<tool_call>\s*(\{.*?\})\s*</tool_call>", text, re.DOTALL):
        try:
            payload = json.loads(m.group(1).strip())
        except json.JSONDecodeError:
            continue
        name = payload.get("name")
        if not name:
            continue
        args = payload.get("arguments", {})
        if not isinstance(args, str):
            args = json.dumps(args, ensure_ascii=False)
        calls.append({"name": name, "arguments": args})
    return calls


def _openai_tool_calls(calls: list[dict]) -> list[dict]:
    return [
        {
            "id": f"call_{uuid.uuid4().hex[:8]}",
            "type": "function",
            "function": {"name": c["name"], "arguments": c["arguments"]},
        }
        for c in calls
    ]


# ------------------------------------------------------------------ #
# Prompt building (Qwen chat template, with tool support)             #
# ------------------------------------------------------------------ #
def _msg_to_dict(m) -> dict:
    d = {"role": m.role, "content": m.content or ""}
    if getattr(m, "tool_calls", None):
        d["tool_calls"] = m.tool_calls
    if getattr(m, "tool_call_id", None):
        d["tool_call_id"] = m.tool_call_id
    return d


def _manual_prompt(messages: list[dict], tools=None) -> str:
    """Fallback prompt builder if the tokenizer has no chat template."""
    out = []
    has_system = any(m["role"] == "system" for m in messages)
    if not has_system:
        sys_text = "You are a helpful assistant."
        if tools:
            sys_text += (
                "\n\n# Tools\n\nYou may call one or more functions. Available tools:\n"
                + json.dumps([t.get("function", t) for t in tools], ensure_ascii=False)
                + "\n\nReturn calls as <tool_call>{\"name\": ..., \"arguments\": ...}</tool_call>."
            )
        out.append(f"<|im_start|>system\n{sys_text}<|im_end|>\n")

    for m in messages:
        role, content = m["role"], m.get("content", "")
        if role == "assistant" and m.get("tool_calls"):
            blocks = []
            for tc in m["tool_calls"]:
                fn = tc.get("function", {})
                args = fn.get("arguments", "{}")
                if isinstance(args, str):
                    try:
                        args = json.loads(args)
                    except json.JSONDecodeError:
                        pass
                blocks.append(
                    "<tool_call>\n"
                    + json.dumps({"name": fn.get("name", ""), "arguments": args}, ensure_ascii=False)
                    + "\n</tool_call>"
                )
            body = (content + "\n" if content else "") + "\n".join(blocks)
            out.append(f"<|im_start|>assistant\n{body}<|im_end|>\n")
        elif role == "tool":
            out.append(f"<|im_start|>user\n<tool_response>\n{content}\n</tool_response><|im_end|>\n")
        else:
            out.append(f"<|im_start|>{role}\n{content}<|im_end|>\n")

    out.append("<|im_start|>assistant\n")
    return "".join(out)


def build_input_ids(req: ChatCompletionRequest) -> list[int]:
    tok = generator.tokenizer
    msgs = [_msg_to_dict(m) for m in req.messages]

    if hasattr(tok, "apply_chat_template"):
        try:
            out = tok.apply_chat_template(
                msgs,
                tools=(req.tools or None),
                add_generation_prompt=True,
                tokenize=True,
            )
            return _coerce_ids(out)
        except Exception as e:
            print(f"[WARN] apply_chat_template failed ({e}); using manual builder")

    return _coerce_ids(tok.encode(_manual_prompt(msgs, req.tools)))


# ------------------------------------------------------------------ #
# Health & model listing                                              #
# ------------------------------------------------------------------ #
@app.get("/health")
def health():
    return {"status": "ok"}


@app.get("/v1/models")
def models():
    return {
        "object": "list",
        "data": [{"id": MODEL_ID, "object": "model", "created": 0, "owned_by": "qwen"}],
    }


# ------------------------------------------------------------------ #
# Streaming                                                           #
# ------------------------------------------------------------------ #
def stream_response(req: ChatCompletionRequest):
    request_id = f"chatcmpl-{uuid.uuid4()}"
    created = int(time.time())
    tokens = build_input_ids(req)

    first_chunk = {
        "id": request_id,
        "object": "chat.completion.chunk",
        "created": created,
        "model": req.model or MODEL_ID,
        "choices": [{"index": 0, "delta": {"role": "assistant"}, "finish_reason": None}],
    }
    yield f"data: {json.dumps(first_chunk, ensure_ascii=False)}\n\n"

    try:
        generated: list[int] = []
        content_emitted = 0
        in_tool = False

        for token in generator.generate(
            prompt_tokens=tokens,
            stop_tokens=generator.stop_tokens,
            temperature=req.temperature,
            max_tokens=req.max_tokens,
            return_logprobs=False,
        ):
            generated.append(token)
            text = safe_decode(generated, skip_special=True)

            # Once a tool call begins, stop streaming free-text content.
            if "<tool_call>" in text:
                in_tool = True

            if not in_tool:
                piece = text[content_emitted:]
                if piece:
                    content_emitted = len(text)
                    chunk = {
                        "id": request_id,
                        "object": "chat.completion.chunk",
                        "created": created,
                        "model": req.model or MODEL_ID,
                        "choices": [
                            {"index": 0, "delta": {"content": piece}, "finish_reason": None}
                        ],
                    }
                    yield f"data: {json.dumps(chunk, ensure_ascii=False)}\n\n"

        full_text = safe_decode(generated, skip_special=True)
        tool_calls = parse_qwen_tool_calls(full_text)

        if tool_calls:
            chunk = {
                "id": request_id,
                "object": "chat.completion.chunk",
                "created": created,
                "model": req.model or MODEL_ID,
                "choices": [
                    {
                        "index": 0,
                        "delta": {"tool_calls": _openai_tool_calls(tool_calls)},
                        "finish_reason": "tool_calls",
                    }
                ],
            }
            yield f"data: {json.dumps(chunk, ensure_ascii=False)}\n\n"
        else:
            final_chunk = {
                "id": request_id,
                "object": "chat.completion.chunk",
                "created": created,
                "model": req.model or MODEL_ID,
                "choices": [{"index": 0, "delta": {}, "finish_reason": "stop"}],
            }
            yield f"data: {json.dumps(final_chunk, ensure_ascii=False)}\n\n"

        yield "data: [DONE]\n\n"

    except Exception as e:
        error_chunk = {
            "id": request_id,
            "object": "chat.completion.chunk",
            "created": created,
            "model": req.model or MODEL_ID,
            "choices": [
                {"index": 0, "delta": {"content": f"\n[SERVER ERROR] {e}"}, "finish_reason": "stop"}
            ],
        }
        yield f"data: {json.dumps(error_chunk, ensure_ascii=False)}\n\n"
        yield "data: [DONE]\n\n"


# ------------------------------------------------------------------ #
# Chat completions                                                   #
# ------------------------------------------------------------------ #
@app.post("/v1/chat/completions")
async def chat(request: Request):
    body = await request.json()
    print(json.dumps(body, indent=2, ensure_ascii=False))
    req = ChatCompletionRequest(**body)

    if req.stream:
        return StreamingResponse(stream_response(req), media_type="text/event-stream")

    tokens = build_input_ids(req)

    out: list[int] = []
    for token in generator.generate(
        prompt_tokens=tokens,
        stop_tokens=generator.stop_tokens,
        temperature=req.temperature,
        max_tokens=req.max_tokens,
        return_logprobs=False,
    ):
        out.append(token)

    text = safe_decode(out, skip_special=True)
    tool_calls = parse_qwen_tool_calls(text)
    usage = {
        "prompt_tokens": len(tokens),
        "completion_tokens": len(out),
        "total_tokens": len(tokens) + len(out),
    }

    if tool_calls:
        return {
            "id": f"chatcmpl-{uuid.uuid4()}",
            "object": "chat.completion",
            "created": int(time.time()),
            "model": req.model or MODEL_ID,
            "choices": [
                {
                    "index": 0,
                    "message": {
                        "role": "assistant",
                        "content": None,
                        "tool_calls": _openai_tool_calls(tool_calls),
                    },
                    "finish_reason": "tool_calls",
                }
            ],
            "usage": usage,
        }

    return {
        "id": f"chatcmpl-{uuid.uuid4()}",
        "object": "chat.completion",
        "created": int(time.time()),
        "model": req.model or MODEL_ID,
        "choices": [
            {
                "index": 0,
                "message": {"role": "assistant", "content": clean_model_output(text)},
                "finish_reason": "stop",
            }
        ],
        "usage": usage,
    }
