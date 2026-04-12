# Versioned Java Grammar System

## Overview

This spec defines a comprehensive grammar system covering every significant Java version
from Java 1.0 (1996) through Java 21 (2023). Each version gets its own self-contained
`.tokens` and `.grammar` file pair. No file inherits from or extends another -- each is
a complete, standalone definition of the language at that point in time.

The grammar files use the existing `.tokens` and `.grammar` formats documented in
`tokens-format.md` and `grammar-format.md`, plus extensions from `lexer-parser-extensions.md`
(syntactic predicates, context keywords, bracket depth tracking).

### Why versioned Java grammars?

Java's evolution is unusual among programming languages. Unlike JavaScript, which bolted on
major features every year after 2015, Java evolved slowly and deliberately through a formal
Java Community Process (JCP) and later through JDK Enhancement Proposals (JEPs). Each
version of the Java Language Specification (JLS) is a precise, thorough document, which
makes versioned grammars especially clean to define.

By giving each version its own grammar, we can:

1. **Parse historically** -- feed a 2004-era Java 5 program through the `java5` grammar
   and get an accurate AST, without worrying about Java 8 lambda ambiguity
2. **Teach evolution** -- read the files in order to see exactly how Java grew from a
   simple OO language into one with generics, lambdas, records, and pattern matching
3. **Test precisely** -- verify that `var x = 10` is a syntax error in Java 8 but valid
   in Java 10, or that `->` in a switch is illegal before Java 14
4. **Build version-aware tooling** -- linters, formatters, and IDEs that know which
   features are available in the target JDK

### Why these versions?

Not every Java release changes the grammar. Java 2, 3, 6, 9, 11, 12, 13, 15, 16, 18, 19,
and 20 either made no syntax changes or only promoted preview features to final (which we
capture at the version where the feature finalized). We include only versions that introduced
new syntax:

- **Java 1.0** -- the foundation
- **Java 1.1** -- inner classes (the biggest pre-generics grammar change)
- **Java 1.4** -- `assert` keyword
- **Java 5** -- generics, enums, annotations, enhanced for, varargs (the "second founding")
- **Java 7** -- diamond, try-with-resources, multi-catch
- **Java 8** -- lambdas, method references, default methods (the "third founding")
- **Java 10** -- local variable type inference (`var`)
- **Java 14** -- records, switch expressions, text blocks
- **Java 17** -- sealed classes, pattern matching for instanceof
- **Java 21** -- record patterns, unnamed patterns, string templates

### Relationship to existing grammars

There are no existing Java grammar files in `code/grammars/`. This spec establishes the
Java grammar system from scratch. The versioned files will live in a `java/` subdirectory
alongside the existing `ecmascript/` and `typescript/` directories.

---

## 1. File Naming Convention

| Version | Year | Tokens File | Grammar File | JLS Edition | Notes |
|---------|------|-------------|--------------|-------------|-------|
| 1.0 | 1996 | `java1.0.tokens` | `java1.0.grammar` | JLS 1 | The foundation. Oak becomes Java. |
| 1.1 | 1997 | `java1.1.tokens` | `java1.1.grammar` | JLS 1 (amended) | Inner classes, the biggest early change. |
| 1.4 | 2002 | `java1.4.tokens` | `java1.4.grammar` | JLS 2 | `assert` keyword. |
| 5 | 2004 | `java5.tokens` | `java5.grammar` | JLS 3 | Generics, enums, annotations. The big one. |
| 7 | 2011 | `java7.tokens` | `java7.grammar` | JLS 7 | Diamond, try-with-resources. |
| 8 | 2014 | `java8.tokens` | `java8.grammar` | JLS 8 | Lambdas, method references. |
| 10 | 2018 | `java10.tokens` | `java10.grammar` | JLS 10 | `var` for local variables. |
| 14 | 2020 | `java14.tokens` | `java14.grammar` | JLS 14 | Records, switch expressions. |
| 17 | 2021 | `java17.tokens` | `java17.grammar` | JLS 17 | Sealed classes, pattern instanceof. |
| 21 | 2023 | `java21.tokens` | `java21.grammar` | JLS 21 | Record patterns, unnamed patterns. |

### Directory Structure

```
code/grammars/
  java/
    java1.0.tokens    java1.0.grammar
    java1.1.tokens    java1.1.grammar
    java1.4.tokens    java1.4.grammar
    java5.tokens      java5.grammar
    java7.tokens      java7.grammar
    java8.tokens      java8.grammar
    java10.tokens     java10.grammar
    java14.tokens     java14.grammar
    java17.tokens     java17.grammar
    java21.tokens     java21.grammar
```

### Magic Comments

Every file includes version metadata:

```
# Java 1.0 lexical grammar
# @version 1
# @java_version 1.0
```

```
# Java 21 parser grammar
# @version 1
# @java_version 21
```

---

## 2. Java Version Feature Inventory

### Java 1.0 (1996) -- The Foundation

Java 1.0 is where it all begins. James Gosling designed Oak (later renamed Java) at Sun
Microsystems for embedded systems, but it found its calling as the language of the early
web via applets. Java 1.0 shipped with a remarkably complete type system: classes, interfaces,
packages, exceptions, threads, and a C-derived expression syntax. Unlike C++, Java was
garbage-collected, had no pointer arithmetic, no multiple inheritance of classes, and no
operator overloading. These deliberate omissions made the grammar simpler and the language
safer.

The JLS 1st edition (Gosling, Joy, Steele, 1996) is the authoritative reference.

#### Tokens

**Identifiers:**

```
NAME = /[a-zA-Z_$][a-zA-Z0-9_$]*/
```

Like JavaScript, Java allows `$` in identifiers. In practice, `$` is used by the compiler
for synthetic names (inner class bridges, lambda desugaring) and is discouraged in user code.
Unlike JavaScript, Java identifiers are Unicode-aware from day one -- the full spec uses
`Character.isJavaIdentifierStart` and `Character.isJavaIdentifierPart` -- but the token
pattern above covers the ASCII-only approximation used by our grammar system.

**Numbers:**

```
INT_LITERAL    = /0[xX][0-9a-fA-F]+[lL]?/           # hex: 0xFF, 0xFFL
               | /0[0-7]+[lL]?/                       # octal: 077, 077L
               | /[0-9]+[lL]?/                         # decimal: 42, 42L
FLOAT_LITERAL  = /[0-9]+\.[0-9]*([eE][+-]?[0-9]+)?[fFdD]?/   # 3.14, 3.14f
               | /\.[0-9]+([eE][+-]?[0-9]+)?[fFdD]?/          # .5, .5d
               | /[0-9]+[eE][+-]?[0-9]+[fFdD]?/               # 1e10, 1e10f
               | /[0-9]+[fFdD]/                                 # 42f, 42d
```

Java distinguishes integer from floating-point literals at the token level. The `L`/`l`
suffix marks `long` literals. The `f`/`F`/`d`/`D` suffixes mark `float` and `double`.
No binary literals (`0b`), no underscores in numbers -- those come in Java 7.

