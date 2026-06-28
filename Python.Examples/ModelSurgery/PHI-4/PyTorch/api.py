import re
import json
import time
import uuid
from collections.abc import Mapping

from fastapi import FastAPI, Request
from fastapi.responses import StreamingResponse

from inference import TokenGenerator
from schemas import ChatCompletionRequest

app = FastAPI()
generator = TokenGenerator()

# The backend now serves a Phi-4 checkpoint rather than GPT-OSS-20B.
DEFAULT_MODEL = "phi-4"


# ------------------------------------------------------------------ #
# Text helpers                                                         #
# ------------------------------------------------------------------ #

def _strip_controls(text: str) -> str:
    """Remove any <|...|> control tokens that leak into decoded text."""
    return re.sub(r"<\|[^|]*\|>", "", text)


def clean_model_output(text: str) -> str:
    """Final cleanup for non-streaming / buffered content."""
    text = re.sub(r"```(?:json)?", "", text, flags=re.IGNORECASE)
    text = text.replace("```", "")
    text = _strip_controls(text)
    return text.strip()


def safe_decode(tokenizer, token_ids: list) -> str:
    """
    Decode a list of token ids with the HF tokenizer, skipping special tokens.
    Falls back to per-token decoding if a batch decode ever fails.
    """
    try:
        return tokenizer.decode(token_ids, skip_special_tokens=True)
    except Exception:
        pass
    parts = []
    for tid in token_ids:
        try:
            parts.append(tokenizer.decode([tid], skip_special_tokens=True))
        except Exception:
            pass
    return "".join(parts)


def _to_id_list(encoded):
    """Normalize apply_chat_template / encode output into a flat list[int]."""
    if isinstance(encoded, Mapping):  # BatchEncoding (UserDict) or dict
        encoded = encoded["input_ids"]
    if hasattr(encoded, "tolist"):
        encoded = encoded.tolist()
    if isinstance(encoded, (list, tuple)) and len(encoded) > 0 and isinstance(encoded[0], (list, tuple)):
        encoded = encoded[0]
    return [int(x) for x in encoded]


# ------------------------------------------------------------------ #
# Tool-call parsing (JSON based)                                       #
# ------------------------------------------------------------------ #
# Phi-4 has no Harmony <|call|> dispatch token. Tool calls are emitted as a
# JSON object (optionally fenced in ```json or wrapped in <|tool_call|> tags)
# carrying a `name` and `arguments`. We detect that after generation.

def parse_tool_from_text(text: str, known_tools: list[str] | None = None):
    """Return {"name": str, "arguments": dict} or None."""
    candidates = []
    candidates += re.findall(r"```(?:json)?\s*(.*?)```", text, re.DOTALL)
    candidates += re.findall(r"<\|tool_call\|>\s*(.*?)(?:<\|/tool_call\|>|$)", text, re.DOTALL)
    m = re.search(r"(\{.*\}|\[.*\])", text, re.DOTALL)
    if m:
        candidates.append(m.group(1))

    parsed_known = None
    for raw in candidates:
        raw = raw.strip()
        try:
            parsed = json.loads(raw)
        except json.JSONDecodeError:
            continue

        if isinstance(parsed, list):
            parsed = parsed[0] if parsed else None
        if not isinstance(parsed, dict):
            continue

        name = parsed.get("name")
        if not name:
            continue

        args = parsed.get("arguments", parsed.get("parameters", {}))
        if isinstance(args, str):
            try:
                args = json.loads(args)
            except json.JSONDecodeError:
                args = {}
        if not isinstance(args, dict):
            args = {}

        result = {"name": name, "arguments": args}
        # Prefer a call whose name is in the advertised tool list.
        if known_tools and name in known_tools:
            return result
        if parsed_known is None:
            parsed_known = result

    return parsed_known


# ------------------------------------------------------------------ #
# Health & model listing                                               #
# ------------------------------------------------------------------ #

@app.get("/health")
def health():
    return {"status": "ok"}


@app.get("/v1/models")
def models():
    return {
        "object": "list",
        "data": [
            {"id": DEFAULT_MODEL, "object": "model", "created": 0, "owned_by": "microsoft"}
        ],
    }


# ------------------------------------------------------------------ #
# JSON-Schema rendering for the tool system message                    #
# ------------------------------------------------------------------ #

def _resolve_ref(ref: str, defs: dict) -> dict:
    if not ref.startswith("#/$defs/"):
        return {}
    key = ref[len("#/$defs/"):]
    return defs.get(key, {})


