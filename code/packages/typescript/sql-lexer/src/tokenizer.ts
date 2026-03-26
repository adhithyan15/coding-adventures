/**
 * SQL Lexer -- tokenizes SQL text using the grammar-driven approach.
 *
 * This module is a **thin wrapper** around the generic `grammarTokenize` function
 * from the `@coding-adventures/lexer` package. It loads the `sql.tokens` grammar
 * file and delegates all tokenization work to the generic engine.
 *
 * What Is SQL?
 * ------------
 *
 * SQL (Structured Query Language) is the standard language for interacting with
 * relational databases. Originally defined by IBM researchers in the 1970s and
 * later standardized as ANSI SQL, it lets you describe *what data you want*
 * rather than *how to retrieve it* — a declarative language.
 *
 * The most common SQL statement is SELECT:
 *
 *     SELECT name, age FROM users WHERE age > 18 ORDER BY name;
 *
 * This single line retrieves names and ages from the "users" table, filtering
 * for adults, and sorting alphabetically. The database figures out the most
 * efficient retrieval strategy automatically.
 *
 * SQL vs JSON
 * -----------
 *
 * SQL is considerably more complex to tokenize than JSON:
 *
 *   - **Keywords** -- SQL has dozens of reserved words (SELECT, FROM, WHERE,
 *     JOIN, etc.) that are matched from identifiers. Unlike JSON, which has no
 *     keywords, the SQL lexer must reclassify NAME tokens as KEYWORD tokens.
 *   - **Case-insensitive keywords** -- SQL is case-insensitive by convention:
 *     `SELECT`, `select`, and `Select` are all the same keyword. The grammar
 *     uses `@case_insensitive true` and normalizes keyword values to uppercase.
 *   - **Comments** -- SQL has two comment styles: line comments (`-- text`) and
 *     block comments (`/* text *\/`). Both are skipped silently.
 *   - **Operators** -- SQL has comparison operators including multi-character
 *     forms: `!=`, `<>`, `<=`, `>=`. The grammar handles longest-match first.
 *   - **Quoted identifiers** -- Backtick-quoted identifiers like `` `column_name` ``
 *     allow using reserved words as names. They tokenize as NAME.
 *   - **Single-quoted strings** -- SQL strings use single quotes: `'hello'`.
 *     The entire quoted string (without stripping quotes) becomes a STRING token.
 *
 * Token Types
 * -----------
 *
 * The `sql.tokens` file defines these token types:
 *
 *   | Token         | Example          | Description                          |
 *   |---------------|------------------|--------------------------------------|
 *   | NAME          | users, id        | Identifier (table/column name)       |
 *   | NUMBER        | 42, 3.14         | Integer or decimal number            |
 *   | STRING        | 'hello'          | Single-quoted string literal         |
 *   | KEYWORD       | SELECT, FROM     | Reserved SQL keyword (uppercase)     |
 *   | EQUALS        | =                | Equality comparison / assignment     |
 *   | NOT_EQUALS    | != or <>         | Inequality (both forms aliased)      |
 *   | LESS_THAN     | <                | Less-than comparison                 |
 *   | GREATER_THAN  | >                | Greater-than comparison              |
 *   | LESS_EQUALS   | <=               | Less-than-or-equal comparison        |
 *   | GREATER_EQUALS| >=               | Greater-than-or-equal comparison     |
 *   | PLUS          | +                | Addition                             |
 *   | MINUS         | -                | Subtraction or negation              |
 *   | STAR          | *                | Multiplication or SELECT *           |
 *   | SLASH         | /                | Division                             |
 *   | PERCENT       | %                | Modulo                               |
 *   | LPAREN        | (                | Left parenthesis                     |
 *   | RPAREN        | )                | Right parenthesis                    |
 *   | COMMA         | ,                | Separator                            |
 *   | SEMICOLON     | ;                | Statement terminator                 |
 *   | DOT           | .                | Qualifier (schema.table.column)      |
 *   | EOF           | (synthetic)      | End of input                         |
 *
 * Case-Insensitive Keyword Normalization
 * ---------------------------------------
 *
 * Because `sql.tokens` declares `# @case_insensitive true`, any NAME token
 * whose uppercase value appears in the keyword list is emitted as:
 *
 *   - type: "KEYWORD"
 *   - value: uppercase form (e.g., "select" → "SELECT")
 *
 * This means the parser grammar can always compare against uppercase strings
 * without worrying about how the user typed the keyword.
 *
 * Comment Skipping
 * ----------------
 *
 * SQL supports two comment styles, both defined in the `skip:` section of
 * sql.tokens and silently consumed by the lexer:
 *
 *   - Line comments: `-- this is a comment` (from `--` to end of line)
 *   - Block comments: `/* this is a comment *\/` (can span multiple lines)
 *
 * Locating the Grammar File
 * -------------------------
 *
 * The `sql.tokens` file lives in `code/grammars/` at the repository root.
 * We navigate from this module's location up to that directory:
 *
 *     src/tokenizer.ts -> sql-lexer/ -> typescript/ -> packages/ -> code/ -> grammars/
 */

import { fileURLToPath } from "url";
import { dirname, join } from "path";
import { readFileSync } from "fs";

