# Lexical Analysis -- From Source Text to Tokens

## What is Lexical Analysis?

Lexical analysis (also called *lexing*, *scanning*, or *tokenization*) is the
very first phase of understanding a programming language. It takes raw source
code -- a string of characters -- and breaks it into meaningful chunks called
**tokens**.

Think of it like reading a sentence in English. When you see:

    The cat sat on the mat.

Your brain automatically groups the letters into words: "The", "cat", "sat",
"on", "the", "mat", and the period ".". You don't think about individual
letters -- you think about *words* and *punctuation*.

A lexer does the same thing for source code:

```
Source text:    x = 1 + 2

                    |
                    | Lexical Analysis
                    v

Token stream:   NAME("x")  EQUALS("=")  NUMBER("1")  PLUS("+")  NUMBER("2")  EOF
```

Each token pairs a **type** (what kind of thing it is) with a **value** (the
actual text from the source), plus location information for error reporting:

```
    +------------+
    | NAME       |  <-- type (what kind of token)
    | "x"        |  <-- value (the actual text)
    | line 1     |  <-- where it appeared in the source
    | col 1      |
    +------------+
```

### Why is lexing useful?

The lexer simplifies everything that comes after it. The *parser* (the next
stage) doesn't have to worry about whitespace, or whether a number is one
digit or five digits. It just sees a clean stream of tokens to work with.

This separation of concerns is a fundamental principle of compiler design:

```
Source code  -->  [ Lexer ]  -->  Tokens  -->  [ Parser ]  -->  AST  --> ...
                  ^^^^^^^^^                    ^^^^^^^^^^
                  "What are      "What do these tokens
                   the words?"     mean together?"
```

---

## Token Types

Every programming language has several categories of tokens. Here are the
ones our lexer recognizes:

### 1. Keywords

Reserved words that have special meaning in the language. You can't use them
as variable names.

```
Python:     if  else  elif  while  for  def  return  True  False  None
Ruby:       if  else  elsif  end  while  def  return  true  false  nil
JavaScript: if  else  while  for  function  return  let  const  true  false  null
```

Keywords look exactly like identifiers syntactically -- the lexer distinguishes
them by checking against a configurable list.

### 2. Identifiers (Names)

Names that refer to variables, functions, or other user-defined entities.

```
x       total       my_variable       _private       calculateSum
```

Identifiers follow a common pattern across most languages:
- Start with a letter or underscore: `a-z`, `A-Z`, `_`
- Continue with letters, digits, or underscores: `a-z`, `A-Z`, `0-9`, `_`

### 3. Literals

Actual values written directly in the source code.

```
Number literals:   42    0    1000    3
String literals:   "Hello, World!"    ""    "abc 123"
```

### 4. Operators

Symbols that represent operations.

```
Arithmetic:   +  -  *  /
Assignment:   =
Comparison:   ==  !=  <  >  <=  >=
```

Multi-character operators (like `==`) require special handling -- the lexer
must use *lookahead* to distinguish `=` (assignment) from `==` (comparison).

### 5. Delimiters

Punctuation that structures the code.

```
Parentheses:   (  )
Braces:        {  }
Brackets:      [  ]
Others:        ,  :  ;  .
```

### 6. Whitespace and Newlines

Spaces and tabs are usually skipped silently. Newlines may or may not be
significant -- in Python, they mark the end of a statement; in C or Java,
they are treated as whitespace.

### 7. EOF (End of File)

A synthetic token the lexer appends at the very end. It gives the parser
a clean stop signal so it doesn't have to constantly check "am I past the
end?".

---

## How a Lexer Works Step-by-Step

Let's walk through tokenizing the source code `x = 1 + 2` character by
character.

### The Setup

The lexer maintains:
- A **position** (index into the source string) -- like a finger pointing
  at the current character
- A **line** and **column** counter for error reporting
- A growing **list of tokens**

```
Source:  x   =   1   +   2
         ^
         |
       position = 0
       (line 1, column 1)
```

### Step 1: See 'x' (a letter) -- read an identifier

The lexer sees a letter and knows an identifier is starting. It reads
characters as long as they are letters, digits, or underscores:

