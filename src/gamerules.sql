DROP TABLE IF EXISTS gamerules;
CREATE TABLE gamerules (
	day SMALLINT PRIMARY KEY,
	daytype SMALLINT NOT NULL, -- 1 = raceday, 2 = primary path day, 3 = secondary path day, 4 = perk day
	perksonday SMALLINT
);

INSERT INTO gamerules (day, daytype, perksonday) VALUES
	(1,1,null),
	(2,4,13),
	(3,4,13),
	(4,4,13),
	(5,4,13),
	(6,4,13),
	(7,4,13)

