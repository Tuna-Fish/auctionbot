DROP TABLE IF EXISTS admin;
CREATE TABLE admin (
	id BIGINT PRIMARY KEY, -- discord u64 transmuted to i64
	name TEXT
);

INSERT INTO admin (id,name) VALUES
	(333600565919481876, 'Tuna');

