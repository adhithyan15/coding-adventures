# PY01 — Versioned Python Grammars

## Overview

This spec describes a set of versioned grammar and token files that capture
how the Python language evolved across its major milestones. Each version pair
(`pythonX.Y.grammar` and `pythonX.Y.tokens`) is a self-contained description
of Python's syntax at that point in time, written in the same EBNF-like
notation used throughout the coding-adventures grammar infrastructure.

The existing `code/grammars/python.grammar` and `python.tokens` files describe
a minimal Python subset (assignments, arithmetic, a handful of keywords). The
versioned files replace that toy subset with real, comprehensive grammar
definitions that track the actual language as it shipped.

## Why Versioned Grammars?

Programming languages are not static. They evolve — sometimes gradually,
sometimes through breaking changes. Python is a particularly instructive case
because:

1. **Python 2 to 3 was the most visible breaking change in mainstream
   language history.** Print went from a statement to a function. Division
   changed semantics. Unicode became the default string type. Studying the
   grammar diff makes these changes concrete, not just folklore.

2. **Each subsequent Python 3.x release added exactly one or two headline
   syntax features.** F-strings in 3.6. The walrus operator in 3.8. Pattern
   matching in 3.10. Type parameter syntax in 3.12. These are small,
   well-bounded grammar changes — perfect for studying how languages grow.

3. **Starlark (already in our grammar collection) is a deliberate Python
   subset.** Having full Python grammars lets us draw an exact boundary
   around what Starlark keeps and what it removes. That boundary is one of
   the best illustrations of language design tradeoffs in existence.

4. **Our lexer and parser infrastructure needs real stress tests.** The
   existing grammars are either trivial (the MVP `python.tokens`) or narrow
   (JSON, TOML, SQL). Full Python — with indentation sensitivity, f-string
   nesting, pattern matching, and soft keywords — is the hardest target
   we can aim at.

## Layer Position

```
Grammar Files (.tokens, .grammar)          <── YOU ARE HERE
        |
Grammar Tools (parse, validate, cross-check)
        |
Lexer (grammar-driven: .tokens -> token stream)
        |
Parser (grammar-driven: .grammar -> AST)
        |
Bytecode Compiler / VM / Starlark evaluator
```

**Input from:** CPython's Grammar/Grammar file and PEP documents (the
authoritative source for each version's syntax).

**Output to:** The grammar-driven lexer and parser, which consume these files
to tokenize and parse real Python source code.


## Directory Structure

All versioned Python grammars live under `code/grammars/python/`, one pair
per version:

```
code/grammars/python/
    python2.7.grammar       python2.7.tokens
    python3.0.grammar       python3.0.tokens
    python3.6.grammar       python3.6.tokens
    python3.8.grammar       python3.8.tokens
    python3.10.grammar      python3.10.tokens
    python3.12.grammar      python3.12.tokens
```

The existing top-level files remain as-is:

```
code/grammars/
    python.grammar          (minimal MVP subset — unchanged)
    python.tokens           (minimal MVP subset — unchanged)
    starlark.grammar        (Starlark — updated to cross-reference)
    starlark.tokens         (Starlark — updated to cross-reference)
```

The minimal MVP files serve as the "hello world" introduction. The versioned
files are the real thing.


## Version Selection Matrix

We do not cover every CPython release. We cover seven snapshots that
represent the sharpest inflection points in the language's grammar. Each
version is chosen because it introduced a syntactic feature that changed
how people write Python.

| Version   | Year | Why This Version                                                  |
|-----------|------|-------------------------------------------------------------------|
| **2.7**   | 2010 | Last Python 2 release. Historical reference point.                |
| **3.0**   | 2008 | The breaking change. `print` becomes a function.                  |
| **3.6**   | 2016 | F-strings (`f"..."`), variable annotations, async generators.     |
| **3.8**   | 2019 | Walrus operator (`:=`), positional-only parameters (`/`).         |
| **3.10**  | 2021 | Structural pattern matching (`match`/`case`).                     |
| **3.12**  | 2023 | Type parameter syntax (`type X = ...`), improved f-strings.       |
| Starlark  | 2018 | Python subset. No `class`, no `import`, no `while`, no mutation.  |

