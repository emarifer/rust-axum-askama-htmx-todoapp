-- Add up migration script here

CREATE TABLE
    IF NOT EXISTS "users" (
        id TEXT PRIMARY KEY NOT NULL,
        username TEXT NOT NULL,
        email TEXT NOT NULL UNIQUE,
        password TEXT NOT NULL
    );

CREATE INDEX users_email_idx ON users (email);

CREATE TABLE
    IF NOT EXISTS "todos" (
		id INTEGER PRIMARY KEY AUTOINCREMENT,
		created_by TEXT NOT NULL,
		title TEXT NOT NULL,
		description TEXT NOT NULL,
		status BOOLEAN NOT NULL DEFAULT(FALSE),
		created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
		FOREIGN KEY(created_by) REFERENCES users(id)
    );

-- sqlx database create
-- sqlx migrate run
-- sqlx migrate revert
