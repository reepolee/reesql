# Plan: MySQL dump cleanup and preamble spacing

## Goal

Format consecutive MySQL version-comment statements from `mysqldump` on adjacent
lines, without inserting unnecessary blank lines between them. Add an opt-in `--clean`
mode that removes mysqldump's character-set, collation, and table-option values from
`CREATE TABLE` statements.

Example:

```sql
/*!40101 SET NAMES utf8 */;
/*!50503 SET NAMES utf8mb4 */;
```

## Scope

- Inspect the statement-boundary layout in `src/main.rs`.
- Preserve the existing blank-line separation for ordinary independent SQL
  statements unless the expected behavior requires a narrower shared rule.
- Add regression coverage for the complete preamble pattern supplied in the issue.
- Keep token text and MySQL conditional comments unchanged; this is a whitespace-only
  formatting change.
- With `--clean`, remove a column's `CHARACTER SET value COLLATE value` pair and a trailing
  table `ENGINE`, `DEFAULT CHARSET`, and `COLLATE` option sequence. Leave default output
  unchanged because those clauses may be intentional.
- Preserve unrelated working-tree changes, including the currently deleted
  `BUG-FIX-sqlite-cast-concat-line-break.md`.

## Implementation

1. Define how consecutive standalone MySQL version-comment statements are identified.
2. Adjust statement-stream rendering so those setup statements use single newlines
   rather than blank-line separation.
3. Add a focused integration test and assert formatting is idempotent.
4. Check that ordinary comments and multi-statement SQL retain their current layout.
5. Add `--clean` as a post-format, opt-in transformation and format its result again to restore
   normal spacing and column alignment.
6. Add regression coverage for the supplied dump table, default behavior, and idempotence.

## Verification on macOS

- Run `cargo fmt -- --check`.
- Compile with `cargo build`.
- Run the focused regression test.
- Run the full test suite with `cargo test`.
- Format the supplied input twice and confirm the second output is identical.
- Review the final diff and rescan touched files for minimal scope and formatting issues.
- Run the `--clean` regression test, then format the supplied input twice with `--clean` and
  confirm the second output is identical.

## Acceptance criteria

- The eight supplied `/*!...*/;` statements appear on consecutive lines with no
  blank lines between them.
- The final output remains valid SQL and preserves every conditional-comment token.
- Existing formatter behavior and tests remain stable outside this case.
- The project compiles and tests pass on macOS.
- `--clean` converts the supplied dump table's `value_json` definition to `LONGTEXT NOT NULL`
  and removes its trailing dump table options, leaving `);`.
