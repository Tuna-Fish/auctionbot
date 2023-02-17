DROP TABLE IF EXISTS gamestate;
CREATE TABLE gamestate (
	phase SMALLINT DEFAULT -1 NOT NULL,
	deadline TIMESTAMP,
	rate INTEGER,
    channel BIGINT -- Discord ids are u64, this is just that one transmuted.
);
