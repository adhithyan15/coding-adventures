"""Record CPython's own token stream for each fixture (LANG84 phase 7a oracle).

Run with each host interpreter the Python release pairs cite:

    python3.11 -I tokenize_oracle.py tokenize   # release 3.11.17
    python3.12 -I tokenize_oracle.py tokenize   # release 3.12.3
    python3    -I tokenize_oracle.py tokenize   # release 3.13.16

It writes `<case>.pytokens` (one token per line: `KIND<TAB>line<TAB>column<TAB>value`)
and refuses to overwrite a fixture with a different stream, so running it under
all three interpreters proves they agree. Kinds use the names in
`python<release>.tokens`; CPython's `NL` and `COMMENT` tokens are trivia the
grammar-driven lexer skips, so they are dropped. Columns are 1-based. Synthetic
tokens (NEWLINE, INDENT, DEDENT, EOF) carry no position or value.
"""

import io
import keyword
import sys
import token
import tokenize
from pathlib import Path

RENAME = {
    "LPAR": "LPAREN", "RPAR": "RPAREN", "LSQB": "LBRACKET", "RSQB": "RBRACKET",
    "SEMI": "SEMICOLON", "VBAR": "PIPE", "AMPER": "AMP", "LESS": "LESS_THAN",
    "GREATER": "GREATER_THAN", "EQUAL": "EQUALS", "EQEQUAL": "EQUALS_EQUALS",
    "NOTEQUAL": "NOT_EQUALS", "LESSEQUAL": "LESS_EQUALS", "GREATEREQUAL": "GREATER_EQUALS",
    "CIRCUMFLEX": "CARET", "LEFTSHIFT": "LEFT_SHIFT", "RIGHTSHIFT": "RIGHT_SHIFT",
    "DOUBLESTAR": "DOUBLE_STAR", "PLUSEQUAL": "PLUS_EQUALS", "MINEQUAL": "MINUS_EQUALS",
    "STAREQUAL": "STAR_EQUALS", "SLASHEQUAL": "SLASH_EQUALS", "PERCENTEQUAL": "PERCENT_EQUALS",
    "AMPEREQUAL": "AMP_EQUALS", "VBAREQUAL": "PIPE_EQUALS", "CIRCUMFLEXEQUAL": "CARET_EQUALS",
    "LEFTSHIFTEQUAL": "LEFT_SHIFT_EQUALS", "RIGHTSHIFTEQUAL": "RIGHT_SHIFT_EQUALS",
    "DOUBLESTAREQUAL": "DOUBLE_STAR_EQUALS", "DOUBLESLASH": "FLOOR_DIV",
    "DOUBLESLASHEQUAL": "FLOOR_DIV_EQUALS", "ATEQUAL": "AT_EQUALS", "RARROW": "ARROW",
    "COLONEQUAL": "WALRUS", "ENDMARKER": "EOF",
}


def kind(tok):
    name = token.tok_name[tok.exact_type]
    if tok.type == token.NUMBER:
        return "INT"
    if tok.type == token.NAME:
        return "KEYWORD" if keyword.iskeyword(tok.string) else "NAME"
    return RENAME.get(name, name)


def stream(source):
    lines = []
    for tok in tokenize.generate_tokens(io.StringIO(source).readline):
        if tok.type in (tokenize.NL, tokenize.COMMENT):
            continue
        k = kind(tok)
        if k in ("NEWLINE", "INDENT", "DEDENT", "EOF"):
            lines.append(k)
        else:
            lines.append(f"{k}\t{tok.start[0]}\t{tok.start[1] + 1}\t{tok.string}")
    return "\n".join(lines) + "\n"


def main(directory):
    for case in sorted(Path(directory).glob("*.py")):
        if case.name == Path(__file__).name:
            continue
        text = stream(case.read_text(encoding="utf-8"))
        out = case.with_suffix(".pytokens")
        if out.exists() and out.read_text(encoding="utf-8") != text:
            sys.exit(f"{case.name}: {sys.version.split()[0]} disagrees with the recorded stream")
        out.write_text(text, encoding="utf-8")
        print(f"{sys.version.split()[0]}: {case.name}")


if __name__ == "__main__":
    main(sys.argv[1])
