DROP TABLE IF EXISTS item CASCADE;
CREATE TABLE item (
    name VARCHAR(30) PRIMARY KEY,
    day SMALLINT,
    nr SMALLINT,
    descr TEXT
);

INSERT INTO item (day, nr, name, descr) VALUES
    (1,1,'A','single shared auction test a'),
    (1,1,'B','single shared auction test b'),
    (1,1,'C','single shared auction test c'),
    (1,1,'D','single shared auction test d'),

    (2,1,'E','separate auction test e'),
    (2,2,'F','separate auction test f'),
    (2,3,'G','separate auction test g'),
    (2,4,'H','separate auction gets h')