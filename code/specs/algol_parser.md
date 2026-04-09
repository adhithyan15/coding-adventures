# ALGOL 60 Parser

## Overview

This spec defines the parser for ALGOL 60. The parser takes the flat token stream produced by the
ALGOL 60 lexer and builds an Abstract Syntax Tree (AST) — a tree that captures the grammatical
structure of the program.

ALGOL 60 is historically significant for its parser because:

- The **ALGOL 60 report was the first language specification written using BNF** — Backus-Naur
  Form. John Backus invented the notation; Peter Naur edited the report and refined it. The grammar
  in this spec derives directly from that 1960 document.
- The language introduced **block structure** — the `begin...end` pair that creates a new lexical
  scope. Every language with `{...}` or indented blocks inherits this concept.
- ALGOL 60 introduced the **dangling else problem** — a famous grammar ambiguity that every
  subsequent language with if/then/else has had to resolve. This spec shows exactly how ALGOL 60
  resolves it.
- The **conditional expression** (`if b then x else y` as an expression, not just a statement) is
  a clean feature that C replaced with the awkward `?:` ternary operator.
- **Call-by-name vs. call-by-value** parameters are captured in the AST — which parameters are
  passed by value must be declared explicitly in procedure headers.

## Layer Position

```
Logic Gates → Arithmetic → CPU → ARM → Assembler → Lexer → [YOU ARE HERE] → Compiler → VM
```

**Input from:** ALGOL 60 lexer (provides the token stream).
**Output to:** Bytecode compiler or interpreter (walks the AST to execute or compile the program).

## Concepts

### Block Structure and Lexical Scoping

A **block** is the fundamental unit of ALGOL 60 programs. It consists of:
1. The keyword `begin`
2. Zero or more declarations (variables, arrays, procedures)
3. One or more statements
4. The keyword `end`

```algol
begin
  integer x, y;        (* declarations come first *)
  real z;
  x := 10;             (* then statements *)
  y := x + 1;
  z := y * 3.14
end
```

Declarations are **only allowed at the start of a block**, before any statements. You cannot
declare a variable in the middle of executable code. This is the same rule C enforced through
C89 (C99 relaxed it).

Blocks can be nested. Each block creates a new **lexical scope** — a new namespace for variable
names. A variable declared in an inner block shadows a same-named variable in an outer block, and
the inner variable disappears when its block ends:

```algol
begin
  integer x;
  x := 1;
  begin
    integer x;     (* different x, shadows outer x *)
    x := 2;
    (* x is 2 here *)
  end;
  (* x is 1 here — inner x is gone *)
end
```

This scoping model — names resolved by the static, textual structure of the program — is called
**lexical scoping** or **static scoping**. It is the model used by every mainstream language today
(Python, JavaScript, Rust, Go, Java). The alternative, **dynamic scoping** (names resolved by
the call stack at runtime), was used by early Lisp but was found to be difficult to reason about.

### The Dangling Else Problem

Consider:

```algol
if a then if b then s1 else s2
```

This is grammatically ambiguous: does `else s2` belong to the outer `if a` or the inner `if b`?
Two readings are possible:

```
Reading 1: if a then (if b then s1 else s2)   ← else belongs to inner if
Reading 2: if a then (if b then s1) else s2   ← else belongs to outer if
```

ALGOL 60 resolves this by restricting what can appear as the `then`-branch of a conditional
statement: the then-branch must be an **unconditional statement** (one that cannot itself be a
conditional). The `else`-branch, however, can be any statement including another conditional.

This is captured in the grammar:

```
cond_stmt    = IF bool_expr THEN unlabeled_stmt [ ELSE statement ] ;
```

Note: `then` requires `unlabeled_stmt` (which excludes `cond_stmt`), but `else` takes `statement`
(which includes `cond_stmt`). This means:

- `if a then if b then s1 else s2` is **syntactically invalid** — the then-branch `if b then s1`
  is a conditional statement, which is not an unlabeled_stmt
- The programmer must use `begin...end` to make the intent explicit:
  ```algol
  if a then begin if b then s1 else s2 end   (* else belongs to inner if *)
  if a then begin if b then s1 end else s2   (* else belongs to outer if *)
  ```

Most later languages (C, Java, etc.) resolved the dangling else differently: the else always
binds to the nearest if. ALGOL's approach forces explicitness, which eliminates the ambiguity
entirely at the grammar level.

### Conditional Expressions

ALGOL 60 allows `if...then...else` **as an expression**, not just a statement:

