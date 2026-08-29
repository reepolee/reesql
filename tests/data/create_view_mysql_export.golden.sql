DROP VIEW IF EXISTS `v_aufmass`;

CREATE ALGORITHM = UNDEFINED SQL SECURITY INVOKER VIEW `v_aufmass` AS
SELECT
    `z`.`zeile_blattNr`   AS `zeile_blattNr`,
    `z`.`zeile_nr`        AS `zeile_nr`,
    `z`.`zeile_pos`       AS `zeile_pos`,
    `z`.`zeile_menge`     AS `zeile_menge`,
    `z`.`zeile_text_1`    AS `zeile_text_1`,
    `z`.`zeile_text_2`    AS `zeile_text_2`,
    `z`.`zeile_text_3`    AS `zeile_text_3`,
    `z`.`zeile_text_4`    AS `zeile_text_4`,
    `z`.`zeile_bemerkung` AS `zeile_bemerkung`,
    `lv`.`lv_name`        AS `lv_name`,
    `lv`.`lv_unit`        AS `lv_unit`,
    `b`.`blatt_datum`     AS `blatt_datum`,
    `lv`.`lv_price`       AS `lv_price`,
    TRIM(BOTH '@' FROM TRIM(BOTH ' ' FROM concat(`z`.`zeile_text_1`, ' @ ', `z`.`zeile_text_2`, ' ', `z`.`zeile_text_3`, ' ', `z`.`zeile_text_4`))) AS `zeile_beschreibung`
FROM ((`zeile` `z` LEFT JOIN `lv` ON((`z`.`zeile_pos` = `lv`.`lv_pos`))) LEFT JOIN `blatt` `b` ON((`z`.`zeile_blattNr` = `b`.`blatt_id`)))
ORDER BY `z`.`zeile_blattNr`, `z`.`zeile_nr`;
