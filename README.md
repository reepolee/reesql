<img src="/github-reepolee.svg" style="margin-bottom:1rem; width:200px">

# reesql

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

A MySQL SQL formatter written in Rust. Reads SQL from stdin or a file, and writes back consistently formatted SQL.

## Features

- **Keyword uppercasing** - SQL keywords (`SELECT`, `FROM`, `WHERE`, `CREATE`, `INSERT`, etc.) are uppercased
- **CREATE TABLE formatting** - column names and types aligned, constraints on separate lines, types uppercased
- **CREATE VIEW formatting** - select columns on separate lines with `AS` alignment, JOINs indented
- **UPDATE formatting** - `SET` assignments on indented lines, `WHERE` on its own line
- **DELETE formatting** - `WHERE` clause on its own line
- **INSERT formatting** - multi-line value tuples when they exceed 100 characters
- **Subquery formatting** - `(SELECT ...)` subqueries indented on separate lines
- **Operator support** - comparison operators (`>`, `<`, `>=`, `<=`, `!=`, `<>`) are preserved and spaced correctly
- **Comment preservation** - `--`, `/* */`, and `#` comments are preserved and positioned correctly
- **In-place file editing** - pass a file path to format it in place, or use stdin/stdout
- **Cross-platform** - normalizes `\r\n` and `\n` line endings consistently

## Installation

### Quick install

**macOS / Linux:**

```bash
curl -fsSL https://raw.githubusercontent.com/reepolee/reesql/main/install.sh | bash
```

**Windows:**

```powershell
irm https://raw.githubusercontent.com/reepolee/reesql/main/install.ps1 | iex
```

The script detects your OS and architecture, downloads the correct binary from the latest GitHub Release, and adds it to your PATH.

