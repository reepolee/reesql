CREATE TABLE `orders` (
    `id`         INT(11)                           NOT NULL AUTO_INCREMENT,
    `user_id`    INT(11) UNSIGNED                  NOT NULL,
    `total`      DECIMAL(10, 2)                    NOT NULL DEFAULT '0.00',
    `discount`   DECIMAL(10, 2)                    GENERATED ALWAYS AS(`total` * 0.1) STORED,
    `status`     ENUM('new', 'paid')               NOT NULL DEFAULT 'new',
    `notes`      VARCHAR(255) CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci DEFAULT NULL,
    `created_at` TIMESTAMP                         NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    PRIMARY KEY(`id`),
    UNIQUE KEY `uq_user`(`user_id`, `status`),
    KEY `idx_total`(`total`),
    CONSTRAINT `fk_user` FOREIGN KEY(`user_id`) REFERENCES `users`(`id`) ON DELETE CASCADE
) ENGINE = InnoDB DEFAULT CHARSET = utf8mb4 COLLATE = utf8mb4_unicode_ci COMMENT = 'order rows';

# hash comment, MySQL only
INSERT INTO `orders` (`id`, `total`) VALUES
(1,10.50),
(2,20.00) ON DUPLICATE KEY UPDATE `total` = VALUES(`total`) + 1;

UPDATE `orders`
SET
    `total` = `total` * 1.2 - 5 / 2,
    `status` = 'paid'
WHERE `user_id` <> 3 AND `total` >= 100 % 7;

SELECT o.`id`, o.`total` + o.`discount` AS gross, CONCAT(u.`first`, ' ', u.`last`) AS full_name, CASE WHEN o.`total` > 100 THEN 'big' ELSE 'small' END AS bucket FROM `orders` o INNER JOIN `users` u ON u.`id` = o.`user_id` WHERE o.`status` != 'new' GROUP BY o.`id` HAVING gross > 0 ORDER BY o.`created_at` DESC LIMIT 10 OFFSET 5;

DELETE FROM `orders`
WHERE `created_at` < NOW() - INTERVAL 30 DAY;
