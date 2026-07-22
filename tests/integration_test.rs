use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

const DATA_DIR: &str = "tests/data";

#[test]
fn test_select_join() {
    run_golden_test("select_join");
}

#[test]
fn test_create_table() {
    run_golden_test("create_table");
}

#[test]
fn test_create_view() {
    run_golden_test("create_view");
}

#[test]
fn test_create_view_sqlite() {
    run_golden_test("create_view_sqlite");
}


#[test]
fn test_create_view_movies() {
    run_golden_test("create_view_movies");
}

#[test]
fn test_insert_short() {
    run_golden_test("insert_short");
}

#[test]
fn test_insert_long() {
    run_golden_test("insert_long");
}

#[test]
fn test_create_index() {
    run_golden_test("create_index");
}

#[test]
fn test_drop() {
    run_golden_test("drop");
}

#[test]
fn test_comments() {
    run_golden_test("comments");
}

#[test]
fn test_mixed() {
    run_golden_test("mixed");
}

#[test]
fn test_empty() {
    run_golden_test("empty");
}

#[test]
fn test_no_semicolon() {
    run_golden_test("no_semicolon");
}

#[test]
fn test_subquery() {
    run_golden_test("subquery");
}

#[test]
fn test_multi_create_table() {
    run_golden_test("multi_create_table");
}

#[test]
fn test_multi_select() {
    run_golden_test("multi_select");
}

#[test]
fn test_operators() {
    run_golden_test("operators");
}

#[test]
fn test_update() {
    run_golden_test("update");
}

#[test]
fn test_update_no_where() {
    run_golden_test("update_no_where");
}

#[test]
fn test_delete() {
    run_golden_test("delete");
}

#[test]
fn test_delete_no_where() {
    run_golden_test("delete_no_where");
}

#[test]
fn test_pg_cast() {
    run_golden_test("pg_cast");
}

#[test]
fn test_pg_ilike() {
    run_golden_test("pg_ilike");
}

#[test]
fn test_pg_returning() {
    run_golden_test("pg_returning");
}

#[test]
fn test_sqlite_features() {
    run_golden_test("sqlite_features");
}

#[test]
fn test_create_trigger() {
    run_golden_test("create_trigger");
}

#[test]
fn test_arithmetic() {
    run_golden_test("arithmetic");
}

#[test]
fn test_mysql_syntax() {
    run_golden_test("mysql_syntax");
}

/// A trigger's BEGIN...END body holds its own `;`, which must not split the statement
/// and leave a stray `END;` behind.
#[test]
fn test_trigger_body_is_not_split_on_inner_semicolon() {
    let input = "CREATE TRIGGER t AFTER UPDATE ON x FOR EACH ROW BEGIN \
                 UPDATE x SET a = 1 WHERE id = NEW.id;\n\nEND;\n";
    let (status, stdout, stderr) = run_reesql(input);

    assert!(status.success(), "reesql failed: {stderr}");
    assert!(
        stdout.trim_end().ends_with("END;"),
        "END; should close the trigger, got:\n{stdout}"
    );
    assert!(
        !stdout.contains("\n\nEND;"),
        "END; was left stranded as its own statement:\n{stdout}"
    );
}

#[test]
fn test_unterminated_trigger_body_is_rejected() {
    let input = "CREATE TRIGGER t AFTER UPDATE ON x FOR EACH ROW BEGIN \
                 UPDATE x SET a = 1 WHERE id = NEW.id;\n";
    let (status, stdout, stderr) = run_reesql(input);

    assert_eq!(status.code(), Some(1), "expected a clean refusal: {stderr}");
    assert!(stdout.is_empty(), "nothing should be written on refusal, got: {stdout}");
    assert!(stderr.contains("missing `END`"), "unexpected error: {stderr}");
}

/// Operators used to be dropped by the tokenizer, silently changing what the SQL computed.
#[test]
fn test_operators_are_never_dropped() {
    for (input, expected) in [
        ("select a + b from t;", "SELECT a + b FROM t;"),
        ("select a - b from t;", "SELECT a - b FROM t;"),
        ("select a / b from t;", "SELECT a / b FROM t;"),
        ("select a % b from t;", "SELECT a % b FROM t;"),
        // Both spellings of "not equal" survive as written.
        ("select * from t where a <> 1;", "SELECT * FROM t WHERE a <> 1;"),
        ("select * from t where a != 1;", "SELECT * FROM t WHERE a != 1;"),
        // Double-quoted identifiers keep their quotes.
        ("select \"my col\" from t;", "SELECT \"my col\" FROM t;"),
        // A lone `-` must not be mistaken for the start of a comment.
        ("select 5-3 from t;", "SELECT 5 - 3 FROM t;"),
        // MySQL's null-safe equal, and the bit shifts.
        ("select a<=>b from t;", "SELECT a <=> b FROM t;"),
        ("select a<<2, b>>3 from t;", "SELECT a << 2, b >> 3 FROM t;"),
    ] {
        let (status, stdout, stderr) = run_reesql(input);
        assert!(status.success(), "reesql failed on {input:?}: {stderr}");
        assert_eq!(stdout.trim_end(), expected, "for input {input:?}");
    }
}