```algol
x := if y > 0 then y else -y     (* absolute value *)
z := if flag then 1.0 else 0.0   (* conditional real *)
```

This is called a **conditional arithmetic expression** (for numeric results) or a **conditional
boolean expression** (for boolean results). C replaced this with the ternary operator `?:`, which
is less readable. Haskell and most functional languages use `if/then/else` as an expression,
following ALGOL's lead.

The grammar expresses this by making `IF bool_expr THEN ... ELSE ...` a valid production in the
expression rules, not just in the statement rules.

### Exponentiation Associativity

The ALGOL 60 report defines `^` (exponentiation) as **left-associative**:

```
2 ^ 3 ^ 4 = (2 ^ 3) ^ 4 = 8 ^ 4 = 4096
```

Most mathematical notation and most modern languages (Python's `**`, Ruby's `**`) treat
exponentiation as right-associative:

```
2 ** 3 ** 4 = 2 ** (3 ** 4) = 2 ** 81   (a much larger number)
```

This spec follows the ALGOL 60 report: left-associative. The grammar rule:

```
factor = primary { ( CARET | POWER ) primary } ;
```

produces left-associativity because `{ x }` means "zero or more, left-folded". This is a
historically notable quirk — document it when implementing.

### Multiple Assignment

ALGOL 60 allows multiple targets in one assignment statement:

```algol
x := y := z := 0     (* assign 0 to z, then y, then x — right to left *)
```

The grammar rule `assign_stmt = left_part { left_part } expression` handles this. Each
`left_part` is a variable followed by `:=`. The value flows right-to-left: the rightmost
expression is evaluated once and assigned to all targets.

### Call-by-Value vs. Call-by-Name

ALGOL 60 has two parameter passing modes:

**Call by value** (default is call by name — must opt into value):
```algol
procedure increment(x); value x; integer x;
begin
  (* x is a copy — changes don't affect the caller *)
end
```

**Call by name** (the default — no `value` declaration needed):
```algol
procedure badSwap(x, y); integer x, y;
begin
  integer temp;
  temp := x; x := y; y := temp    (* BUG: call-by-name makes this broken *)
end
```

With call-by-name, `x` and `y` are not copies — they are **textual substitutions** (similar in
concept to C macros). Every time `x` is used inside the procedure, the actual argument expression
is re-evaluated. This enables **Jensen's Device**, a powerful but surprising idiom:

```algol
real procedure sigma(i, lo, hi, term); value lo, hi; integer i, lo, hi; real term;
begin
  real s;
  s := 0;
  for i := lo step 1 until hi do
    s := s + term;   (* term is re-evaluated each iteration with current i *)
  sigma := s
end;

sigma(i, 1, 100, 1/i)   (* computes 1/1 + 1/2 + ... + 1/100 *)
```

Here `i` and `term` are passed by name. Each iteration of the loop uses the current value of `i`
to re-evaluate `1/i`. This is the conceptual ancestor of lazy evaluation and closures.

The parser captures the by-value/by-name distinction in `ProcedureDecl.by_value` — a list of
parameter names that were declared with `value`. All others are by-name.

### Labels and Goto

ALGOL 60 labels can be either identifiers or **integer literals**:

```algol
10: x := 1;
    goto 10;
    (* or *)
    start: x := 2;
    goto start;
```

Integer labels came from Fortran's line numbering convention. The `goto` statement transfers
control to a label. Labels are also used with `switch` declarations for computed gotos:

```algol
switch choices := start, middle, finish;
goto choices[n];   (* jumps to start, middle, or finish depending on n *)
```

### Switch Declarations

A `switch` is a named list of designational expressions (jump targets). It provides a computed
goto — a way to jump to different labels based on an integer index:

```algol
switch s := L1, L2, L3;
goto s[i];   (* jumps to L1 if i=1, L2 if i=2, L3 if i=3 *)
```

This is the predecessor of C's `switch` statement, though the mechanism is different. C's switch
uses value matching; ALGOL's uses index-based dispatch into a table of labels.

## Grammar

The grammar uses the repo's notation:
- `UPPERCASE` — token kinds from the lexer
- `lowercase` — grammar rules (non-terminals)
- `|` — alternation (or)
- `{ x }` — zero or more repetitions of x
- `[ x ]` — optional (zero or one x)
- `( x )` — grouping

