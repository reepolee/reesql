use clap::Parser;
use std::collections::HashSet;
use std::fs;
use std::io::{self, Read};
use std::path::PathBuf;
use std::sync::LazyLock;

fn display_version() -> String {
    let mut parts = env!("CARGO_PKG_VERSION").split('.');
    format!("{}.{:02}.{}", parts.next().unwrap_or("0"), parts.next().unwrap_or("0").parse::<u32>().unwrap_or(0), parts.next().unwrap_or("0"))
}

#[derive(Parser)]
#[command(name = "reesql", disable_version_flag = true, about = "MySQL SQL formatter")]
struct Cli {
    #[arg(short = 'v', long = "version", action = clap::ArgAction::SetTrue, help = "Print the bare version number")]
    version: bool,

    #[arg(long = "where", action = clap::ArgAction::SetTrue)]
    where_: bool,
    file: Option<String>,
}

fn executable_dir() -> PathBuf {
    let exe = std::env::current_exe().unwrap_or_else(|e| {
        eprintln!("reesql: error determining executable path: {}", e);
        std::process::exit(1);
    });

    exe.parent()
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            eprintln!("reesql: executable path has no parent directory");
            std::process::exit(1);
        })
}

/// What a token is, for spacing and statement-shape decisions. The token's *text* always
/// comes from the source, never from this tag — the tag only says how to space it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Word,
    Comment,
    OpenParen,
    CloseParen,
    Comma,
    Semicolon,
    Equals,
    Dot,
    Star,
    GreaterThan,
    LessThan,
    GreaterOrEqual,
    LessOrEqual,
    /// `!=` or `<>`; which one is answered by the token's text, not by this tag.
    NotEquals,
    Concat,
    DoubleColon,
    /// Arithmetic or bitwise operator, spaced like the comparison operators.
    Operator,
    /// Any other character the tokenizer has no specific rule for.
    Symbol,
}

/// A token borrows its text directly from the input. Formatting can therefore only ever
/// re-emit the user's own bytes (optionally upper-cased, when they spell a keyword) plus
/// whitespace it chooses to put between them — it has no way to invent or alter SQL.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Token<'a> {
    kind: Kind,
    text: &'a str,
    line: usize,
}

impl<'a> Token<'a> {
    fn is(&self, kind: Kind) -> bool {
        self.kind == kind
    }

    fn is_word(&self, word: &str) -> bool {
        self.kind == Kind::Word && self.text.eq_ignore_ascii_case(word)
    }

    /// A line comment runs to end of line, so what follows it must start on a new line.
    fn is_line_comment(&self) -> bool {
        self.kind == Kind::Comment && (self.text.starts_with("--") || self.text.starts_with('#'))
    }

    /// The only transformation the formatter is allowed to apply to a token's text.
    fn emit(&self) -> String {
        if self.kind == Kind::Word && is_keyword(self.text) {
            self.text.to_uppercase()
        } else {
            self.text.to_string()
        }
    }
}

