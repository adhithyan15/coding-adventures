-- AUTO-GENERATED FILE — DO NOT EDIT
-- Source: sql.tokens
-- Regenerate with: grammar-tools compile-tokens sql.tokens
--
-- This file embeds a TokenGrammar as native Lua data structures.
-- Call token_grammar() instead of reading and parsing the .tokens file.

local gt = require("coding_adventures.grammar_tools")

local function token_grammar()
  local g = gt.TokenGrammar.new()
  g.definitions = {
        {
          name="BLOB_HEX",
          pattern="[xX]'[0-9A-Fa-f]*'",
          is_regex=true,
          line_number=23,
          alias="BLOB",
        },
        {
          name="HEX_INT",
          pattern="0[xX][0-9A-Fa-f]+",
          is_regex=true,
          line_number=29,
          alias="NUMBER",
        },
        {
          name="NAME",
          pattern="[a-zA-Z_][a-zA-Z0-9_]*",
          is_regex=true,
          line_number=30,
          alias="",
        },
        {
          name="NUMBER",
          pattern="[0-9]+\\.?[0-9]*",
          is_regex=true,
          line_number=31,
          alias="",
        },
        {
          name="STRING_SQ",
          pattern="'(''|[^'])*'",
          is_regex=true,
          line_number=32,
          alias="STRING",
        },
        {
          name="QUOTED_ID",
          pattern="`[^`]+`",
          is_regex=true,
          line_number=33,
          alias="NAME",
        },
        {
          name="QUOTED_ID_DQ",
          pattern="\"([^\"]|\"\")*\"",
          is_regex=true,
          line_number=38,
          alias="NAME",
        },
        {
          name="LESS_EQUALS",
          pattern="<=",
          is_regex=false,
          line_number=40,
          alias="",
        },
        {
          name="GREATER_EQUALS",
          pattern=">=",
          is_regex=false,
          line_number=41,
          alias="",
        },
        {
          name="NOT_EQUALS",
          pattern="!=",
          is_regex=false,
          line_number=42,
          alias="",
        },
        {
          name="NEQ_ANSI",
          pattern="<>",
          is_regex=false,
          line_number=43,
          alias="NOT_EQUALS",
        },
        {
          name="CONCAT_OP",
          pattern="||",
          is_regex=false,
          line_number=44,
          alias="",
        },
        {
          name="SHIFT_LEFT",
          pattern="<<",
          is_regex=false,
          line_number=48,
          alias="",
        },
        {
          name="SHIFT_RIGHT",
          pattern=">>",
          is_regex=false,
          line_number=49,
          alias="",
        },
        {
          name="JSON_ARROW_TEXT",
          pattern="->>",
          is_regex=false,
          line_number=56,
          alias="",
        },
        {
          name="JSON_ARROW",
          pattern="->",
          is_regex=false,
          line_number=57,
          alias="",
        },
        {
          name="EQUALS",
          pattern="=",
          is_regex=false,
          line_number=59,
          alias="",
        },
        {
          name="LESS_THAN",
          pattern="<",
          is_regex=false,
          line_number=60,
          alias="",
        },
        {
          name="GREATER_THAN",
          pattern=">",
          is_regex=false,
          line_number=61,
          alias="",
        },
        {
          name="PLUS",
          pattern="+",
          is_regex=false,
          line_number=62,
          alias="",
        },
        {
          name="MINUS",
          pattern="-",
          is_regex=false,
          line_number=63,
          alias="",
        },
        {
          name="STAR",
          pattern="*",
          is_regex=false,
          line_number=64,
          alias="",
        },
        {
          name="SLASH",
          pattern="/",
          is_regex=false,
          line_number=65,
          alias="",
        },
        {
          name="PERCENT",
          pattern="%",
          is_regex=false,
          line_number=66,
          alias="",
        },
        {
          name="BIT_AND_OP",
          pattern="&",
          is_regex=false,
          line_number=70,
          alias="",
        },
        {
          name="BIT_OR_OP",
          pattern="|",
          is_regex=false,
          line_number=71,
          alias="",
        },
        {
          name="BIT_NOT_OP",
          pattern="~",
          is_regex=false,
          line_number=72,
          alias="",
        },
        {
          name="LPAREN",
          pattern="(",
          is_regex=false,
          line_number=74,
          alias="",
        },
        {
          name="RPAREN",
          pattern=")",
          is_regex=false,
          line_number=75,
          alias="",
        },
        {
          name="COMMA",
          pattern=",",
          is_regex=false,
          line_number=76,
          alias="",
        },
        {
          name="SEMICOLON",
          pattern=";",
          is_regex=false,
          line_number=77,
          alias="",
        },
        {
          name="DOT",
          pattern=".",
          is_regex=false,
          line_number=78,
          alias="",
        },
      }
  g.keywords = {"WITH", "RECURSIVE", "SELECT", "FROM", "WHERE", "GROUP", "BY", "HAVING", "ORDER", "LIMIT", "OFFSET", "INSERT", "INTO", "VALUES", "UPDATE", "SET", "DELETE", "ALTER", "ADD", "COLUMN", "CREATE", "DROP", "RENAME", "TABLE", "IF", "EXISTS", "NOT", "AND", "OR", "NULL", "IS", "IN", "BETWEEN", "LIKE", "ESCAPE", "AS", "DISTINCT", "ALL", "UNION", "INTERSECT", "EXCEPT", "JOIN", "INNER", "LEFT", "RIGHT", "OUTER", "CROSS", "FULL", "ON", "ASC", "DESC", "TRUE", "FALSE", "CASE", "WHEN", "THEN", "ELSE", "END", "PRIMARY", "KEY", "AUTOINCREMENT", "UNIQUE", "INDEX", "INDEXED", "CHECK", "REFERENCES", "DEFAULT", "COLLATE", "ATTACH", "DETACH", "DATABASE", "STRICT", "WITHOUT", "VIEW", "BEGIN", "COMMIT", "ROLLBACK", "TRANSACTION", "SAVEPOINT", "RELEASE", "TO", "OVER", "PARTITION", "WINDOW", "TRIGGER", "BEFORE", "AFTER", "FOR", "EACH", "ROW", "RETURNING", "CAST", "GLOB", "NATURAL", "USING", "REPLACE", "IGNORE", "ABORT", "FAIL", "CONFLICT", "DO", "NOTHING", "ROWS", "RANGE", "GROUPS", "PRECEDING", "FOLLOWING", "UNBOUNDED", "CURRENT", "MATERIALIZED", "NULLS"}
  g.mode = ""
  g.escape_mode = "none"
  g.skip_definitions = {
        {
          name="WHITESPACE",
          pattern="[ \\t\\r\\n]+",
          is_regex=true,
          line_number=194,
          alias="",
        },
        {
          name="LINE_COMMENT",
          pattern="--[^\\n]*",
          is_regex=true,
          line_number=195,
          alias="",
        },
        {
          name="BLOCK_COMMENT",
          pattern="\\x2f\\*([^*]|\\*[^\\x2f])*\\*\\x2f",
          is_regex=true,
          line_number=196,
          alias="",
        },
      }
  g.reserved_keywords = {}
  g.context_keywords = {}
  g.layout_keywords = {}
  g.soft_keywords = {}
  g.error_definitions = {}
  g.groups = {}
  g.case_sensitive = true
  g.version = 0
  g.case_insensitive = true
  return g
end

return { token_grammar = token_grammar }