```
program        = block ;

# A block is the fundamental unit: declarations then statements, wrapped in begin/end.
# Declarations must precede all statements — no interleaving allowed.
block          = BEGIN { declaration SEMICOLON } statement { SEMICOLON statement } END ;

# ---------------------------------------------------------------------------
# Declarations
# ---------------------------------------------------------------------------

declaration    = type_decl | array_decl | switch_decl | procedure_decl ;

# Simple variable declaration: integer x, y, z
type_decl      = type ident_list ;
type           = INTEGER | REAL | BOOLEAN | STRING ;
ident_list     = IDENT { COMMA IDENT } ;

# Array declaration with explicit bounds (bounds may be runtime expressions).
# The type is optional — defaults to real if omitted.
# array A[1:10]         — single-dimensional
# integer array B[1:m, 1:n]  — two-dimensional integer array
array_decl     = [ type ] ARRAY array_segment { COMMA array_segment } ;
array_segment  = ident_list LBRACKET bound_pair { COMMA bound_pair } RBRACKET ;
bound_pair     = arith_expr COLON arith_expr ;

# Switch declaration: a named list of labels for computed goto.
switch_decl    = SWITCH IDENT ASSIGN switch_list ;
switch_list    = desig_expr { COMMA desig_expr } ;

# Procedure declaration. The return type is optional (void if omitted).
# Parameters are declared in the formal_params list and further refined by
# value_part (which params are call-by-value) and spec_part (what type each param is).
procedure_decl = [ type ] PROCEDURE IDENT [ formal_params ] SEMICOLON
                 [ value_part ] { spec_part } proc_body ;
formal_params  = LPAREN ident_list RPAREN ;
value_part     = VALUE ident_list SEMICOLON ;
spec_part      = specifier ident_list SEMICOLON ;
specifier      = INTEGER | REAL | BOOLEAN | STRING | ARRAY | LABEL | SWITCH | PROCEDURE ;
proc_body      = block | statement ;

# ---------------------------------------------------------------------------
# Statements
# ---------------------------------------------------------------------------

# Any statement may be preceded by a label.
statement      = [ label COLON ] unlabeled_stmt ;
label          = IDENT | INTEGER_LIT ;

# An unlabeled statement is one of: assignment, goto, procedure call,
# compound statement (begin...end without declarations), block, conditional,
# for loop, or empty.
unlabeled_stmt = assign_stmt
               | goto_stmt
               | proc_stmt
               | compound_stmt
               | block
               | for_stmt
               | empty_stmt ;

# Note: cond_stmt is NOT in unlabeled_stmt — this is the dangling-else fix.
# A conditional cannot appear as the then-branch of another conditional.
# To nest conditionals, wrap in begin...end.
cond_stmt      = IF bool_expr THEN unlabeled_stmt [ ELSE statement ] ;

# A compound statement is begin...end without declarations (just statements).
compound_stmt  = BEGIN statement { SEMICOLON statement } END ;

# Assignment: one or more left-hand sides, then one expression.
# x := y := 0 assigns 0 to both (right to left).
assign_stmt    = left_part { left_part } expression ;
left_part      = variable ASSIGN ;

goto_stmt      = GOTO desig_expr ;

# A procedure call as a statement (no return value used).
proc_stmt      = IDENT [ LPAREN actual_params RPAREN ] ;
actual_params  = expression { COMMA expression } ;

empty_stmt     = ;

# For loop with a list of "for elements". Each element is one of:
#   simple:      for i := 5 do ...              (single value)
#   step/until:  for i := 1 step 1 until 10 do (range with step)
#   while:       for i := expr while cond do    (conditional advancement)
# Multiple elements can be chained: for i := 1 step 1 until 5, 10, 20 do
for_stmt       = FOR IDENT ASSIGN for_list DO statement ;
for_list       = for_elem { COMMA for_elem } ;
for_elem       = arith_expr STEP arith_expr UNTIL arith_expr
               | arith_expr WHILE bool_expr
               | arith_expr ;

# ---------------------------------------------------------------------------
# Expressions
# ---------------------------------------------------------------------------

# An expression is either arithmetic or boolean. Designational expressions
# (for goto targets) are handled separately.
expression     = arith_expr | bool_expr ;

# Arithmetic expressions.
# The conditional form (if b then x else y) is the ALGOL conditional expression —
# it produces a numeric value based on a boolean condition.
arith_expr     = IF bool_expr THEN simple_arith ELSE arith_expr
               | simple_arith ;

# simple_arith handles addition and subtraction.
# The optional leading sign handles unary + and -.
simple_arith   = [ PLUS | MINUS ] term { ( PLUS | MINUS ) term } ;

# term handles multiplication, division, integer division, and modulo.
term           = factor { ( STAR | SLASH | DIV | MOD ) factor } ;

# factor handles exponentiation (left-associative per ALGOL 60 report).
# Note: most languages/math uses right-associative exponentiation.
# ALGOL 60 uses left-associative: 2^3^4 = (2^3)^4 = 4096, NOT 2^(3^4).
factor         = primary { ( CARET | POWER ) primary } ;

primary        = INTEGER_LIT
               | REAL_LIT
               | STRING_LIT
               | TRUE
               | FALSE
               | variable
               | proc_call
               | LPAREN arith_expr RPAREN ;

# Boolean expressions, ordered by increasing precedence:
#   eqv   (lowest precedence)
#   impl
#   or
#   and
#   not   (unary, highest among boolean)
# Then relation and primary boolean values.
bool_expr      = IF bool_expr THEN simple_bool ELSE bool_expr
               | simple_bool ;
simple_bool    = implication { EQV implication } ;
implication    = bool_term { IMPL bool_term } ;
bool_term      = bool_factor { OR bool_factor } ;
bool_factor    = bool_secondary { AND bool_secondary } ;
bool_secondary = NOT bool_secondary | bool_primary ;
bool_primary   = TRUE
               | FALSE
               | variable
               | proc_call
               | LPAREN bool_expr RPAREN
               | relation ;

# A relation compares two arithmetic expressions.
relation       = simple_arith ( EQ | NEQ | LT | LEQ | GT | GEQ ) simple_arith ;

# Designational expressions: used as goto targets.
# Can be a label, a switch subscript, or a conditional jump.
desig_expr     = IF bool_expr THEN simple_desig ELSE desig_expr
               | simple_desig ;
simple_desig   = IDENT LBRACKET arith_expr RBRACKET
               | LPAREN desig_expr RPAREN
               | label ;

# Variables and procedure calls.
variable       = IDENT [ LBRACKET subscripts RBRACKET ] ;
subscripts     = arith_expr { COMMA arith_expr } ;
proc_call      = IDENT LPAREN actual_params RPAREN ;
```