/// An INSERT missing its `;` used to swallow the next statement into a VALUES tuple,
/// silently deleting `CREATE TABLE members` and joining the rest without spaces.
#[test]
fn test_unterminated_insert_is_rejected() {
    let input = "\
INSERT INTO teams (id, name) VALUES
(1,'U18'),

CREATE TABLE members (
    id   INTEGER NOT NULL PRIMARY KEY,
    name TEXT    NOT NULL DEFAULT ''
);
";
    let (status, stdout, stderr) = run_reesql(input);

    assert!(!status.success(), "expected a non-zero exit, got success");
    assert!(stdout.is_empty(), "nothing should be written on refusal, got: {stdout}");
    assert!(
        stderr.contains("<stdin>:1:") && stderr.contains("CREATE"),
        "error should point at line 1 and name the offending token, got: {stderr}"
    );
}

#[test]
fn test_trailing_comma_after_values_is_rejected() {
    let (status, stdout, stderr) = run_reesql("INSERT INTO t (a) VALUES (1),\n");

    assert!(!status.success(), "expected a non-zero exit, got success");
    assert!(stdout.is_empty(), "nothing should be written on refusal, got: {stdout}");
    assert!(stderr.contains("trailing `,`"), "unexpected error: {stderr}");
}

/// Clauses after the value list used to be dropped silently; they must survive intact.
#[test]
fn test_clauses_after_values_are_preserved() {
    for (input, expected) in [
        (
            "insert into t (a) values (1) returning id;",
            "INSERT INTO t (a) VALUES (1) RETURNING id;",
        ),
        (
            "insert into t (a) values (1) on conflict (a) do nothing;",
            "INSERT INTO t (a) VALUES (1) ON CONFLICT(a) DO NOTHING;",
        ),
        (
            "insert into t (a) values (1),(2) on duplicate key update a = 1;",
            "INSERT INTO t (a) VALUES (1), (2) ON DUPLICATE KEY UPDATE a = 1;",
        ),
    ] {
        let (status, stdout, stderr) = run_reesql(input);
        assert!(status.success(), "reesql failed on {input:?}: {stderr}");
        assert_eq!(stdout.trim_end(), expected, "for input {input:?}");
    }
}

#[test]
fn test_unterminated_values_tuple_is_rejected() {
    let (status, stdout, stderr) = run_reesql("INSERT INTO t VALUES (1,\n");

    assert!(!status.success(), "expected a non-zero exit, got success");
    assert!(stdout.is_empty(), "nothing should be written on refusal, got: {stdout}");
    assert!(
        stderr.contains("unterminated VALUES tuple"),
        "unexpected error: {stderr}"
    );
}

/// The same INSERT with its `;` restored must format cleanly, keeping every statement.
#[test]
fn test_terminated_insert_keeps_following_statement() {
    let input = "\
INSERT INTO teams (id, name) VALUES
(1,'U18');

CREATE TABLE members (
    id   INTEGER NOT NULL PRIMARY KEY,
    name TEXT    NOT NULL DEFAULT ''
);
";
    let (status, stdout, stderr) = run_reesql(input);

    assert!(status.success(), "reesql failed: {stderr}");
    assert!(
        stdout.contains("CREATE TABLE members ("),
        "CREATE TABLE was lost or mangled: {stdout}"
    );
    assert!(
        stdout.contains("INSERT INTO teams (id, name) VALUES (1,'U18');"),
        "INSERT was not formatted as expected: {stdout}"
    );
}

/// Saving mid-edit, before the closing `)` is typed, used to panic.
#[test]
fn test_unterminated_create_table_is_rejected() {
    let (status, stdout, stderr) = run_reesql("CREATE TABLE members (\n    id INTEGER\n");

    assert_eq!(status.code(), Some(1), "expected a clean refusal, not a panic: {stderr}");
    assert!(stdout.is_empty(), "nothing should be written on refusal, got: {stdout}");
    assert!(
        stderr.contains("unterminated column list"),
        "unexpected error: {stderr}"
    );
}

