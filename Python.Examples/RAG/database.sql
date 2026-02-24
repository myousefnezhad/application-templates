-- PostgreSQL + pg_vector
-- Enable pg_vector
CREATE EXTENSION IF NOT EXISTS vector;
-- Sample database
CREATE DATABASE rag;
-- Sample table
CREATE TABLE public.knowledge_based
(
    id         bigserial not null,
    chunk      text,
    embedding  VECTOR(1024)
);