static KEYWORDS: LazyLock<HashSet<&str>> = LazyLock::new(|| {
    [
        // MySQL keywords (original)
        "ACCESSIBLE", "ADD", "ALL", "ALTER", "ANALYZE", "AND", "AS", "ASC", "ASENSITIVE",
        "AUTO_INCREMENT",
        "BEFORE", "BETWEEN", "BIGINT", "BINARY", "BLOB", "BOTH", "BY", "CALL", "CASCADE",
        "CASE", "CHANGE", "CHAR", "CHARACTER", "CHECK", "COLLATE", "COLUMN", "COMMENT", "CONDITION",
        "CONSTRAINT", "CONTINUE", "CONVERT", "CREATE", "CROSS", "CUBE", "CUME_DIST",
        "CURRENT_DATE", "CURRENT_TIME", "CURRENT_TIMESTAMP", "CURRENT_USER", "CURSOR",
        "DATABASE", "DATABASES", "DAY_HOUR", "DAY_MICROSECOND", "DAY_MINUTE",
        "DAY_SECOND", "DEC", "DECIMAL", "DECLARE", "DEFAULT", "DELAYED", "DELETE",
        "DENSE_RANK", "DESC", "DESCRIBE", "DETERMINISTIC", "DISTINCT", "DISTINCTROW",
        "DIV", "DOUBLE", "DROP", "DUAL", "DUPLICATE", "EACH", "ELSE", "ELSEIF", "EMPTY", "ENCLOSED",
        "ESCAPED", "EXCEPT", "EXISTS", "EXIT", "EXPLAIN", "FALSE", "FETCH", "FLOAT",
        "FLOAT4", "FLOAT8", "FOR", "FORCE", "FOREIGN", "FROM", "FULLTEXT", "FUNCTION",
        "GENERATED", "GET", "GRANT", "GROUP", "GROUPING", "GROUPS", "HAVING",
        "HIGH_PRIORITY", "HOUR_MICROSECOND", "HOUR_MINUTE", "HOUR_SECOND", "IF",
        "IGNORE", "IN", "INDEX", "INFILE", "INNER", "INOUT", "INSENSITIVE", "INSERT",
        "INT", "INT1", "INT2", "INT3", "INT4", "INT8", "INTEGER", "INTERVAL", "INTO",
        "IS", "ITERATE", "JOIN", "JSON_TABLE", "KEY", "KEYS", "KILL", "LAG", "LATERAL",
        "LEAD", "LEADING", "LEAVE", "LEFT", "LIKE", "LIMIT", "LINEAR", "LINES",
        "LOAD", "LOCALTIME", "LOCALTIMESTAMP", "LOCK", "LONG", "LONGBLOB", "LONGTEXT",
        "LOOP", "LOW_PRIORITY", "MASTER_BIND", "MASTER_SSL_VERIFY_SERVER_CERT",
        "MATCH", "MAXVALUE", "MEDIUMBLOB", "MEDIUMINT", "MEDIUMTEXT",
        "MEMBER", "MIDDLEINT", "MINUTE_MICROSECOND", "MINUTE_SECOND", "MOD", "MODIFIES",
        "NATURAL", "NOT", "NO_WRITE_TO_BINLOG", "NTH_VALUE", "NTILE", "NULL",
        "NUMERIC", "OF", "ON", "OPTIMIZE", "OPTIMIZER_COSTS", "OPTION", "OPTIONALLY",
        "OR", "ORDER", "OUT", "OUTER", "OUTFILE", "OVER", "PARTIAL", "PARTITION",
        "PERCENT_RANK", "PRECISION", "PRIMARY", "PROCEDURE", "PURGE", "RANGE", "RANK",
        "READ", "READS", "READ_WRITE", "REAL", "RECURSIVE", "REFERENCES", "REGEXP",
        "RELEASE", "RENAME", "REPEAT", "REPLACE", "REQUIRE", "RESIGNAL", "RESTRICT",
        "RETURN", "REVOKE", "RIGHT", "RLIKE", "ROW", "ROWS", "ROW_NUMBER",
        "SCHEMA", "SCHEMAS", "SECOND_MICROSECOND", "SELECT", "SENSITIVE", "SEPARATOR",
        "SET", "SHOW", "SIGNAL", "SMALLINT", "SPATIAL", "SPECIFIC", "SQL",
        "SQLEXCEPTION", "SQLSTATE", "SQLWARNING", "SQL_BIG_RESULT",
        "SQL_CALC_FOUND_ROWS", "SQL_SMALL_RESULT", "SSL", "STARTING", "STORED",
        "STRAIGHT_JOIN", "SYSTEM", "TABLE", "TERMINATED", "THEN",
        "TEXT", "TIME", "TIMESTAMP", "TINYBLOB",
        "TINYINT", "TINYTEXT", "TO", "TRAILING", "TRIGGER", "TRUE", "UNDO", "UNION",
        "UNIQUE", "UNLOCK", "UNSIGNED", "UPDATE", "USAGE", "USE", "USING",
        "UTC_DATE", "UTC_TIME", "UTC_TIMESTAMP",
        "VALUES",
        "VARBINARY", "VARCHAR",
        "VARCHARACTER", "VARYING", "VIRTUAL", "VIEW",
        "YEAR",
        "WHEN", "WHERE", "WHILE",
        "WINDOW", "WITH", "WRITE", "XOR", "YEAR_MONTH",
        // Common data types
        "BIT", "BOOL", "BOOLEAN", "DATETIME", "DATE", "ENUM", "JSON", "SERIAL",
        "ZEROFILL",
        // Common functions to uppercase
        "CONCAT_WS", "IFNULL", "COALESCE", "NOW",

        // ===== SQLite-specific keywords =====
        "ABORT", "ATTACH", "AUTOINCREMENT", "CONFLICT", "DETACH",
        "EXCLUSIVE", "FAIL", "IGNORE", "IMMEDIATE", "INDEXED",
        "INSTEAD", "OID", "PLAN", "PRAGMA", "QUERY",
        "RAISE", "REINDEX", "ROLLBACK", "ROWID", "SAVEPOINT",
        "VACUUM", "WITHOUT",
        // SQLite types
        "INT", "INTEGER", "TEXT", "REAL", "BLOB", "NUMERIC",
        // SQLite functions
        "ABS", "CHANGES", "CHAR", "GLOB", "HEX",
        "INSTR", "LAST_INSERT_ROWID", "LENGTH", "LIKELIHOOD", "LIKELY",
        "LOWER", "LTRIM", "MAX", "MIN", "NULLIF",
        "PRINTF", "QUOTE", "RANDOM", "RANDOMBLOB",
        "ROUND", "RTRIM", "SIGN", "SUBSTR", "SUBSTRING",
        "TRIM", "TYPEOF", "UNICODE", "UNLIKELY",
        "UPPER", "ZEROBLOB",
        // SQLite pragmas / misc
        "ANALYZE", "COMMIT", "END", "TRANSACTION", "BEGIN",
        "DEALLOCATE", "PREPARE", "EXECUTE", "NOTHING",

        // ===== PostgreSQL-specific keywords =====
        "ADMIN", "AFTER", "AGGREGATE", "ALSO", "ARRAY",
        "ASSERTION", "ASSIGNMENT", "ASYMMETRIC", "AT", "AUTHORIZATION",
        "BEFORE",
        "CACHE", "CALL", "CALLED", "CATALOG", "CHAIN",
        "CHECKPOINT", "CLASS", "CLOSE", "CLUSTER",
        "COMMENTS", "COMMIT", "COMMITTED", "CONCURRENTLY", "CONFIGURATION",
        "CONNECTION", "CONTENT", "CONTENTS", "CONVERSION", "COPY",
        "COST", "CSV", "CURRENT_CATALOG", "CURRENT_ROLE", "CURRENT_SCHEMA",
        "CYCLE",
        "DAY", "DEALLOCATE", "DEFAULTS", "DEFERRABLE",
        "DEFERRED", "DEFINER", "DELIMITER", "DELIMITERS", "DEPENDS",
        "DICTIONARY", "DISABLE", "DISCARD", "DO", "DOCUMENT",
        "DOMAIN",
        "ENABLE", "ENCODING", "ENCRYPTED", "ENUM", "ESCAPE",
        "EVENT", "EXCLUDE", "EXCLUDING", "EXECUTE",
        "EXTENSION", "EXTERNAL",
        "FAMILY", "FETCH", "FILTER", "FIRST", "FLOOR",
        "FOLLOWING", "FORCE", "FORMAT", "FORWARD", "FREEZE",
        "FUNCTIONS",
        "GLOBAL", "GRANTED", "GREATEST",
        "HANDLER", "HEADER",
        "HOLD", "HOUR",
        "IDENTITY", "ILIKE", "IMMUTABLE", "IMPLICIT", "IMPORT",
        "INCLUDING", "INCREMENT", "INDEXES", "INHERIT", "INHERITS",
        "INLINE", "INPUT", "INVOKER", "ISOLATION",
        "ISNULL",
        "LABEL", "LANGUAGE", "LARGE", "LAST", "LEAKPROOF",
        "LEAST", "LEVEL", "LISTEN", "LOCAL", "LOCATION",
        "LOCKED", "LOGGED",
        "MAPPING", "MATERIALIZED", "METHOD", "MINUTE", "MINVALUE",
        "MONTH", "MOVE",
        "NAMES", "NATIONAL", "NCHAR",
        "NEXT", "NONE", "NOTHING", "NOTIFY", "NOTNULL",
        "NOWAIT", "NULLS",
        "OBJECT", "OFF", "OIDS", "ONLY",
        "OPERATOR", "OPTION", "OPTIONS", "ORDINALITY", "OTHERS",
        "OVERLAY", "OWNED", "OWNER",
        "PARALLEL", "PARSER", "PARTITION", "PARTITIONS", "PASSING",
        "PASSWORD", "PLACING", "PLANS", "POLICY", "POSITION",
        "PRECEDING", "PRESERVE", "PREPARE", "PREPARED", "PRIOR",
        "PRIVILEGES", "PROCEDURAL", "PROCEDURES", "PROGRAM", "PUBLICATION",
        "QUOTE",
        "RANGE", "REASSIGN", "RECHECK", "REF", "REFERENCING",
        "REINDEX", "RELATIVE", "RELEASE", "REPEATABLE", "REPLICA",
        "RESET", "RESTART", "RESTRICT", "RETURNING", "RETURNS",
        "ROLLUP", "ROUTINE", "ROUTINES",
        "RULE",
        "SAVEPOINT", "SCROLL", "SEARCH", "SECOND", "SECURITY",
        "SEQUENCE", "SEQUENCES", "SERIALIZABLE", "SERVER", "SESSION",
        "SHARE", "SIMILAR", "SIMPLE", "SKIP", "SNAPSHOT",
        "SOME", "SPECIFICTYPE", "STANDALONE", "STATEMENT",
        "STATISTICS", "STDIN", "STDOUT", "STORAGE", "STRICT",
        "STRIP", "SUBSCRIPTION", "SUPPORT", "SYMMETRIC", "SYSID",
        "TABLES", "TABLESAMPLE", "TABLESPACE", "TEMP", "TEMPORARY",
        "THAN", "TIES", "TRANSACTION", "TRANSFORM", "TREAT",
        "TRUNCATE", "TRUSTED", "TYPE", "TYPES",
        "UNBOUNDED", "UNCOMMITTED", "UNENCRYPTED", "UNKNOWN", "UNLISTEN",
        "UNLOGGED", "UNTIL",
        "VACUUM", "VALID", "VALIDATE", "VALIDATOR",
        "VARIADIC", "VERBOSE", "VIEWS", "VOLATILE",
        "WHITESPACE", "WITHIN", "WORK", "WRAPPER",
        "XMLAGG", "XMLATTRIBUTES", "XMLCONCAT", "XMLELEMENT",
        "XMLFOREST", "XMLNAMESPACES", "XMLPARSE", "XMLPI", "XMLROOT",
        "XMLSERIALIZE", "XMLTABLE",
        "YES",
        "ZONE",
        // PostgreSQL types
        "BIGSERIAL", "SMALLSERIAL", "SERIAL", "SERIAL2", "SERIAL4", "SERIAL8",
        "UUID", "CIDR", "INET", "MACADDR",
        "BYTEA", "MONEY", "MACADDR8",
        "INTERVAL", "TIMETZ", "TIMESTAMPTZ",
        // PostgreSQL functions
        "ARRAY_AGG", "STRING_AGG",
        "JSONB_BUILD_OBJECT",
        "JSONB_AGG", "TO_JSONB",
        "EXTRACT", "DATE_TRUNC", "STATEMENT_TIMESTAMP",
        "CLOCK_TIMESTAMP",
        "FIRST_VALUE", "LAST_VALUE", "NTH_VALUE",
        "SPLIT_PART",
    ]
    .iter()
    .copied()
    .collect()
});

// Words that mark the start of constraints in a column definition
static CONSTRAINT_STARTS: LazyLock<HashSet<&str>> = LazyLock::new(|| {
    [
        "NOT", "NULL", "DEFAULT", "AUTO_INCREMENT", "PRIMARY", "UNIQUE",
        "REFERENCES", "CHECK", "ON", "COMMENT", "COLLATE", "GENERATED",
    ]
    .iter()
    .copied()
    .collect()
});