**Characters and Strings:**

```
CHAR_LITERAL   = /'([^'\\]|\\.)'/ -> CHAR
STRING_LITERAL = /"([^"\\]|\\.)*"/ -> STRING
```

Character literals are single-quoted and contain exactly one character or escape sequence.
String literals are double-quoted. Escape sequences: `\\`, `\"`, `\'`, `\n`, `\r`, `\t`,
`\b`, `\f`, `\0`, `\xHH` (not standard -- use `\uHHHH`), `\uHHHH` (Unicode BMP),
and octal escapes `\0` through `\377`. No text blocks -- those arrive in Java 14.

**Boolean and Null Literals:**

```
TRUE_LITERAL  = "true"
FALSE_LITERAL = "false"
NULL_LITERAL  = "null"
```

These are technically keywords that produce literal values, not identifiers.

**Operators (37 total):**

Arithmetic: `+`, `-`, `*`, `/`, `%`
Assignment: `=`, `+=`, `-=`, `*=`, `/=`, `%=`
Bitwise: `&`, `|`, `^`, `~`, `<<`, `>>`, `>>>`
Bitwise assignment: `&=`, `|=`, `^=`, `<<=`, `>>=`, `>>>=`
Comparison: `==`, `!=`, `<`, `>`, `<=`, `>=`
Logical: `&&`, `||`, `!`
Increment/decrement: `++`, `--`
Ternary: `?` (paired with `:`)
Member access: `.`
instanceof: (keyword operator, see keywords)

Java has `>>>` (unsigned right shift) from day one -- a feature C and C++ lack.
Note: NO `->` (lambda arrow), NO `::` (method reference). Those come in Java 8.

**Punctuation:**

```
( ) { } [ ] ; , . : @
```

Wait -- `@` is not in Java 1.0. Annotations come in Java 5. Remove it:

```
( ) { } [ ] ; , . :
```

**Keywords (50):**

```
keywords:
  abstract boolean break byte case catch char class
  continue default do double else extends final finally
  float for if implements import instanceof int interface
  long native new package private protected public return
  short static strictfp super switch synchronized this throw
  throws transient try void volatile while
```

Java reserved far more keywords from the start than JavaScript did. `strictfp` was actually
added in Java 1.2, but we include it here for simplicity since Java 1.2 had no other syntax
changes worth a separate grammar version. `const` and `goto` are reserved but never used.

**Reserved words (unused):**

```
reserved:
  const goto
```

These are reserved to produce better error messages if C/C++ programmers try to use them.

**Skip patterns:**

```
skip:
  WHITESPACE    = /[ \t\r\n]+/
  LINE_COMMENT  = /\/\/[^\n]*/
  BLOCK_COMMENT = /\/\*([^*]|\*[^\/])*\*\//
```

Java also has Javadoc comments (`/** ... */`) but they are lexically identical to block
comments -- the distinction is semantic, handled by documentation tools, not the parser.

#### Grammar Rules

