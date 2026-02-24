INSERT INTO app.counters (app_id, user_id, session_id, counter)
VALUES ($1, $2, $3, $4)
ON CONFLICT (app_id, user_id, session_id)
DO UPDATE
SET counter = app.counters.counter + $4
RETURNING counter;