```
Source:  x   =   1   +   2
         ^
         Read 'x' -- is 'x' a letter/digit/underscore? Yes. Advance.
             ^
         Next char is ' ' -- not part of the identifier. Stop.

         Check: is "x" in the keyword list? No.
         Emit: NAME("x") at line 1, column 1
```

### Step 2: See ' ' (space) -- skip whitespace

```
Source:  x   =   1   +   2
             ^
         Space is whitespace. Advance past it. No token emitted.
```

### Step 3: See '=' -- lookahead for '==' vs '='

This is the tricky part. The lexer sees `=` and needs to decide: is this
assignment (`=`) or comparison (`==`)? It *peeks* at the next character
without consuming it:

```
Source:  x   =   1   +   2
                 ^
         Current: '='
         Peek ahead: ' ' (not '=')
         Decision: this is a single '=' (assignment)
         Emit: EQUALS("=") at line 1, column 3
```

If the source had been `x == 1`, the peek would see another `=`, and the
lexer would consume both characters and emit EQUALS_EQUALS("==") instead.

### Step 4: Skip space

```
Source:  x   =   1   +   2
                     ^
         Space. Skip.
```

### Step 5: See '1' (a digit) -- read a number

```
Source:  x   =   1   +   2
                     ^
         Read '1' -- is next char a digit? No (it's ' ').
         Emit: NUMBER("1") at line 1, column 5
```

### Step 6: Skip space, see '+' -- simple single-character token

```
Source:  x   =   1   +   2
                         ^
         '+' is in the simple tokens table.
         Emit: PLUS("+") at line 1, column 7
```

### Step 7: Skip space, read '2'

```
Source:  x   =   1   +   2
                             ^
         Emit: NUMBER("2") at line 1, column 9
```

### Step 8: End of input

```
         Emit: EOF("") at line 1, column 10
```

### The Final Token Stream

```
Token #  Type       Value  Line  Col
------  ---------  -----  ----  ---
  1     NAME       "x"      1    1
  2     EQUALS     "="      1    3
  3     NUMBER     "1"      1    5
  4     PLUS       "+"      1    7
  5     NUMBER     "2"      1    9
  6     EOF        ""       1   10
```

---

## Hand-Written vs Grammar-Driven Lexing

This repo implements **two** different approaches to lexing. Both produce
identical `Token` objects, so downstream consumers (the parser, the compiler)
don't care which one generated the tokens.

### Hand-Written Lexer

**Location:** `code/packages/python/lexer/src/lexer/tokenizer.py`

The hand-written lexer uses **dispatch on first character**. A large
if/elif chain looks at the current character and delegates to specialized
reading methods:

```
    if char is a digit      -->  _read_number()
    if char is a letter     -->  _read_name()
    if char is '"'          -->  _read_string()
    if char is '='          -->  peek ahead (= vs ==)
    if char in simple_table -->  look up the token type
    else                    -->  error
```

**Pros:**
- Easy to understand and debug -- you can step through it line by line
- Full control over every aspect of tokenization
- Good performance (direct character dispatch)
- Perfect for teaching

**Cons:**
- Grammar rules are baked into the code
- Supporting a new language means writing new Python code
- Changes to the token set require code changes

### Grammar-Driven Lexer

**Location:** `code/packages/python/lexer/src/lexer/grammar_lexer.py`

The grammar-driven lexer reads token definitions from a `.tokens` file and
uses compiled regex patterns to match tokens:

```
    For each position in the source:
        For each pattern in definition order (first match wins):
            Try to match the pattern at the current position
            If it matches:
                Emit a token with the matched text
                Advance past the match
                Break
```

**Pros:**
- Language-agnostic -- swap the `.tokens` file to tokenize a different language
- Data-driven -- no code changes needed for new tokens
- Mirrors classic tools like Lex/Flex
- Same codebase handles Python, Ruby, JavaScript, TypeScript

**Cons:**
- Regex matching can be slower than direct character dispatch
- Harder to debug (regex failures give less context)
- Some tokenization patterns are hard to express as regex

### Comparison Table

```
                Hand-Written          Grammar-Driven
                ============          ==============
Approach:       Dispatch on char      Regex matching
Grammar:        In the code           In .tokens file
New language:   Write new code        Write new .tokens file
Performance:    Fast (direct)         Good (regex)
Debugging:      Easy (step-through)   Harder (regex)
Use case:       Reference/teaching    Multi-language support
```