```
# A Java 1.0 compilation unit is a single source file. It optionally declares
# a package, optionally imports other packages/classes, and then contains zero
# or more type declarations (classes or interfaces).

compilation_unit = [ package_declaration ] { import_declaration } { type_declaration } ;

package_declaration = "package" qualified_name SEMICOLON ;

import_declaration = "import" qualified_name [ DOT STAR ] SEMICOLON ;

qualified_name = NAME { DOT NAME } ;

type_declaration = class_declaration
                 | interface_declaration
                 | SEMICOLON ;

# --- Class Declarations ---
#
# A class can be abstract or final (but not both), and has an optional
# superclass (single inheritance) and optional interface list.

class_declaration = { class_modifier } "class" NAME
                    [ "extends" qualified_name ]
                    [ "implements" qualified_name_list ]
                    class_body ;

class_modifier = "public" | "abstract" | "final"
               | "strictfp" ;

qualified_name_list = qualified_name { COMMA qualified_name } ;

class_body = LBRACE { class_body_declaration } RBRACE ;

class_body_declaration = field_declaration
                       | method_declaration
                       | constructor_declaration
                       | static_initializer
                       | SEMICOLON ;

# --- Fields ---

field_declaration = { field_modifier } type variable_declarators SEMICOLON ;

field_modifier = "public" | "protected" | "private"
               | "static" | "final" | "transient" | "volatile" ;

variable_declarators = variable_declarator { COMMA variable_declarator } ;

variable_declarator = NAME { LBRACKET RBRACKET } [ EQUALS variable_initializer ] ;

variable_initializer = expression
                     | array_initializer ;

array_initializer = LBRACE [ variable_initializer { COMMA variable_initializer } [ COMMA ] ] RBRACE ;

# --- Methods ---

method_declaration = { method_modifier } result_type NAME
                     LPAREN [ formal_parameter_list ] RPAREN { LBRACKET RBRACKET }
                     [ "throws" qualified_name_list ]
                     ( block | SEMICOLON ) ;

method_modifier = "public" | "protected" | "private"
                | "static" | "abstract" | "final"
                | "native" | "synchronized" | "strictfp" ;

result_type = type | "void" ;

formal_parameter_list = formal_parameter { COMMA formal_parameter } ;

formal_parameter = [ "final" ] type NAME { LBRACKET RBRACKET } ;

# --- Constructors ---

constructor_declaration = { constructor_modifier } NAME
                          LPAREN [ formal_parameter_list ] RPAREN
                          [ "throws" qualified_name_list ]
                          constructor_body ;

constructor_modifier = "public" | "protected" | "private" ;

constructor_body = LBRACE [ explicit_constructor_invocation ] { block_statement } RBRACE ;

explicit_constructor_invocation = "this" arguments SEMICOLON
                                | "super" arguments SEMICOLON ;

# --- Interfaces ---

interface_declaration = { interface_modifier } "interface" NAME
                        [ "extends" qualified_name_list ]
                        interface_body ;

interface_modifier = "public" | "abstract" | "strictfp" ;

interface_body = LBRACE { interface_member_declaration } RBRACE ;

interface_member_declaration = constant_declaration
                            | abstract_method_declaration
                            | SEMICOLON ;

constant_declaration = { constant_modifier } type variable_declarators SEMICOLON ;

constant_modifier = "public" | "static" | "final" ;

abstract_method_declaration = { abstract_method_modifier } result_type NAME
                              LPAREN [ formal_parameter_list ] RPAREN { LBRACKET RBRACKET }
                              [ "throws" qualified_name_list ] SEMICOLON ;

abstract_method_modifier = "public" | "abstract" ;

# --- Static Initializer ---

static_initializer = "static" block ;

# --- Types ---
#
# Java's type system is split between primitive types (value types on the stack)
# and reference types (objects on the heap). Arrays can be of any type.

type = primitive_type { LBRACKET RBRACKET }
     | qualified_name { LBRACKET RBRACKET } ;

primitive_type = "boolean" | "byte" | "char" | "short"
               | "int" | "long" | "float" | "double" ;

# --- Statements ---

block = LBRACE { block_statement } RBRACE ;

block_statement = local_variable_declaration SEMICOLON
               | statement ;

local_variable_declaration = [ "final" ] type variable_declarators ;

statement = block
          | empty_statement
          | expression_statement
          | if_statement
          | while_statement
          | do_while_statement
          | for_statement
          | break_statement
          | continue_statement
          | return_statement
          | throw_statement
          | synchronized_statement
          | try_statement
          | switch_statement
          | labelled_statement ;

empty_statement = SEMICOLON ;

expression_statement = expression SEMICOLON ;

if_statement = "if" LPAREN expression RPAREN statement [ "else" statement ] ;

while_statement = "while" LPAREN expression RPAREN statement ;

do_while_statement = "do" statement "while" LPAREN expression RPAREN SEMICOLON ;

for_statement = "for" LPAREN
                  [ for_init ] SEMICOLON
                  [ expression ] SEMICOLON
                  [ expression_list ]
                RPAREN statement ;

for_init = local_variable_declaration | expression_list ;

expression_list = expression { COMMA expression } ;

break_statement = "break" [ NAME ] SEMICOLON ;

continue_statement = "continue" [ NAME ] SEMICOLON ;

return_statement = "return" [ expression ] SEMICOLON ;

throw_statement = "throw" expression SEMICOLON ;

synchronized_statement = "synchronized" LPAREN expression RPAREN block ;

try_statement = "try" block { catch_clause } [ finally_clause ]
              | "try" block finally_clause ;

catch_clause = "catch" LPAREN formal_parameter RPAREN block ;

finally_clause = "finally" block ;

switch_statement = "switch" LPAREN expression RPAREN
                   LBRACE { switch_block_group } RBRACE ;

switch_block_group = { switch_label } { block_statement } ;

switch_label = "case" expression COLON
             | "default" COLON ;

labelled_statement = NAME COLON statement ;

# --- Expressions ---
#
# Operator precedence (lowest to highest):
#   assignment < conditional < logical_or < logical_and
#   < bitwise_or < bitwise_xor < bitwise_and < equality
#   < relational < shift < additive < multiplicative
#   < unary < postfix < primary

expression = assignment_expression ;

assignment_expression = conditional_expression
                      | unary_expression assignment_operator assignment_expression ;

assignment_operator = EQUALS | PLUS_EQUALS | MINUS_EQUALS | STAR_EQUALS
                    | SLASH_EQUALS | PERCENT_EQUALS | AMPERSAND_EQUALS
                    | PIPE_EQUALS | CARET_EQUALS | LEFT_SHIFT_EQUALS
                    | RIGHT_SHIFT_EQUALS | UNSIGNED_RIGHT_SHIFT_EQUALS ;

conditional_expression = logical_or_expression
                         [ QUESTION expression COLON conditional_expression ] ;

logical_or_expression = logical_and_expression { OR_OR logical_and_expression } ;

logical_and_expression = bitwise_or_expression { AND_AND bitwise_or_expression } ;

bitwise_or_expression = bitwise_xor_expression { PIPE bitwise_xor_expression } ;

bitwise_xor_expression = bitwise_and_expression { CARET bitwise_and_expression } ;

bitwise_and_expression = equality_expression { AMPERSAND equality_expression } ;

equality_expression = relational_expression { ( EQUALS_EQUALS | NOT_EQUALS ) relational_expression } ;

relational_expression = shift_expression { ( LESS_THAN | GREATER_THAN | LESS_EQUALS
                        | GREATER_EQUALS | "instanceof" ) shift_expression } ;

shift_expression = additive_expression { ( LEFT_SHIFT | RIGHT_SHIFT
                   | UNSIGNED_RIGHT_SHIFT ) additive_expression } ;

additive_expression = multiplicative_expression { ( PLUS | MINUS ) multiplicative_expression } ;

multiplicative_expression = unary_expression { ( STAR | SLASH | PERCENT ) unary_expression } ;

unary_expression = PLUS_PLUS unary_expression
                 | MINUS_MINUS unary_expression
                 | PLUS unary_expression
                 | MINUS unary_expression
                 | unary_expression_not_plus_minus ;

unary_expression_not_plus_minus = TILDE unary_expression
                                | BANG unary_expression
                                | cast_expression
                                | postfix_expression ;

cast_expression = LPAREN primitive_type RPAREN unary_expression
                | LPAREN qualified_name RPAREN unary_expression_not_plus_minus ;

postfix_expression = primary { DOT NAME | DOT "this" | DOT "class"
                     | DOT "new" inner_class_creation
                     | LBRACKET expression RBRACKET
                     | arguments }
                     [ PLUS_PLUS | MINUS_MINUS ] ;

# Wait -- DOT "class" and inner class creation are Java 1.1 features.
# For Java 1.0, postfix_expression is simpler:

postfix_expression = primary { DOT NAME
                     | LBRACKET expression RBRACKET
                     | arguments }
                     [ PLUS_PLUS | MINUS_MINUS ] ;

primary = NAME
        | "this"
        | "super" DOT NAME
        | "super" arguments
        | literal
        | "new" class_instance_creation
        | "new" array_creation
        | LPAREN expression RPAREN ;

literal = INT_LITERAL | FLOAT_LITERAL | CHAR_LITERAL
        | STRING_LITERAL | TRUE_LITERAL | FALSE_LITERAL | NULL_LITERAL ;

class_instance_creation = qualified_name arguments [ class_body ] ;
# Wait -- anonymous class bodies are Java 1.1. For 1.0:
class_instance_creation = qualified_name arguments ;

array_creation = type dimension_expressions { LBRACKET RBRACKET }
               | type { LBRACKET RBRACKET } array_initializer ;

dimension_expressions = LBRACKET expression RBRACKET { LBRACKET expression RBRACKET } ;

arguments = LPAREN [ expression { COMMA expression } ] RPAREN ;
```

**What Java 1.0 does NOT have:**

- No inner classes, anonymous classes, or local classes
- No `assert` keyword
- No generics (`<T>`)
- No enums, annotations, or enhanced for-each
- No varargs (`...`)
- No autoboxing
- No lambda expressions or method references
- No `var` keyword
- No records, sealed classes, or pattern matching
- No text blocks or string templates
- No binary literals or underscores in numbers
- No try-with-resources or multi-catch
- No diamond operator (`<>`)
- No switch expressions

---

### Java 1.1 (1997) -- Inner Classes

Java 1.1 introduced inner classes, and this was the single biggest grammar change in Java's
first decade. Inner classes let you nest class definitions inside other classes, methods, and
even expressions (anonymous classes). This addition dramatically changed how Java code was
structured -- listeners, callbacks, and iterators all used anonymous inner classes heavily
until lambdas arrived 17 years later in Java 8.

The grammar changes are substantial because inner classes interact with many existing rules:
instance creation, member access, constructor invocation, and name resolution.

Reference: JLS 1st edition, inner classes addendum (1997).

#### Tokens Added (delta from Java 1.0)

No new token types. The lexer is unchanged.

#### Grammar Rules Added

