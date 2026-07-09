select u.name, o.total from users u inner join orders o on u.id = o.user_id where o.total > 100;
