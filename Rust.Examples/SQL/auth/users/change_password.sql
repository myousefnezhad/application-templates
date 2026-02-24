UPDATE auth.users
SET
    password_hash = $1
WHERE
    email = $2;
