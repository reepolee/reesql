-- Non-reserved keywords used as identifiers must keep the case the user wrote.
DROP TABLE IF EXISTS tables;

CREATE TABLE tournament (
    id         INTEGER NULL PRIMARY KEY AUTOINCREMENT,
    location   TEXT    NULL DEFAULT '',
    location_2 TEXT    NULL DEFAULT '',
    date       TEXT    NULL DEFAULT '',
    rounds     INTEGER NULL DEFAULT 3
);
CREATE TABLE tables (
    id    INTEGER NULL PRIMARY KEY AUTOINCREMENT,
    title TEXT    NULL DEFAULT ''
);

INSERT INTO tables (title) VALUES ('table_1');
INSERT INTO tournament (id, location, location_2, date) VALUES (1,'a','b','c');

-- The same words in keyword position still upper-case.
CREATE VIEW v_ranking AS
SELECT
    ROW_NUMBER() OVER(ORDER BY total_points DESC) AS rank,
    cast(x AS DATE) AS d
FROM v_results;