---

## The .tokens Grammar File Format

The `.tokens` file format is a declarative way to define the tokens of a
programming language. It is parsed by the `grammar_tools` package.

**Location:** `code/grammars/` -- contains `python.tokens`, `ruby.tokens`,
`javascript.tokens`, `typescript.tokens`

### Format

```
# Comments start with #

# Regex-based token pattern:
TOKEN_NAME = /regex/

# Literal (exact string) match:
TOKEN_NAME = "literal"

# Keywords section -- names that become KEYWORD tokens instead of NAME:
keywords:
  if
  else
  while
```

### How It Works

1. Each line defines a token: a name and a pattern
2. **Regex patterns** (`/[a-zA-Z_][a-zA-Z0-9_]*/`) match using regular
   expressions
3. **Literal patterns** (`"+"`) match an exact string (special characters
   like `+` and `*` are escaped automatically)
4. **Order matters**: first match wins. Multi-character tokens must come
   before single-character ones (e.g., `==` before `=`)
5. The `keywords:` section lists words that should be classified as KEYWORD
   tokens instead of NAME tokens

### Example: Python Token Definitions

```
# Literals
NAME        = /[a-zA-Z_][a-zA-Z0-9_]*/
NUMBER      = /[0-9]+/
STRING      = /"([^"\\]|\\.)*"/

# Multi-character operators (must come before single-char versions)
EQUALS_EQUALS = "=="

# Single-character operators
EQUALS      = "="
PLUS        = "+"
MINUS       = "-"
STAR        = "*"
SLASH       = "/"

# Delimiters
LPAREN      = "("
RPAREN      = ")"
COMMA       = ","
COLON       = ":"

# Keywords
keywords:
  if
  else
  elif
  while
  def
  return
  True
  False
  None
```

### Why Order Matters

Consider the source text `==`. If `EQUALS = "="` appeared before
`EQUALS_EQUALS = "=="` in the `.tokens` file, the lexer would match the
first `=` as an EQUALS token, then the second `=` as another EQUALS token.
We'd get two assignment operators instead of one comparison operator.

By putting `EQUALS_EQUALS = "=="` first, the two-character pattern wins:

```
    Token definitions order:
        1. EQUALS_EQUALS = "=="    <-- tried first
        2. EQUALS         = "="    <-- tried second

    Source: "=="
        Pattern "==" matches at position 0  -->  EQUALS_EQUALS("==")

    Source: "= 1"
        Pattern "==" does NOT match (only one =)
        Pattern "=" matches                 -->  EQUALS("=")
```

---

## How the Grammar-Driven Lexer Works

The `GrammarLexer` class takes a `TokenGrammar` (parsed from a `.tokens`
file) and tokenizes source code using compiled regex patterns.

### Initialization

```
1. Parse the .tokens file into a TokenGrammar object
   (grammar_tools.parse_token_grammar)

2. Compile each token definition into a Python regex:
   - Regex patterns: compiled as-is
   - Literal patterns: escaped (so "+" becomes r"\+")

3. Pre-compute the keyword set for O(1) lookup
```

### Tokenization Loop

```
while characters remain:
    1. Skip whitespace (spaces, tabs)
    2. If newline: emit NEWLINE token
    3. Otherwise, try each compiled pattern in order:
       - Use regex.match() at the current position
       - First pattern that matches wins
       - Emit a token with the matched text and type
       - Advance past the matched characters
    4. If no pattern matches: raise LexerError

Append EOF token
```

### Keyword Reclassification

When the lexer matches a NAME token, it checks whether the matched value
is in the keyword set. If so, the token type is changed from NAME to KEYWORD:

```
    Source: "if x"

    Match 1: Pattern NAME = /[a-zA-Z_][a-zA-Z0-9_]*/ matches "if"
             Is "if" in the keyword set? Yes.
             Emit: KEYWORD("if")

    Match 2: Pattern NAME matches "x"
             Is "x" in the keyword set? No.
             Emit: NAME("x")
```

This is why the `keywords:` section in the `.tokens` file is separate from
the token definitions -- keywords are syntactically identical to identifiers
and are only distinguished by a lookup table.

