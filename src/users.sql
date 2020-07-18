DROP TABLE IF EXISTS users;
CREATE TABLE users (
	id BIGINT PRIMARY KEY, -- Discord userids are u64, this is just that one transmuted.
	name TEXT,
	points INTEGER DEFAULT 1000
);