/// A CREATE TABLE with no column list used to have ` (` appended to it out of nowhere.
#[test]
fn test_create_table_without_column_list_is_left_alone() {
    for (input, expected) in [
        ("create table t2 as select * from t1;", "CREATE TABLE t2 AS SELECT * FROM t1;"),
        ("create table t2 like t1;", "CREATE TABLE t2 LIKE t1;"),
        // Still being typed: pass through rather than invent syntax.
        ("CREATE TABLE members", "CREATE TABLE members"),
    ] {
        let (status, stdout, stderr) = run_reesql(input);
        assert!(status.success(), "reesql failed on {input:?}: {stderr}");
        assert_eq!(stdout.trim_end(), expected, "for input {input:?}");
    }
}

/// The core guarantee: formatting only ever changes whitespace and the case of keywords.
/// Every other character must survive, in order. Stripping whitespace and uppercasing both
/// sides normalises away exactly the two permitted changes, so any remaining difference is
/// the formatter altering the SQL itself.
#[test]
fn test_formatting_only_changes_whitespace_and_case() {
    let squash = |s: &str| -> String {
        s.chars().filter(|c| !c.is_whitespace()).collect::<String>().to_uppercase()
    };

    let mut checked = 0;
    for entry in fs::read_dir(DATA_DIR).expect("read tests/data") {
        let path = entry.expect("dir entry").path();
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        if !name.ends_with(".input.sql") {
            continue;
        }

        let input = fs::read_to_string(&path).expect("read input");
        let (status, stdout, stderr) = run_reesql(&input);
        assert!(status.success(), "reesql failed on {name}: {stderr}");

        assert_eq!(
            squash(&input),
            squash(&stdout),
            "\n❌ {name}: formatting changed more than whitespace and keyword case\n\
             input:\n{input}\noutput:\n{stdout}"
        );
        checked += 1;
    }

    assert!(checked > 0, "no .input.sql fixtures were checked");
}

fn run_reesql(input: &str) -> (std::process::ExitStatus, String, String) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_reesql"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("Failed to spawn reesql process: {}", e));

    child
        .stdin
        .take()
        .expect("Failed to take stdin")
        .write_all(input.as_bytes())
        .unwrap_or_else(|e| panic!("Failed to write to stdin: {}", e));

    let output = child
        .wait_with_output()
        .expect("Failed to wait for reesql process");

    (
        output.status,
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

fn run_golden_test(name: &str) {
    let input_path = Path::new(DATA_DIR).join(format!("{}.input.sql", name));
    let golden_path = Path::new(DATA_DIR).join(format!("{}.golden.sql", name));

    let input = fs::read_to_string(&input_path)
        .unwrap_or_else(|e| panic!("Failed to read input file {:?}: {}", input_path, e));

    let expected = fs::read_to_string(&golden_path)
        .unwrap_or_else(|e| panic!("Failed to read golden file {:?}: {}", golden_path, e));

    let mut child = Command::new(env!("CARGO_BIN_EXE_reesql"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("Failed to spawn reesql process: {}", e));

    child
        .stdin
        .take()
        .expect("Failed to take stdin")
        .write_all(input.as_bytes())
        .unwrap_or_else(|e| panic!("Failed to write to stdin: {}", e));

    let output = child
        .wait_with_output()
        .expect("Failed to wait for reesql process");

    assert!(
        output.status.success(),
        "reesql exited with {}: {}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );

    let actual = String::from_utf8(output.stdout)
        .expect("Output is not valid UTF-8");

    // Normalize line endings for cross-platform golden file matching
    let expected = expected.replace("\r\n", "\n").replace('\r', "");
    let actual = actual.replace("\r\n", "\n").replace('\r', "");

    assert_eq!(
        actual, expected,
        "\n❌ Test '{}' failed\n{}\nExpected:\n{}───────\nActual:\n{}───────\n",
        name,
        fmt_diff(&expected, &actual),
        expected,
        actual,
    );
}

fn fmt_diff(expected: &str, actual: &str) -> String {
    let expected_lines: Vec<&str> = expected.lines().collect();
    let actual_lines: Vec<&str> = actual.lines().collect();

    let max = expected_lines.len().max(actual_lines.len());
    let mut diff = String::from("Diff:\n");

    for i in 0..max {
        let e = expected_lines.get(i).copied().unwrap_or("<EOF>");
        let a = actual_lines.get(i).copied().unwrap_or("<EOF>");
        if e != a {
            diff.push_str(&format!("  Line {}:\n    - {e:?}\n    + {a:?}\n", i + 1));
        }
    }
    diff
}
