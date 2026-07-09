use clap::Parser;
use std::collections::HashSet;
use std::fs;
use std::io::{self, Read};
use std::path::PathBuf;
use std::sync::LazyLock;

#[derive(Parser)]
#[command(name = "reesql", disable_version_flag = true, about = "MySQL SQL formatter")]
struct Cli {
    #[arg(long = "version", action = clap::ArgAction::SetTrue, help = "Print the bare version number")]
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

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Word(String),
    Comment(String),
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
    NotEquals,
    Concat,
    DoubleColon,
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
        "DIV", "DOUBLE", "DROP", "DUAL", "EACH", "ELSE", "ELSEIF", "EMPTY", "ENCLOSED",
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
    match tok {
        Token::Word(w) => w.len(),
        Token::Star => 1,
        Token::Comment(_) => 0,
        Token::GreaterOrEqual | Token::LessOrEqual | Token::NotEquals | Token::Concat | Token::DoubleColon => 2,
        _ => 1,
    }
}

fn tokens_display_width(tokens: &[Token]) -> usize {
    let mut width = 0;
    let mut prev_was_word = false;
    for tok in tokens {
        if prev_was_word && matches!(tok, Token::Word(_)) {
            width += 1;
        }
        width += token_width(tok);
        prev_was_word = matches!(tok, Token::Word(_));
    }
    width
}

fn tokenize(input: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = input.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];
        match c {
            '(' => { tokens.push(Token::OpenParen); i += 1; }
            ')' => { tokens.push(Token::CloseParen); i += 1; }
            ',' => { tokens.push(Token::Comma); i += 1; }
            ';' => { tokens.push(Token::Semicolon); i += 1; }
            '=' => { tokens.push(Token::Equals); i += 1; }
            '.' => { tokens.push(Token::Dot); i += 1; }
            '*' => { tokens.push(Token::Star); i += 1; }
            '>' => {
                if i + 1 < chars.len() && chars[i + 1] == '=' {
                    tokens.push(Token::GreaterOrEqual);
                    i += 2;
                } else {
                    tokens.push(Token::GreaterThan);
                    i += 1;
                }
            }
            '<' => {
                if i + 1 < chars.len() && chars[i + 1] == '=' {
                    tokens.push(Token::LessOrEqual);
                    i += 2;
                } else if i + 1 < chars.len() && chars[i + 1] == '>' {
                    tokens.push(Token::NotEquals);
                    i += 2;
                } else {
                    tokens.push(Token::LessThan);
                    i += 1;
                }
            }
            '|' => {
                if i + 1 < chars.len() && chars[i + 1] == '|' {
                    tokens.push(Token::Concat);
                    i += 2;
                } else {
                    i += 1;
                }
            }
            '!' => {
                if i + 1 < chars.len() && chars[i + 1] == '=' {
                    tokens.push(Token::NotEquals);
                    i += 2;
                } else {
                    i += 1;
                }
            }
            ':' => {
                if i + 1 < chars.len() && chars[i + 1] == ':' {
                    tokens.push(Token::DoubleColon);
                    i += 2;
                } else {
                    i += 1;
                }
            }
            '\'' => {
                let start = i;
                i += 1;
                while i < chars.len() {
                    if chars[i] == '\'' {
                        if i + 1 < chars.len() && chars[i + 1] == '\'' {
                            i += 2;
                        } else {
                            i += 1;
                            break;
                        }
                    } else {
                        i += 1;
                    }
                }
                tokens.push(Token::Word(chars[start..i].iter().collect()));
            }
            '`' => {
                let start = i;
                i += 1;
                while i < chars.len() && chars[i] != '`' {
                    i += 1;
                }
                if i < chars.len() { i += 1; }
                tokens.push(Token::Word(chars[start..i].iter().collect()));
            }
            _ if c == '-' && i + 1 < chars.len() && chars[i + 1] == '-' => {
                let start = i;
                while i < chars.len() && chars[i] != '\n' { i += 1; }
                tokens.push(Token::Comment(chars[start..i].iter().collect()));
            }
            _ if c == '/' && i + 1 < chars.len() && chars[i + 1] == '*' => {
                let start = i;
                i += 2;
                while i + 1 < chars.len() && !(chars[i] == '*' && chars[i + 1] == '/') {
                    i += 1;
                }
                if i + 1 < chars.len() { i += 2; } else { i = chars.len(); }
                tokens.push(Token::Comment(chars[start..i].iter().collect()));
            }
            _ if c == '#' => {
                let start = i;
                while i < chars.len() && chars[i] != '\n' { i += 1; }
                tokens.push(Token::Comment(chars[start..i].iter().collect()));
            }
            _ if c.is_whitespace() => {
                i += 1;
            }
            _ if c.is_alphanumeric() || c == '_' => {
                let start = i;
                while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                    i += 1;
                }
                tokens.push(Token::Word(chars[start..i].iter().collect()));
            }
            _ => {
                i += 1;
            }
        }
    }

    tokens
}