static TABLE_CONSTRAINT_STARTS: LazyLock<HashSet<&str>> = LazyLock::new(|| {
    [
        "UNIQUE", "PRIMARY", "FOREIGN", "CHECK", "INDEX", "KEY", "CONSTRAINT",
        "FULLTEXT", "SPATIAL",
    ]
    .iter()
    .copied()
    .collect()
});

fn is_keyword(word: &str) -> bool {
    KEYWORDS.contains(word.to_uppercase().as_str())
}

fn is_constraint_start(word: &str) -> bool {
    CONSTRAINT_STARTS.contains(word.to_uppercase().as_str())
}

fn is_table_constraint_start(word: &str) -> bool {
    TABLE_CONSTRAINT_STARTS.contains(word.to_uppercase().as_str())
}

fn token_width(tok: &Token) -> usize {
    // Comments do not participate in column alignment.
    if tok.kind == Kind::Comment {
        0
    } else {
        tok.text.chars().count()
    }
}

fn tokens_display_width(tokens: &[Token]) -> usize {
    let mut width = 0;
    let mut prev_was_word = false;
    for tok in tokens {
        if prev_was_word && tok.is(Kind::Word) {
            width += 1;
        }
        width += token_width(tok);
        prev_was_word = tok.is(Kind::Word);
    }
    width
}

/// Tokenizes `input`. Every token borrows its text directly from `input`, so the tokens plus
/// the whitespace between them account for every byte of the source — see [`tokens_tile`].
fn tokenize(input: &str) -> Vec<Token<'_>> {
    let chars: Vec<char> = input.chars().collect();
    // Byte offset of each char, so a char range can be turned into a slice of `input`.
    let byte_at: Vec<usize> = input.char_indices().map(|(b, _)| b).collect();
    let byte_of = |idx: usize| byte_at.get(idx).copied().unwrap_or(input.len());

    let mut line_of = Vec::with_capacity(chars.len());
    let mut line = 1;
    for &c in &chars {
        line_of.push(line);
        if c == '\n' {
            line += 1;
        }
    }

    let mut tokens = Vec::new();
    let mut i = 0;

    while i < chars.len() {
        let start = i;
        let c = chars[i];

        // Each arm advances `i` past the token and names its kind; the text is then taken
        // from the source. No arm builds text of its own.
        let kind = match c {
            '(' => { i += 1; Some(Kind::OpenParen) }
            ')' => { i += 1; Some(Kind::CloseParen) }
            ',' => { i += 1; Some(Kind::Comma) }
            ';' => { i += 1; Some(Kind::Semicolon) }
            '=' => { i += 1; Some(Kind::Equals) }
            '.' => { i += 1; Some(Kind::Dot) }
            '*' => { i += 1; Some(Kind::Star) }
            '>' => {
                if chars.get(i + 1) == Some(&'=') {
                    i += 2;
                    Some(Kind::GreaterOrEqual)
                } else {
                    i += 1;
                    Some(Kind::GreaterThan)
                }
            }
            '<' => {
                if chars.get(i + 1) == Some(&'=') {
                    i += 2;
                    Some(Kind::LessOrEqual)
                } else if chars.get(i + 1) == Some(&'>') {
                    i += 2;
                    Some(Kind::NotEquals)
                } else {
                    i += 1;
                    Some(Kind::LessThan)
                }
            }
            '|' => {
                if chars.get(i + 1) == Some(&'|') {
                    i += 2;
                    Some(Kind::Concat)
                } else {
                    i += 1;
                    Some(Kind::Operator)
                }
            }
            '!' => {
                if chars.get(i + 1) == Some(&'=') {
                    i += 2;
                    Some(Kind::NotEquals)
                } else {
                    i += 1;
                    Some(Kind::Operator)
                }
            }
            ':' => {
                if chars.get(i + 1) == Some(&':') {
                    i += 2;
                    Some(Kind::DoubleColon)
                } else {
                    // Lone `:` — part of MySQL's `:=`, or a bind parameter.
                    i += 1;
                    Some(Kind::Symbol)
                }
            }
            '\'' => {
                i += 1;
                while i < chars.len() {
                    if chars[i] == '\'' {
                        // A doubled quote is an escaped quote, not the end of the literal.
                        if chars.get(i + 1) == Some(&'\'') {
                            i += 2;
                        } else {
                            i += 1;
                            break;
                        }
                    } else {
                        i += 1;
                    }
                }
                Some(Kind::Word)
            }
            // Quoted identifiers keep their quotes: they are part of the name.
            '`' | '"' => {
                let quote = c;
                i += 1;
                while i < chars.len() && chars[i] != quote {
                    i += 1;
                }
                if i < chars.len() {
                    i += 1;
                }
                Some(Kind::Word)
            }
            _ if c == '-' && chars.get(i + 1) == Some(&'-') => {
                while i < chars.len() && chars[i] != '\n' {
                    i += 1;
                }
                Some(Kind::Comment)
            }
            _ if c == '/' && chars.get(i + 1) == Some(&'*') => {
                i += 2;
                while i + 1 < chars.len() && !(chars[i] == '*' && chars[i + 1] == '/') {
                    i += 1;
                }
                if i + 1 < chars.len() {
                    i += 2;
                } else {
                    i = chars.len();
                }
                Some(Kind::Comment)
            }
            '#' => {
                while i < chars.len() && chars[i] != '\n' {
                    i += 1;
                }
                Some(Kind::Comment)
            }
            _ if c.is_whitespace() => {
                i += 1;
                None
            }
            _ if c.is_alphanumeric() || c == '_' => {
                while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                    i += 1;
                }
                Some(Kind::Word)
            }
            // Reached only after the `--`, `/*` and `#` rules above, so a lone `-` or `/`
            // here really is an operator. Every remaining character is still kept: dropping
            // one would silently change what the SQL means.
            _ => {
                i += 1;
                if matches!(c, '+' | '-' | '/' | '%' | '&' | '^' | '~') {
                    Some(Kind::Operator)
                } else {
                    Some(Kind::Symbol)
                }
            }
        };

        if let Some(kind) = kind {
            tokens.push(Token {
                kind,
                text: &input[byte_of(start)..byte_of(i)],
                line: line_of[start],
            });
        }
    }

    tokens
}

/// Byte offset of `sub` within `src`. Both must come from the same allocation, which holds
/// for token texts because they are always slices of the tokenizer's input.
fn offset_in(src: &str, sub: &str) -> usize {
    sub.as_ptr() as usize - src.as_ptr() as usize
}

/// Checks that the tokens tile the source: every byte is either inside a token or is
/// whitespace between two of them.
///
/// This is the guarantee the formatter rests on. Output is assembled only from token texts
/// and chosen whitespace, so if the tokens account for all the non-whitespace input, then no
/// character can be lost no matter what any formatter does. A lexer rule that skipped a
/// character it did not recognise would leave a non-whitespace gap here and be caught, rather
/// than silently deleting part of someone's SQL.
fn tokens_tile(src: &str, tokens: &[Token]) -> bool {
    let mut cursor = 0;
    for tok in tokens {
        let start = offset_in(src, tok.text);
        if start < cursor || !src[cursor..start].chars().all(char::is_whitespace) {
            return false;
        }
        cursor = start + tok.text.len();
    }
    src[cursor..].chars().all(char::is_whitespace)
}

struct Statement<'a> {
    tokens: &'a [Token<'a>],
    line: usize,
}

