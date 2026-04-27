# CodingAdventures::StarlarkParser

Parses Starlark source code into an Abstract Syntax Tree (AST) using a
hand-written recursive-descent parser.

## What is Starlark?

Starlark is a deterministic subset of Python used for configuration files,
most famously in [Bazel](https://bazel.build/) BUILD files. It looks like
Python but with key constraints that guarantee termination and determinism:

- No `while` loops (all iteration is over finite collections via `for`)
- No classes or class definitions
- No `try`/`except`/`raise`
- No `global`/`nonlocal`
- Recursion is disabled (checked at runtime)

These constraints make Starlark safe for build systems: every file terminates,
and repeated evaluation always produces the same result.

## Architecture

The parser is hand-written recursive descent (not grammar-driven). Each grammar
rule from `starlark.grammar` is implemented as a `_parse_RULENAME` method:

```
starlark source
    ↓
CodingAdventures::StarlarkLexer->tokenize()
    ↓   flat token stream with INDENT/DEDENT/NEWLINE
CodingAdventures::StarlarkParser->parse()
    ↓   recursive-descent rule methods
AST root (rule_name "program")
```

## Usage

```perl
use CodingAdventures::StarlarkParser;

# Object-oriented
my $parser = CodingAdventures::StarlarkParser->new("x = 1\n");
my $ast    = $parser->parse();
print $ast->rule_name;   # "program"

# Convenience class method
my $ast = CodingAdventures::StarlarkParser->parse_starlark("x = 1\n");

# Parse a BUILD file
my $build_ast = CodingAdventures::StarlarkParser->parse_starlark(<<'END');
cc_library(
    name = "foo",
    srcs = ["foo.cc"],
)
END
```

## API

### `new($source)`

Tokenize `$source` with `StarlarkLexer` and return a parser instance.

### `parse()`

Parse and return the root AST node (rule_name `"program"`). Dies on error.

### `parse_starlark($source)`

Class method — tokenize and parse in one call. Returns the root ASTNode.

## AST Node Format

```perl
$node->rule_name   # e.g. "assign_stmt", "if_stmt", "def_stmt"
$node->children    # arrayref of child nodes
$node->is_leaf     # 1 for leaf (token) nodes, 0 for inner nodes
$node->token       # token hashref (leaf nodes only): {type, value, line, col}
```

## Supported Constructs

| Construct | Example |
|-----------|---------|
| Assignment | `x = 1` |
| Augmented assignment | `x += 1` |
| Tuple unpacking | `a, b = 1, 2` |
| Function call | `print("hello")` |
| Load statement | `load("//rules.star", "sym")` |
| Function definition | `def foo(x): return x + 1` |
| If/elif/else | `if x > 0: ...` |
| For loop | `for item in items: ...` |
| Return/break/continue/pass | `return x` |
| List literal | `[1, 2, 3]` |
| Dict literal | `{"a": 1}` |
| List comprehension | `[x*2 for x in lst]` |
| Dict comprehension | `{k: v for k, v in d.items()}` |
| Lambda | `lambda x: x + 1` |
| Ternary | `a if cond else b` |
| BUILD rules | `cc_library(name="foo", srcs=["foo.cc"])` |

## Dependencies

- `CodingAdventures::StarlarkLexer` — tokenizes Starlark source
- `CodingAdventures::GrammarTools` — transitive dep via StarlarkLexer
- `CodingAdventures::Lexer` — transitive dep via StarlarkLexer

## Stack position

```
StarlarkParser         ← this package
└── StarlarkLexer
    ├── GrammarTools
    │   ├── Lexer
    │   │   └── StateMachine
    │   └── DirectedGraph
    └── Lexer
```

## License

MIT
