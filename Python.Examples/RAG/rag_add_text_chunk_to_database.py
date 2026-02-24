"""
RAG ingestion: text -> llama.cpp embeddings -> Postgres (pgvector)

Table:
  knowledge_based(
    id bigserial PK,
    chunk text,
    embedding vector(1024)
  )

Requirements:
  pip install requests psycopg[binary] pgvector

Env vars (recommended):
  LLAMA_URL=http://localhost:8000/v1/embeddings
  LLAMA_API_KEY=<TOKEN>
  DATABASE_URL=postgresql://user:pass@localhost:5432/rag
  LLAMA_MODEL=gpt-oss-20b
"""

import os
import sys
import json
import requests
import psycopg
from pgvector.psycopg import register_vector


LLAMA_URL = os.getenv("LLAMA_URL", "http://localhost:8000/v1/embeddings")
LLAMA_API_KEY = os.getenv("LLAMA_API_KEY", "<TOKEN>")
DATABASE_URL = os.getenv("DATABASE_URL", "postgresql://postgres:postgres@localhost:5432/rag")
LLAMA_MODEL = os.getenv("LLAMA_MODEL", "bge-m3")

# Based on https://huggingface.co/gpustack/bge-m3-GGUF
# Original Model: https://huggingface.co/BAAI/bge-m3
EMBED_DIM = 1024 

def get_embedding(text: str) -> list[float]:
    headers = {
        "Content-Type": "application/json",
        "Authorization": f"Bearer {LLAMA_API_KEY}",
    }
    payload = {"input": text}
    # Some OpenAI-compatible servers accept/expect a model field.
    if LLAMA_MODEL:
        payload["model"] = LLAMA_MODEL
    resp = requests.post(LLAMA_URL, headers=headers, json=payload, timeout=60)
    resp.raise_for_status()
    data = resp.json()
    # OpenAI-compatible response: {"data":[{"embedding":[...]}], ...}
    emb = data["data"][0]["embedding"]
    if not isinstance(emb, list):
        raise ValueError("Unexpected embeddings format (expected list).")
    if len(emb) != EMBED_DIM:
        raise ValueError(f"Embedding dim mismatch: expected {EMBED_DIM}, got {len(emb)}")
    # Ensure floats (some servers may return ints)
    return [float(x) for x in emb]

def ensure_schema(conn: psycopg.Connection) -> None:
    # Enable pgvector + table
    conn.execute("CREATE EXTENSION IF NOT EXISTS vector;")
    conn.execute(
        f"""
        CREATE TABLE IF NOT EXISTS knowledge_based (
            id        BIGSERIAL PRIMARY KEY,
            chunk     TEXT,
            embedding VECTOR({EMBED_DIM})
        );
        """
    )

def insert_chunk(conn: psycopg.Connection, chunk: str, embedding: list[float]) -> int:
    # With pgvector adapter registered, you can pass a Python list directly.
    row = conn.execute(
        "INSERT INTO knowledge_based (chunk, embedding) VALUES (%s, %s) RETURNING id;",
        (chunk, embedding),
    ).fetchone()
    return int(row[0])


def main():
    chunk = input("Please enter a chunk of text: ")
    embedding = get_embedding(chunk)
    with psycopg.connect(DATABASE_URL) as conn:
        register_vector(conn)  # important!
        ensure_schema(conn)
        new_id = insert_chunk(conn, chunk, embedding)
        conn.commit()

    print(json.dumps({"id": new_id, "dim": len(embedding)}, indent=2))
    print("Chunk is added to the database")

if __name__ == "__main__":
    main()