### Why not 3.1 through 3.5?

These releases added important *library* features (OrderedDict, asyncio,
type hints in comments) but made minimal grammar changes. The syntax of a
3.1 program is nearly identical to 3.0. We skip them to keep the collection
focused on grammar evolution.

### Why not 3.7, 3.9, 3.11?

- **3.7** made `async` and `await` reserved keywords (they were soft keywords
  in 3.5/3.6). This is a token-level change, not a grammar-level one. We
  capture it in the 3.8 tokens file since 3.8 is the next major grammar
  milestone.
- **3.9** added the `|` union syntax for type hints in annotations, but this
  is an expression-level change (just allowing `|` in more contexts), not a
  new statement or production. The grammar change is trivial.
- **3.11** added exception groups (`except*`) — a real grammar change, but
  a narrow one. We may add a `python3.11.grammar` in the future.

### Why include 2.7 when Python 2 is dead?

Because understanding *what changed* requires seeing *what came before*.
You cannot appreciate why `print("hello")` is a function call without seeing
that `print "hello"` was a statement. You cannot understand the `from
__future__ import division` dance without seeing that `/` was integer
division by default. The 2.7 grammar is the "before" photo.


## Token Differences Between Versions

Each Python version adds, removes, or reclassifies tokens. The table below
tracks every change across our seven snapshots.

### Legend

- **+** = added in this version
- **-** = removed in this version
- **~** = reclassified (e.g., from soft keyword to reserved keyword)

### Token Change Log

```
Python 2.7 (baseline)
    Tokens: NAME, NUMBER, STRING, all standard operators
    Keywords: and, as, assert, break, class, continue, def, del, elif, else,
              except, exec, finally, for, from, global, if, import, in, is,
              lambda, not, or, pass, print, raise, return, try, while, with,
              yield
    Operators: <> (not-equal, legacy), ` (backtick repr)

Python 2.7 -> 3.0:
    + ELLIPSIS       "..."    (literal token, not just in slice context)
    + ARROW          "->"     (function return annotation)
    + AT             "@"      (existed for decorators, now also matmul later)
    - BACKTICK       "`"      (repr shorthand removed)
    - DIAMOND        "<>"     (legacy not-equal removed, use != only)
    ~ print          keyword -> removed (now a builtin function, lexed as NAME)
    ~ exec           keyword -> removed (now a builtin function, lexed as NAME)
    ~ True           NAME -> keyword (was assignable in 2.x!)
    ~ False          NAME -> keyword (was assignable in 2.x!)
    ~ None           NAME -> keyword (was assignable in 2.x!)
    + nonlocal       keyword  (PEP 3104)
    - has_key        (not a token, but worth noting: dict method removed)

Python 3.0 -> 3.6:
    + FSTRING_START  f" or f' (opening of f-string, PEP 498)
    + FSTRING_MIDDLE           (literal text segments between { } in f-strings)
    + FSTRING_END              (closing quote of f-string)
    + LBRACE_EXPR   "{"       (expression opening inside f-string)
    + RBRACE_EXPR   "}"       (expression closing inside f-string)
    ~ async          soft keyword -> reserved keyword (PEP 492, finalized 3.7)
    ~ await          soft keyword -> reserved keyword (PEP 492, finalized 3.7)
    Note: variable annotations (PEP 526) use existing COLON token in new
    grammar context — no new tokens needed.

Python 3.6 -> 3.8:
    + COLONEQUALS    ":="     (walrus operator, PEP 572)
    + SLASH_SEP      "/"      (positional-only parameter separator, PEP 570)
    Note: The "/" token already exists as SLASH (division). In the *grammar*,
    "/" gains a second role as a parameter separator. Some implementations
    use context to distinguish; others define SLASH_SEP as a grammar-level
    concept, not a new token. We follow the latter approach: the *token*
    is still SLASH, but the grammar rule funcdef uses it in parameter
    position.

