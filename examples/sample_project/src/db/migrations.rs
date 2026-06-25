pub const CREATE_USERS: &str = "CREATE TABLE users (id SERIAL PRIMARY KEY, username TEXT NOT NULL)";
pub const CREATE_SESSIONS: &str = "CREATE TABLE sessions (token TEXT PRIMARY KEY, user_id INTEGER)";
pub const INSERT_USER: &str = "INSERT INTO users (username) VALUES ($1)";
pub const SELECT_USER: &str = "SELECT * FROM users WHERE id = $1";
pub const UPDATE_USER: &str = "UPDATE users SET username = $1 WHERE id = $2";
pub const DELETE_USER: &str = "DELETE FROM users WHERE id = $1";
