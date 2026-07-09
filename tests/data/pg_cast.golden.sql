SELECT id::TEXT, name::VARCHAR(255), created_at::DATE FROM users WHERE active::INTEGER = 1;