Python 3.8 -> 3.10:
    ~ match          NAME -> soft keyword (PEP 634)
    ~ case           NAME -> soft keyword (PEP 634)
    ~ _              NAME -> soft keyword (wildcard pattern, PEP 635)
    Note: "soft keyword" means these are keywords only in match/case context.
    In all other contexts, `match` and `case` remain valid identifiers.
    This is the first time Python introduced context-dependent keywords.
    The lexer does NOT reclassify them — the parser handles the ambiguity.

Python 3.10 -> 3.12:
    ~ type           NAME -> soft keyword (PEP 695)
    Note: Like match/case, `type` is only a keyword in `type X = ...`
    statements. You can still have a variable named `type` elsewhere.
    F-strings are also reworked internally (PEP 701) — they can now contain
    backslashes, nested quotes, and multi-line expressions. This changes
    the f-string tokenization significantly but does not add new token
    *types*.

Starlark (relative to Python 3.6):
    - class          keyword removed
    - import         keyword removed
    - from           keyword removed
    - while          keyword removed (use for + range instead)
    - try            keyword removed
    - except         keyword removed
    - finally        keyword removed
    - raise          keyword removed
    - with           keyword removed
    - yield          keyword removed
    - async          keyword removed
    - await          keyword removed
    - global         keyword removed
    - nonlocal       keyword removed
    - del            keyword removed
    - is             keyword removed
    - assert         keyword removed
    + load           keyword added (replaces import)
    Note: Starlark also makes True, False, None immutable constants (as in
    Python 3), and adds the reserved: section to its .tokens file to give
    clear errors when Python-only keywords are used by mistake.
```


## Grammar Differences Between Versions

Token changes tell you *what new symbols the language recognizes*. Grammar
changes tell you *what new structures you can build from those symbols*.
Here we walk through the production rules that change at each version.

### Python 2.7 — The Baseline

Key grammar rules that exist in 2.7 but change or disappear later:

```ebnf
# print is a STATEMENT, not a function call
print_stmt     = "print" [ expression { "," expression } [ "," ] ] ;

# exec is a STATEMENT
exec_stmt      = "exec" expression [ "in" expression [ "," expression ] ] ;

# Integer division: / means floor division for int operands
# (This is a semantic difference, not a grammar one, but it matters)

# Old-style string formatting is the only option
# "hello %s" % name  (no f-strings, no .format template grammar)

# No function annotations
funcdef        = "def" NAME "(" [ paramlist ] ")" ":" suite ;
```

### Python 3.0 — The Breaking Change

```ebnf
# print_stmt is GONE. print is now a regular function call.
# The grammar simply does not have a print_stmt production.

# exec_stmt is GONE. exec() is a builtin function.

# Function annotations arrive (PEP 3107):
funcdef        = "def" NAME "(" [ paramlist ] ")" [ "->" expression ] ":" suite ;

# Star expressions in assignments (PEP 3132):
# a, *b, c = [1, 2, 3, 4, 5]  -->  b = [2, 3, 4]
star_expr      = "*" expression ;
assignment     = target_list "=" ( star_expr | expression_list ) ;

# Keyword-only arguments after * (PEP 3102):
# def f(a, *, key): ...
paramlist      = ... "*" [ NAME ] { "," NAME [ "=" expression ] } ... ;
```

### Python 3.6 — F-Strings and Variable Annotations

```ebnf
# F-string expression (PEP 498):
# f"Hello, {name}!" is parsed as a sequence of literal + expression parts
f_string       = FSTRING_START { fstring_part } FSTRING_END ;
fstring_part   = FSTRING_MIDDLE | "{" expression [ "!" conversion ] [ ":" format_spec ] "}" ;
conversion     = "s" | "r" | "a" ;
format_spec    = { fstring_part | FORMAT_TEXT } ;

# Variable annotations (PEP 526):
# x: int = 5
annotation_assignment = target ":" expression [ "=" expression ] ;