Or download a binary directly from the [latest release](https://github.com/reepolee/reesql/releases/latest).

### Build from source

Requires [Rust](https://rustup.rs/).

```bash
cargo build --release
# Binary at ./target/release/reesql
```

## Usage

### Format a file (in-place)

```bash
reesql path/to/file.sql
```

### Format from stdin

```bash
cat query.sql | reesql
```

### Pipe directly

```bash
echo "SELECT * FROM users WHERE active = 1;" | reesql
```

### Invalid SQL

When reesql cannot format a statement without losing or garbling SQL, it refuses rather than
guessing. It reports the line and reason on stderr, exits `1`, writes nothing to stdout, and
leaves the input file untouched:

```
reesql: schema.sql:35: this INSERT runs into `CREATE` between its VALUES tuples (is it missing its `;`, or a value list?)
reesql: refusing to format invalid SQL; input left unchanged
```

## Examples

### SELECT with JOIN

**Input:**
```sql
select u.name, o.total from users u inner join orders o on u.id = o.user_id where o.total > 100;
```

**Output:**
```sql
SELECT u.name, o.total FROM users u INNER JOIN orders o ON u.id = o.user_id WHERE o.total > 100;
```

### SELECT with subquery

**Input:**
```sql
select id, (select max(price) from orders o where o.user_id = u.id) as max_order from users u where u.active = 1;
```

**Output:**
```sql
SELECT id, (
    SELECT max(price) FROM orders o WHERE o.user_id = u.id
) AS max_order FROM users u WHERE u.active = 1;
```

### CREATE TABLE

**Input:**
```sql
create table users (id bigint unsigned not null auto_increment primary key, name varchar(255) not null, email varchar(255) not null unique, created_at timestamp default current_timestamp);
```

**Output:**
```sql
CREATE TABLE users (
    id         BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
    name       VARCHAR(255)    NOT NULL,
    email      VARCHAR(255)    NOT NULL UNIQUE,
    created_at TIMESTAMP       DEFAULT CURRENT_TIMESTAMP
);
```

### CREATE VIEW

**Input:**
```sql
create view active_users as select u.id as user_id, u.name as user_name, o.total from users u inner join orders o on u.id = o.user_id where o.total > 100;
```

**Output:**
```sql
CREATE VIEW active_users AS
SELECT
    u.id   AS user_id,
    u.name AS user_name,
    o.total
FROM users u
    INNER JOIN orders o
        ON u.id = o.user_id
WHERE o.total > 100;
```

### UPDATE

**Input:**
```sql
update users set name = 'Alice', email = 'alice@example.com' where id = 1;
```

**Output:**
```sql
UPDATE users
SET
    name = 'Alice',
    email = 'alice@example.com'
WHERE id = 1;
```

### DELETE

**Input:**
```sql
delete from users where id = 1;
```

**Output:**
```sql
DELETE FROM users
WHERE id = 1;
```

### INSERT (short)

**Input:**
```sql
insert into users (id, name) values (1, 'Alice');
```

**Output:**
```sql
INSERT INTO users (id, name) VALUES (1,'Alice');
```

### INSERT (long - multi-line when > 100 chars)

**Input:**
```sql
insert into users (id, name, email, status, role, department) values (1, 'Alexander Hamilton', 'alex@example.com', 'active', 'admin', 'engineering'), (2, 'Benjamin Franklin', 'ben@example.com', 'active', 'user', 'marketing');
```

**Output:**
```sql
INSERT INTO users (id, name, email, status, role, department) VALUES
(1,'Alexander Hamilton','alex@example.com','active','admin','engineering'),
(2,'Benjamin Franklin','ben@example.com','active','user','marketing');
```

### With comments preserved

**Input:**
```sql
select * from users; -- inline comment
/* block comment */ select id, name from products; # hash comment
```

**Output:**
```sql
SELECT * FROM users;

-- inline comment
/* block comment */ SELECT id, name FROM products;

# hash comment
```

### Operators

**Input:**
```sql
select * from users where age >= 18 and age <= 65 and name != 'admin' and status <> 'inactive';
```

**Output:**
```sql
SELECT * FROM users WHERE age >= 18 AND age <= 65 AND name != 'admin' AND status != 'inactive';
```

## Supported statement types

| Statement | Description |
|-----------|-------------|
| `SELECT` | Generic formatting with subquery indentation |
| `CREATE TABLE` | Aligned column definitions, types uppercased, constraints on separate lines |
| `CREATE VIEW` | Aligned SELECT columns with AS, indented JOINs |
| `CREATE INDEX` | Uppercased, compact |
| `INSERT` | Multi-line value tuples for long inputs |
| `UPDATE` | SET assignments on indented lines, WHERE on its own line |
| `DELETE` | WHERE clause on its own line |
| `DROP` | Uppercased, compact |

## How it works

1. **Tokenization** - raw SQL is split into tokens: keywords, identifiers, operators (`>`, `<`, `>=`, `<=`, `!=`, `<>`), comments, and strings
2. **Statement splitting** - tokens are split at semicolons into individual statements
3. **Type detection** - each statement is classified (CREATE TABLE, SELECT, INSERT, UPDATE, etc.)
4. **Formatting** - each statement type has a dedicated formatter that produces well-structured output; subqueries are formatted recursively with indentation

## Testing

The project includes **32 integration tests**. Most format sample SQL inputs and compare the output against golden files; the rest check that malformed SQL is refused rather than reformatted.

```bash
# Run all integration tests
cargo test
```

Test inputs and expected outputs live in the [`tests/data/`](tests/data) directory. Each test case has an `.input.sql` file and a matching `.golden.sql` file with the expected formatted output. To add a new test, create a pair of files and add a test function in [`tests/integration_test.rs`](tests/integration_test.rs).



## Development

This is a Rust project. To test locally without releasing:

```bash
cargo build --release
cp target/release/reesql ~/.local/bin/   # macOS/Linux
```

### Release workflow

Releases are cut from a single machine (the Mac mini). `release.sh` cross-builds
**all six targets** and publishes them as one GitHub Release:

Version numbers use date-based `YY.MM.PATCH` releases.

- macOS arm64/x64 (native `cargo build`)
- Linux x64/arm64 (`cargo zigbuild`)
- Windows x64/arm64 (`cargo xwin build`)

```bash
bash release.sh            # bump, tag, cross-build all targets, publish
bash release.sh --minor    # bump the month component instead of patch
bash release.sh --draft    # publish the release as a draft
```

One-time setup on the Mac:

```bash
brew install zig
cargo install cargo-zigbuild cargo-xwin
rustup target add x86_64-unknown-linux-gnu aarch64-unknown-linux-gnu x86_64-pc-windows-msvc aarch64-pc-windows-msvc
```