## AST Node Types

```elixir
# Top level
%Program{block: Block.t()}

# Block: declarations then statements
%Block{
  declarations: [Declaration.t()],
  statements:   [Statement.t()]
}

# --- Declarations ---

%TypeDecl{
  type:  :integer | :real | :boolean | :string,
  names: [String.t()]
}

%ArrayDecl{
  type:     :integer | :real | :boolean | :string | nil,  # nil = default real
  segments: [ArraySegment.t()]
}

%ArraySegment{
  names:  [String.t()],
  bounds: [{Expr.t(), Expr.t()}]   # [{lower, upper}, ...] one pair per dimension
}

%ProcedureDecl{
  return_type: :integer | :real | :boolean | :string | nil,  # nil = void
  name:        String.t(),
  params:      [String.t()],       # all parameter names in declaration order
  by_value:    [String.t()],       # subset of params declared with VALUE
  spec:        [{specifier, [String.t()]}],  # type specifiers for params
  body:        Statement.t()
}

%SwitchDecl{
  name:   String.t(),
  labels: [Expr.t()]   # list of designational expressions
}

# --- Statements ---

%AssignStmt{
  targets: [Expr.t()],   # one or more left-hand side variables (right-to-left)
  value:   Expr.t()
}

%GotoStmt{target: Expr.t()}

%ProcStmt{
  name: String.t(),
  args: [Expr.t()]   # empty list for calls with no arguments
}

%CompoundStmt{statements: [Statement.t()]}

%IfStmt{
  condition:   Expr.t(),
  then_branch: Statement.t(),
  else_branch: Statement.t() | nil
}

%ForStmt{
  var:  String.t(),
  list: [ForElem.t()],
  body: Statement.t()
}

%LabeledStmt{
  label:     String.t() | integer(),
  statement: Statement.t()
}

%EmptyStmt{}

# --- For list elements ---

%StepUntilElem{
  start: Expr.t(),
  step:  Expr.t(),
  limit: Expr.t()
}

%WhileElem{
  value:     Expr.t(),
  condition: Expr.t()
}

%SimpleElem{value: Expr.t()}

# --- Expressions ---

%IntLit{value: integer()}
%RealLit{value: float()}
%BoolLit{value: boolean()}
%StringLit{value: String.t()}

%Var{
  name:       String.t(),
  subscripts: [Expr.t()]   # empty list for scalar variables
}

%BinaryOp{
  op:    String.t(),   # "+", "-", "*", "/", "div", "mod", "^", "**",
                       # "=", "!=", "<", "<=", ">", ">=",
                       # "and", "or", "impl", "eqv"
  left:  Expr.t(),
  right: Expr.t()
}

%UnaryOp{
  op:      String.t(),   # "+" | "-" | "not"
  operand: Expr.t()
}

%CondExpr{
  condition: Expr.t(),
  then_expr: Expr.t(),
  else_expr: Expr.t()
}

%ProcCall{
  name: String.t(),
  args: [Expr.t()]
}
```