fn split_statements(tokens: &[Token]) -> Vec<Vec<Token>> {
    let mut statements = Vec::new();
    let mut current = Vec::new();

    for tok in tokens {
        let is_semi = matches!(tok, Token::Semicolon);
        current.push(tok.clone());
        if is_semi {
            statements.push(std::mem::take(&mut current));
        }
    }

    if !current.is_empty() {
        statements.push(current);
    }

    statements
}

fn token_upper_string(tok: &Token) -> String {
    match tok {
        Token::Word(w) => {
            if is_keyword(w) { w.to_uppercase() } else { w.clone() }
        }
        Token::Comment(c) => c.clone(),
        Token::OpenParen => "(".into(),
        Token::CloseParen => ")".into(),
        Token::Comma => ",".into(),
        Token::Semicolon => ";".into(),
        Token::Equals => "=".into(),
        Token::Dot => ".".into(),
        Token::Star => "*".into(),
        Token::GreaterThan => ">".into(),
        Token::LessThan => "<".into(),
        Token::GreaterOrEqual => ">=".into(),
        Token::LessOrEqual => "<=".into(),            Token::NotEquals => "!=".into(),
        Token::Concat => "||".into(),
        Token::DoubleColon => "::".into(),
    }
}

fn tokens_upper_string(tokens: &[Token]) -> String {
    let mut s = String::new();
    let mut prev: Option<&Token> = None;
    for tok in tokens {
        match tok {
            Token::Comment(c) => {
                if matches!(prev, Some(Token::Word(_)))
                    || matches!(prev, Some(Token::Star))
                    || matches!(prev, Some(Token::CloseParen))
                    || matches!(prev, Some(Token::Comma))
                {
                    s.push(' ');
                }
                s.push_str(c);
                if c.starts_with("--") || c.starts_with('#') {
                    s.push('\n');
                    prev = None;
                } else {
                    prev = Some(tok);
                }
                continue;
            }
            _ => {}
        }
        let need_space = match (prev, tok) {
            (Some(Token::Word(_)), Token::Word(_)) => true,
            (Some(Token::Word(_)), Token::Equals) => true,
            (Some(Token::Equals), Token::Word(_)) => true,
            (Some(Token::CloseParen), Token::Word(_)) => true,
            (Some(Token::Comma), Token::Word(_)) => true,
            (Some(Token::Word(_)), Token::Star) => true,
            (Some(Token::Star), Token::Word(_)) => true,
            (Some(Token::Star), Token::Comment(_)) => true,
            (Some(Token::Comment(c)), Token::Word(_)) if !c.starts_with("--") && !c.starts_with('#') => true,
            // Operators need spaces around them
            (Some(Token::Word(_)), Token::GreaterThan) => true,
            (Some(Token::GreaterThan), Token::Word(_)) => true,
            (Some(Token::Word(_)), Token::LessThan) => true,
            (Some(Token::LessThan), Token::Word(_)) => true,
            (Some(Token::Word(_)), Token::GreaterOrEqual) => true,
            (Some(Token::GreaterOrEqual), Token::Word(_)) => true,
            (Some(Token::Word(_)), Token::LessOrEqual) => true,
            (Some(Token::LessOrEqual), Token::Word(_)) => true,
            (Some(Token::Word(_)), Token::NotEquals) => true,
            (Some(Token::NotEquals), Token::Word(_)) => true,
            (Some(Token::Word(_)), Token::Concat) => true,
            (Some(Token::Concat), Token::Word(_)) => true,
            _ => false,
        };
        if need_space {
            s.push(' ');
        }
        s.push_str(&token_upper_string(tok));
        prev = Some(tok);
    }
    s
}