```
# Member class declaration (a class declared inside another class)
# This is just a class_declaration appearing as a class_body_declaration.
# The grammar change is in class_body_declaration gaining class_declaration
# and interface_declaration as alternatives.

# Local class declaration (a class declared inside a method body)
# This is a class_declaration appearing as a block_statement.

# Anonymous class (a class body attached to a new-expression)
# class_instance_creation gains an optional class_body.

# Instance initializer (a block that runs when an object is created)
instance_initializer = block ;
```

#### Grammar Rules Changed

- `class_body_declaration` gains `class_declaration` and `interface_declaration` alternatives
  (member classes and interfaces)
- `class_body_declaration` gains `instance_initializer` (bare block in class body)
- `block_statement` gains `class_declaration` alternative (local classes)
- `class_instance_creation` gains optional `class_body` at the end (anonymous classes)
- `postfix_expression` gains `DOT "class"` for class literals (`String.class`)
- `postfix_expression` gains `DOT "new"` for qualified inner class creation
  (`outer.new Inner()`)
- `explicit_constructor_invocation` gains qualified forms:
  `primary DOT "super" arguments SEMICOLON` and
  `primary DOT "this" arguments SEMICOLON`
- `primary` gains `NAME DOT "this"` for qualified `this` (`Outer.this`)

**Why this matters:** Inner classes made Java viable for event-driven GUI programming (AWT,
Swing). Without them, every button listener required a separate top-level class. Anonymous
classes became so common that they were the primary motivation for adding lambdas in Java 8.

---

### Java 1.4 (2002) -- Assert

Java 1.4 added exactly one syntactic feature: the `assert` statement. This is the smallest
grammar delta in Java's history, but it's worth its own version because `assert` is a
reserved keyword that changes the lexer.

Reference: JLS 2nd edition (Gosling, Joy, Steele, Bracha, 2000), updated for 1.4.

#### Tokens Added (delta from Java 1.1)

**New keyword:**

```
assert
```

This is a full keyword, not a context keyword. Code that used `assert` as a variable name
(rare but possible) broke. The compiler provided a `-source 1.3` flag to maintain backward
compatibility.

#### Grammar Rules Added

```
assert_statement = "assert" expression [ COLON expression ] SEMICOLON ;
```

The first expression must be boolean. The optional second expression (after `:`) provides
a detail message for `AssertionError`. Example: `assert x > 0 : "x must be positive";`

#### Grammar Rules Changed

- `statement` gains `assert_statement` alternative

**What Java 1.4 still does NOT have:**

- No generics, enums, or annotations (Java 5)
- No enhanced for-each (Java 5)
- No varargs (Java 5)
- No autoboxing syntax changes (Java 5 -- semantics only, not grammar)

---

### Java 5 (2004) -- The Second Founding

Java 5 (originally "Java 1.5" or "J2SE 5.0") is the most transformative Java release. It
introduced generics, enums, annotations, enhanced for-each, varargs, static imports, and
autoboxing. The grammar nearly doubled in complexity. If Java 1.0 was the founding of the
language, Java 5 was its reinvention.

Every modern Java program uses Java 5 features. Generics alone changed how every collection,
every API, and every library was designed. Annotations spawned an entire ecosystem of
frameworks (Spring, JPA, JUnit). Enums replaced the "int constant" antipattern. The
enhanced for-each loop eliminated an entire class of off-by-one errors.

Reference: JLS 3rd edition (Gosling, Joy, Steele, Bracha, 2005).

#### Tokens Added (delta from Java 1.4)

**New operators/punctuation:**

```
AT        = "@"        # annotation prefix
ELLIPSIS  = "..."      # varargs
```

`@` is now a meaningful token. In earlier versions it had no syntactic role.

**New keyword:**

```
enum
```

`enum` was previously a reserved word in some contexts but is now a full keyword.

**Context keywords (identifiers with special meaning in certain positions):**

None formally added to the keyword list, but `@interface` uses `interface` in a new context.

**No new literal formats.** Autoboxing is purely semantic -- `Integer x = 42` uses the same
`INT_LITERAL` token; the compiler inserts `Integer.valueOf()`.

#### Grammar Rules Added

**Generics (type parameters and type arguments):**

```
# Type parameters appear on class, interface, and method declarations
type_parameters = LESS_THAN type_parameter { COMMA type_parameter } GREATER_THAN ;

type_parameter = NAME [ "extends" type_bound ] ;

type_bound = qualified_name { AMPERSAND qualified_name } ;

# Type arguments appear on type references
type_arguments = LESS_THAN type_argument { COMMA type_argument } GREATER_THAN ;

type_argument = type
              | QUESTION [ ( "extends" | "super" ) type ] ;
```

Generics are the most complex grammar addition. The `<` and `>` tokens are overloaded --
they serve as both comparison operators and generic brackets. Disambiguation requires
context-sensitive parsing. The classic example: `List<List<String>>` -- the `>>` must be
parsed as two closing angle brackets, not a right-shift operator. Our grammar handles this
with bracket depth tracking (Extension 2).

**Enums:**

```
enum_declaration = { class_modifier } "enum" NAME
                   [ "implements" qualified_name_list ]
                   enum_body ;

enum_body = LBRACE [ enum_constant { COMMA enum_constant } [ COMMA ] ]
            [ SEMICOLON { class_body_declaration } ] RBRACE ;

enum_constant = { annotation } NAME [ arguments ] [ class_body ] ;
```

Enums can have constructors, methods, fields, and even per-constant anonymous class bodies.
They're far more powerful than C enums.

**Annotations:**

```
annotation = AT qualified_name [ LPAREN [ annotation_element ] RPAREN ] ;

annotation_element = element_value
                   | { element_value_pair { COMMA element_value_pair } } ;

element_value_pair = NAME EQUALS element_value ;

element_value = expression
              | annotation
              | element_value_array ;

element_value_array = LBRACE [ element_value { COMMA element_value } [ COMMA ] ] RBRACE ;

# Annotation type declaration
annotation_type_declaration = { class_modifier } AT "interface" NAME
                              annotation_type_body ;

annotation_type_body = LBRACE { annotation_type_member } RBRACE ;

annotation_type_member = annotation_method_declaration
                       | constant_declaration
                       | class_declaration
                       | interface_declaration
                       | enum_declaration
                       | annotation_type_declaration
                       | SEMICOLON ;

annotation_method_declaration = { method_modifier } type NAME LPAREN RPAREN
                                [ "default" element_value ] SEMICOLON ;
```

**Enhanced for-each:**

```
enhanced_for_statement = "for" LPAREN [ "final" ] type NAME COLON expression RPAREN statement ;
```

This is the `for (String s : list)` syntax. It compiles to an iterator loop but is far
more readable.

**Varargs:**

```
# The last formal parameter can use ... to accept variable arguments
varargs_parameter = [ "final" ] type ELLIPSIS NAME ;
```

`formal_parameter_list` is modified so the last parameter can be a `varargs_parameter`.

**Static imports:**

```
static_import_declaration = "import" "static" qualified_name [ DOT STAR ] SEMICOLON ;
```

