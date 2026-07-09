select id::text, name::varchar(255), created_at::date from users where active::integer = 1;