def _render_parameters(params: dict) -> str:
    props = params.get("properties", {})
    required = set(params.get("required", []))
    defs = params.get("$defs", {})

    if not props:
        return "  (no arguments required)"

    lines = []
    for name, schema in props.items():
        req_marker = " [REQUIRED]" if name in required else " [optional]"
        desc = schema.get("description", "")

        if "$ref" in schema:
            resolved = _resolve_ref(schema["$ref"], defs)
            base_type = resolved.get("type", "string")
            enum_vals = resolved.get("enum", [])
            typ = f"string, one of: {{{', '.join(enum_vals)}}}" if enum_vals else base_type
        else:
            raw_type = schema.get("type", "any")
            if isinstance(raw_type, list):
                non_null = [t for t in raw_type if t != "null"]
                typ = non_null[0] if non_null else "any"
                if "null" in raw_type and name not in required:
                    typ += " (nullable)"
            else:
                typ = raw_type
            enum_vals = schema.get("enum", [])
            if enum_vals:
                typ += f", one of: {{{', '.join(str(v) for v in enum_vals)}}}"

        line = f"  - {name} ({typ}){req_marker}"
        if desc:
            line += f": {desc}"
        lines.append(line)

    return "\n".join(lines)


_TOOL_INSTRUCTIONS = (
    "\nTo call a tool, respond with ONLY a single JSON object on its own line:\n"
    '{"name": "<tool_name>", "arguments": {"arg": "value"}}\n\n'
    "Rules:\n"
    "- Use only tools from the list above.\n"
    "- 'arguments' must be a valid JSON object; use {} when there are no arguments.\n"
    "- If no tool is needed, answer normally in plain text.\n"
)


# ------------------------------------------------------------------ #
# Prompt builder (model-agnostic via the tokenizer chat template)      #
# ------------------------------------------------------------------ #

def build_chat_messages(messages, tools=None):
    """
    Convert OpenAI-style messages (+ tools) into a clean role/content list that
    the tokenizer's chat template can render. Tool schemas are folded into a
    single system message; tool *results* become user turns for maximum
    template compatibility across Phi-4 variants.
    """
    system_parts = ["You are a helpful assistant."]

    if tools:
        system_parts.append("\nYou have access to the following tools:\n")
        for t in tools:
            f = t["function"]
            params = f.get("parameters", {})
            print(f"[TOOL SCHEMA] {f['name']}: {json.dumps(params)}")
            system_parts.append(
                f"Tool name: {f['name']}\n\n"
                f"Description:\n{f.get('description', '')}\n\n"
                f"Parameters:\n{_render_parameters(params)}\n"
            )
        system_parts.append(_TOOL_INSTRUCTIONS)

    convo = []
    for m in messages:
        role = m.role.lower()

        if role == "system":
            system_parts.append(m.content or "")

        elif role == "user":
            convo.append({"role": "user", "content": m.content or ""})

        elif role == "assistant":
            if m.tool_calls:
                parts = []
                for tc in m.tool_calls:
                    fn = tc.get("function", {})
                    name = fn.get("name", "")
                    raw_args = fn.get("arguments", "{}")
                    try:
                        args = raw_args if isinstance(raw_args, dict) else json.loads(raw_args or "{}")
                    except json.JSONDecodeError:
                        args = {}
                    parts.append(json.dumps({"name": name, "arguments": args}))
                content_prefix = (m.content or "").strip()
                block = (content_prefix + "\n" if content_prefix else "") + "\n".join(parts)
                convo.append({"role": "assistant", "content": block})
            else:
                convo.append({"role": "assistant", "content": m.content or ""})

        elif role == "tool":
            convo.append({"role": "user", "content": f"Tool result:\n{m.content or ''}"})

    return [{"role": "system", "content": "\n".join(system_parts)}] + convo


def tokenize_chat(tokenizer, chat):
    """Render chat messages to token ids using the tokenizer's own template."""
    try:
        if getattr(tokenizer, "chat_template", None):
            enc = tokenizer.apply_chat_template(chat, add_generation_prompt=True, tokenize=True)
            ids = _to_id_list(enc)
            print("=" * 80)
            print(safe_decode(tokenizer, ids))
            print("=" * 80)
            return ids
    except Exception as e:
        print(f"[WARN] apply_chat_template failed ({e}); using manual template")

    # Fallback: simple per-role markup.
    text = ""
    for m in chat:
        text += f"<|{m['role']}|>{m['content']}<|end|>"
    text += "<|assistant|>"
    return _to_id_list(tokenizer.encode(text))


def _tool_names_from_req(req: ChatCompletionRequest) -> list[str]:
    return [t["function"]["name"] for t in (req.tools or []) if "function" in t]


# ------------------------------------------------------------------ #
# Streaming                                                            #
# ------------------------------------------------------------------ #