import { parseTokenGrammar } from "@coding-adventures/grammar-tools";
import { grammarTokenize } from "@coding-adventures/lexer";
import type { Token } from "@coding-adventures/lexer";

/**
 * Resolve __dirname for ESM modules.
 *
 * In CommonJS, __dirname is a global. In ESM, it does not exist -- we must
 * derive it from import.meta.url, which gives the file URL of the current
 * module (e.g., "file:///path/to/tokenizer.ts"). The fileURLToPath + dirname
 * pattern converts this to a directory path.
 */
const __dirname = dirname(fileURLToPath(import.meta.url));

/**
 * Navigate from src/ up to the grammars/ directory.
 *
 * The path traversal:
 *   __dirname = .../sql-lexer/src/
 *   ..         = .../sql-lexer/
 *   ../..      = .../typescript/
 *   ../../..   = .../packages/
 *   ../../../.. = .../code/
 *   + grammars  = .../code/grammars/
 */
const GRAMMARS_DIR = join(__dirname, "..", "..", "..", "..", "grammars");
const SQL_TOKENS_PATH = join(GRAMMARS_DIR, "sql.tokens");

/**
 * Create a configured SQL lexer for the given source text.
 *
 * This function returns the same result as `tokenizeSQL` but makes it explicit
 * that we are constructing a lexer for SQL. It is useful when callers want to
 * verify that the lexer is non-null / successfully constructed.
 *
 * @param source - The SQL text to tokenize.
 * @returns An array of Token objects. The last token is always EOF.
 *
 * @example
 *     const tokens = createSQLLexer("SELECT 1");
 *     // Returns same result as tokenizeSQL("SELECT 1")
 */
export function createSQLLexer(source: string): Token[] {
  return tokenizeSQL(source);
}

/**
 * Tokenize SQL text and return an array of tokens.
 *
 * The function reads the `sql.tokens` grammar file, parses it into a
 * `TokenGrammar` object, then passes the source text and grammar to the
 * generic `grammarTokenize` engine.
 *
 * The `sql.tokens` grammar sets `@case_insensitive true`, which means:
 *   - All SQL keywords are matched regardless of case
 *   - Keyword token values are normalized to uppercase
 *   - `select`, `SELECT`, and `Select` all produce KEYWORD("SELECT")
 *
 * The grammar engine also handles:
 *   - Skip patterns (whitespace, line comments, block comments)
 *   - Position tracking (line and column for each token)
 *   - Alias resolution (e.g., `<>` is aliased to NOT_EQUALS)
 *
 * @param source - The SQL text to tokenize.
 * @returns An array of Token objects. The last token is always EOF.
 *
 * @example
 *     const tokens = tokenizeSQL("SELECT id, name FROM users");
 *     // KEYWORD("SELECT"), NAME("id"), COMMA(","), NAME("name"),
 *     // KEYWORD("FROM"), NAME("users"), EOF
 *
 * @example
 *     const tokens = tokenizeSQL("SELECT * FROM orders WHERE price > 100");
 *     // ...KEYWORD("WHERE"), NAME("price"), GREATER_THAN(">"), NUMBER("100")...
 *
 * @example
 *     // Case-insensitive: all three produce the same output
 *     tokenizeSQL("select * from users")
 *     tokenizeSQL("SELECT * FROM users")
 *     tokenizeSQL("Select * From Users")  // Names stay as typed, keywords normalize
 */
export function tokenizeSQL(source: string): Token[] {
  /**
   * Read the grammar file from disk. In a production system, you would
   * cache this -- but for an educational codebase, reading on every call
   * keeps the code simple and makes the data flow obvious.
   */
  const grammarText = readFileSync(SQL_TOKENS_PATH, "utf-8");

  /**
   * Parse the grammar text into a structured TokenGrammar object.
   * This extracts:
   *   - Token patterns (regex and literal), including aliases
   *   - Skip patterns (whitespace, line comments, block comments)
   *   - Keyword list (normalized to uppercase for case-insensitive matching)
   *   - caseInsensitive flag (true for SQL)
   *
   * Because sql.tokens declares `@case_insensitive true`, the grammar object
   * will have `grammar.caseInsensitive = true`. The GrammarLexer (which
   * `grammarTokenize` uses internally) reads this flag automatically and
   * normalizes keyword values to uppercase on match.
   */
  const grammar = parseTokenGrammar(grammarText);

  /**
   * Run the generic grammar-driven tokenizer. This is the same engine
   * used for JSON, Python, Ruby, and other languages -- the only thing
   * that changes between languages is the grammar file.
   *
   * For SQL, the key behaviors are:
   *   1. NAME tokens whose uppercase value is in the keyword list → KEYWORD
   *   2. KEYWORD values are normalized to uppercase (select → SELECT)
   *   3. `!=` and `<>` both produce NOT_EQUALS tokens (via alias)
   *   4. Backtick-quoted identifiers produce NAME tokens (via alias)
   *   5. Single-quoted strings produce STRING tokens (via alias)
   *   6. Comments and whitespace are silently skipped
   */
  return grammarTokenize(source, grammar);
}
