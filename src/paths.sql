DROP TABLE IF EXISTS paths;
CREATE TABLE paths (
	name CHAR(1) PRIMARY KEY,
	longname VARCHAR(10)
);

INSERT INTO paths (name, longname) VALUES
	('F','Fire'),
	('A','Air'),
	('W','Water'),
	('E','Earth'),
	('S','Astral'),
	('N','Nature'),
	('D','Death'),
	('B','Blood');