fn tokens_upper_string_nospace(tokens: &[Token]) -> String {
    let mut s = String::new();
    for tok in tokens {
        let t = token_upper_string(tok);
        if let Token::Comment(c) = tok {
            if c.starts_with("--") || c.starts_with('#') {
                s.push('\n');
            } else {
                s.push(' ');
            }
        }
        s.push_str(&t);
    }
    s
}

fn is_create_table(tokens: &[Token]) -> bool {
    let words: Vec<&Token> = tokens.iter().filter(|t| !matches!(t, Token::Comment(_))).collect();
    if words.len() >= 2 {
        if let (Token::Word(a), Token::Word(b)) = (&words[0], &words[1]) {
            return a.to_uppercase() == "CREATE" && b.to_uppercase() == "TABLE";
        }
    }
    false
}

fn is_create_view(tokens: &[Token]) -> Option<usize> {
    let words: Vec<(usize, &Token)> = tokens.iter().enumerate().filter(|(_, t)| !matches!(t, Token::Comment(_))).collect();
    if words.len() >= 3 {
        if let (Token::Word(a), Token::Word(b)) = (&words[0].1, &words[1].1) {
            if a.to_uppercase() == "CREATE"
                && (b.to_uppercase() == "VIEW" || b.to_uppercase() == "OR")
            {
                for (idx, tok) in words.iter().skip(2) {
                    if let Token::Word(w) = tok {
                        if w.to_uppercase() == "SELECT" {
                            return Some(*idx);
                        }
                    }
                }
            }
        }
    }
    None
}

fn is_insert(tokens: &[Token]) -> bool {
    for tok in tokens {
        if let Token::Comment(_) = tok {
            continue;
        }
        if let Token::Word(a) = tok {
            return a.to_uppercase() == "INSERT";
        }
        return false;
    }
    false
}

fn is_drop(tokens: &[Token]) -> bool {
    for tok in tokens {
        if let Token::Comment(_) = tok {
            continue;
        }
        if let Token::Word(a) = tok {
            return a.to_uppercase() == "DROP";
        }
        return false;
    }
    false
}

fn is_create_index(tokens: &[Token]) -> bool {
    let words: Vec<&Token> = tokens.iter().filter(|t| !matches!(t, Token::Comment(_))).collect();
    if words.len() >= 3 {
        if let (Token::Word(a), Token::Word(b)) = (&words[0], &words[1]) {
            if a.to_uppercase() == "CREATE" {
                return b.to_uppercase() == "INDEX" || b.to_uppercase() == "UNIQUE";
            }
        }
    }
    false
}

#[derive(Debug)]
struct ColumnDef {
    name_tokens: Vec<Token>,
    type_tokens: Vec<Token>,
    constraint_tokens: Vec<Token>,
}

fn parse_column_defs(inner_tokens: &[Token]) -> (Vec<ColumnDef>, Vec<Vec<Token>>) {
    let mut columns = Vec::new();
    let mut table_constraints = Vec::new();
    let mut current = Vec::new();
    let mut depth = 0;

    for tok in inner_tokens {
        match tok {
            Token::OpenParen => {
                depth += 1;
                current.push(tok.clone());
            }
            Token::CloseParen => {
                depth -= 1;
                current.push(tok.clone());
            }
            Token::Comma if depth == 0 => {
                table_constraints.push(std::mem::take(&mut current));
            }
            _ => {
                current.push(tok.clone());
            }
        }
    }
    if !current.is_empty() {
        table_constraints.push(current);
    }

    let mut actual_table_constraints = Vec::new();

    for item in table_constraints {
        if item.is_empty() {
            continue;
        }

        if let Token::Word(first) = &item[0] {
            if is_table_constraint_start(&first) {
                actual_table_constraints.push(item);
                continue;
            }
        }

        let (name_tokens, rest) = split_first_word(&item);
        let (type_tokens, constraint_tokens) = split_type_and_constraints(&rest);

        columns.push(ColumnDef {
            name_tokens,
            type_tokens,
            constraint_tokens,
        });
    }

    (columns, actual_table_constraints)
}