# Async for / async with (PEP 525, PEP 530):
async_for_stmt   = "async" for_stmt ;
async_with_stmt  = "async" with_stmt ;
async_funcdef    = "async" funcdef ;

# Underscores in numeric literals (PEP 515):
# 1_000_000, 0xFF_FF — this is a token-level change, not grammar
```

### Python 3.8 — Walrus Operator and Positional-Only Parameters

```ebnf
# Named expression / walrus operator (PEP 572):
# if (n := len(a)) > 10: ...
named_expression = NAME ":=" expression ;

# This can appear inside if conditions, while conditions, comprehensions,
# and as the value in an assignment expression:
#   while chunk := f.read(8192): ...
#   filtered = [y for x in data if (y := f(x)) is not None]

# Positional-only parameters (PEP 570):
# def f(pos_only, /, normal, *, kw_only): ...
# The / separator means: everything before it is positional-only.
paramlist      = [ param { "," param } [ "," "/" { "," param } ] ]
                 [ "*" [ param ] { "," param } ]
                 [ "," "**" param ] ;
```

### Python 3.10 — Structural Pattern Matching

This is the largest grammar addition since Python 3.0. Pattern matching adds
an entire sub-grammar:

```ebnf
# Match statement (PEP 634):
match_stmt     = "match" subject_expr ":" NEWLINE INDENT
                 { case_clause } DEDENT ;

case_clause    = "case" pattern [ guard ] ":" suite ;
guard          = "if" expression ;

# Patterns form their own mini-language:
pattern        = or_pattern | as_pattern ;
as_pattern     = or_pattern "as" NAME ;
or_pattern     = closed_pattern { "|" closed_pattern } ;
closed_pattern = literal_pattern
               | capture_pattern
               | wildcard_pattern
               | group_pattern
               | sequence_pattern
               | mapping_pattern
               | class_pattern
               | value_pattern ;

literal_pattern   = NUMBER | STRING | "None" | "True" | "False" ;
capture_pattern   = NAME ;          # binds the matched value to NAME
wildcard_pattern  = "_" ;           # matches anything, binds nothing
group_pattern     = "(" pattern ")" ;
sequence_pattern  = "[" [ pattern { "," pattern } ] "]"
                  | "(" pattern "," [ pattern { "," pattern } ] ")" ;
mapping_pattern   = "{" [ key_pattern ":" pattern { "," key_pattern ":" pattern } ] "}" ;
class_pattern     = NAME "(" [ pattern_args ] ")" ;
value_pattern     = NAME { "." NAME } ;  # dotted name — distinguishes from capture
```

Note the subtlety: `match` and `case` are *soft keywords*. This means the
grammar must handle ambiguity. `match = 5` is still a valid assignment
because `match` is a NAME in that context. The parser disambiguates by
looking ahead: if NAME is followed by an expression and `:`, it is a match
statement; otherwise, it is an identifier.

### Python 3.12 — Type Parameter Syntax

```ebnf
# Type alias statement (PEP 695):
# type Vector = list[float]
type_alias_stmt  = "type" NAME [ type_params ] "=" expression ;

# Type parameters on class and function definitions:
# def f[T](x: T) -> T: ...
# class Stack[T]: ...
type_params      = "[" type_param { "," type_param } "]" ;
type_param       = NAME [ ":" expression ]           # TypeVar
                 | "**" NAME                          # ParamSpec
                 | "*"  NAME ;                        # TypeVarTuple

# F-string improvements (PEP 701):
# F-strings can now contain:
#   - Backslash escapes inside expressions: f"newline: {chr(10)}"  (was SyntaxError)
#   - Nested quotes of the same type: f"{'hello'}"  (was SyntaxError)
#   - Multi-line expressions: f"{
#       x + y
#   }"
# These changes affect the *tokenizer* (f-string tokenization is rewritten
# to use a state machine) rather than the grammar productions. The grammar
# rules for f_string remain structurally the same.
```

### Starlark — A Deliberate Subset

Starlark removes entire categories of grammar rules. The full mapping is
in the Starlark section below, but the key grammar differences from Python
3.6 are:

```ebnf
# No class definitions
# class_def is entirely absent from the grammar.

