# TOML Parser

Parses TOML v1.0.0 text into Python dictionaries using the grammar-driven
infrastructure from coding-adventures.

## How It Works

The parser operates in two phases:

1. **Syntax phase** — tokenizes the input (via `toml-lexer`) and builds an AST
   using the grammar rules in `toml.grammar` (via the generic `GrammarParser`).

2. **Semantic phase** — walks the AST, validates context-sensitive constraints
   (key uniqueness, table consistency, inline table immutability), and converts
   all values to native Python types.

## Usage

```python
from toml_parser import parse_toml

doc = parse_toml("""
[server]
host = "localhost"
port = 8080
enabled = true

[database]
name = "mydb"
connection_max = 5000
ports = [8001, 8002]
""")

print(doc["server"]["host"])      # "localhost"
print(doc["database"]["ports"])   # [8001, 8002]
```

## Value Types

| TOML Type | Python Type |
|-----------|-------------|
| String | `str` |
| Integer | `int` |
| Float | `float` |
| Boolean | `bool` |
| Offset Date-Time | `datetime.datetime` (with tzinfo) |
| Local Date-Time | `datetime.datetime` |
| Local Date | `datetime.date` |
| Local Time | `datetime.time` |
| Array | `list` |
| Table | `TOMLDocument` (dict subclass) |

## Dependencies

- `toml-lexer` — tokenizes TOML text
- `parser` — generic grammar-driven parser engine
- `grammar-tools` — parses `.tokens` and `.grammar` files
- `lexer` — token types and grammar-driven lexer engine
- `state-machine` — DFA engine used by the lexer
- `directed-graph` — graph library used by the state machine
