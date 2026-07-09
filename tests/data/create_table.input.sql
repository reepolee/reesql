create table users (id bigint unsigned not null auto_increment primary key, name varchar(255) not null, email varchar(255) not null unique, created_at timestamp default current_timestamp);
