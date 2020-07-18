DROP TABLE IF EXISTS admins;
CREATE TABLE admins (
	id BIGINT PRIMARY KEY, -- discord u64 transmuted to i64
	name TEXT
);

INSERT INTO admins (id,name) VALUES
	(333600565919481876, 'Tuna');

