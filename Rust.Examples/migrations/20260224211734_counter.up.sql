CREATE TABLE IF NOT EXISTS app.counters (
    app_id      TEXT    NOT NULL,
    user_id     TEXT    NOT NULL,
    session_id  TEXT    NOT NULL,
    counter     BIGINT  NOT NULL DEFAULT 0,
    PRIMARY KEY (app_id, user_id, session_id)
);
