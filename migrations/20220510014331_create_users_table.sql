CREATE TABLE users (
    user_id uuid PRIMARY KEY NOT NULL,
    username TEXT NOT NULL UNIQUE,
    password TEXT NOT NULL
)