#### Grammar Rules Changed

- `type` gains `type_arguments` after `qualified_name` (parameterized types: `List<String>`)
- `type_declaration` gains `enum_declaration` and `annotation_type_declaration`
- `class_declaration` gains optional `type_parameters` after `NAME`
- `interface_declaration` gains optional `type_parameters` after `NAME`
- `method_declaration` gains optional `type_parameters` before `result_type` (generic methods)
- `constructor_declaration` gains optional `type_parameters`
- `formal_parameter` can have annotations
- `import_declaration` is split into regular and static imports
- `for_statement` is augmented with the enhanced for-each variant
- `statement` gains `assert_statement` (already present from 1.4, carried forward)
- `class_modifier` gains annotations as a modifier position

**Why this matters:** Before Java 5, every collection stored `Object` and required casting.
`List list = new ArrayList(); String s = (String) list.get(0);` became
`List<String> list = new ArrayList<String>(); String s = list.get(0);`. This alone probably
prevented millions of `ClassCastException`s.

---

### Java 7 (2011) -- Project Coin

Java 7 was part of "Project Coin" -- small language changes that added up to significantly
better ergonomics. The diamond operator reduced generics verbosity, try-with-resources
eliminated resource leak bugs, multi-catch reduced boilerplate in exception handling, and
new numeric literal formats improved readability.

Reference: JLS 7 (Gosling, Joy, Steele, Bracha, Buckley, 2011).

#### Tokens Added (delta from Java 5)

**Binary integer literals:**

```
BINARY_INT_LITERAL = /0[bB][01]([01_]*[01])?[lL]?/
```

Example: `int mask = 0b1010_1010;` -- much clearer than hex for bit patterns.

**Underscores in numeric literals:**

```
INT_LITERAL   = /[0-9]([0-9_]*[0-9])?[lL]?/            # 1_000_000
              | /0[xX][0-9a-fA-F]([0-9a-fA-F_]*[0-9a-fA-F])?[lL]?/  # 0xFF_FF
              | /0[bB][01]([01_]*[01])?[lL]?/            # 0b1010_0101
FLOAT_LITERAL = # ... similar patterns with underscores allowed between digits
```

Underscores can appear between digits (not at start, end, or adjacent to `.`, `x`, `b`,
`e`, `L`, `f`, etc.). This makes large numbers readable: `1_000_000` instead of `1000000`.

**Strings in switch (semantic, not syntactic):** The `switch` grammar doesn't change --
the expression in `switch(expr)` could always be any expression. Java 7 widens the set of
types the compiler accepts (adding `String`), but the grammar is unchanged.

**Diamond operator:** Not a new token -- it's `LESS_THAN GREATER_THAN` parsed in type
argument position. The grammar change is that `type_arguments` can be empty: `<>`.

#### Grammar Rules Added

**Try-with-resources:**

```
try_with_resources = "try" resource_specification block { catch_clause } [ finally_clause ] ;

resource_specification = LPAREN resource { SEMICOLON resource } [ SEMICOLON ] RPAREN ;

resource = [ "final" ] type NAME EQUALS expression ;
```

This is the `try (InputStream is = new FileInputStream("f"))` syntax. Resources that
implement `AutoCloseable` are automatically closed at the end of the try block, even if
an exception is thrown. This eliminated one of Java's most common bug patterns: forgetting
to close resources in a `finally` block.

**Multi-catch:**

```
catch_clause = "catch" LPAREN catch_type NAME RPAREN block ;

catch_type = qualified_name { PIPE qualified_name } ;
```

Example: `catch (IOException | SQLException e)`. The caught variable is effectively `final`.

#### Grammar Rules Changed

- `type_arguments` gains the empty diamond `<>` case (for `new ArrayList<>()`)
- `try_statement` gains `try_with_resources` as an alternative form
- `catch_clause` changes `formal_parameter` to `catch_type NAME` (multi-catch with `|`)

**What Java 7 still does NOT have:**

- No lambda expressions or method references (Java 8)
- No default or static methods in interfaces (Java 8)
- No type annotations (Java 8)
- No `var` keyword (Java 10)
- No records, switch expressions, or text blocks (Java 14)

---

### Java 8 (2014) -- Lambdas and Functional Interfaces

Java 8 is the third transformative release after 1.0 and 5. Lambda expressions and method
references brought functional programming to Java, ending the 17-year reign of anonymous
inner classes as the primary callback mechanism. Default methods in interfaces enabled
API evolution without breaking existing implementations -- a critical capability for the
new `java.util.stream` API.

This release changed not just the grammar but the entire way Java developers think about
code. The `Stream` API, while not a grammar feature, was only possible because of lambdas.

Reference: JLS 8 (Gosling, Joy, Steele, Bracha, Buckley, Smith, 2014).

#### Tokens Added (delta from Java 7)

**New operators:**

```
ARROW          = "->"     # lambda arrow
DOUBLE_COLON   = "::"     # method reference
```

These two operators are the syntactic heart of Java 8. The arrow `->` introduces lambda
bodies. The double colon `::` creates method references.

**No new keywords.** `default` was already a keyword (for switch). Java 8 reuses it in
interface method declarations. This is an example of context-sensitive keyword reuse.

#### Grammar Rules Added

**Lambda expressions:**

```
lambda_expression = lambda_parameters ARROW lambda_body ;

lambda_parameters = NAME
                  | LPAREN [ formal_parameter_list ] RPAREN
                  | LPAREN inferred_parameter_list RPAREN ;

inferred_parameter_list = NAME { COMMA NAME } ;

lambda_body = expression
            | block ;
```

Lambda syntax has three parameter forms:
- Single untyped parameter without parens: `x -> x + 1`
- Typed parameters in parens: `(int x, int y) -> x + y`
- Inferred parameters in parens: `(x, y) -> x + y`

The body can be a single expression (implicit return) or a block (explicit return needed).

**Method references:**

```
method_reference = qualified_name DOUBLE_COLON [ type_arguments ] NAME
                 | primary DOUBLE_COLON [ type_arguments ] NAME
                 | qualified_name DOUBLE_COLON [ type_arguments ] "new"
                 | type DOUBLE_COLON [ type_arguments ] "new"
                 | "super" DOUBLE_COLON [ type_arguments ] NAME ;
```

Four kinds of method references:
- `String::length` -- reference to an instance method via type
- `System.out::println` -- reference to an instance method via expression
- `String::new` -- reference to a constructor
- `super::toString` -- reference to a superclass method

**Default and static interface methods:**

```
default_method_declaration = { method_modifier } "default" result_type NAME
                             LPAREN [ formal_parameter_list ] RPAREN
                             [ "throws" qualified_name_list ]
                             block ;

static_interface_method = { method_modifier } "static" result_type NAME
                          LPAREN [ formal_parameter_list ] RPAREN
                          [ "throws" qualified_name_list ]
                          block ;
```

**Type annotations (JSR 308):**

```
# Annotations can now appear on any type use, not just declarations:
#   @NonNull String s;
#   List<@NonNull String> list;
#   String @NonNull [] array;
#   (@NonNull String) obj;
```

