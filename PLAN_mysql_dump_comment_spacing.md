# Plan: MySQL dump preamble spacing

## Goal

Format consecutive MySQL version-comment statements from `mysqldump` on adjacent
lines, without inserting unnecessary blank lines between them.

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
- Preserve unrelated working-tree changes, including the currently deleted
  `BUG-FIX-sqlite-cast-concat-line-break.md`.

## Implementation

1. Define how consecutive standalone MySQL version-comment statements are identified.
2. Adjust statement-stream rendering so those setup statements use single newlines
   rather than blank-line separation.
3. Add a focused integration test and assert formatting is idempotent.
4. Check that ordinary comments and multi-statement SQL retain their current layout.

## Verification on macOS

- Run `cargo fmt -- --check`.
- Compile with `cargo build`.
- Run the focused regression test.
- Run the full test suite with `cargo test`.
- Format the supplied input twice and confirm the second output is identical.
- Review the final diff and rescan touched files for minimal scope and formatting issues.

## Acceptance criteria

- The eight supplied `/*!...*/;` statements appear on consecutive lines with no
  blank lines between them.
- The final output remains valid SQL and preserves every conditional-comment token.
- Existing formatter behavior and tests remain stable outside this case.
- The project compiles and tests pass on macOS.
