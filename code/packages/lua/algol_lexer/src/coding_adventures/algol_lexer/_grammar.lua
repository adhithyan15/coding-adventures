-- AUTO-GENERATED FILE — DO NOT EDIT
-- Source: algol60.tokens
-- Regenerate with: grammar-tools compile-tokens algol60.tokens
--
-- This file embeds a TokenGrammar as native Lua data structures.
-- Call token_grammar() instead of reading and parsing the .tokens file.

local gt = require("coding_adventures.grammar_tools")

local function token_grammar()
  local g = gt.TokenGrammar.new()
  g.definitions = {
        {
          name="REAL_LIT",
          pattern="[0-9]+\\.[0-9]*([eE][+-]?[0-9]+)?|[0-9]+[eE][+-]?[0-9]+",
          is_regex=true,
          line_number=38,
          alias="",
        },
        {
          name="INTEGER_LIT",
          pattern="[0-9]+",
          is_regex=true,
          line_number=41,
          alias="",
        },
        {
          name="STRING_LIT",
          pattern="'[^']*'|\"[^\"]*\"",
          is_regex=true,
          line_number=46,
          alias="",
        },
        {
          name="NAME",
          pattern="[a-zA-Z][a-zA-Z0-9]*",
          is_regex=true,
          line_number=53,
          alias="",
        },
        {
          name="ASSIGN",
          pattern=":=",
          is_regex=false,
          line_number=61,
          alias="",
        },
        {
          name="POWER",
          pattern="**",
          is_regex=false,
          line_number=66,
          alias="",
        },
        {
          name="LEQ",
          pattern="<=|≤",
          is_regex=true,
          line_number=70,
          alias="",
        },
        {
          name="GEQ",
          pattern=">=|≥",
          is_regex=true,
          line_number=71,
          alias="",
        },
        {
          name="NEQ",
          pattern="!=|<>|≠",
          is_regex=true,
          line_number=72,
          alias="",
        },
        {
          name="NOT_SYM",
          pattern="¬",
          is_regex=false,
          line_number=76,
          alias="",
        },
        {
          name="AND_SYM",
          pattern="∧",
          is_regex=false,
          line_number=77,
          alias="",
        },
        {
          name="OR_SYM",
          pattern="∨",
          is_regex=false,
          line_number=78,
          alias="",
        },
        {
          name="IMPL_SYM",
          pattern="⊃",
          is_regex=false,
          line_number=79,
          alias="",
        },
        {
          name="EQV_SYM",
          pattern="≡",
          is_regex=false,
          line_number=80,
          alias="",
        },
        {
          name="PLUS",
          pattern="+",
          is_regex=false,
          line_number=86,
          alias="",
        },
        {
          name="MINUS",
          pattern="-",
          is_regex=false,
          line_number=87,
          alias="",
        },
        {
          name="STAR",
          pattern="\\*|×",
          is_regex=true,
          line_number=90,
          alias="",
        },
        {
          name="SLASH",
          pattern="\\/|÷",
          is_regex=true,
          line_number=91,
          alias="",
        },
        {
          name="CARET",
          pattern="\\^|↑",
          is_regex=true,
          line_number=95,
          alias="",
        },
        {
          name="EQ",
          pattern="=",
          is_regex=false,
          line_number=98,
          alias="",
        },
        {
          name="LT",
          pattern="<",
          is_regex=false,
          line_number=100,
          alias="",
        },
        {
          name="GT",
          pattern=">",
          is_regex=false,
          line_number=101,
          alias="",
        },
        {
          name="LPAREN",
          pattern="(",
          is_regex=false,
          line_number=107,
          alias="",
        },
        {
          name="RPAREN",
          pattern=")",
          is_regex=false,
          line_number=108,
          alias="",
        },
        {
          name="LBRACKET",
          pattern="[",
          is_regex=false,
          line_number=109,
          alias="",
        },
        {
          name="RBRACKET",
          pattern="]",
          is_regex=false,
          line_number=110,
          alias="",
        },
        {
          name="SEMICOLON",
          pattern=";",
          is_regex=false,
          line_number=111,
          alias="",
        },
        {
          name="COMMA",
          pattern=",",
          is_regex=false,
          line_number=112,
          alias="",
        },
        {
          name="COLON",
          pattern=":",
          is_regex=false,
          line_number=116,
          alias="",
        },
      }
  g.keywords = {"begin", "end", "if", "then", "else", "for", "do", "step", "until", "while", "goto", "switch", "procedure", "own", "array", "label", "value", "integer", "real", "boolean", "string", "true", "false", "not", "and", "or", "impl", "eqv", "div", "mod", "comment"}
  g.mode = ""
  g.escape_mode = ""
  g.skip_definitions = {
        {
          name="WHITESPACE",
          pattern="[ \\t\\r\\n]+",
          is_regex=true,
          line_number=183,
          alias="",
        },
        {
          name="COMMENT",
          pattern="comment[^a-zA-Z0-9_][^;]*;",
          is_regex=true,
          line_number=192,
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
  g.case_insensitive = false
  return g
end

return { token_grammar = token_grammar }