# No import statements
# import_stmt and from_import_stmt are absent.
# Instead: load("file.star", "symbol")
load_stmt        = "load" "(" STRING { "," load_arg } [ "," ] ")" ;
load_arg         = [ NAME "=" ] STRING ;

# No while loops
# while_stmt is absent. Use: for i in range(n)

# No try/except/finally
# try_stmt is absent. Errors are fatal. (Deliberate: no error swallowing.)

# No yield (no generators)
# yield_expr and yield_stmt are absent.

# No global/nonlocal declarations
# global_stmt and nonlocal_stmt are absent.

# No del statement
# del_stmt is absent. Bindings are immutable once created.

# Restricted top-level: only def, load, and simple assignments at file scope.
# No top-level if/for — ensures BUILD files are declarative.
file             = { NEWLINE | def_stmt | load_stmt | simple_stmt } ;
```


## Starlark as a Python Subset

One of the most instructive exercises in language design is drawing the exact
boundary between a language and its subset. Starlark deliberately keeps
Python's *expression* language (almost) intact while removing the *statement*
language features that make programs unpredictable.

### What Starlark Keeps (from Python 3.6)

- All arithmetic and comparison operators
- String operations (slicing, concatenation, methods)
- Lists, tuples, dicts (creation, indexing, methods)
- List/dict comprehensions
- `if`/`elif`/`else` statements
- `for`/`in` loops (but not `while`)
- Function definitions (`def`) with default args, `*args`, `**kwargs`
- `lambda` expressions
- Boolean operators (`and`, `or`, `not`)
- `pass`, `break`, `continue`, `return`
- `True`, `False`, `None`

### What Starlark Removes (and Why)

| Removed Feature     | Python Syntax         | Why Removed                                      |
|---------------------|-----------------------|--------------------------------------------------|
| Classes             | `class Foo:`          | OOP adds complexity; functions suffice for config |
| Import              | `import os`           | Sandbox escape; `load()` is controlled            |
| While loops         | `while True:`         | Infinite loops break determinism                  |
| Try/except          | `try: ... except:`    | Error swallowing hides bugs in config             |
| Raise               | `raise ValueError()`  | No exceptions = no exception hierarchy            |
| With                | `with open() as f:`   | Context managers imply resources / side effects    |
| Yield               | `yield x`             | Generators add hidden state                       |
| Global/nonlocal     | `global x`            | Mutable global state breaks reproducibility       |
| Del                 | `del x`               | Deletion complicates reasoning about bindings      |
| Is                  | `x is None`           | Identity is an implementation detail               |
| Assert              | `assert x > 0`        | Use `fail()` instead — explicit is better          |
| Async/await         | `async def f():`      | No concurrency in config evaluation                |
| Star expressions    | `a, *b = [1,2,3]`    | Kept simple: no unpacking assignment               |

### Venn Diagram

```
+---------------------------------------------------------------+
|                      Python 3.6                                |
|                                                                |
|   +---------------------------------------------------+       |
|   |                                                   |       |
|   |           Starlark (Python subset)                |       |
|   |                                                   |       |
|   |   Arithmetic    String ops    Lists/Dicts         |       |
|   |   Comprehensions    if/elif/else    for/in        |       |
|   |   def/lambda    pass/break/continue/return        |       |
|   |   True/False/None    and/or/not                   |       |
|   |   Tuple unpacking (read)    Slicing               |       |
|   |   load() [Starlark-only]                          |       |
|   |                                                   |       |
|   +---------------------------------------------------+       |
|                                                                |
|   Python-only features (not in Starlark):                      |
|   class, import, while, try/except/finally, raise, with,      |
|   yield, global, nonlocal, del, is, assert, async/await,      |
|   decorators (@), star assignment, walrus :=, match/case,      |
|   type aliases, f-strings, generators, context managers        |
|                                                                |
+---------------------------------------------------------------+
```


## Token File Format

All `.tokens` files in this collection follow the format established in
`starlark.tokens` and documented in spec 02 (Lexer):

```
# @version 1                          Magic comment identifying format version

