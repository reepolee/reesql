CREATE TRIGGER clubs_update_timestamp AFTER UPDATE ON clubs FOR EACH ROW BEGIN
    UPDATE clubs
    SET
        updated_at = CURRENT_TIMESTAMP
    WHERE id = new.id;
END;
CREATE TRIGGER audit_orders AFTER INSERT ON orders FOR EACH ROW WHEN new.total > 100 BEGIN
    INSERT INTO audit (msg) VALUES ('big');
    UPDATE stats
    SET
        n = n + 1
    WHERE id = new.id;
END;