---

## Cross-Language Lexing

One of the most powerful features of the grammar-driven lexer is
**cross-language tokenization**. The same `GrammarLexer` class can tokenize
Python, Ruby, JavaScript, and TypeScript -- just by loading a different
`.tokens` file.

### How the same lexer handles different languages

```
                python.tokens
                     |
Source "if x"  -->  GrammarLexer  -->  KEYWORD("if") NAME("x")
                     |
                ruby.tokens
                     |
Source "if x"  -->  GrammarLexer  -->  KEYWORD("if") NAME("x")
                     |
             javascript.tokens
                     |
Source "let x" -->  GrammarLexer  -->  KEYWORD("let") NAME("x")
```

### What changes between languages

| Feature            | Python           | Ruby              | JavaScript         |
|--------------------|------------------|-------------------|--------------------|
| Keywords           | `elif`, `None`   | `elsif`, `nil`    | `let`, `const`     |
| Boolean literals   | `True`, `False`  | `true`, `false`   | `true`, `false`    |
| Block delimiters   | Indentation      | `end`             | `{ }`              |
| Equality           | `==`             | `==`              | `===` and `==`     |
| Identifiers        | `[a-zA-Z_]...`   | `[a-zA-Z_]...`    | `[a-zA-Z_$]...`    |

JavaScript's NAME pattern includes `$` (valid in JS identifiers like `$scope`).
JavaScript also has three-character operators like `===` and `!==` that must
be defined before their two-character counterparts.

### Language-specific .tokens files

```
code/grammars/
  python.tokens         -- Python token definitions
  ruby.tokens           -- Ruby token definitions
  javascript.tokens     -- JavaScript token definitions
  typescript.tokens     -- TypeScript token definitions
```

### Language-specific lexer packages

The repo also has hand-written lexer variants that pre-configure the
`LexerConfig` with language-specific keywords:

```
code/packages/python/
  lexer/                -- Core lexer (hand-written + grammar-driven)
  javascript-lexer/     -- JavaScript keyword configuration
  ruby-lexer/           -- Ruby keyword configuration
  typescript-lexer/     -- TypeScript keyword configuration
```

---

## Error Handling in Lexers

A good lexer produces helpful error messages when it encounters something
it doesn't understand.

### Common Lexer Errors

**1. Unexpected Character**

```python
x = 1 @ 2
          ^
Lexer error at 1:7: Unexpected character: '@'
```

The lexer doesn't know what `@` means (it's not a defined operator or
delimiter in our language).

**2. Unterminated String Literal**

```python
name = "hello
               ^
Lexer error at 1:8: Unterminated string literal
```

The lexer reached the end of the line (or file) without finding the closing
quote.

**3. Unterminated String with Trailing Backslash**

```python
path = "C:\
            ^
Lexer error at 1:8: Unterminated string literal (ends with backslash)
```

The backslash started an escape sequence, but there's no character after it.

### Error Location Tracking

The lexer tracks line and column numbers as it advances through the source.
Every time it encounters a newline character, it increments the line counter
and resets the column to 1. This means error messages always point to the
exact position of the problem:

```
    Line tracking:
        'x'   -> line 1, col 1
        ' '   -> line 1, col 2
        '='   -> line 1, col 3
        '\n'  -> increment line to 2, reset col to 1
        'y'   -> line 2, col 1
```

### The LexerError Class

Both the hand-written and grammar-driven lexers raise `LexerError` with
the same information:
- A human-readable message describing the problem
- The line number where it occurred
- The column number where it occurred

This consistency means the parser doesn't need to know which lexer was used.

---

## References

| File | Description |
|------|-------------|
| `code/packages/python/lexer/src/lexer/tokenizer.py` | Hand-written lexer with detailed comments |
| `code/packages/python/lexer/src/lexer/grammar_lexer.py` | Grammar-driven lexer |
| `code/packages/python/grammar-tools/` | Parses `.tokens` and `.grammar` files |
| `code/grammars/python.tokens` | Python token definitions |
| `code/grammars/ruby.tokens` | Ruby token definitions |
| `code/grammars/javascript.tokens` | JavaScript token definitions |
| `code/grammars/typescript.tokens` | TypeScript token definitions |