mode: indentation                      Activates INDENT/DEDENT/NEWLINE synthesis

skip:                                  Patterns consumed but not emitted
  COMMENT    = /#[^\n]*/
  WHITESPACE = /[ \t]+/

TOKEN_NAME = /regex/                   Regex-based token pattern
TOKEN_NAME = "literal"                 Exact literal match
TOKEN_NAME = /regex/ -> ALIAS          Multiple patterns emitting same type

keywords:                              Names reclassified as KEYWORD tokens
  if
  else
  ...

reserved:                              Names that produce errors if used
  class
  import
  ...
```

### Key Rules

1. **Order matters: first match wins.** Multi-character operators must come
   before their single-character prefixes (`**` before `*`, `==` before `=`,
   `//` before `/`). Triple-quoted strings before single-quoted strings.

2. **Keywords are matched after NAME.** The lexer first matches `NAME`, then
   checks if the value is in the `keywords:` list. If so, the token type is
   changed to `KEYWORD` (or a version-specific keyword type).

3. **Reserved words produce errors.** If a NAME matches the `reserved:` list,
   the lexer emits a diagnostic error. This is used in Starlark to reject
   Python-only keywords with clear messages.

4. **The `-> ALIAS` mechanism** lets multiple regex patterns emit the same
   token type. For example, all string variations (raw, byte, triple-quoted)
   emit `STRING`.

5. **`mode: indentation`** activates the INDENT/DEDENT/NEWLINE synthesis
   algorithm. All Python and Starlark token files use this mode.


## Grammar File Format

All `.grammar` files follow the notation established in `starlark.grammar`:

```
# @version 1

rule_name = definition ;

# UPPERCASE = token references (from corresponding .tokens file)
# lowercase = grammar rule references (can be recursive)
#
# Notation:
#   |       alternation (or)
#   { x }   zero or more repetitions
#   [ x ]   optional
#   ( x )   grouping
#   "lit"   literal keyword match
```

### Soft Keywords in the Grammar

Python 3.10 introduced a new concept that our grammar format must handle:
*soft keywords*. A soft keyword is a word that acts as a keyword only in
specific grammatical contexts.

For example, `match` is a keyword in:
```python
match command:
    case "quit":
        exit()
```

But `match` is a regular identifier in:
```python
match = re.match(pattern, string)
```

Our grammar files handle this by matching the *token value* rather than a
keyword token type. The grammar rule uses `"match"` (a literal value match
on a NAME token) rather than MATCH (a dedicated keyword token). This means
the lexer does not need to know about soft keywords — the parser resolves
the ambiguity using grammatical context and lookahead.


## Implementation Plan

Each versioned grammar pair needs lexer and parser wrappers that select the
correct grammar files and configure version-specific behavior.

### Priority Languages (for the Python VM pipeline)

| Language   | Package Name                  | Purpose                            |
|------------|-------------------------------|------------------------------------|
| Go         | `python_grammar_selector`     | Build tool, grammar validation     |
| TypeScript | `python-grammar-selector`     | Interactive visualizer             |
| Rust       | `python_grammar_selector`     | Performance-critical parsing       |

### Secondary Languages (educational implementations)

| Language   | Package Name                  | Purpose                            |
|------------|-------------------------------|------------------------------------|
| Python     | `python_grammar_selector`     | Self-hosting, bootstrapping        |
| Elixir     | `python_grammar_selector`     | Pattern matching showcase          |
| Ruby       | `python_grammar_selector`     | Metaprogramming showcase           |
| Lua        | `python_grammar_selector`     | Minimal embedding showcase         |
| Perl       | `python_grammar_selector`     | Regex showcase                     |

### What the Selector Does

