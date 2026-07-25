# SQLite `CAST(... || ... AS TEXT)` line-break regression

## 1. Bug Summary

`reesql` unnecessarily splits a short SQLite concatenation inside a `CREATE VIEW`
column:

```sql
CAST(r.display || ' - ' || i.display AS TEXT) AS display
```

is formatted as:

```sql
CAST(r.display || ' - ' ||
    i.display AS TEXT) AS display
```

The behavior is deterministic. It affects `CREATE VIEW ... SELECT` columns containing
three or more operands joined by SQLite/PostgreSQL `||`. It is not caused by SQLite
parsing or `CAST` itself; `CAST` makes the forced continuation particularly awkward
because the continuation indentation does not reflect the surrounding parenthesis.

Severity is low for query semantics but medium for formatter quality: the SQL remains
valid, while short expressions become less readable and cannot remain compact.

## 2. Evidence Collected

- Reproduced on `main` at `7a1ca60` with:

  ```sql
  CREATE VIEW v AS SELECT CAST(r.display || ' - ' || i.display AS TEXT) AS display FROM r JOIN i;
  ```

- Actual output:

  ```sql
  CREATE VIEW v AS
  SELECT
      CAST(r.display || ' - ' ||
      i.display AS TEXT) AS display
  FROM r
      JOIN i;
  ```

- `format_create_view` routes every column containing any `Kind::Concat` token to
  `format_view_column`, bypassing the normal compact expression renderer.
- `format_view_column` splits on every `||`, groups operands in pairs, and inserts
  `"\n    "` whenever a third operand exists. It has no line-length check.
- The split is parenthesis-depth agnostic. For the reported expression, the final
  segment includes `i.display AS TEXT) AS display`, so the fixed four-space
  continuation cannot represent the nesting of the `CAST`.
- The existing `create_view_sqlite` fixture contains one deliberately long
  concatenation. Its input and golden output are already multiline, so it codifies
  the long-expression layout but does not cover short concatenations.
- All 45 current tests pass (6 unit and 39 integration), confirming the regression is
  an uncovered formatting policy rather than an existing failing invariant.
- `git blame` traces the special concatenation formatter to commit `938e5cd`.

## 3. Root Cause Hypothesis

### High confidence

The special `CREATE VIEW` concatenation renderer treats operand count as the only wrap
condition. Any expression with at least two `||` operators is forced onto multiple
lines, even when its compact rendering is short.

The formatter was shaped around the existing long search-text expression, where
pairing values produces acceptable output. That fixture-specific layout was applied
as a universal rule.

### Contributing weakness

The wrapping helper receives only the column tokens and emits a hard-coded
continuation indentation. It neither knows the rendered column width nor tracks the
parenthesis depth at each concatenation operator.

### Alternative theories ruled out

- The tokenizer correctly emits `||` as `Kind::Concat`.
- Keyword uppercasing and SQLite `TEXT` handling do not choose the newline.
- Generic `SELECT` statements are not the affected path; the special behavior is in
  `CREATE VIEW` formatting.

## 4. Affected Systems

- `src/main.rs`
  - `format_create_view`
  - `format_view_column`
  - `tokens_upper_string`
- SQLite and PostgreSQL view definitions using `||`
- Golden fixtures under `tests/data`
- No schema, migration, runtime database, API, cache, async, or transaction impact

## 5. Fix Strategy

The safest fix location is `format_view_column`.

1. Render the whole column compactly with the existing token renderer.
2. Keep the compact form when it fits the formatter's chosen line-width policy,
   including the four-space `SELECT` column indentation.
3. Use the existing multiline concatenation layout only when the compact form exceeds
   that width.
4. Add a focused regression test for the reported short `CAST` expression.
5. Retain the long `create_view_sqlite` fixture to verify that long concatenations
   still wrap.
6. Verify idempotency by formatting both the compact and multiline results twice.

This changes whitespace only and needs no migration or compatibility handling. A
single explicit line-width constant is preferable to another operand-count heuristic.
If the project does not want a general width policy yet, an even smaller behavioral
rule is to keep all concatenations compact unless their source expression was already
multiline; however, that would make output depend on input layout and is less
consistent with a formatter.

## 6. Risks

- A new width threshold can alter existing golden output for concatenations near the
  boundary.
- Counting token/source characters rather than rendered characters can be wrong after
  keyword uppercasing or with non-ASCII text; measure the compact rendered string.
- The column's leading indentation must be included in the width decision.
- Long concatenations nested inside `CAST(...)` will still have imperfect
  continuation indentation if the existing fallback is retained unchanged.
- Comments, especially line comments, can make a nominally compact rendering contain
  a newline and must always force the multiline-safe path.
- Splitting on all `||` tokens without tracking parenthesis depth may produce awkward
  layouts in other nested function expressions.

## Prioritized TODO

- [x] P0: Add a regression test asserting the reported short `CAST` concatenation stays
      on one line.
- [x] P0: Add a compact-render length guard before the special multiline concat logic.
- [x] P1: Assert the existing long SQLite view concatenation remains multiline.
- [ ] P1: Add a format-twice idempotency test for both short and long concatenations.
- [ ] P2: Make long-expression continuation indentation aware of parenthesis depth.
- [ ] P2: Add coverage for comments and nested function calls around `||`.
