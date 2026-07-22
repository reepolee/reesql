# Changelog

All notable changes to reesql are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/). Versions are
date-based (`YY.M.PATCH`) and cut by `release.sh`, which inserts a heading for each new
version and uses the text under it as the GitHub Release notes.


## [26.7.8] - 2026-07-22

## [26.7.7] - 2026-07-22

### Added

- `CREATE TRIGGER` formatting. A trigger's `BEGIN ... END` body holds its own semicolons,
  which used to split the statement and leave `END;` stranded on its own. The header and
  `BEGIN` now stay on one line, body statements are formatted normally and indented, and
  `END;` closes the block.
- The formatting guarantee is now structural: a token borrows its text from the input, so
  emission can only copy those bytes or upper-case them when they spell a keyword.
- `tokens_tile()` verifies the tokens account for every non-whitespace byte of the input,
  and formatting is refused if they ever do not. A lexer rule that skipped a character it
  did not recognise becomes a refusal instead of a corrupted file.

### Fixed

- Characters the tokenizer had no rule for were silently discarded, changing what the SQL
  computed: `n = n + 1` became `n = n 1`, `a - b` and `a / b` became `a b`, `100 % 7`
  became `100 7`, `set @x := 5` became `set @x = 5` (assignment turned into comparison),
  and `"my col"` lost its quotes. All characters are now preserved.
- `<>` was rewritten to `!=`. Both spellings are now left as written, as the README already
  claimed.
- An unterminated trigger body is refused rather than swallowing the rest of the file.
- Subqueries were formatted by rendering their tokens back to text and re-tokenizing the
  result, an extra lossy round trip. They now recurse on the tokens directly.

## [26.7.6] - 2026-07-22

### Added

- Invalid SQL is refused rather than reformatted. reesql reports the offending line and
  reason on stderr, exits `1`, writes nothing to stdout, and leaves the input file
  untouched — so a format-on-save mid-edit can no longer damage a file.
- Trailing `RETURNING`, `ON CONFLICT` and `ON DUPLICATE KEY UPDATE` clauses after a value
  list are kept.

### Fixed

- An `INSERT` missing its `;` swallowed the following statement: the `CREATE TABLE` header
  was deleted outright and its column list was folded into a value tuple, which renders
  without spaces between tokens.
- `CREATE TABLE` with an unterminated column list panicked instead of reporting an error.
- `CREATE TABLE` with no column list had a `(` appended that was never in the source,
  corrupting valid `CREATE TABLE ... AS SELECT` and `... LIKE` as well as definitions still
  being typed.
- Clauses following a value list were dropped from the output entirely.
- Added the missing `DUPLICATE` keyword.

## [26.7.5] - 2026-07-21

Earlier releases predate this changelog; see the
[commit history](https://github.com/reepolee/reesql/commits/main) for details.
