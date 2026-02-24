INSERT INTO app.counters (app_id, user_id, session_id, counter)
VALUES ($1, $2, $3, 0)
ON CONFLICT (app_id, user_id, session_id)
DO UPDATE
SET counter = app.counters.counter
RETURNING counter;