The grammar change is that `annotation` can appear before any type reference, including
in generics, array types, casts, and `instanceof` expressions.

#### Grammar Rules Changed

- `assignment_expression` gains `lambda_expression` as an alternative
- `primary` gains `method_reference` as an alternative
- `interface_member_declaration` gains `default_method_declaration` and
  `static_interface_method`
- `type` gains annotation positions throughout (type annotations)
- `cast_expression` gains annotation positions
- `class_instance_creation` gains annotation positions on type arguments

**Why this matters:** Before Java 8, sorting a list of strings by length required:
```java
Collections.sort(list, new Comparator<String>() {
    public int compare(String a, String b) {
        return Integer.compare(a.length(), b.length());
    }
});
```
After Java 8: `list.sort(Comparator.comparingInt(String::length));`

---

### Java 10 (2018) -- Local Variable Type Inference

Java 10 added one syntactic feature: the `var` keyword for local variable type inference.
This is the smallest grammar change since `assert` in Java 1.4, but its impact on
day-to-day coding is significant.

`var` is a **context keyword** (technically a "reserved type name"), not a full keyword.
You can still use `var` as a variable name, method name, or package name -- it only has
special meaning when it appears as the type in a local variable declaration. This was a
deliberate design choice to avoid breaking existing code that used `var` as an identifier
(which was common in some codebases).

Reference: JLS 10, JEP 286.

#### Tokens Added (delta from Java 8)

**Context keyword:**

```
context_keywords:
  var
```

`var` is NOT a reserved keyword. It is recognized as a type name only in local variable
declarations (including in `for` loop initializers and `try-with-resources`). Everywhere
else, `var` is a normal identifier.

#### Grammar Rules Added

No new rules. The change is to existing rules.

#### Grammar Rules Changed

- `local_variable_declaration` gains `"var"` as an alternative to an explicit type:
  `"var" variable_declarators` (initializer is required when using `var`)
- `enhanced_for_statement` gains `"var"` alternative:
  `"for" LPAREN "var" NAME COLON expression RPAREN statement`
- `resource` in try-with-resources gains `"var"` alternative:
  `"var" NAME EQUALS expression`

**What `var` does NOT do:**

- Does not work for fields, method parameters, or return types
- Does not work without an initializer (`var x;` is illegal)
- Does not work with `null` initializer (`var x = null;` is illegal)
- Does not work with array initializers (`var x = {1, 2};` is illegal)
- Does not work with lambda expressions (`var f = x -> x;` is illegal -- target type needed)

**What Java 10 still does NOT have:**

- No records (Java 14)
- No switch expressions (Java 14)
- No text blocks (Java 14)
- No sealed classes (Java 17)
- No pattern matching (Java 17/21)

---

### Java 14 (2020) -- Records, Switch Expressions, and Text Blocks

Java 14 is the most grammar-heavy release since Java 5. It finalizes three major features
that had been in preview: records (compact data classes), switch expressions (switch that
returns a value), and text blocks (multi-line string literals). Each one addresses a
long-standing pain point.

Records eliminate the boilerplate of simple data carriers -- no more writing `equals()`,
`hashCode()`, `toString()`, getters, and constructors for every DTO. Switch expressions
make switch usable in expression context and eliminate fall-through bugs. Text blocks make
multi-line strings (JSON, SQL, HTML) readable.

Reference: JLS 14, JEP 359 (records), JEP 361 (switch expressions), JEP 378 (text blocks).

#### Tokens Added (delta from Java 10)

**Text block literal:**

```
TEXT_BLOCK = /"""[ \t]*\n([^"\\]|\\.|\"{1,2}[^"])*"""/ -> STRING
```

Text blocks start with `"""` followed by optional whitespace and a mandatory newline. They
end with `"""`. The opening `"""` must be on its own line (no content on the same line).
Common leading whitespace is stripped (incidental whitespace removal).

Example:
```java
String json = """
              {
                  "name": "Java",
                  "version": 14
              }
              """;
```

**New keywords:**

```
record
yield
```

`record` is a context keyword (restricted identifier) -- like `var`, it can still be used
as a variable name. `yield` is also a context keyword, meaningful only inside switch
expressions.

#### Grammar Rules Added

**Records:**

```
record_declaration = { class_modifier } "record" NAME [ type_parameters ]
                     record_header
                     [ "implements" qualified_name_list ]
                     record_body ;

record_header = LPAREN [ record_component_list ] RPAREN ;

record_component_list = record_component { COMMA record_component } ;

record_component = { annotation } type NAME ;

record_body = LBRACE { record_body_declaration } RBRACE ;

record_body_declaration = compact_constructor_declaration
                        | class_body_declaration ;

compact_constructor_declaration = { constructor_modifier } NAME
                                  constructor_body ;
```

Records automatically generate: a constructor matching the component list, accessor methods
for each component (e.g., `name()` not `getName()`), `equals()`, `hashCode()`, and
`toString()`. The compact constructor lets you validate and normalize without repeating
the parameter list.

**Switch expressions:**

```
switch_expression = "switch" LPAREN expression RPAREN
                    LBRACE { switch_expression_rule } RBRACE ;

switch_expression_rule = switch_label_list ARROW ( expression SEMICOLON
                                                 | block
                                                 | "throw" expression SEMICOLON ) ;

switch_label_list = switch_label { COMMA switch_label } ;

# yield statement (only valid inside switch expressions)
yield_statement = "yield" expression SEMICOLON ;
```

Switch expressions use `->` instead of `:` and don't fall through. They can be used
anywhere an expression is expected: `int result = switch(x) { case 1 -> 10; ... };`

The `yield` keyword is used to return a value from a switch expression block:
```java
int result = switch(x) {
    case 1 -> 10;
    case 2 -> {
        int temp = compute();
        yield temp * 2;
    }
    default -> 0;
};
```

#### Grammar Rules Changed

- `type_declaration` gains `record_declaration`
- `class_body_declaration` gains `record_declaration` (nested records)
- `primary` gains `switch_expression` as an alternative
- `switch_label` gains comma-separated multiple values: `case 1, 2, 3 ->`
- `statement` gains `yield_statement`
- `switch_statement` gains arrow-form switch rules (switch statements can also use `->`)
- `primary` gains `TEXT_BLOCK` as a literal alternative

---

### Java 17 (2021) -- Sealed Classes and Pattern Matching for instanceof

Java 17 is an LTS (Long-Term Support) release that finalizes sealed classes and pattern
matching for `instanceof`. Sealed classes restrict which classes can extend or implement
a type, creating closed hierarchies that the compiler can reason about exhaustively. Pattern
matching for `instanceof` eliminates the ubiquitous cast-after-check pattern.

Together, these features lay the groundwork for Java's ongoing algebraic data types story:
sealed interfaces define the shape, records define the data, and pattern matching
destructures it.

Reference: JLS 17, JEP 409 (sealed classes), JEP 394 (pattern matching for instanceof).

