select id, (select max(price) from orders o where o.user_id = u.id) as max_order from users u where u.active = 1;