def stream_response(req: ChatCompletionRequest):
    request_id = f"chatcmpl-{uuid.uuid4()}"
    created = int(time.time())
    model_id = req.model or DEFAULT_MODEL

    chat = build_chat_messages(req.messages, req.tools)
    tokenizer = generator.tokenizer
    tokens = tokenize_chat(tokenizer, chat)
    known_tools = _tool_names_from_req(req)

    first_chunk = {
        "id": request_id,
        "object": "chat.completion.chunk",
        "created": created,
        "model": model_id,
        "choices": [{"index": 0, "delta": {"role": "assistant"}, "finish_reason": None}],
    }
    yield f"data: {json.dumps(first_chunk, ensure_ascii=False)}\n\n"

    def _content_chunk(piece):
        return {
            "id": request_id,
            "object": "chat.completion.chunk",
            "created": created,
            "model": model_id,
            "choices": [{"index": 0, "delta": {"content": piece}, "finish_reason": None}],
        }

    try:
        generated = []
        prev_clean = ""
        # When tools are advertised, buffer the whole completion so we can decide
        # cleanly between a tool call and plain content (Phi-4 gives no mid-stream
        # signal like Harmony's <|call|>). Otherwise stream token deltas live.
        buffering = bool(req.tools)

        for token in generator.generate(
            prompt_tokens=tokens,
            stop_tokens=generator.stop_tokens,
            temperature=req.temperature,
            max_tokens=req.max_tokens,
            return_logprobs=False,
        ):
            generated.append(token)
            if buffering:
                continue

            clean = _strip_controls(safe_decode(tokenizer, generated))
            piece = clean[len(prev_clean):]
            prev_clean = clean
            if piece:
                yield f"data: {json.dumps(_content_chunk(piece), ensure_ascii=False)}\n\n"

        full_text = safe_decode(tokenizer, generated)
        tool = parse_tool_from_text(full_text, known_tools)

        if tool:
            chunk = {
                "id": request_id,
                "object": "chat.completion.chunk",
                "created": created,
                "model": model_id,
                "choices": [
                    {
                        "index": 0,
                        "delta": {
                            "tool_calls": [
                                {
                                    "id": f"call_{uuid.uuid4().hex[:8]}",
                                    "type": "function",
                                    "function": {
                                        "name": tool["name"],
                                        "arguments": json.dumps(tool["arguments"]),
                                    },
                                }
                            ]
                        },
                        "finish_reason": "tool_calls",
                    }
                ],
            }
            yield f"data: {json.dumps(chunk)}\n\n"
            yield "data: [DONE]\n\n"
            return

        if buffering:
            content = clean_model_output(full_text)
            if content:
                yield f"data: {json.dumps(_content_chunk(content), ensure_ascii=False)}\n\n"

        final_chunk = {
            "id": request_id,
            "object": "chat.completion.chunk",
            "created": created,
            "model": model_id,
            "choices": [{"index": 0, "delta": {}, "finish_reason": "stop"}],
        }
        yield f"data: {json.dumps(final_chunk, ensure_ascii=False)}\n\n"
        yield "data: [DONE]\n\n"

    except Exception as e:
        error_chunk = {
            "id": request_id,
            "object": "chat.completion.chunk",
            "created": created,
            "model": model_id,
            "choices": [
                {"index": 0, "delta": {"content": f"\n[SERVER ERROR] {str(e)}"}, "finish_reason": "stop"}
            ],
        }
        yield f"data: {json.dumps(error_chunk, ensure_ascii=False)}\n\n"
        yield "data: [DONE]\n\n"


# ------------------------------------------------------------------ #
# Chat completions                                                     #
# ------------------------------------------------------------------ #

@app.post("/v1/chat/completions")
async def chat(request: Request):
    body = await request.json()
    print(json.dumps(body, indent=2, ensure_ascii=False))
    req = ChatCompletionRequest(**body)
    known_tools = _tool_names_from_req(req)
    model_id = req.model or DEFAULT_MODEL

    if req.stream:
        return StreamingResponse(stream_response(req), media_type="text/event-stream")

    chat_messages = build_chat_messages(req.messages, req.tools)
    tokenizer = generator.tokenizer
    tokens = tokenize_chat(tokenizer, chat_messages)

    out = []
    for token in generator.generate(
        prompt_tokens=tokens,
        stop_tokens=generator.stop_tokens,
        temperature=req.temperature,
        max_tokens=req.max_tokens,
        return_logprobs=False,
    ):
        out.append(token)

    text = safe_decode(tokenizer, out)
    tool = parse_tool_from_text(text, known_tools)

    if tool:
        return {
            "id": f"chatcmpl-{uuid.uuid4()}",
            "object": "chat.completion",
            "created": int(time.time()),
            "model": model_id,
            "choices": [
                {
                    "index": 0,
                    "message": {
                        "role": "assistant",
                        "content": None,
                        "tool_calls": [
                            {
                                "id": f"call_{uuid.uuid4().hex[:8]}",
                                "type": "function",
                                "function": {
                                    "name": tool["name"],
                                    "arguments": json.dumps(tool["arguments"]),
                                },
                            }
                        ],
                    },
                    "finish_reason": "tool_calls",
                }
            ],
            "usage": {
                "prompt_tokens": len(tokens),
                "completion_tokens": len(out),
                "total_tokens": len(tokens) + len(out),
            },
        }

    clean_text = clean_model_output(text)
    return {
        "id": f"chatcmpl-{uuid.uuid4()}",
        "object": "chat.completion",
        "created": int(time.time()),
        "model": model_id,
        "choices": [
            {
                "index": 0,
                "message": {"role": "assistant", "content": clean_text},
                "finish_reason": "stop",
            }
        ],
        "usage": {
            "prompt_tokens": len(tokens),
            "completion_tokens": len(out),
            "total_tokens": len(tokens) + len(out),
        },
    }