#### Tokens Added (delta from Java 14)

**New context keywords:**

```
context_keywords:
  sealed
  permits
  non-sealed
```

`sealed` and `permits` are context keywords (restricted identifiers). `non-sealed` is
unusual -- it's the only hyphenated keyword in Java. It's recognized as a single token
in modifier position. Technically, `non` and `sealed` are both identifiers, and `non-sealed`
is parsed as a modifier only in class/interface declaration position.

```
NON_SEALED = "non" MINUS "sealed"   # special composite token in modifier context
```

This requires context-sensitive lexing: the three-token sequence `non - sealed` is collapsed
into a single modifier only when it appears where a class modifier is expected.

#### Grammar Rules Added

**Sealed class/interface declarations:**

```
# sealed modifier is added to class_modifier and interface_modifier
# permits clause follows implements/extends

sealed_class_declaration = { class_modifier } "sealed" "class" NAME
                           [ type_parameters ]
                           [ "extends" qualified_name ]
                           [ "implements" qualified_name_list ]
                           [ "permits" qualified_name_list ]
                           class_body ;

# non-sealed modifier for subclasses that open the hierarchy back up
# "non-sealed" is a modifier, not a separate declaration form
```

**Pattern matching for instanceof:**

```
# instanceof gains a pattern operand
instanceof_expression = relational_expression "instanceof" type NAME ;
```

The pattern `type NAME` both tests the type and binds the casted variable. Example:
```java
if (obj instanceof String s) {
    System.out.println(s.length());  // s is already a String, no cast needed
}
```

The scope of the pattern variable follows "flow scoping" -- `s` is in scope only where
the compiler can prove the pattern matched. This includes the then-branch of `if`, and
the right operand of `&&`.

#### Grammar Rules Changed

- `class_modifier` gains `"sealed"` and `"non-sealed"`
- `interface_modifier` gains `"sealed"` and `"non-sealed"`
- `class_declaration` gains optional `"permits" qualified_name_list` clause
- `interface_declaration` gains optional `"permits" qualified_name_list` clause
- `relational_expression` gains `instanceof type NAME` alternative (pattern instanceof)

**What Java 17 still does NOT have:**

- No record patterns or destructuring (Java 21)
- No unnamed patterns (Java 21)
- No string templates (Java 21)
- No pattern matching in switch (Java 21)
- No unnamed variables (Java 21)

---

### Java 21 (2023) -- Record Patterns, Unnamed Patterns, and Switch Pattern Matching

Java 21 is an LTS release that brings Java's pattern matching story to maturity. Record
patterns allow destructuring records in `instanceof` and `switch`. Unnamed patterns (`_`)
let you ignore components you don't care about. Switch pattern matching lets you match on
types and destructure in switch cases. Unnamed variables (`_`) reduce noise when a variable
is required by syntax but unused.

These features complete the algebraic data types story that began with sealed classes in
Java 17 and records in Java 14: define closed hierarchies with `sealed`, carry data with
`record`, and destructure with pattern matching.

Reference: JLS 21, JEP 440 (record patterns), JEP 441 (pattern matching for switch),
JEP 443 (unnamed patterns and variables), JEP 430 (string templates -- preview).

#### Tokens Added (delta from Java 17)

**Unnamed pattern/variable token:**

```
# _ as an unnamed pattern/variable (context-sensitive)
# In Java 21, _ is no longer a valid identifier name.
# It is reserved as a special "discard" pattern.
UNDERSCORE = "_"
```

Prior to Java 21, `_` was a legal (though discouraged since Java 9) identifier. Java 21
makes it a reserved keyword in the specific contexts where patterns and variable
declarations appear.

**String template tokens (preview in Java 21):**

```
# String templates use a processor prefix and embedded expressions:
#   STR."Hello, \{name}!"
# The \{ } delimiters are new to the lexer.
STRING_TEMPLATE_BEGIN = /"([^"\\]|\\.)*\\{/
STRING_TEMPLATE_MID   = /}([^"\\]|\\.)*\\{/
STRING_TEMPLATE_END   = /}([^"\\]|\\.)*"/
```

Note: String templates were preview in Java 21 and were later withdrawn. We include them
here because they were part of the Java 21 grammar specification, but they may not appear
in the final grammar files if we choose to track only finalized features.

#### Grammar Rules Added

**Record patterns:**

```
record_pattern = qualified_name [ type_arguments ]
                 LPAREN [ pattern_list ] RPAREN ;

pattern_list = pattern { COMMA pattern } ;

pattern = type_pattern
        | record_pattern
        | unnamed_pattern ;

type_pattern = type NAME ;

unnamed_pattern = UNDERSCORE ;
```

Record patterns allow recursive destructuring:
```java
if (obj instanceof Point(int x, int y)) {
    // x and y are bound directly
}

// Nested destructuring:
if (obj instanceof Line(Point(var x1, var y1), Point(var x2, var y2))) {
    // all four coordinates are bound
}
```

**Pattern matching in switch:**

```
# switch cases can now contain patterns
switch_pattern_label = "case" pattern [ guard ] ;

guard = "when" expression ;
```

Example:
```java
String describe(Object obj) {
    return switch (obj) {
        case Integer i when i > 0 -> "positive integer: " + i;
        case Integer i            -> "non-positive integer: " + i;
        case String s             -> "string of length " + s.length();
        case null                 -> "null";
        default                   -> "something else";
    };
}
```

The `when` keyword is a context keyword that introduces a guard clause on a pattern case.

**Unnamed variables:**

```
# _ can appear wherever a variable declaration requires a name but the value is unused
unnamed_variable = UNDERSCORE ;
```

Examples:
```java
for (var _ : list) { count++; }          // unused loop variable
try { ... } catch (Exception _) { ... }  // unused exception
map.forEach((_, v) -> process(v));       // unused lambda parameter
```

#### Grammar Rules Changed

- `switch_label` gains `switch_pattern_label` as an alternative (patterns in switch)
- `instanceof_expression` gains `record_pattern` in addition to `type_pattern`
- `catch_clause` allows `_` as the exception variable name
- `enhanced_for_statement` allows `_` as the loop variable name
- `lambda_parameters` allows `_` as a parameter name (unnamed lambda parameter)
- `formal_parameter` allows `_` as the parameter name
- `local_variable_declaration` allows `_` as the variable name

**Context keyword added:**

```
context_keywords:
  when
```

`when` is only a keyword after `case pattern` in a switch. It is a normal identifier
everywhere else.

---

## 3. Cross-Version Summary

### Keywords by Version

