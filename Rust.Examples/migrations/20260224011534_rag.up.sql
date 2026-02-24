CREATE EXTENSION IF NOT EXISTS vector;

CREATE SCHEMA IF NOT EXISTS rag;

CREATE TABLE IF NOT EXISTS rag.knowledge_based
(
    id         bigserial not null,
    chunk      text,
    embedding  VECTOR(1024),
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);