/// Splits the token stream into statements at top-level semicolons. Statements are slices of
/// the original stream, so no token is copied or rebuilt along the way.
fn split_statements<'a>(tokens: &'a [Token<'a>]) -> Vec<Statement<'a>> {
    let mut statements = Vec::new();
    let mut begin = 0;
    let mut start_line = 1;
    // A statement can open with comments; report the line of the SQL itself, not the comment.
    let mut start_is_comment = false;
    // A trigger's BEGIN...END body holds its own `;`, which must not split the statement.
    let mut words_seen = 0;
    let mut starts_with_create = false;
    let mut is_trigger = false;
    let mut body_depth = 0isize;

    for (idx, tok) in tokens.iter().enumerate() {
        if idx == begin {
            start_line = tok.line;
            start_is_comment = tok.is(Kind::Comment);
        } else if start_is_comment && !tok.is(Kind::Comment) {
            start_line = tok.line;
            start_is_comment = false;
        }

        if tok.is(Kind::Word) {
            if words_seen < 3 {
                words_seen += 1;
                if words_seen == 1 {
                    starts_with_create = tok.text.eq_ignore_ascii_case("CREATE");
                } else if starts_with_create && tok.text.eq_ignore_ascii_case("TRIGGER") {
                    is_trigger = true;
                }
            }
            if is_trigger {
                body_depth = (body_depth + block_depth_delta(tok.text)).max(0);
            }
        }

        if tok.is(Kind::Semicolon) && body_depth == 0 {
            statements.push(Statement {
                tokens: &tokens[begin..=idx],
                line: start_line,
            });
            begin = idx + 1;
            words_seen = 0;
            starts_with_create = false;
            is_trigger = false;
        }
    }

    if begin < tokens.len() {
        statements.push(Statement {
            tokens: &tokens[begin..],
            line: start_line,
        });
    }

    statements
}

/// Whether a space is required between two adjacent tokens. This, plus line breaks and
/// indentation chosen by the formatters, is the entire extent of what formatting may add.
fn needs_space(prev: &Token, tok: &Token) -> bool {
    use Kind::*;

    // A block comment is glued to neither side, so it needs separating from real tokens.
    if prev.kind == Comment {
        return !prev.is_line_comment() && matches!(tok.kind, Word | Operator);
    }

    match (prev.kind, tok.kind) {
        (Word, Word) => true,
        (Word, Equals) | (Equals, Word) | (Equals, OpenParen) => true,
        (CloseParen, Word) | (Comma, Word) => true,
        (Word, Star) | (Star, Word) => true,
        // Keep a comment off the token it trails.
        (Word | Star | CloseParen | Comma, Comment) => true,
        // Binary operators are spaced on both sides.
        (Word, GreaterThan | LessThan | GreaterOrEqual | LessOrEqual | NotEquals | Concat | Operator) => true,
        (GreaterThan | LessThan | GreaterOrEqual | LessOrEqual | NotEquals | Concat | Operator, Word) => true,
        (CloseParen, Operator) => true,
        // A closing bracket separates from what follows, like a closing paren does.
        (Symbol, Word) if prev.text == "]" => true,
        // Detach a symbol from a preceding word (`SET @x`), but never split a bracketed
        // subscript like `a[1]`. Nothing is glued to what follows, so prefixes such as
        // `@x` and `:=` stay intact.
        (Word, Symbol) if !matches!(tok.text, "[" | "]" | "{" | "}") => true,
        _ => false,
    }
}

/// Renders tokens with single spaces where the spacing rules call for them.
fn tokens_upper_string(tokens: &[Token]) -> String {
    let mut s = String::new();
    let mut prev: Option<&Token> = None;

    for tok in tokens {
        if let Some(p) = prev {
            if needs_space(p, tok) {
                s.push(' ');
            }
        }
        s.push_str(&tok.emit());

        // Whatever follows a line comment has to start on the next line.
        if tok.is_line_comment() {
            s.push('\n');
            prev = None;
        } else {
            prev = Some(tok);
        }
    }

    s
}

/// Renders tokens with no spacing between them, for INSERT value tuples.
fn tokens_upper_string_nospace(tokens: &[Token]) -> String {
    let mut s = String::new();
    for tok in tokens {
        if tok.kind == Kind::Comment {
            s.push(if tok.is_line_comment() { '\n' } else { ' ' });
        }
        s.push_str(&tok.emit());
    }
    s
}

/// The `n`th token that is not a comment, since comments may precede or interrupt a statement.
fn nth_code_token<'a, 'b>(tokens: &'b [Token<'a>], n: usize) -> Option<&'b Token<'a>> {
    tokens.iter().filter(|t| !t.is(Kind::Comment)).nth(n)
}

/// Whether the statement's leading words are exactly `words`.
fn starts_with_words(tokens: &[Token], words: &[&str]) -> bool {
    words
        .iter()
        .enumerate()
        .all(|(i, w)| nth_code_token(tokens, i).is_some_and(|t| t.is_word(w)))
}

fn is_create_table(tokens: &[Token]) -> bool {
    starts_with_words(tokens, &["CREATE", "TABLE"])
}

/// Returns the index of the view's `SELECT`, which is where its body starts.
fn is_create_view(tokens: &[Token]) -> Option<usize> {
    if !starts_with_words(tokens, &["CREATE"]) {
        return None;
    }
    let second = nth_code_token(tokens, 1)?;
    if !second.is_word("VIEW") && !second.is_word("OR") {
        return None;
    }

    tokens
        .iter()
        .enumerate()
        .filter(|(_, t)| !t.is(Kind::Comment))
        .skip(2)
        .find(|(_, t)| t.is_word("SELECT"))
        .map(|(idx, _)| idx)
}

fn is_insert(tokens: &[Token]) -> bool {
    starts_with_words(tokens, &["INSERT"])
}

fn is_drop(tokens: &[Token]) -> bool {
    starts_with_words(tokens, &["DROP"])
}

fn is_create_index(tokens: &[Token]) -> bool {
    starts_with_words(tokens, &["CREATE"])
        && nth_code_token(tokens, 1).is_some_and(|t| t.is_word("INDEX") || t.is_word("UNIQUE"))
}

/// Matches `CREATE [TEMP|TEMPORARY] TRIGGER ...`.
fn is_create_trigger(tokens: &[Token]) -> bool {
    if !starts_with_words(tokens, &["CREATE"]) {
        return false;
    }
    (1..3).any(|i| nth_code_token(tokens, i).is_some_and(|t| t.is_word("TRIGGER")))
}

/// `BEGIN` and `CASE` open a block that `END` closes. Used to tell a trigger's body
/// apart from the statement terminator, since its inner `;` do not end the trigger.
fn block_depth_delta(word: &str) -> isize {
    if word.eq_ignore_ascii_case("BEGIN") || word.eq_ignore_ascii_case("CASE") {
        1
    } else if word.eq_ignore_ascii_case("END") {
        -1
    } else {
        0
    }
}

#[derive(Debug)]
struct ColumnDef<'a> {
    name_tokens: &'a [Token<'a>],
    type_tokens: &'a [Token<'a>],
    constraint_tokens: &'a [Token<'a>],
}

/// Splits a slice at top-level commas, keeping each piece as a slice of the original.
fn split_top_level_commas<'a>(tokens: &'a [Token<'a>]) -> Vec<&'a [Token<'a>]> {
    let mut pieces = Vec::new();
    let mut begin = 0;
    let mut depth = 0;

    for (idx, tok) in tokens.iter().enumerate() {
        match tok.kind {
            Kind::OpenParen => depth += 1,
            Kind::CloseParen => depth -= 1,
            Kind::Comma if depth == 0 => {
                pieces.push(&tokens[begin..idx]);
                begin = idx + 1;
            }
            _ => {}
        }
    }
    if begin < tokens.len() {
        pieces.push(&tokens[begin..]);
    }

    pieces
}

fn parse_column_defs<'a>(inner_tokens: &'a [Token<'a>]) -> (Vec<ColumnDef<'a>>, Vec<&'a [Token<'a>]>) {
    let mut columns = Vec::new();
    let mut table_constraints = Vec::new();

    for item in split_top_level_commas(inner_tokens) {
        let Some(first) = item.first() else { continue };

        if first.is(Kind::Word) && is_table_constraint_start(first.text) {
            table_constraints.push(item);
            continue;
        }

        let (name_tokens, rest) = split_first_word(item);
        let (type_tokens, constraint_tokens) = split_type_and_constraints(rest);

        columns.push(ColumnDef {
            name_tokens,
            type_tokens,
            constraint_tokens,
        });
    }

    (columns, table_constraints)
}