fn split_first_word(tokens: &[Token]) -> (Vec<Token>, Vec<Token>) {
    if tokens.is_empty() {
        return (vec![], vec![]);
    }
    // Skip leading comments
    for i in 0..tokens.len() {
        if let Token::Comment(_) = &tokens[i] {
            continue;
        }
        if let Token::Word(_) = &tokens[i] {
            return (tokens[..=i].to_vec(), tokens[i + 1..].to_vec());
        } else {
            return (vec![], tokens[i..].to_vec());
        }
    }
    (vec![], vec![])
}

fn split_type_and_constraints(tokens: &[Token]) -> (Vec<Token>, Vec<Token>) {
    for i in 0..tokens.len() {
        if let Token::Word(w) = &tokens[i] {
            if is_constraint_start(w) {
                return (tokens[..i].to_vec(), tokens[i..].to_vec());
            }
        }
    }
    (tokens.to_vec(), vec![])
}

fn format_create_table(tokens: &[Token]) -> String {
    // Find opening paren position
    let open_paren_pos = tokens.iter().position(|t| matches!(t, Token::OpenParen));

    let mut result = String::new();

    // Format: CREATE TABLE [IF NOT EXISTS] name (
    let open_pos = open_paren_pos.unwrap_or(tokens.len());
    let prelude = &tokens[..open_pos];
    let prelude_str = tokens_upper_string(prelude);
    result.push_str(&prelude_str);
    result.push_str(" (\n");

    if let Some(paren_pos) = open_paren_pos {
        // Find matching close paren
        let mut depth = 0;
        let mut close_pos = tokens.len();
        for (i, tok) in tokens.iter().enumerate().skip(paren_pos) {
            match tok {
                Token::OpenParen => depth += 1,
                Token::CloseParen => {
                    depth -= 1;
                    if depth == 0 {
                        close_pos = i;
                        break;
                    }
                }
                _ => {}
            }
        }

        let inner_tokens = &tokens[paren_pos + 1..close_pos];

        let (col_defs, table_constraints) = parse_column_defs(inner_tokens);

        if !col_defs.is_empty() {
            let max_name_width = col_defs
                .iter()
                .map(|c| tokens_display_width(&c.name_tokens))
                .max()
                .unwrap_or(0);

            let max_type_width = col_defs
                .iter()
                .map(|c| tokens_display_width(&c.type_tokens))
                .max()
                .unwrap_or(0);

            for (idx, col) in col_defs.iter().enumerate() {
                let name_str = tokens_upper_string(&col.name_tokens);
                let type_str = tokens_upper_string(&col.type_tokens);
                let constraint_str = tokens_upper_string(&col.constraint_tokens);

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
        let has_trailing_semi = matches!(trailing_tokens.last(), Some(Token::Semicolon));
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
    if matches!(tokens.last(), Some(Token::Semicolon)) {
        result.push(';');
    }

    result
}

fn format_insert(tokens: &[Token]) -> String {
    let values_pos = tokens.iter().position(|t| {
        if let Token::Word(w) = t {
            w.to_uppercase() == "VALUES"
        } else {
            false
        }
    });

    let Some(values_idx) = values_pos else {
        return tokens_upper_string(tokens);
    };

    let prelude = &tokens[..=values_idx];
    let values_tokens = &tokens[values_idx + 1..];

    let mut prelude_str = tokens_upper_string(prelude);
    // Insert space before the column list paren
    prelude_str = prelude_str.replacen("(", " (", 1);

    // Parse value tuples
    let tuples = parse_value_tuples(values_tokens);

    let semicolon = if matches!(tokens.last(), Some(Token::Semicolon)) {
        ";"
    } else {
        ""
    };

    let compact = {
        let tuple_strs: Vec<String> = tuples
            .iter()
            .map(|t| format!("({})", tokens_upper_string_nospace(t)))
            .collect();
        format!("{} {}{}", prelude_str, tuple_strs.join(", "), semicolon)
    };

    if compact.len() <= 100 {
        return compact;
    }

    // Multi-line format
    let mut result = format!("{}\n", prelude_str);
    for (i, tup) in tuples.iter().enumerate() {
        let tup_str = format!("({})", tokens_upper_string_nospace(tup));
        if i < tuples.len() - 1 {
            result.push_str(&format!("{},\n", tup_str));
        } else {
            result.push_str(&format!("{}{}\n", tup_str, semicolon));
        }
    }

    // Trim trailing newline if it ends with the semicolon line already
    result.trim_end_matches('\n').to_string()
}

fn parse_value_tuples(tokens: &[Token]) -> Vec<Vec<Token>> {
    let mut tuples = Vec::new();
    let mut current = Vec::new();
    let mut depth = 0;
    let mut in_tuple = false;

    for tok in tokens {
        match tok {
            Token::OpenParen if !in_tuple => {
                in_tuple = true;
            }
            Token::OpenParen if in_tuple => {
                depth += 1;
                current.push(tok.clone());
            }
            Token::CloseParen if in_tuple && depth == 0 => {
                tuples.push(std::mem::take(&mut current));
                in_tuple = false;
            }
            Token::CloseParen if in_tuple => {
                depth -= 1;
                current.push(tok.clone());
            }
            Token::Comma if !in_tuple => {}
            Token::Semicolon => {}
            _ if in_tuple => {
                current.push(tok.clone());
            }
            _ => {}
        }
    }

    tuples
}

fn format_create_index(tokens: &[Token]) -> String {
    tokens_upper_string(tokens)
}

fn format_drop(tokens: &[Token]) -> String {
    tokens_upper_string(tokens)
}

fn find_as_position(tokens: &[Token]) -> Option<usize> {
    let mut depth = 0;
    for (i, tok) in tokens.iter().enumerate() {
        match tok {
            Token::OpenParen => depth += 1,
            Token::CloseParen => depth -= 1,
            Token::Word(w) if depth == 0 && w.to_uppercase() == "AS" => return Some(i),
            _ => {}
        }
    }
    None
}

/// Returns true if the tokens represent a simple column expression (just identifiers and dots),
/// meaning it should participate in AS-alignment width calculation.
fn is_simple_expression(tokens: &[Token]) -> bool {
    tokens
        .iter()
        .all(|t| matches!(t, Token::Word(_) | Token::Dot))
}

fn format_create_view(tokens: &[Token], select_pos: usize) -> String {
    let prelude = &tokens[..select_pos];
    let select_tokens = &tokens[select_pos..];

    let prelude_str = tokens_upper_string(prelude);

    // Parse SELECT columns until FROM
    let from_pos = select_tokens.iter().position(|t| {
        if let Token::Word(w) = t {
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
        let has_concat = col_tokens.iter().any(|t| matches!(t, Token::Concat));

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
    if matches!(tokens.last(), Some(Token::Semicolon)) && !result.ends_with(';') {
        result.push(';');
    }

    result
}

fn format_view_column(tokens: &[Token]) -> String {
    // Check if this column expression contains || operators
    let has_concat = tokens.iter().any(|t| matches!(t, Token::Concat));
    if !has_concat {
        return tokens_upper_string(tokens);
    }

    // Split at || to get value segments: [folder], ['__'], [filename], ...
    let mut segments = Vec::new();
    let mut current = Vec::new();
    for tok in tokens {
        if matches!(tok, Token::Concat) {
            if !current.is_empty() {
                segments.push(std::mem::take(&mut current));
            }
        } else {
            current.push(tok.clone());
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
        if let Token::Word(w) = &tokens[i] {
            let upper = w.to_uppercase();
            if upper == "FROM" {
                result.push_str("FROM");
                i += 1;
                // Collect table reference
                let start = i;
                while i < tokens.len() {
                    if let Token::Word(w) = &tokens[i] {
                        let wu = w.to_uppercase();
                        if wu == "JOIN" || wu == "LEFT" || wu == "RIGHT" || wu == "INNER"
                            || wu == "CROSS" || wu == "NATURAL" || wu == "WHERE"
                            || wu == "GROUP" || wu == "ORDER" || wu == "LIMIT" || wu == "HAVING"
                        {
                            break;
                        }
                    }
                    if matches!(&tokens[i], Token::Semicolon) {
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
                    if let Token::Word(w) = &tokens[i] {
                        if w.to_uppercase() == "ON" {
                            break;
                        }
                    }
                    if matches!(&tokens[i], Token::Semicolon) {
                        break;
                    }
                    i += 1;
                }
                let join_part = tokens_upper_string(&tokens[start..i]);
                result.push_str(&join_part);
                if i < tokens.len() {
                    if let Token::Word(w) = &tokens[i] {
                        if w.to_uppercase() == "ON" {
                            result.push('\n');
                            result.push_str("        ");
                            let on_start = i;
                            while i < tokens.len() {
                                if let Token::Word(w) = &tokens[i] {
                                    let wu = w.to_uppercase();
                                    if wu == "JOIN" || wu == "LEFT" || wu == "RIGHT"
                                        || wu == "INNER" || wu == "CROSS" || wu == "NATURAL"
                                        || wu == "WHERE" || wu == "GROUP" || wu == "ORDER"
                                    {
                                        break;
                                    }
                                }
                                if matches!(&tokens[i], Token::Semicolon) {
                                    break;
                                }
                                i += 1;
                            }
                            let on_part = tokens_upper_string(&tokens[on_start..i]);
                            result.push_str(&on_part);
                            continue;
                        }
                    }
                }
            } else if upper == "WHERE" || upper == "GROUP" || upper == "ORDER" || upper == "HAVING" {
                result.push('\n');
                let start = i;
                while i < tokens.len() && !matches!(&tokens[i], Token::Semicolon) {
                    i += 1;
                }
                result.push_str(&tokens_upper_string(&tokens[start..i]));
                continue;
            } else {
                result.push_str(&token_upper_string(&tokens[i]));
                i += 1;
            }
        } else if matches!(&tokens[i], Token::Semicolon) {
            break;
        } else {
            result.push_str(&token_upper_string(&tokens[i]));
            i += 1;
        }
    }

    result
}

fn parse_select_columns(tokens: &[Token]) -> Vec<Vec<Token>> {
    let mut columns = Vec::new();
    let mut current = Vec::new();
    let mut depth = 0;

    for tok in tokens {
        match tok {
            Token::OpenParen => {
                depth += 1;
                current.push(tok.clone());
            }
            Token::CloseParen => {
                depth -= 1;
                current.push(tok.clone());
            }
            Token::Comma if depth == 0 => {
                columns.push(std::mem::take(&mut current));
            }
            _ => {
                current.push(tok.clone());
            }
        }
    }
    if !current.is_empty() {
        columns.push(current);
    }

    columns
}

fn find_matching_paren(tokens: &[Token], open_idx: usize) -> Option<usize> {
    let mut depth = 0;
    for i in open_idx..tokens.len() {
        match &tokens[i] {
            Token::OpenParen => depth += 1,
            Token::CloseParen => {
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
    matches!(&tokens[idx], Token::OpenParen)
        && idx + 1 < tokens.len()
        && matches!(&tokens[idx + 1], Token::Word(w) if w.to_uppercase() == "SELECT")
        && find_matching_paren(tokens, idx).is_some()
}

fn is_update(tokens: &[Token]) -> bool {
    for tok in tokens {
        if let Token::Comment(_) = tok {
            continue;
        }
        if let Token::Word(w) = tok {
            return w.to_uppercase() == "UPDATE";
        }
        return false;
    }
    false
}

fn is_delete(tokens: &[Token]) -> bool {
    for tok in tokens {
        if let Token::Comment(_) = tok {
            continue;
        }
        if let Token::Word(w) = tok {
            return w.to_uppercase() == "DELETE";
        }
        return false;
    }
    false
}

fn format_update(tokens: &[Token]) -> String {
    let set_pos = tokens.iter().position(|t| {
        matches!(t, Token::Word(w) if w.to_uppercase() == "SET")
    });

    let Some(set_pos) = set_pos else {
        return tokens_upper_string(tokens);
    };

    // Find WHERE position (only after SET)
    let after_set = &tokens[set_pos + 1..];
    let where_in_set = after_set.iter().position(|t| {
        matches!(t, Token::Word(w) if w.to_uppercase() == "WHERE")
    });
    let where_pos = where_in_set.map(|p| set_pos + 1 + p);

    let mut result = String::new();

    // UPDATE table [clauses before SET]
    let before_str = tokens_upper_string(&tokens[..set_pos]);
    result.push_str(&before_str);
    result.push_str("\nSET");

    // SET assignments
    let set_end = where_pos.unwrap_or({
        if matches!(tokens.last(), Some(Token::Semicolon)) {
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
        let remaining_str = if matches!(remaining.last(), Some(Token::Semicolon)) {
            tokens_upper_string(&remaining[..remaining.len() - 1])
        } else {
            tokens_upper_string(remaining)
        };
        result.push_str(&remaining_str);
    }

    // Semicolon
    if matches!(tokens.last(), Some(Token::Semicolon)) {
        result.push(';');
    }

    result
}

fn format_delete(tokens: &[Token]) -> String {
    let where_pos = tokens.iter().position(|t| {
        matches!(t, Token::Word(w) if w.to_uppercase() == "WHERE")
    });

    let mut result = String::new();

    if let Some(wp) = where_pos {
        // DELETE FROM table [clauses]
        let before_str = tokens_upper_string(&tokens[..wp]);
        result.push_str(&before_str);

        // WHERE clause on new line
        result.push('\n');
        let remaining = &tokens[wp..];
        let remaining_str = if matches!(remaining.last(), Some(Token::Semicolon)) {
            tokens_upper_string(&remaining[..remaining.len() - 1])
        } else {
            tokens_upper_string(remaining)
        };
        result.push_str(&remaining_str);
    } else {
        result.push_str(&tokens_upper_string(tokens));
    }

    // Semicolon
    if matches!(tokens.last(), Some(Token::Semicolon)) && !result.ends_with(';') {
        result.push(';');
    }

    result
}

fn format_generic(tokens: &[Token]) -> String {
    let mut result = String::new();
    let mut prev_idx: Option<usize> = None;
    let mut i = 0;

    while i < tokens.len() {
        // Check for subquery: (SELECT ...)
        if is_subquery_start(tokens, i) {
            let close = find_matching_paren(tokens, i).unwrap();

            // Add spacing before subquery based on previous token
            let need_space_before = prev_idx.is_some_and(|p| {
                matches!(
                    &tokens[p],
                    Token::Word(_)
                        | Token::Comma
                        | Token::CloseParen
                        | Token::Equals
                        | Token::Star
                        | Token::GreaterThan
                        | Token::LessThan
                        | Token::GreaterOrEqual
                        | Token::LessOrEqual
                        | Token::NotEquals
                )
            });
            if need_space_before {
                result.push(' ');
            }

            // Format subquery contents recursively
            let inner = &tokens[i + 1..close];
            let inner_sql = tokens_upper_string(inner);
            let formatted = format_sql(&inner_sql);
            let trimmed = formatted.trim();

            // Wrap in indented parens
            result.push_str("(\n");
            for line in trimmed.lines() {
                result.push_str("    ");
                result.push_str(line);
                result.push('\n');
            }
            result.push(')');

            i = close + 1;
            prev_idx = Some(close);
            continue;
        }

        // Handle comments (same logic as tokens_upper_string)
        if let Token::Comment(c) = &tokens[i] {
            if prev_idx.is_some_and(|p| {
                matches!(&tokens[p], Token::Word(_) | Token::Star | Token::CloseParen | Token::Comma)
            }) {
                result.push(' ');
            }
            result.push_str(c);
            if c.starts_with("--") || c.starts_with('#') {
                result.push('\n');
                prev_idx = None;
            } else {
                prev_idx = Some(i);
            }
            i += 1;
            continue;
        }

        // Spacing logic (same as tokens_upper_string)
        let need_space = match (prev_idx.map(|idx| &tokens[idx]), &tokens[i]) {
            (Some(Token::Word(_)), Token::Word(_)) => true,
            (Some(Token::Word(_)), Token::Equals) => true,
            (Some(Token::Equals), Token::Word(_)) => true,
            (Some(Token::CloseParen), Token::Word(_)) => true,
            (Some(Token::Comma), Token::Word(_)) => true,
            (Some(Token::Word(_)), Token::Star) => true,
            (Some(Token::Star), Token::Word(_)) => true,
            (Some(Token::Star), Token::Comment(_)) => true,
            (Some(Token::Comment(c)), Token::Word(_)) if !c.starts_with("--") && !c.starts_with('#') => true,
            // Operators need spaces around them
            (Some(Token::Word(_)), Token::GreaterThan) => true,
            (Some(Token::GreaterThan), Token::Word(_)) => true,
            (Some(Token::Word(_)), Token::LessThan) => true,
            (Some(Token::LessThan), Token::Word(_)) => true,
            (Some(Token::Word(_)), Token::GreaterOrEqual) => true,
            (Some(Token::GreaterOrEqual), Token::Word(_)) => true,
            (Some(Token::Word(_)), Token::LessOrEqual) => true,
            (Some(Token::LessOrEqual), Token::Word(_)) => true,
            (Some(Token::Word(_)), Token::NotEquals) => true,
            (Some(Token::NotEquals), Token::Word(_)) => true,
            (Some(Token::Word(_)), Token::Concat) => true,
            (Some(Token::Concat), Token::Word(_)) => true,
            _ => false,
        };

        if need_space {
            result.push(' ');
        }

        result.push_str(&token_upper_string(&tokens[i]));
        prev_idx = Some(i);
        i += 1;
    }

    result
}

fn format_statement(tokens: &[Token]) -> String {
    if tokens.is_empty() {
        return String::new();
    }

    if is_create_table(tokens) {
        format_create_table(tokens)
    } else if let Some(select_pos) = is_create_view(tokens) {
        format_create_view(tokens, select_pos)
    } else if is_insert(tokens) {
        format_insert(tokens)
    } else if is_create_index(tokens) {
        format_create_index(tokens)
    } else if is_drop(tokens) {
        format_drop(tokens)
    } else if is_update(tokens) {
        format_update(tokens)
    } else if is_delete(tokens) {
        format_delete(tokens)
    } else {
        format_generic(tokens)
    }
}

fn format_sql(input: &str) -> String {
    // Normalize line endings: strip \r so Windows CRLF and Unix LF are handled identically
    let input = input.replace("\r\n", "\n").replace('\r', "");
    let tokens = tokenize(&input);
    let statements = split_statements(&tokens);

    let mut result = String::new();
    let mut prev_type: Option<String> = None;

    for stmt in &statements {
        if stmt.is_empty() || (stmt.len() == 1 && matches!(stmt[0], Token::Semicolon)) {
            continue;
        }

        let formatted = format_statement(stmt);
        if formatted.is_empty() {
            continue;
        }

        // Detect if we should add a blank line separator
        let current_type = {
            let mut stype = String::new();
            for tok in stmt {
                if let Token::Word(w) = tok {
                    stype.push_str(&w.to_uppercase());
                    stype.push(' ');
                    if stype.split_whitespace().count() >= 2 {
                        break;
                    }
                }
            }
            stype.trim().to_string()
        };

        if let Some(ref prev) = prev_type {
            if *prev != current_type {
                result.push('\n');
            }
        }

        result.push_str(&formatted);
        result.push('\n');
        prev_type = Some(current_type);
    }

    result
}

fn main() {
    let cli = Cli::parse();

    if cli.version {
        println!("{}", env!("CARGO_PKG_VERSION"));
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

    let output = format_sql(&input);

    if let Some(path) = &cli.file {
        fs::write(path, &output).unwrap_or_else(|e| {
            eprintln!("reesql: error writing '{}': {}", path, e);
            std::process::exit(1);
        });
    } else {
        print!("{}", output);
    }
}
