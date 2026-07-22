create trigger clubs_update_timestamp after update on clubs for each row begin update clubs set updated_at = CURRENT_TIMESTAMP where id = new.id;

end;

create trigger audit_orders after insert on orders for each row when new.total > 100 begin insert into audit (msg) values ('big'); update stats set n = n + 1 where id = new.id; end;
