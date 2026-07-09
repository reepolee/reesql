CREATE VIEW active_users AS
SELECT
    u.id   AS user_id,
    u.name AS user_name,
    o.total
FROM users u
    INNER JOIN orders o
        ON u.id = o.user_id
WHERE o.total > 100;