/// Splits off the leading identifier (the column name) from the rest of its definition.
fn split_first_word<'a>(tokens: &'a [Token<'a>]) -> (&'a [Token<'a>], &'a [Token<'a>]) {
    for (i, tok) in tokens.iter().enumerate() {
        if tok.is(Kind::Comment) {
            continue;
        }
        return if tok.is(Kind::Word) {
            (&tokens[..=i], &tokens[i + 1..])
        } else {
            (&[], &tokens[i..])
        };
    }
    (&[], &[])
}

/// Splits a column's type from the constraints that follow it.
fn split_type_and_constraints<'a>(tokens: &'a [Token<'a>]) -> (&'a [Token<'a>], &'a [Token<'a>]) {
    for (i, tok) in tokens.iter().enumerate() {
        if tok.is(Kind::Word) && is_constraint_start(tok.text) {
            return (&tokens[..i], &tokens[i..]);
        }
    }
    (tokens, &[])
}

fn format_create_table(tokens: &[Token]) -> Result<String, String> {
    // Find opening paren position
    let open_paren_pos = tokens.iter().position(|t| t.is(Kind::OpenParen));

    // No column list: CREATE TABLE ... AS SELECT / LIKE, or a definition still being typed.
    // Emitting the column-list layout here would append a `(` that is not in the source.
    let Some(paren_pos) = open_paren_pos else {
        return format_generic(tokens);
    };

    let mut result = String::new();

    // Format: CREATE TABLE [IF NOT EXISTS] name (
    let prelude = &tokens[..paren_pos];
    let prelude_str = tokens_upper_string(prelude);
    result.push_str(&prelude_str);
    result.push_str(" (\n");

    {
        // Find matching close paren
        let mut depth = 0;
        let mut close_pos = None;
        for (i, tok) in tokens.iter().enumerate().skip(paren_pos) {
            match tok.kind {
                Kind::OpenParen => depth += 1,
                Kind::CloseParen => {
                    depth -= 1;
                    if depth == 0 {
                        close_pos = Some(i);
                        break;
                    }
                }
                _ => {}
            }
        }

        let Some(close_pos) = close_pos else {
            return Err(
                "this CREATE TABLE has an unterminated column list: missing `)`".to_string()
            );
        };

        let inner_tokens = &tokens[paren_pos + 1..close_pos];

        let (col_defs, table_constraints) = parse_column_defs(inner_tokens);

        if !col_defs.is_empty() {
            let max_name_width = col_defs
                .iter()
                .map(|c| tokens_display_width(c.name_tokens))
                .max()
                .unwrap_or(0);

            let max_type_width = col_defs
                .iter()
                .map(|c| tokens_display_width(c.type_tokens))
                .max()
                .unwrap_or(0);

            for (idx, col) in col_defs.iter().enumerate() {
                let name_str = tokens_upper_string(col.name_tokens);
                let type_str = tokens_upper_string(col.type_tokens);
                let constraint_str = tokens_upper_string(col.constraint_tokens);

                let name_padded = format!("{:width$}", name_str, width = max_name_width);
                let type_padded = format!("{:width$}", type_str, width = max_type_width);

                if idx < col_defs.len() - 1 || !table_constraints.is_empty() {
                    result.push_str(&format!(
                        "    {} {} {},\n",
                        name_padded, type_padded, constraint_str
                    ));
                } else {
                    result.push_str(&format!(
                        "    {} {} {}\n",
                        name_padded, type_padded, constraint_str
                    ));
                }


            }
        }

        for (idx, tc) in table_constraints.iter().enumerate() {
            let s = tokens_upper_string(tc);
            if idx < table_constraints.len() - 1 {
                result.push_str(&format!("    {},\n", s));
            } else {
                result.push_str(&format!("    {}\n", s));
            }
        }

        let mut trailing_tokens = &tokens[close_pos + 1..];
        let has_trailing_semi = trailing_tokens.last().is_some_and(|t| t.is(Kind::Semicolon));
        if has_trailing_semi {
            trailing_tokens = &trailing_tokens[..trailing_tokens.len() - 1];
        }
        let trailing = tokens_upper_string(trailing_tokens);
        if trailing.is_empty() {
            result.push(')');
        } else {
            result.push_str(&format!(") {}", trailing));
        }
    }

    // Semicolon
    if tokens.last().is_some_and(|t| t.is(Kind::Semicolon)) {
        result.push(';');
    }

    Ok(result)
}

fn format_insert(tokens: &[Token]) -> Result<String, String> {
    let values_pos = tokens.iter().position(|t| {
        if t.is(Kind::Word) {
            let w = t.text;
            w.to_uppercase() == "VALUES"
        } else {
            false
        }
    });

    let Some(values_idx) = values_pos else {
        return Ok(tokens_upper_string(tokens));
    };

    let prelude = &tokens[..=values_idx];
    let values_tokens = &tokens[values_idx + 1..];

    let mut prelude_str = tokens_upper_string(prelude);
    // Insert space before the column list paren
    prelude_str = prelude_str.replacen("(", " (", 1);

    // Parse value tuples, plus any clause that follows them (ON CONFLICT, RETURNING, ...)
    let (tuples, tail) = parse_value_tuples(values_tokens)?;

    let semicolon = if tokens.last().is_some_and(|t| t.is(Kind::Semicolon)) {
        ";"
    } else {
        ""
    };

    let tail_str = if tail.is_empty() {
        String::new()
    } else {
        format!(" {}", tokens_upper_string(tail))
    };

    let compact = {
        let tuple_strs: Vec<String> = tuples
            .iter()
            .map(|t| format!("({})", tokens_upper_string_nospace(t)))
            .collect();
        format!(
            "{} {}{}{}",
            prelude_str,
            tuple_strs.join(", "),
            tail_str,
            semicolon
        )
    };

    if compact.len() <= 100 {
        return Ok(compact);
    }

    // Multi-line format
    let mut result = format!("{}\n", prelude_str);
    for (i, tup) in tuples.iter().enumerate() {
        let tup_str = format!("({})", tokens_upper_string_nospace(tup));
        if i < tuples.len() - 1 {
            result.push_str(&format!("{},\n", tup_str));
        } else {
            result.push_str(&format!("{}{}{}\n", tup_str, tail_str, semicolon));
        }
    }

    // Trim trailing newline if it ends with the semicolon line already
    Ok(result.trim_end_matches('\n').to_string())
}

/// Splits the tokens after `VALUES` into the value tuples and whatever clause trails them
/// (`ON CONFLICT ...`, `ON DUPLICATE KEY ...`, `RETURNING ...`), which is returned verbatim.
///
/// Tuples are rendered without spaces between their tokens, so anything that is not really a
/// value tuple must never be folded into one — and dropping it would lose SQL outright. When
/// the token stream is not a value list, this fails instead of guessing.
fn parse_value_tuples<'a>(
    tokens: &'a [Token<'a>],
) -> Result<(Vec<&'a [Token<'a>]>, &'a [Token<'a>]), String> {
    let mut tuples = Vec::new();
    let mut depth = 0;
    // Index just past the opening paren while inside a tuple.
    let mut tuple_start: Option<usize> = None;
    // Set by a comma that closed a tuple: the value list promises another tuple next.
    let mut expect_tuple = false;

    for (idx, tok) in tokens.iter().enumerate() {
        match tok.kind {
            Kind::OpenParen if tuple_start.is_none() => {
                tuple_start = Some(idx + 1);
                expect_tuple = false;
            }
            Kind::OpenParen => depth += 1,
            Kind::CloseParen if tuple_start.is_some() && depth == 0 => {
                let begin = tuple_start.take().unwrap_or(idx);
                tuples.push(&tokens[begin..idx]);
            }
            Kind::CloseParen if tuple_start.is_some() => depth -= 1,
            _ if tuple_start.is_some() => {}
            Kind::Comma => expect_tuple = true,
            Kind::Semicolon => {}
            _ => {
                // A keyword directly after a closed tuple opens a trailing clause. After a
                // comma, or before any tuple at all, a value tuple was promised instead — so
                // this is a statement that ran into the INSERT, not a clause.
                if expect_tuple || tuples.is_empty() || !tok.is(Kind::Word) {
                    return Err(format!(
                        "this INSERT runs into `{}` between its VALUES tuples \
                         (is it missing its `;`, or a value list?)",
                        tok.emit()
                    ));
                }
                // The statement's own `;` is re-emitted by the caller.
                let mut tail = &tokens[idx..];
                if let Some((last, rest)) = tail.split_last() {
                    if last.is(Kind::Semicolon) {
                        tail = rest;
                    }
                }
                return Ok((tuples, tail));
            }
        }
    }

    if tuple_start.is_some() {
        return Err("this INSERT has an unterminated VALUES tuple: missing `)`".to_string());
    }
    if expect_tuple {
        return Err("this INSERT ends on a trailing `,`: a VALUES tuple is missing".to_string());
    }

    Ok((tuples, &[]))
}