The selector is a thin wrapper around the grammar-driven lexer/parser:

```python
# Pseudocode — the API is the same in every language
selector = PythonGrammarSelector(version="3.10")

tokens = selector.tokenize("match command:\n    case 'quit': exit()")
# Uses python3.10.tokens -> correctly handles soft keywords

ast = selector.parse(tokens)
# Uses python3.10.grammar -> produces match_stmt AST node
```

The selector:

1. Resolves the grammar file pair for the requested version.
2. Configures the lexer with the correct token definitions (including
   version-specific keywords, operators, and f-string handling).
3. Configures the parser with the correct production rules.
4. Exposes `tokenize()` and `parse()` methods.
5. Optionally provides a `diff(version_a, version_b)` method that returns
   the structural differences between two versions' grammars.


## Testing Strategy

Grammar correctness is not something we can eyeball. We need automated
verification against the authoritative source: CPython itself.

### Level 1: Self-Consistency

Verify that each `.tokens` and `.grammar` file pair is internally consistent:

- Every token referenced in the grammar exists in the tokens file.
- Every grammar rule referenced by another rule exists.
- No unreachable rules (rules defined but never referenced from `file`).
- No left-recursive rules that would cause infinite loops in recursive
  descent.
- Keywords listed in `keywords:` do not collide with token patterns.

Our existing `grammar_tools` package (spec F04) provides cross-validation
for exactly this purpose.

### Level 2: Token-Level Verification

Tokenize real Python source files and verify no unknown tokens:

1. Obtain CPython's standard library source for each version (e.g.,
   the `Lib/` directory from the CPython 3.10 release tag).
2. Tokenize every `.py` file using our lexer with the corresponding
   version's `.tokens` file.
3. Assert: zero `UNKNOWN` or `ERROR` tokens.
4. Cross-check: compare our token stream against CPython's own `tokenize`
   module output for the same file. Every token type and span must match.

### Level 3: Parse-Level Verification

Parse real Python source files and verify valid AST construction:

1. Parse every `.py` file from CPython's test suite using our parser with
   the corresponding version's `.grammar` file.
2. Assert: zero parse errors on files that CPython itself accepts.
3. Negative testing: parse files with syntax errors and verify our parser
   rejects them (compare against CPython's error messages).

### Level 4: Cross-Version Validation

Verify version-specific features are correctly gated:

- Tokenize `x := 5` with python3.6.tokens: should produce `NAME COLON
  EQUALS` (three tokens, not a walrus operator).
- Tokenize `x := 5` with python3.8.tokens: should produce `NAME
  COLONEQUALS INT` (walrus operator recognized).
- Parse `match x: ...` with python3.8.grammar: should fail (no match
  statement).
- Parse `match x: ...` with python3.10.grammar: should succeed.

### Level 5: Starlark Boundary Verification

Verify that Starlark correctly rejects Python-only features:

- Tokenize `class Foo: pass` with starlark.tokens: should produce a
  reserved-keyword error on `class`.
- Parse `while True: pass` with starlark.grammar: should fail (no
  `while_stmt` rule).
- Parse `load("file.star", "sym")` with python3.6.grammar: should fail
  (no `load_stmt` rule).


## Future Work

- **Python 3.11 grammar** (exception groups with `except*`) — add if there
  is demand.
- **Python 3.13+ grammars** — add as new versions ship meaningful syntax
  changes.
- **Grammar diff tool** — a utility that takes two `.grammar` files and
  produces a human-readable diff of production rules (added, removed,
  modified). This would be invaluable for studying language evolution.
- **Interactive grammar explorer** — a TypeScript web app where you can
  select a Python version, type code, and see how the tokenizer and parser
  behave differently across versions. Click on a token to see which grammar
  rule it participates in. Click on a grammar rule to see its evolution
  across versions.
- **CPython Grammar/Grammar import tool** — a script that converts CPython's
  official PEG grammar (`Grammar/python.gram`) to our EBNF `.grammar`
  format. This would automate grammar file creation for new versions.