## Data Flow

```
Input:  List of Token maps (from algol_lexer)
           ↓
        [Grammar-driven parser engine]
           reads algol.grammar
           recursive descent, driven by grammar rules
           resolves ambiguities per spec (dangling else, left-assoc ^)
           ↓
Output: %Program{} AST root node + list of parse errors
```

## Test Strategy

### Minimal programs
```algol
begin integer x; x := 42 end
```
→ `Program(Block(decls: [TypeDecl(:integer, ["x"])], stmts: [AssignStmt([Var("x")], IntLit(42))]))`

### Nested blocks and scoping
```algol
begin
  integer x;
  x := 1;
  begin
    real x;
    x := 3.14
  end
end
```
→ Outer block declares integer x; inner block declares real x (separate nodes, same name).

### All for-loop variants
```algol
for i := 1 step 1 until 10 do x := x + i    (* step/until *)
for i := x while x > 0 do x := x - 1        (* while *)
for i := 1, 3, 7, 10 step 2 until 20 do y   (* mixed list *)
```

### Conditional expression in arithmetic context
```algol
z := if x > 0 then x else -x
```
→ `AssignStmt([Var("z")], CondExpr(relation, Var("x"), UnaryOp("-", Var("x"))))`

### Dangling else — must use begin/end for nesting
```algol
if a then begin if b then s1 else s2 end    (* valid *)
if a then if b then s1 else s2              (* parse error: then-branch is a cond_stmt *)
```

### Procedure declaration with value/name parameters
```algol
real procedure sigma(i, lo, hi, term);
  value lo, hi;
  integer i, lo, hi;
  real term;
begin
  real s;
  s := 0;
  for i := lo step 1 until hi do
    s := s + term;
  sigma := s
end
```
→ `ProcedureDecl(return_type: :real, name: "sigma", params: ["i","lo","hi","term"],
                 by_value: ["lo","hi"], ...)`

### Multiple assignment
```algol
x := y := z := 0
```
→ `AssignStmt(targets: [Var("x"), Var("y"), Var("z")], value: IntLit(0))`

### Array declaration with runtime bounds
```algol
real array A[1:n, 1:m]
```
→ `ArrayDecl(type: :real, segments: [ArraySegment(names: ["A"], bounds: [(IntLit(1), Var("n")), (IntLit(1), Var("m"))])])`

### Switch and computed goto
```algol
switch s := start, middle, finish;
goto s[n]
```
→ `SwitchDecl("s", [label "start", label "middle", label "finish"])`
   `GotoStmt(Var("s", subscripts: [Var("n")]))`

### Boolean operator precedence
```algol
a or b and not c impl d eqv e
```
Should parse as: `(a or (b and (not c))) impl d) eqv e` — reflecting the precedence table
(eqv lowest, then impl, or, and, not highest).

### Exponentiation left-associativity
```algol
2 ^ 3 ^ 4
```
→ `BinaryOp("^", BinaryOp("^", IntLit(2), IntLit(3)), IntLit(4))` — left fold, value = 4096

### Error recovery
- Missing `end`: report error at EOF
- Missing `then` after `if`: report error at unexpected token
- Statement before declaration in block: report error

## Future Extensions

- **ALGOL 68**: A substantially richer type system (structs, unions, references), fully
  orthogonal design. Substantially harder to parse (non-context-free in places).
- **Pretty printer**: AST → formatted ALGOL 60 source. Good for verifying parse correctness.
- **Scope resolver**: Walk the AST after parsing to bind each name reference to its declaration,
  reporting undeclared variable errors. This is the semantic analysis phase.
- **Type checker**: Verify that arithmetic operations receive numeric operands, boolean operations
  receive boolean operands, array subscripts are integers, etc.
- **Call-by-name evaluator**: The interpreter must implement Jensen's Device correctly — re-evaluate
  by-name arguments on every use, maintaining the calling environment.