fn format_create_index(tokens: &[Token]) -> String {
    tokens_upper_string(tokens)
}

/// Lays out `CREATE TRIGGER ... BEGIN <body> END;` with the header and `BEGIN` on one line,
/// each body statement formatted normally and indented, and `END;` on its own line.
fn format_create_trigger(tokens: &[Token]) -> Result<String, String> {
    let begin_idx = tokens
        .iter()
        .position(|t| t.is_word("BEGIN"));

    // No body block to lay out (or one not yet typed): keep every token as-is.
    let Some(begin_idx) = begin_idx else {
        return format_generic(tokens);
    };

    let mut depth = 1isize;
    let mut end_idx = None;
    for (i, tok) in tokens.iter().enumerate().skip(begin_idx + 1) {
        if tok.is(Kind::Word) {
            let w = tok.text;
            depth += block_depth_delta(w);
            if depth == 0 {
                end_idx = Some(i);
                break;
            }
        }
    }

    let Some(end_idx) = end_idx else {
        return Err("this CREATE TRIGGER has an unterminated body: missing `END`".to_string());
    };

    let mut result = tokens_upper_string(&tokens[..=begin_idx]);
    result.push('\n');

    for stmt in split_trigger_body(&tokens[begin_idx + 1..end_idx]) {
        let formatted = format_statement(stmt)?;
        for line in formatted.lines() {
            if line.is_empty() {
                result.push('\n');
            } else {
                result.push_str("    ");
                result.push_str(line);
                result.push('\n');
            }
        }
    }

    result.push_str("END");
    // Anything between END and the terminator (rare, but never drop it).
    let trailing = tokens_upper_string(
        tokens[end_idx + 1..]
            .split_last()
            .filter(|(last, _)| last.is(Kind::Semicolon))
            .map_or(&tokens[end_idx + 1..], |(_, rest)| rest),
    );
    if !trailing.is_empty() {
        result.push(' ');
        result.push_str(&trailing);
    }
    if tokens.last().is_some_and(|t| t.is(Kind::Semicolon)) {
        result.push(';');
    }

    Ok(result)
}

