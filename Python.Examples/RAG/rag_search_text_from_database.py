#!/usr/bin/env python3
"""
Search top-N most similar chunks in Postgres using pgvector.

Env vars (optional):
  LLAMA_URL       default: http://localhost:8000/v1/embeddings
  LLAMA_API_KEY   default: <TOKEN>
  LLAMA_MODEL     default: bga-m3
  DATABASE_URL    default: postgresql://postgres:postgres@localhost:5432/rag
"""

import os
import sys
import argparse
import requests
import psycopg
from pgvector.psycopg import register_vector

LLAMA_URL = os.getenv("LLAMA_URL", "http://localhost:8000/v1/embeddings")
LLAMA_API_KEY = os.getenv("LLAMA_API_KEY", "<TOKEN>")
LLAMA_MODEL = os.getenv("LLAMA_MODEL", "bga-m3")
DATABASE_URL = os.getenv("DATABASE_URL", "postgresql://postgres:postgres@localhost:5432/rag")

# Based on https://huggingface.co/gpustack/bge-m3-GGUF
# Original Model: https://huggingface.co/BAAI/bge-m3
EMBED_DIM = 1024
TOP_N = 5

def get_embedding(text: str) -> list[float]:
    headers = {
        "Content-Type": "application/json",
        "Authorization": f"Bearer {LLAMA_API_KEY}",
    }
    payload = {"input": text}
    if LLAMA_MODEL:
        payload["model"] = LLAMA_MODEL
    try:
        r = requests.post(LLAMA_URL, headers=headers, json=payload, timeout=60)
        r.raise_for_status()
    except requests.RequestException as e:
        raise RuntimeError(f"Embedding request failed: {e}") from e
    data = r.json()
    try:
        emb = data["data"][0]["embedding"]
    except Exception as e:
        raise RuntimeError(f"Unexpected embedding response shape: {data}") from e
    if not isinstance(emb, list):
        raise RuntimeError("Embedding is not a list.")
    if len(emb) != EMBED_DIM:
        raise RuntimeError(f"Embedding dim mismatch: expected {EMBED_DIM}, got {len(emb)}")
    return [float(x) for x in emb]

def search_topn(conn, query_emb, top):
    sql = """
    SELECT id, chunk, (embedding <=> %s::vector) AS distance
    FROM knowledge_based
    ORDER BY embedding <=> %s::vector
    LIMIT %s;
    """
    return conn.execute(sql, (query_emb, query_emb, top)).fetchall()

def main():
    text = input("Please enter search content: ")
    try:
        qemb = get_embedding(text)
    except Exception as e:
        print(f"Error: {e}", file=sys.stderr)
        sys.exit(1)

    try:
        with psycopg.connect(DATABASE_URL) as conn:
            register_vector(conn)  # allow passing Python list as vector param
            rows = search_topn(conn, qemb, TOP_N)
    except Exception as e:
        print(f"Database error: {e}", file=sys.stderr)
        sys.exit(1)

    if not rows:
        print("No rows found in knowledge_based.")
        return

    # Print results
    for (rid, chunk, dist) in rows:
        print(f"\nID: {rid}")
        print(f"Distance: {dist}")
        print(f"Chunk: {chunk}")
        
if __name__ == "__main__":
    main()
