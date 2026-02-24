-- Drop trigger first
DROP TRIGGER IF EXISTS trg_users_updated_at ON auth.users;

-- Drop function
DROP FUNCTION IF EXISTS auth.set_updated_at;

-- Drop table
DROP TABLE IF EXISTS auth.users;

-- Drop schema (safe if empty)
DROP SCHEMA IF EXISTS auth;

-- Optional: keep extension (usually shared)
-- DROP EXTENSION IF EXISTS pgcrypto;