| Keyword | Introduced | Notes |
|---------|------------|-------|
| `abstract` | 1.0 | Class/method modifier |
| `assert` | 1.4 | Assert statement |
| `boolean` | 1.0 | Primitive type |
| `break` | 1.0 | Loop/switch control |
| `byte` | 1.0 | Primitive type |
| `case` | 1.0 | Switch label |
| `catch` | 1.0 | Exception handling |
| `char` | 1.0 | Primitive type |
| `class` | 1.0 | Type declaration |
| `continue` | 1.0 | Loop control |
| `default` | 1.0 | Switch label; interface default method (8) |
| `do` | 1.0 | Loop |
| `double` | 1.0 | Primitive type |
| `else` | 1.0 | Conditional |
| `enum` | 5 | Enumeration type |
| `extends` | 1.0 | Inheritance |
| `final` | 1.0 | Modifier |
| `finally` | 1.0 | Exception handling |
| `float` | 1.0 | Primitive type |
| `for` | 1.0 | Loop; enhanced for-each (5) |
| `if` | 1.0 | Conditional |
| `implements` | 1.0 | Interface implementation |
| `import` | 1.0 | Package import; static import (5) |
| `instanceof` | 1.0 | Type test; pattern matching (17, 21) |
| `int` | 1.0 | Primitive type |
| `interface` | 1.0 | Type declaration; annotation type (5) |
| `long` | 1.0 | Primitive type |
| `native` | 1.0 | JNI modifier |
| `new` | 1.0 | Object creation |
| `package` | 1.0 | Package declaration |
| `private` | 1.0 | Access modifier |
| `protected` | 1.0 | Access modifier |
| `public` | 1.0 | Access modifier |
| `return` | 1.0 | Method return |
| `short` | 1.0 | Primitive type |
| `static` | 1.0 | Modifier; static import (5); static interface method (8) |
| `strictfp` | 1.2 | Floating-point modifier (included in 1.0 grammar for simplicity) |
| `super` | 1.0 | Superclass reference; type bounds (5) |
| `switch` | 1.0 | Switch statement; switch expression (14) |
| `synchronized` | 1.0 | Thread modifier/statement |
| `this` | 1.0 | Self-reference |
| `throw` | 1.0 | Exception throw |
| `throws` | 1.0 | Method exception specification |
| `transient` | 1.0 | Serialization modifier |
| `try` | 1.0 | Exception handling; try-with-resources (7) |
| `void` | 1.0 | Return type |
| `volatile` | 1.0 | Thread modifier |
| `while` | 1.0 | Loop |

### Context Keywords by Version

| Context Keyword | Introduced | Context |
|-----------------|------------|---------|
| `var` | 10 | Local variable type inference |
| `record` | 14 | Record declaration |
| `yield` | 14 | Switch expression value |
| `sealed` | 17 | Sealed class/interface modifier |
| `permits` | 17 | Permitted subclass list |
| `non-sealed` | 17 | Open subclass modifier |
| `when` | 21 | Pattern guard in switch |
| `_` | 21 | Unnamed pattern/variable |

### Operators by Version

| Operator | Introduced | Purpose |
|----------|------------|---------|
| `@` | 5 | Annotation prefix |
| `...` | 5 | Varargs |
| `->` | 8 | Lambda arrow; switch expression arrow (14) |
| `::` | 8 | Method reference |

### Literal Formats by Version

| Format | Introduced | Example |
|--------|------------|---------|
| Decimal int | 1.0 | `42`, `42L` |
| Hex int | 1.0 | `0xFF`, `0xFFL` |
| Octal int | 1.0 | `077` |
| Float/double | 1.0 | `3.14`, `3.14f`, `1e10` |
| Char | 1.0 | `'a'`, `'\n'` |
| String | 1.0 | `"hello"` |
| Binary int | 7 | `0b1010` |
| Underscores in numbers | 7 | `1_000_000` |
| Text blocks | 14 | `"""..."""` |
| String templates | 21 (preview) | `STR."Hello \{name}"` |

---

## 4. Implementation Notes

### Generics and Angle Brackets (Java 5+)

The `<` and `>` tokens are overloaded in Java 5+ as both comparison operators and generic
type delimiters. This creates parsing ambiguity. For example:

```java
boolean b = a < b;           // comparison
List<String> list;           // generic type
a = (List<String>) obj;      // generic cast
f(a<b, c>d);                 // two comparisons, or one generic with two args?
```

Resolution strategy: Use syntactic predicates (Extension 1) to look ahead when `<` is
encountered in a type position. If the matching `>` is followed by a token that can follow
a type (NAME, `[`, `.`, `,`, `)`, `>`), interpret it as a type argument list. Otherwise,
interpret it as less-than.

The `>>` and `>>>` tokens must be split in type argument context. `List<List<String>>`
ends with `>>` which is two closing angle brackets, not a right-shift.

### Switch Expression Ambiguity (Java 14+)

Switch can appear as both a statement and an expression. In statement position, this is
unambiguous (switch at statement level). In expression position, the switch is parsed as
a primary expression. The `->` in switch rules creates no ambiguity with lambda because
the switch context is established by the `switch` keyword and `case` labels.

### Context Keywords vs Reserved Keywords

Java's approach to new keywords evolved over time:
- Java 1.0-1.4: New features use fully reserved keywords (`assert`). This breaks code.
- Java 5: `enum` was reserved, breaking some code.
- Java 10+: New features use context keywords (`var`, `record`, `yield`, `sealed`, `when`).
  These don't break existing code because they're only special in specific syntactic positions.

The grammar must distinguish context keywords from identifiers based on position. Our
`context_keywords` directive (from `lexer-parser-extensions.md`) handles this: the lexer
emits NAME for these tokens, and the parser recognizes them as keywords only in the
documented positions.

### Pattern Matching Scope (Java 17+)

Pattern variables have "flow scoping" -- their scope depends on definite assignment analysis.
This is a semantic concern, not a grammar concern. The grammar simply allows `instanceof`
with a pattern and lets the compiler determine where the pattern variable is in scope.

---

## 5. Testing Strategy

Each grammar version should be tested with programs that:

1. **Parse valid code** -- programs using only features available in that version
2. **Reject future features** -- e.g., lambda syntax must be a parse error in the Java 7 grammar
3. **Handle edge cases** -- nested generics, diamond in complex expressions, pattern matching
   in deeply nested switch expressions
4. **Match JLS examples** -- use code examples from the corresponding JLS edition

### Test file naming:

```
code/grammars/java/tests/
  java1.0_valid.java
  java1.0_invalid.java
  java5_generics.java
  java8_lambdas.java
  java14_records.java
  java17_sealed.java
  java21_patterns.java
```

---

## 6. Open Questions

1. **Java 9 module system:** `module-info.java` has its own mini-grammar (`module`, `requires`,
   `exports`, `opens`, `uses`, `provides`). Should we include a Java 9 grammar for module
   declarations, even though regular `.java` files didn't change syntactically?

2. **Preview features:** Java 12+ uses preview features that may change before finalization.
   Should we track preview features in the version where they preview, or only when they
   finalize? This spec currently tracks finalized features only, except for string templates
   (Java 21 preview) which were later withdrawn.

3. **Annotation processing:** The grammar for annotation values (`element_value`) allows
   expressions, but only constant expressions are semantically valid. Should the grammar
   restrict this or leave it to semantic analysis?

4. **Record patterns depth:** Java 21 allows arbitrarily nested record patterns. Should
   the grammar limit nesting depth for practical purposes, or allow unbounded recursion?