/// Splits a trigger body into its statements, keeping each terminating `;` attached.
fn split_trigger_body<'a>(tokens: &'a [Token<'a>]) -> Vec<&'a [Token<'a>]> {
    let mut statements = Vec::new();
    let mut begin = 0;
    let mut depth = 0isize;

    for (idx, tok) in tokens.iter().enumerate() {
        if tok.is(Kind::Word) {
            depth = (depth + block_depth_delta(tok.text)).max(0);
        }
        if tok.is(Kind::Semicolon) && depth == 0 {
            statements.push(&tokens[begin..=idx]);
            begin = idx + 1;
        }
    }

    if begin < tokens.len() {
        statements.push(&tokens[begin..]);
    }

    statements
}

fn format_drop(tokens: &[Token]) -> String {
    tokens_upper_string(tokens)
}

fn find_as_position(tokens: &[Token]) -> Option<usize> {
    let mut depth = 0;
    for (i, tok) in tokens.iter().enumerate() {
        if tok.is(Kind::OpenParen) {
            depth += 1;
        } else if tok.is(Kind::CloseParen) {
            depth -= 1;
        } else if depth == 0 && tok.is_word("AS") {
            return Some(i);
        }
    }
    None
}

/// Returns true if the tokens represent a simple column expression (just identifiers and dots),
/// meaning it should participate in AS-alignment width calculation.
fn is_simple_expression(tokens: &[Token]) -> bool {
    tokens
        .iter()
        .all(|t| t.is(Kind::Word) || t.is(Kind::Dot))
}

fn format_create_view(tokens: &[Token], select_pos: usize) -> String {
    let prelude = &tokens[..select_pos];
    let select_tokens = &tokens[select_pos..];

    let prelude_str = tokens_upper_string(prelude);

    // Parse SELECT columns until FROM
    let from_pos = select_tokens.iter().position(|t| {
        if t.is(Kind::Word) {
            let w = t.text;
            w.to_uppercase() == "FROM"
        } else {
            false
        }
    });

    let from_pos = match from_pos {
        Some(p) => p,
        None => return format!("{} {}", prelude_str, tokens_upper_string(select_tokens)),
    };

    let select_cols = &select_tokens[1..from_pos];
    let columns = parse_select_columns(select_cols);

    // Calculate max expression width before AS for vertical alignment.
    // Only simple expressions (plain column refs) participate in alignment;
    // complex expressions like CONCAT(...), COALESCE(...), etc. just flow naturally.
    let max_expr_width = {
        let mut max_width = 0;
        for col in &columns {
            if let Some(as_pos) = find_as_position(col) {
                let expr = &col[..as_pos];
                if is_simple_expression(expr) {
                    let width = tokens_display_width(expr);
                    if width > max_width {
                        max_width = width;
                    }
                }
            }
        }
        max_width
    };

    let mut result = prelude_str;
    result.push('\n');
    result.push_str("SELECT\n");

    for (idx, col_tokens) in columns.iter().enumerate() {
        let last = idx == columns.len() - 1;
        let has_concat = col_tokens.iter().any(|t| t.is(Kind::Concat));

        let col_str = if has_concat {
            // Concat columns use their own multi-line formatting
            format_view_column(col_tokens)
        } else if let Some(as_pos) = find_as_position(col_tokens) {
            let expr = &col_tokens[..as_pos];
            let expr_str = tokens_upper_string(expr);
            let rest_str = tokens_upper_string(&col_tokens[as_pos..]);
            if is_simple_expression(expr) {
                // Align AS keyword vertically with other simple columns
                format!("{:width$} {}", expr_str, rest_str, width = max_expr_width)
            } else {
                // Complex expressions (CONCAT, COALESCE, etc.) just flow naturally
                format!("{} {}", expr_str, rest_str)
            }
        } else {
            tokens_upper_string(col_tokens)
        };

        if last {
            result.push_str(&format!("    {}\n", col_str));
        } else {
            result.push_str(&format!("    {},\n", col_str));
        }
    }

    // FROM and rest - format with proper line breaks for JOIN/ON
    let rest_tokens = &select_tokens[from_pos..];
    let rest_formatted = format_from_clause_tokens(rest_tokens);
    result.push_str(&rest_formatted);

    // Ensure semicolon
    if tokens.last().is_some_and(|t| t.is(Kind::Semicolon)) && !result.ends_with(';') {
        result.push(';');
    }

    result
}

fn format_view_column(tokens: &[Token]) -> String {
    // Check if this column expression contains || operators
    let has_concat = tokens.iter().any(|t| t.is(Kind::Concat));
    if !has_concat {
        return tokens_upper_string(tokens);
    }

    // Split at || to get value segments: [folder], ['__'], [filename], ...
    let mut segments = Vec::new();
    let mut current = Vec::new();
    for tok in tokens {
        if tok.is(Kind::Concat) {
            if !current.is_empty() {
                segments.push(std::mem::take(&mut current));
            }
        } else {
            current.push(*tok);
        }
    }
    if !current.is_empty() {
        segments.push(current);
    }

    // Pair consecutive value segments as (a || b), with trailing || on each
    // pair except the last one
    let mut result = String::new();
    let mut i = 0;
    while i < segments.len() {
        let first_str = tokens_upper_string(&segments[i]);
        if i + 1 < segments.len() {
            let second_str = tokens_upper_string(&segments[i + 1]);
            result.push_str(&format!("{} || {}", first_str, second_str));
            if i + 2 < segments.len() {
                result.push_str(" ||");
                result.push_str("\n    ");
            }
            i += 2;
        } else {
            result.push_str(&first_str);
            i += 1;
        }
    }

    result
}

fn format_from_clause_tokens(tokens: &[Token]) -> String {
    let mut result = String::new();
    let mut i = 0;

    while i < tokens.len() {
        if tokens[i].is(Kind::Word) {
            let w = tokens[i].text;
            let upper = w.to_uppercase();
            if upper == "FROM" {
                result.push_str("FROM");
                i += 1;
                // Collect table reference
                let start = i;
                while i < tokens.len() {
                    if tokens[i].is(Kind::Word) {
                        let w = tokens[i].text;
                        let wu = w.to_uppercase();
                        if wu == "JOIN" || wu == "LEFT" || wu == "RIGHT" || wu == "INNER"
                            || wu == "CROSS" || wu == "NATURAL" || wu == "WHERE"
                            || wu == "GROUP" || wu == "ORDER" || wu == "LIMIT" || wu == "HAVING"
                        {
                            break;
                        }
                    }
                    if tokens[i].is(Kind::Semicolon) {
                        break;
                    }
                    i += 1;
                }
                let table_str = tokens_upper_string(&tokens[start..i]);
                result.push(' ');
                result.push_str(&table_str);
            } else if upper == "JOIN" || upper == "LEFT" || upper == "RIGHT"
                || upper == "INNER" || upper == "CROSS" || upper == "NATURAL"
            {
                result.push('\n');
                result.push_str("    ");
                let start = i;
                // Collect up to ON (or next JOIN)
                while i < tokens.len() {
                    if tokens[i].is(Kind::Word) {
                        let w = tokens[i].text;
                        if w.to_uppercase() == "ON" {
                            break;
                        }
                    }
                    if tokens[i].is(Kind::Semicolon) {
                        break;
                    }
                    i += 1;
                }
                let join_part = tokens_upper_string(&tokens[start..i]);
                result.push_str(&join_part);
                if i < tokens.len()
                    && tokens[i].is(Kind::Word) {
                        let w = tokens[i].text;
                        if w.to_uppercase() == "ON" {
                            result.push('\n');
                            result.push_str("        ");
                            let on_start = i;
                            while i < tokens.len() {
                                if tokens[i].is(Kind::Word) {
                                    let w = tokens[i].text;
                                    let wu = w.to_uppercase();
                                    if wu == "JOIN" || wu == "LEFT" || wu == "RIGHT"
                                        || wu == "INNER" || wu == "CROSS" || wu == "NATURAL"
                                        || wu == "WHERE" || wu == "GROUP" || wu == "ORDER"
                                    {
                                        break;
                                    }
                                }
                                if tokens[i].is(Kind::Semicolon) {
                                    break;
                                }
                                i += 1;
                            }
                            let on_part = tokens_upper_string(&tokens[on_start..i]);
                            result.push_str(&on_part);
                            continue;
                        }
                    }
            } else if upper == "WHERE" || upper == "GROUP" || upper == "ORDER" || upper == "HAVING" {
                result.push('\n');
                let start = i;
                while i < tokens.len() && !tokens[i].is(Kind::Semicolon) {
                    i += 1;
                }
                result.push_str(&tokens_upper_string(&tokens[start..i]));
                continue;
            } else {
                result.push_str(&tokens[i].emit());
                i += 1;
            }
        } else if tokens[i].is(Kind::Semicolon) {
            break;
        } else {
            result.push_str(&tokens[i].emit());
            i += 1;
        }
    }

    result
}

/// Splits a SELECT list into its columns at top-level commas.
fn parse_select_columns<'a>(tokens: &'a [Token<'a>]) -> Vec<&'a [Token<'a>]> {
    split_top_level_commas(tokens)
}

fn find_matching_paren(tokens: &[Token], open_idx: usize) -> Option<usize> {
    let mut depth = 0;
    for (i, tok) in tokens.iter().enumerate().skip(open_idx) {
        match tok.kind {
            Kind::OpenParen => depth += 1,
            Kind::CloseParen => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
    }
    None
}

fn is_subquery_start(tokens: &[Token], idx: usize) -> bool {
    tokens[idx].is(Kind::OpenParen)
        && idx + 1 < tokens.len()
        && tokens[idx + 1].is_word("SELECT")
        && find_matching_paren(tokens, idx).is_some()
}

fn is_update(tokens: &[Token]) -> bool {
    for tok in tokens {
        if tok.is(Kind::Comment) {
            continue;
        }
        if tok.is(Kind::Word) {
            let w = tok.text;
            return w.to_uppercase() == "UPDATE";
        }
        return false;
    }
    false
}

fn is_delete(tokens: &[Token]) -> bool {
    for tok in tokens {
        if tok.is(Kind::Comment) {
            continue;
        }
        if tok.is(Kind::Word) {
            let w = tok.text;
            return w.to_uppercase() == "DELETE";
        }
        return false;
    }
    false
}

fn format_update(tokens: &[Token]) -> String {
    let set_pos = tokens.iter().position(|t| {
        t.is_word("SET")
    });

    let Some(set_pos) = set_pos else {
        return tokens_upper_string(tokens);
    };

    // Find WHERE position (only after SET)
    let after_set = &tokens[set_pos + 1..];
    let where_in_set = after_set.iter().position(|t| {
        t.is_word("WHERE")
    });
    let where_pos = where_in_set.map(|p| set_pos + 1 + p);

    let mut result = String::new();

    // UPDATE table [clauses before SET]
    let before_str = tokens_upper_string(&tokens[..set_pos]);
    result.push_str(&before_str);
    result.push_str("\nSET");

    // SET assignments
    let set_end = where_pos.unwrap_or({
        if tokens.last().is_some_and(|t| t.is(Kind::Semicolon)) {
            tokens.len() - 1
        } else {
            tokens.len()
        }
    });
    let assignments_tokens = &tokens[set_pos + 1..set_end];
    let assignments = parse_select_columns(assignments_tokens);

    for (i, assign) in assignments.iter().enumerate() {
        let assign_str = tokens_upper_string(assign);
        if i < assignments.len() - 1 {
            result.push_str(&format!("\n    {},", assign_str));
        } else {
            result.push_str(&format!("\n    {}", assign_str));
        }
    }

    // WHERE and any remaining clauses
    if let Some(wp) = where_pos {
        result.push('\n');
        let remaining = &tokens[wp..];
        let remaining_str = if remaining.last().is_some_and(|t| t.is(Kind::Semicolon)) {
            tokens_upper_string(&remaining[..remaining.len() - 1])
        } else {
            tokens_upper_string(remaining)
        };
        result.push_str(&remaining_str);
    }

    // Semicolon
    if tokens.last().is_some_and(|t| t.is(Kind::Semicolon)) {
        result.push(';');
    }

    result
}

fn format_delete(tokens: &[Token]) -> String {
    let where_pos = tokens.iter().position(|t| {
        t.is_word("WHERE")
    });

    let mut result = String::new();

    if let Some(wp) = where_pos {
        // DELETE FROM table [clauses]
        let before_str = tokens_upper_string(&tokens[..wp]);
        result.push_str(&before_str);

        // WHERE clause on new line
        result.push('\n');
        let remaining = &tokens[wp..];
        let remaining_str = if remaining.last().is_some_and(|t| t.is(Kind::Semicolon)) {
            tokens_upper_string(&remaining[..remaining.len() - 1])
        } else {
            tokens_upper_string(remaining)
        };
        result.push_str(&remaining_str);
    } else {
        result.push_str(&tokens_upper_string(tokens));
    }

    // Semicolon
    if tokens.last().is_some_and(|t| t.is(Kind::Semicolon)) && !result.ends_with(';') {
        result.push(';');
    }

    result
}

/// Renders a statement reesql has no specific layout for, indenting any `(SELECT ...)`
/// subquery it contains.
fn format_generic(tokens: &[Token]) -> Result<String, String> {
    let mut result = String::new();
    let mut prev: Option<&Token> = None;
    let mut i = 0;

    while i < tokens.len() {
        if is_subquery_start(tokens, i) {
            let close = find_matching_paren(tokens, i).unwrap();

            let space_before = prev.is_some_and(|p| {
                matches!(
                    p.kind,
                    Kind::Word
                        | Kind::Comma
                        | Kind::CloseParen
                        | Kind::Equals
                        | Kind::Star
                        | Kind::GreaterThan
                        | Kind::LessThan
                        | Kind::GreaterOrEqual
                        | Kind::LessOrEqual
                        | Kind::NotEquals
                )
            });
            if space_before {
                result.push(' ');
            }

            // Format the subquery from its own tokens. Rendering it back to text and
            // re-tokenizing would put the contents through an extra round trip for no reason.
            let formatted = format_token_stream(&tokens[i + 1..close]).map_err(|e| e.message)?;

            result.push_str("(\n");
            for line in formatted.trim().lines() {
                result.push_str("    ");
                result.push_str(line);
                result.push('\n');
            }
            result.push(')');

            i = close + 1;
            prev = Some(&tokens[close]);
            continue;
        }

        let tok = &tokens[i];
        if let Some(p) = prev {
            if needs_space(p, tok) {
                result.push(' ');
            }
        }
        result.push_str(&tok.emit());

        if tok.is_line_comment() {
            result.push('\n');
            prev = None;
        } else {
            prev = Some(tok);
        }
        i += 1;
    }

    Ok(result)
}

fn format_statement(tokens: &[Token]) -> Result<String, String> {
    if tokens.is_empty() {
        return Ok(String::new());
    }

    if is_create_trigger(tokens) {
        format_create_trigger(tokens)
    } else if is_create_table(tokens) {
        format_create_table(tokens)
    } else if let Some(select_pos) = is_create_view(tokens) {
        Ok(format_create_view(tokens, select_pos))
    } else if is_insert(tokens) {
        format_insert(tokens)
    } else if is_create_index(tokens) {
        Ok(format_create_index(tokens))
    } else if is_drop(tokens) {
        Ok(format_drop(tokens))
    } else if is_update(tokens) {
        Ok(format_update(tokens))
    } else if is_delete(tokens) {
        Ok(format_delete(tokens))
    } else {
        format_generic(tokens)
    }
}

/// Refusal to format, pointing at the line the offending statement starts on.
struct FormatError {
    line: usize,
    message: String,
}

fn format_sql(input: &str) -> Result<String, FormatError> {
    // Normalize line endings: strip \r so Windows CRLF and Unix LF are handled identically
    let input = input.replace("\r\n", "\n").replace('\r', "");
    let tokens = tokenize(&input);

    // The formatters only ever emit token texts and whitespace, so as long as the tokens
    // account for every non-whitespace byte, nothing in the input can be lost. If that ever
    // fails to hold, refuse rather than write SQL that is missing part of the original.
    if !tokens_tile(&input, &tokens) {
        return Err(FormatError {
            line: 1,
            message: "internal error: the tokenizer did not account for all input".to_string(),
        });
    }

    format_token_stream(&tokens)
}

/// Formats an already-tokenized stream. Statement layout happens here so that nested
/// contexts (a subquery, a trigger body) can reuse it without a text round trip.
fn format_token_stream(tokens: &[Token]) -> Result<String, FormatError> {
    let statements = split_statements(tokens);

    let mut result = String::new();
    let mut prev_type: Option<String> = None;

    for stmt in &statements {
        let toks = stmt.tokens;
        if toks.is_empty() || (toks.len() == 1 && toks[0].is(Kind::Semicolon)) {
            continue;
        }

        let formatted = format_statement(toks).map_err(|message| FormatError {
            line: stmt.line,
            message,
        })?;
        if formatted.is_empty() {
            continue;
        }

        // Statements of a different kind than the previous one get a blank line before them.
        let current_type = toks
            .iter()
            .filter(|t| t.is(Kind::Word))
            .take(2)
            .map(|t| t.text.to_uppercase())
            .collect::<Vec<_>>()
            .join(" ");

        if prev_type.as_ref().is_some_and(|prev| *prev != current_type) {
            result.push('\n');
        }

        result.push_str(&formatted);
        result.push('\n');
        prev_type = Some(current_type);
    }

    Ok(result)
}

fn main() {
    let cli = Cli::parse();

    if cli.version {
        println!("{}", display_version());
        return;
    }

    if cli.where_ {
        println!("{}", executable_dir().display());
        return;
    }

    let input = if let Some(path) = &cli.file {
        fs::read_to_string(path).unwrap_or_else(|e| {
            eprintln!("reesql: error reading '{}': {}", path, e);
            std::process::exit(1);
        })
    } else {
        let mut buf = String::new();
        io::stdin().read_to_string(&mut buf).unwrap_or_else(|e| {
            eprintln!("reesql: error reading stdin: {}", e);
            std::process::exit(1);
        });
        buf
    };

    let source = cli.file.as_deref().unwrap_or("<stdin>");
    let output = format_sql(&input).unwrap_or_else(|e| {
        eprintln!("reesql: {}:{}: {}", source, e.line, e.message);
        eprintln!("reesql: refusing to format invalid SQL; input left unchanged");
        std::process::exit(1);
    });

    if let Some(path) = &cli.file {
        fs::write(path, &output).unwrap_or_else(|e| {
            eprintln!("reesql: error writing '{}': {}", path, e);
            std::process::exit(1);
        });
    } else {
        print!("{}", output);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The property the whole design rests on: whatever the input, the tokens account for
    /// every byte that is not whitespace.
    #[test]
    fn tokens_tile_every_input() {
        let cases = [
            "",
            "   \n\t ",
            "SELECT 1;",
            "select a+b, c-d, e/f, g%h from t where x<>1 and y!=2;",
            "set @x := 5;",
            "select `back`, \"double\", 'single', 'it''s' from t;",
            "select a[1], b{2}, c&d, e|f, g^h, ~i from t;",
            "-- line comment\n/* block */ # hash\nselect 1;",
            "create trigger t after update on x for each row begin update x set a=1; end;",
            "unterminated 'string",
            "unterminated /* block",
            "trailing backslash \\ and $dollar and ?param",
        ];

        for src in cases {
            let tokens = tokenize(src);
            assert!(tokens_tile(src, &tokens), "tokens did not tile: {src:?}");
        }
    }

    /// No printable character may be swallowed by the tokenizer, whatever it is. This is the
    /// regression guard for the class of bug where an unhandled character was skipped.
    #[test]
    fn no_printable_character_is_dropped() {
        for c in (0x20u8..0x7f).map(char::from) {
            let src = format!("select a {c} b from t;");
            let tokens = tokenize(&src);
            assert!(tokens_tile(&src, &tokens), "character {c:?} was dropped");
        }
    }

    /// Proves the check above can actually fail — otherwise it guarantees nothing.
    #[test]
    fn tiling_catches_a_dropped_token() {
        let src = "select a + b from t;";
        let mut tokens = tokenize(src);
        let before = tokens.len();
        tokens.retain(|t| t.text != "+");
        assert_eq!(tokens.len(), before - 1, "test setup: no `+` token found");

        assert!(
            !tokens_tile(src, &tokens),
            "a dropped token slipped past the tiling check"
        );
    }

    /// Token text is borrowed from the input, never rebuilt, so it is the user's own bytes.
    #[test]
    fn token_text_is_a_slice_of_the_input() {
        let src = "select a <> 'lit' from t;";
        for tok in tokenize(src) {
            let start = offset_in(src, tok.text);
            assert_eq!(&src[start..start + tok.text.len()], tok.text);
        }
    }

    /// Only keywords change, and only in case.
    #[test]
    fn emit_changes_nothing_but_keyword_case() {
        let src = "select Name, `Mixed`, 'KeepMe' from Users where a <> 1;";
        for tok in tokenize(src) {
            let emitted = tok.emit();
            assert!(
                emitted == tok.text || emitted == tok.text.to_uppercase(),
                "{:?} was rewritten to {:?}",
                tok.text,
                emitted
            );
        }
    }
}
