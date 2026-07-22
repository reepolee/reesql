SELECT a + b, c - d, e * f, g / h, i % j FROM t WHERE n <> 1 AND m != 2;

UPDATE t
SET
    n = n + 1,
    avg = (hi + lo) / 2
WHERE id = 1;
