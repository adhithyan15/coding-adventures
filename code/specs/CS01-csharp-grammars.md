# Versioned C# Grammar System

## Overview

This spec defines a comprehensive grammar system covering every significant C# version
from C# 1.0 (2002) through C# 12.0 (2023). Each version gets its own self-contained
`.tokens` and `.grammar` file pair. No file inherits from or extends another -- each is
a complete, standalone definition of the language at that point in time.

The grammar files use the existing `.tokens` and `.grammar` formats documented in
`tokens-format.md` and `grammar-format.md`, plus extensions from `lexer-parser-extensions.md`
(syntactic predicates, context keywords, bracket depth tracking).

### Why versioned C# grammars?

C# evolved alongside the .NET platform, growing from a Java competitor into one of the
most feature-rich languages in mainstream use. Unlike Java's deliberate, cautious evolution
through the JCP, C# moved aggressively -- adding generics (2.0), LINQ (3.0), dynamic typing
(4.0), async/await (5.0), and pattern matching (7.0-12.0) in rapid succession. Each version
of the ECMA-334 standard or the C# Language Specification captures a distinct point in
this evolution.

Anders Hejlsberg designed C# at Microsoft as a modern, type-safe, object-oriented language
for the .NET Common Language Runtime. It borrowed from Java, C++, and Delphi, but added
its own innovations: properties, events, delegates, value types (structs), operator
overloading, and a preprocessor. Over 20+ years, it has absorbed ideas from functional
programming (LINQ, pattern matching, records), concurrent programming (async/await), and
meta-programming (source generators, attributes).

By giving each version its own grammar, we can:

1. **Parse historically** -- feed a 2005-era C# 2.0 program through the `csharp2.0` grammar
   and get an accurate AST, without worrying about LINQ expression ambiguity
2. **Teach evolution** -- read the files in order to see exactly how C# grew from a
   Java-like OO language into one with LINQ, async/await, records, and pattern matching
3. **Test precisely** -- verify that `var x = 10` is a syntax error in C# 2.0 but valid
   in C# 3.0, or that `=>` in a member body is illegal before C# 6.0
4. **Build version-aware tooling** -- linters, formatters, and IDEs that know which
   features are available in the target C# version

### Why these versions?

Every major C# release changed the grammar. Unlike Java, where some releases (2, 3, 6, 9)
made no syntax changes, C# introduced new syntax in every version. We include all twelve
major versions:

- **C# 1.0** -- the foundation (Java competitor with delegates, properties, events)
- **C# 2.0** -- generics, nullable types, iterators, anonymous methods, partial types
- **C# 3.0** -- LINQ, lambdas, extension methods, var, anonymous types
- **C# 4.0** -- dynamic, named/optional parameters, generic variance
- **C# 5.0** -- async/await, caller info attributes
- **C# 6.0** -- expression-bodied members, string interpolation, null-conditional
- **C# 7.0** -- tuples, pattern matching, local functions, out variables, ref returns
- **C# 8.0** -- nullable reference types, ranges, async streams, switch expressions
- **C# 9.0** -- records, init-only setters, top-level statements, with expressions
- **C# 10.0** -- global usings, file-scoped namespaces, record structs
- **C# 11.0** -- raw string literals, required members, list patterns
- **C# 12.0** -- primary constructors, collection expressions, inline arrays

### Relationship to existing grammars

There are no existing C# grammar files in `code/grammars/`. This spec establishes the
C# grammar system from scratch. The versioned files will live in a `csharp/` subdirectory
alongside the existing `java/`, `ecmascript/`, and `typescript/` directories.

### Comparison with Java grammars

C# and Java started from similar foundations (C-family syntax, single-inheritance OO,
garbage collection) but diverged dramatically. The table below highlights where C# had
features from the start that Java added much later (or never):

| Feature | C# Version | Java Version | Notes |
|---------|-----------|-------------|-------|
| Properties | 1.0 | Never (convention) | Java uses get/set methods by convention |
| Delegates / Events | 1.0 | 8 (lambdas) | Java used anonymous inner classes until Java 8 |
| Value types (structs) | 1.0 | Never (Valhalla?) | Java primitives aren't user-definable |
| Operator overloading | 1.0 | Never | Java forbids it by design |
| Generics (reified) | 2.0 (2005) | 5 (2004, erased) | C# generics exist at runtime; Java's are erased |
| Nullable types | 2.0 | Never (Optional) | `int?` is a value type in C#; Java has `Optional<T>` |
| LINQ | 3.0 | 8 (Streams) | C# has query syntax; Java has method chains |
| Async/await | 5.0 (2012) | 21 (virtual threads) | C# was years ahead on async ergonomics |
| Pattern matching | 7.0 | 17 | C# has more pattern forms |
| Records | 9.0 | 14 | Both are data classes with value semantics |

---

## 1. File Naming Convention

| Version | Year | Tokens File | Grammar File | Spec Edition | Notes |
|---------|------|-------------|--------------|-------------|-------|
| 1.0 | 2002 | `csharp1.0.tokens` | `csharp1.0.grammar` | ECMA-334 1st ed. | The foundation. Java competitor with extras. |
| 2.0 | 2005 | `csharp2.0.tokens` | `csharp2.0.grammar` | ECMA-334 3rd ed. | Generics, nullable, yield, partial. |
| 3.0 | 2007 | `csharp3.0.tokens` | `csharp3.0.grammar` | ECMA-334 4th ed. | LINQ, lambdas, var. The big one. |
| 4.0 | 2010 | `csharp4.0.tokens` | `csharp4.0.grammar` | ECMA-334 4th ed. (amended) | dynamic, named/optional params. |
| 5.0 | 2012 | `csharp5.0.tokens` | `csharp5.0.grammar` | ECMA-334 5th ed. | async/await. |
| 6.0 | 2015 | `csharp6.0.tokens` | `csharp6.0.grammar` | C# 6 spec | Expression-bodied, interpolation, null-conditional. |
| 7.0 | 2017 | `csharp7.0.tokens` | `csharp7.0.grammar` | C# 7 spec | Tuples, patterns, local functions. |
| 8.0 | 2019 | `csharp8.0.tokens` | `csharp8.0.grammar` | C# 8 spec | Nullable refs, ranges, switch expressions. |
| 9.0 | 2020 | `csharp9.0.tokens` | `csharp9.0.grammar` | C# 9 spec | Records, top-level, init-only. |
| 10.0 | 2021 | `csharp10.0.tokens` | `csharp10.0.grammar` | C# 10 spec | Global using, file-scoped namespace. |
| 11.0 | 2022 | `csharp11.0.tokens` | `csharp11.0.grammar` | C# 11 spec | Raw strings, required, list patterns. |
| 12.0 | 2023 | `csharp12.0.tokens` | `csharp12.0.grammar` | C# 12 spec | Primary constructors, collection expressions. |

### Directory Structure

```
code/grammars/
  csharp/
    csharp1.0.tokens    csharp1.0.grammar
    csharp2.0.tokens    csharp2.0.grammar
    csharp3.0.tokens    csharp3.0.grammar
    csharp4.0.tokens    csharp4.0.grammar
    csharp5.0.tokens    csharp5.0.grammar
    csharp6.0.tokens    csharp6.0.grammar
    csharp7.0.tokens    csharp7.0.grammar
    csharp8.0.tokens    csharp8.0.grammar
    csharp9.0.tokens    csharp9.0.grammar
    csharp10.0.tokens   csharp10.0.grammar
    csharp11.0.tokens   csharp11.0.grammar
    csharp12.0.tokens   csharp12.0.grammar
```

### Magic Comments

Every file includes version metadata:

```
# C# 1.0 lexical grammar
# @version 1
# @csharp_version 1.0
```

```
# C# 12.0 parser grammar
# @version 1
# @csharp_version 12.0
```

---

## 2. C# Version Feature Inventory

### C# 1.0 (2002) -- The Foundation

C# 1.0 was designed by Anders Hejlsberg at Microsoft as a modern, type-safe, object-oriented
language for the .NET Common Language Runtime (CLR). Hejlsberg had previously designed Turbo
Pascal and Delphi at Borland, and C#'s design reflects that heritage: properties, events,
and a strong distinction between value types and reference types all come from Delphi.

C# 1.0 launched alongside .NET Framework 1.0 and Visual Studio .NET 2002. It was widely
seen as Microsoft's response to Java, and indeed shares much of Java's syntax. But from
the very first version, C# had features that Java would not get for years or decades:

- **Properties** -- first-class get/set accessors, not the `getX()`/`setX()` convention
- **Delegates** -- type-safe function pointers (Java used anonymous inner classes)
- **Events** -- a language-level observer pattern built on delegates
- **Value types** -- user-defined structs that live on the stack (Java only has primitives)
- **Operator overloading** -- `+`, `-`, `==`, explicit/implicit conversion operators
- **Indexers** -- `this[int index]` property syntax for array-like access
- **Attributes** -- metadata annotations (Java didn't get these until Java 5 in 2004)
- **Preprocessor directives** -- `#if`, `#region`, etc. (Java has nothing comparable)
- **Unsafe code** -- pointers and fixed buffers for interop (Java forbids this entirely)
- **Namespaces** -- hierarchical, can be nested (Java packages are flat declarations)
- **Destructors** -- `~ClassName()` syntax for deterministic finalization (via `IDisposable`)

The ECMA-334 1st edition (2002) and ISO/IEC 23270:2003 are the authoritative references.

#### Tokens

**Identifiers:**

```
NAME = /[a-zA-Z_][a-zA-Z0-9_]*/
```

C# identifiers use underscores and letters but NOT the dollar sign `$` (unlike Java and
JavaScript). C# also supports Unicode identifiers via the full Unicode categories, but we
use the ASCII subset for our grammar system.

C# 1.0 also has **verbatim identifiers**: `@identifier`. The `@` prefix allows any keyword
to be used as an identifier: `@class`, `@if`, `@return`. This was designed for interop with
other .NET languages that might use C# keywords as names. We handle this with a special
token pattern.

**Numbers:**

```
INT_LITERAL    = /0[xX][0-9a-fA-F]+([uU]?[lL]?|[lL]?[uU]?)/   # hex: 0xFF, 0xFFUL
               | /[0-9]+([uU]?[lL]?|[lL]?[uU]?)/               # decimal: 42, 42U, 42L, 42UL
FLOAT_LITERAL  = /[0-9]+\.[0-9]*([eE][+-]?[0-9]+)?[fFdDmM]?/   # 3.14, 3.14f, 3.14m
               | /\.[0-9]+([eE][+-]?[0-9]+)?[fFdDmM]?/          # .5, .5d
               | /[0-9]+[eE][+-]?[0-9]+[fFdDmM]?/               # 1e10, 1e10f
               | /[0-9]+[fFdDmM]/                                 # 42f, 42d, 42m
```

C# has more numeric suffixes than Java:
- `U`/`u` for unsigned integers (uint, ulong) -- Java has no unsigned types
- `L`/`l` for long (same as Java)
- `UL`/`ul` for unsigned long (combined suffix)
- `F`/`f` for float (same as Java)
- `D`/`d` for double (same as Java)
- `M`/`m` for decimal (128-bit decimal floating-point) -- unique to C#

The `decimal` type is designed for financial calculations where `float` and `double` would
introduce rounding errors. `0.1M + 0.2M == 0.3M` is true, unlike with `double`.

No binary literals (`0b`) -- those come in C# 7.0. No underscores in numbers -- C# 7.0.

**Characters and Strings:**

```
CHAR_LITERAL   = /'([^'\\]|\\.)*'/
STRING_LITERAL = /"([^"\\]|\\.)*"/
VERBATIM_STRING = /@"([^"]|"")*"/
```

C# has **verbatim string literals** from day one: `@"C:\Users\file.txt"`. In verbatim
strings, backslashes are literal (no escape processing) and double-quotes are escaped by
doubling: `@"He said ""hello"""`. This is invaluable for Windows file paths and regex
patterns.

Escape sequences in regular strings: `\\`, `\"`, `\'`, `\n`, `\r`, `\t`, `\b`, `\f`,
`\0`, `\a` (alert/bell), `\v` (vertical tab), `\xHH` (hex escape 1-4 digits),
`\uHHHH` (Unicode BMP), `\UHHHHHHHH` (full Unicode including surrogate pairs).

C# has `\a` (alert) and `\v` (vertical tab) which Java does not. C# also has `\U` for
full 32-bit Unicode escapes, while Java is limited to `\u` (16-bit BMP only).

**Boolean and Null Literals:**

```
TRUE_LITERAL  = "true"
FALSE_LITERAL = "false"
NULL_LITERAL  = "null"
```

**Operators (35 total in C# 1.0):**

Arithmetic: `+`, `-`, `*`, `/`, `%`
Assignment: `=`, `+=`, `-=`, `*=`, `/=`, `%=`
Bitwise: `&`, `|`, `^`, `~`, `<<`, `>>`
Bitwise assignment: `&=`, `|=`, `^=`, `<<=`, `>>=`
Comparison: `==`, `!=`, `<`, `>`, `<=`, `>=`
Logical: `&&`, `||`, `!`
Increment/decrement: `++`, `--`
Ternary: `?` (paired with `:`)
Member access: `.`
Pointer: `->` (unsafe code only -- member access through pointer)

Note: C# does NOT have `>>>` (unsigned right shift) -- that was added in C# 11.0. C# 1.0
handles unsigned shifting via unsigned types (`uint`, `ulong`) and the regular `>>` operator.
The `->` operator in C# 1.0 is for unsafe pointer member access, NOT a lambda arrow.

**Punctuation:**

```
( ) { } [ ] ; , . : ::
```

Wait -- `::` is not in C# 1.0. The namespace alias qualifier `::` was added in C# 2.0.
Remove it:

```
( ) { } [ ] ; , . :
```

**Keywords (77 in C# 1.0):**

C# 1.0 has significantly more keywords than Java 1.0 (77 vs 50). This reflects C#'s larger
feature set from the start.

```
keywords:
  abstract as base bool break byte case catch char checked
  class const continue decimal default delegate do double
  else enum event explicit extern false finally fixed float
  for foreach goto if implicit in int interface internal
  is lock long namespace new null object operator out
  override params private protected public readonly ref
  return sbyte sealed short sizeof stackalloc static string
  struct switch this throw true try typeof uint ulong
  unchecked unsafe ushort using virtual void volatile while
```

Notable differences from Java 1.0 keywords:
- `as`, `is` -- type testing operators (Java uses `instanceof` only)
- `base` vs Java's `super` -- same concept, different name
- `bool` vs Java's `boolean` -- C# uses C-style short names
- `checked`, `unchecked` -- arithmetic overflow control (Java has none)
- `decimal` -- 128-bit decimal type (Java has none)
- `delegate` -- function pointer type declaration
- `enum` -- C# had enums from day one (Java got them in Java 5)
- `event` -- observer pattern support
- `explicit`, `implicit` -- user-defined conversion operators
- `fixed` -- pin managed object in memory for unsafe code
- `foreach` -- C# had enhanced for from day one (Java waited until Java 5)
- `goto` -- actually usable in C# (Java reserved it but never implemented)
- `internal` -- assembly-level access (Java's package-private is unnamed)
- `lock` -- syntactic sugar for `Monitor.Enter`/`Exit` (like Java's `synchronized`)
- `namespace` -- hierarchical namespace declaration
- `object` -- keyword alias for `System.Object`
- `operator` -- operator overloading declarations
- `out`, `ref` -- parameter passing modes
- `override` -- explicit method override marker (Java uses `@Override` annotation)
- `params` -- variable-length arguments (Java's `...` varargs came in Java 5)
- `readonly` -- immutable field modifier
- `sbyte` -- signed 8-bit (Java's `byte` is signed; C# has both)
- `sealed` -- prevent inheritance (Java's `final` on classes)
- `sizeof` -- size of value type in bytes
- `stackalloc` -- allocate on stack (unsafe-adjacent)
- `string` -- keyword alias for `System.String`
- `struct` -- value type declaration
- `typeof` -- get `System.Type` object (like Java's `.class` literal)
- `uint`, `ulong`, `ushort` -- unsigned integer types (Java has none)
- `unsafe` -- enable pointer operations
- `using` -- both `using` directive (like `import`) and `using` statement (resource disposal)
- `virtual` -- mark method as overridable (Java methods are virtual by default)

**Reserved words (no meaning in C# 1.0 but reserved for future use):**

None. Unlike Java which reserves `const` and `goto`, C# actually uses both:
`const` is a compile-time constant modifier and `goto` is a real jump statement.

**Skip patterns:**

```
skip:
  WHITESPACE    = /[ \t\r\n\v\f]+/
  LINE_COMMENT  = /\/\/[^\n]*/
  BLOCK_COMMENT = /\/\*([^*]|\*[^\/])*\*\//
```

C# also has XML documentation comments (`/// ...` and `/** ... */`) but they are lexically
identical to line/block comments -- the extra `/` or `*` is just a convention. The C#
compiler processes `///` comments specially for generating XML documentation, but the parser
treats them as comments.

**Preprocessor directives:**

```
preprocessor:
  PP_IF        = /#\s*if\b/
  PP_ELSE      = /#\s*else\b/
  PP_ELIF      = /#\s*elif\b/
  PP_ENDIF     = /#\s*endif\b/
  PP_DEFINE    = /#\s*define\b/
  PP_UNDEF     = /#\s*undef\b/
  PP_REGION    = /#\s*region\b/
  PP_ENDREGION = /#\s*endregion\b/
  PP_ERROR     = /#\s*error\b/
  PP_WARNING   = /#\s*warning\b/
  PP_LINE      = /#\s*line\b/
  PP_PRAGMA    = /#\s*pragma\b/
```

C#'s preprocessor is much simpler than C/C++: no `#include`, no `#define` with values,
no macros. It only supports conditional compilation symbols (boolean flags), code region
markers for IDE folding, and diagnostic directives. This is intentional -- C#'s designers
considered the C preprocessor to be a source of bugs and complexity.

#### Grammar Rules

```
# A C# 1.0 compilation unit is a single source file. It contains optional
# extern alias directives, optional using directives, optional global
# attributes, and namespace member declarations.

compilation_unit = { using_directive } { namespace_member_declaration } ;

using_directive = "using" [ NAME EQUALS ] qualified_name SEMICOLON ;

qualified_name = NAME { DOT NAME } ;

namespace_member_declaration = namespace_declaration
                             | type_declaration ;

namespace_declaration = "namespace" qualified_name
                        LBRACE { using_directive } { namespace_member_declaration } RBRACE
                        [ SEMICOLON ] ;

type_declaration = class_declaration
                 | struct_declaration
                 | interface_declaration
                 | enum_declaration
                 | delegate_declaration
                 | SEMICOLON ;

# --- Class Declarations ---

class_declaration = { attribute_section } { class_modifier } "class" NAME
                    [ ":" class_base_list ]
                    class_body [ SEMICOLON ] ;

class_modifier = "public" | "protected" | "internal" | "private"
               | "new" | "abstract" | "sealed" | "static" ;

class_base_list = class_or_interface_type { COMMA class_or_interface_type } ;

class_or_interface_type = qualified_name ;

class_body = LBRACE { class_member_declaration } RBRACE ;

class_member_declaration = field_declaration
                         | method_declaration
                         | property_declaration
                         | event_declaration
                         | indexer_declaration
                         | operator_declaration
                         | constructor_declaration
                         | destructor_declaration
                         | static_constructor_declaration
                         | type_declaration
                         | SEMICOLON ;

# --- Struct Declarations ---

struct_declaration = { attribute_section } { struct_modifier } "struct" NAME
                     [ ":" interface_type_list ]
                     struct_body [ SEMICOLON ] ;

struct_modifier = "public" | "protected" | "internal" | "private" | "new" ;

struct_body = LBRACE { struct_member_declaration } RBRACE ;

struct_member_declaration = class_member_declaration ;

# --- Interface Declarations ---

interface_declaration = { attribute_section } { interface_modifier } "interface" NAME
                        [ ":" interface_type_list ]
                        interface_body [ SEMICOLON ] ;

interface_modifier = "public" | "protected" | "internal" | "private" | "new" ;

interface_type_list = qualified_name { COMMA qualified_name } ;

interface_body = LBRACE { interface_member_declaration } RBRACE ;

interface_member_declaration = interface_method_declaration
                             | interface_property_declaration
                             | interface_event_declaration
                             | interface_indexer_declaration
                             | SEMICOLON ;

# --- Enum Declarations ---

enum_declaration = { attribute_section } { enum_modifier } "enum" NAME
                   [ ":" integral_type ]
                   enum_body [ SEMICOLON ] ;

enum_modifier = "public" | "protected" | "internal" | "private" | "new" ;

enum_body = LBRACE [ enum_member { COMMA enum_member } [ COMMA ] ] RBRACE ;

enum_member = { attribute_section } NAME [ EQUALS expression ] ;

# --- Delegate Declarations ---

delegate_declaration = { attribute_section } { delegate_modifier } "delegate"
                       return_type NAME LPAREN [ formal_parameter_list ] RPAREN SEMICOLON ;

delegate_modifier = "public" | "protected" | "internal" | "private" | "new" ;

# --- Fields ---

field_declaration = { attribute_section } { field_modifier } type
                    variable_declarators SEMICOLON ;

field_modifier = "public" | "protected" | "internal" | "private"
               | "new" | "static" | "readonly" | "volatile" | "const" ;

variable_declarators = variable_declarator { COMMA variable_declarator } ;

variable_declarator = NAME [ EQUALS variable_initializer ] ;

variable_initializer = expression | array_initializer ;

array_initializer = LBRACE [ variable_initializer { COMMA variable_initializer } [ COMMA ] ] RBRACE ;

# --- Methods ---

method_declaration = { attribute_section } { method_modifier } return_type
                     qualified_name LPAREN [ formal_parameter_list ] RPAREN
                     ( block | SEMICOLON ) ;

method_modifier = "public" | "protected" | "internal" | "private"
                | "new" | "static" | "virtual" | "override"
                | "abstract" | "sealed" | "extern" ;

return_type = type | "void" ;

formal_parameter_list = fixed_parameters [ COMMA parameter_array ]
                      | parameter_array ;

fixed_parameters = fixed_parameter { COMMA fixed_parameter } ;

fixed_parameter = { attribute_section } [ parameter_modifier ] type NAME [ EQUALS expression ] ;

parameter_modifier = "ref" | "out" ;

parameter_array = { attribute_section } "params" type NAME ;

# --- Properties ---

property_declaration = { attribute_section } { property_modifier } type
                       qualified_name LBRACE accessor_declarations RBRACE ;

property_modifier = method_modifier ;

accessor_declarations = get_accessor [ set_accessor ]
                      | set_accessor [ get_accessor ] ;

get_accessor = { attribute_section } [ accessor_modifier ] "get" ( block | SEMICOLON ) ;

set_accessor = { attribute_section } [ accessor_modifier ] "set" ( block | SEMICOLON ) ;

accessor_modifier = "protected" | "internal" | "private"
                   | "protected" "internal" | "internal" "protected" ;

# --- Events ---

event_declaration = { attribute_section } { event_modifier } "event" type
                    ( variable_declarators SEMICOLON
                    | qualified_name LBRACE event_accessor_declarations RBRACE ) ;

event_modifier = method_modifier ;

event_accessor_declarations = add_accessor remove_accessor
                            | remove_accessor add_accessor ;

add_accessor = { attribute_section } "add" block ;

remove_accessor = { attribute_section } "remove" block ;

# --- Indexers ---

indexer_declaration = { attribute_section } { indexer_modifier } type
                      "this" LBRACKET formal_parameter_list RBRACKET
                      LBRACE accessor_declarations RBRACE ;

indexer_modifier = method_modifier ;

# --- Operators ---

operator_declaration = { attribute_section } { operator_modifier } type
                       "operator" overloadable_operator
                       LPAREN formal_parameter_list RPAREN ( block | SEMICOLON ) ;

operator_modifier = "public" | "static" | "extern" ;

overloadable_operator = "+" | "-" | "!" | "~" | "++" | "--"
                      | "true" | "false"
                      | "*" | "/" | "%" | "&" | "|" | "^"
                      | "<<" | ">>"
                      | "==" | "!=" | "<" | ">" | "<=" | ">=" ;

conversion_operator_declaration = { attribute_section } { operator_modifier }
                                  ( "implicit" | "explicit" ) "operator" type
                                  LPAREN type NAME RPAREN ( block | SEMICOLON ) ;

# --- Constructors / Destructors ---

constructor_declaration = { attribute_section } { constructor_modifier } NAME
                          LPAREN [ formal_parameter_list ] RPAREN
                          [ constructor_initializer ] ( block | SEMICOLON ) ;

constructor_modifier = "public" | "protected" | "internal" | "private"
                     | "static" | "extern" ;

constructor_initializer = COLON ( "base" | "this" ) LPAREN [ argument_list ] RPAREN ;

destructor_declaration = { attribute_section } [ "extern" ] TILDE NAME
                         LPAREN RPAREN ( block | SEMICOLON ) ;

static_constructor_declaration = { attribute_section } { static_constructor_modifier }
                                 NAME LPAREN RPAREN ( block | SEMICOLON ) ;

static_constructor_modifier = "static" | "extern" ;

# --- Types ---

type = primitive_type { rank_specifier }
     | qualified_name { rank_specifier }
     | type STAR ;                         # pointer type (unsafe)

primitive_type = "bool" | "byte" | "sbyte" | "short" | "ushort"
               | "int" | "uint" | "long" | "ulong"
               | "char" | "float" | "double" | "decimal"
               | "string" | "object" ;

rank_specifier = LBRACKET { COMMA } RBRACKET ;

# --- Attributes ---

attribute_section = LBRACKET [ attribute_target COLON ] attribute_list [ COMMA ] RBRACKET ;

attribute_target = "assembly" | "module" | "field" | "event" | "method"
                 | "param" | "property" | "return" | "type" ;

attribute_list = attribute { COMMA attribute } ;

attribute = qualified_name [ LPAREN [ attribute_arguments ] RPAREN ] ;

attribute_arguments = attribute_argument { COMMA attribute_argument } ;

attribute_argument = [ NAME EQUALS ] expression ;

# --- Statements ---

statement = block
          | local_variable_declaration SEMICOLON
          | local_constant_declaration SEMICOLON
          | empty_statement
          | expression_statement
          | if_statement
          | while_statement
          | do_while_statement
          | for_statement
          | foreach_statement
          | switch_statement
          | try_statement
          | throw_statement
          | return_statement
          | break_statement
          | continue_statement
          | goto_statement
          | lock_statement
          | using_statement
          | checked_statement
          | unchecked_statement
          | labelled_statement
          | unsafe_statement
          | fixed_statement
          | yield_statement ;

block = LBRACE { statement } RBRACE ;

local_variable_declaration = type variable_declarators ;

local_constant_declaration = "const" type NAME EQUALS expression
                             { COMMA NAME EQUALS expression } ;

empty_statement = SEMICOLON ;

expression_statement = expression SEMICOLON ;

if_statement = "if" LPAREN expression RPAREN statement [ "else" statement ] ;

while_statement = "while" LPAREN expression RPAREN statement ;

do_while_statement = "do" statement "while" LPAREN expression RPAREN SEMICOLON ;

for_statement = "for" LPAREN [ for_init ] SEMICOLON [ expression ] SEMICOLON
                [ expression_list ] RPAREN statement ;

for_init = local_variable_declaration | expression_list ;

expression_list = expression { COMMA expression } ;

foreach_statement = "foreach" LPAREN type NAME "in" expression RPAREN statement ;

switch_statement = "switch" LPAREN expression RPAREN
                   LBRACE { switch_section } RBRACE ;

switch_section = { switch_label } { statement } ;

switch_label = "case" expression COLON
             | "default" COLON ;

try_statement = "try" block ( catch_clauses [ finally_clause ]
                            | finally_clause ) ;

catch_clauses = catch_clause { catch_clause } ;

catch_clause = "catch" [ LPAREN type [ NAME ] RPAREN ] block ;

finally_clause = "finally" block ;

throw_statement = "throw" [ expression ] SEMICOLON ;

return_statement = "return" [ expression ] SEMICOLON ;

break_statement = "break" SEMICOLON ;

continue_statement = "continue" SEMICOLON ;

goto_statement = "goto" ( NAME | "case" expression | "default" ) SEMICOLON ;

lock_statement = "lock" LPAREN expression RPAREN statement ;

using_statement = "using" LPAREN ( local_variable_declaration | expression ) RPAREN statement ;

checked_statement = "checked" block ;

unchecked_statement = "unchecked" block ;

labelled_statement = NAME COLON statement ;

unsafe_statement = "unsafe" block ;

fixed_statement = "fixed" LPAREN type variable_declarators RPAREN statement ;

# --- Expressions ---

expression = assignment_expression ;

assignment_expression = conditional_expression
                      | unary_expression assignment_operator assignment_expression ;

assignment_operator = EQUALS | PLUS_EQUALS | MINUS_EQUALS | STAR_EQUALS
                    | SLASH_EQUALS | PERCENT_EQUALS | AMPERSAND_EQUALS
                    | PIPE_EQUALS | CARET_EQUALS | LEFT_SHIFT_EQUALS
                    | RIGHT_SHIFT_EQUALS ;

conditional_expression = logical_or_expression
                         [ QUESTION expression COLON expression ] ;

logical_or_expression = logical_and_expression { OR_OR logical_and_expression } ;

logical_and_expression = bitwise_or_expression { AND_AND bitwise_or_expression } ;

bitwise_or_expression = bitwise_xor_expression { PIPE bitwise_xor_expression } ;

bitwise_xor_expression = bitwise_and_expression { CARET bitwise_and_expression } ;

bitwise_and_expression = equality_expression { AMPERSAND equality_expression } ;

equality_expression = relational_expression
                      { ( EQUALS_EQUALS | NOT_EQUALS ) relational_expression } ;

relational_expression = shift_expression
                        { ( LESS_THAN | GREATER_THAN | LESS_EQUALS | GREATER_EQUALS )
                          shift_expression
                        | ( "is" | "as" ) type } ;

shift_expression = additive_expression
                   { ( LEFT_SHIFT | RIGHT_SHIFT ) additive_expression } ;

additive_expression = multiplicative_expression
                      { ( PLUS | MINUS ) multiplicative_expression } ;

multiplicative_expression = unary_expression
                            { ( STAR | SLASH | PERCENT ) unary_expression } ;

unary_expression = PLUS_PLUS unary_expression
                 | MINUS_MINUS unary_expression
                 | PLUS unary_expression
                 | MINUS unary_expression
                 | unary_expression_not_plus_minus ;

unary_expression_not_plus_minus = TILDE unary_expression
                                | BANG unary_expression
                                | cast_expression
                                | postfix_expression ;

cast_expression = LPAREN type RPAREN unary_expression ;

postfix_expression = primary { primary_suffix } [ PLUS_PLUS | MINUS_MINUS ] ;

primary_suffix = DOT NAME
               | LPAREN [ argument_list ] RPAREN
               | LBRACKET expression_list RBRACKET ;

primary = literal
        | "this"
        | "base" DOT NAME
        | "base" LBRACKET expression_list RBRACKET
        | "new" class_or_interface_type LPAREN [ argument_list ] RPAREN
        | "new" type rank_specifier { rank_specifier } [ array_initializer ]
        | "new" type array_dimension_exprs { rank_specifier }
        | LPAREN expression RPAREN
        | "typeof" LPAREN type RPAREN
        | "sizeof" LPAREN type RPAREN
        | "checked" LPAREN expression RPAREN
        | "unchecked" LPAREN expression RPAREN
        | "default" LPAREN type RPAREN
        | NAME ;

argument_list = argument { COMMA argument } ;

argument = [ "ref" | "out" ] expression ;

array_dimension_exprs = LBRACKET expression { COMMA expression } RBRACKET ;

literal = NUMBER | CHARACTER | STRING | "true" | "false" | "null" ;
```

**What C# 1.0 does NOT have:**

- No generics (`<T>`) -- C# 2.0
- No nullable types (`int?`) -- C# 2.0
- No iterators (`yield return`) -- C# 2.0
- No anonymous methods (`delegate { }`) -- C# 2.0
- No partial types -- C# 2.0
- No LINQ query expressions -- C# 3.0
- No lambda expressions (`=>`) -- C# 3.0
- No `var` keyword -- C# 3.0
- No extension methods -- C# 3.0
- No anonymous types -- C# 3.0
- No `dynamic` keyword -- C# 4.0
- No named/optional parameters -- C# 4.0
- No `async`/`await` -- C# 5.0
- No string interpolation -- C# 6.0
- No null-conditional `?.` -- C# 6.0
- No pattern matching -- C# 7.0
- No records -- C# 9.0

---

### C# 2.0 (2005) -- Generics, Nullable, and Iterators

C# 2.0 is the most important release after 1.0. Generics alone changed every collection,
every API, and every library. Unlike Java's type-erased generics (added in Java 5 a year
earlier), C# generics are **reified** -- they exist at runtime. `List<int>` and `List<string>`
are genuinely different types in the CLR, not just compile-time sugar. This means C# generics
work with value types without boxing: `List<int>` stores actual integers, not boxed objects.

Nullable types (`int?`) solved the "billion dollar mistake" for value types. Iterators with
`yield return` introduced coroutine-like lazy evaluation. Anonymous methods brought closures
to C# (three years before Java 8's lambdas). Partial types enabled code generation tools
to work alongside hand-written code.

Reference: ECMA-334 3rd edition (2005), C# Language Specification 2.0.

#### Tokens Added (delta from C# 1.0)

**New operator:**

```
QUESTION_QUESTION = "??"     # null coalescing
```

The null-coalescing operator: `x ?? y` returns `x` if non-null, otherwise `y`. This is
C# 2.0's companion to nullable types. Java would not get this until... actually Java still
doesn't have `??`.

**New keywords:**

None fully new, but `yield` becomes a context keyword and `partial` becomes a context keyword.

**Context keywords:**

```
context_keywords:
  partial
  yield
  where
```

`partial` allows splitting a type declaration across multiple files. `yield` is used in
iterators (`yield return`, `yield break`). `where` introduces generic type constraints.
These are context keywords -- they can still be used as identifiers.

#### Grammar Rules Added

**Generics:**

```
type_parameters = LESS_THAN type_parameter { COMMA type_parameter } GREATER_THAN ;

type_parameter = NAME ;

type_arguments = LESS_THAN type_argument { COMMA type_argument } GREATER_THAN ;

type_argument = type ;

type_parameter_constraints = "where" NAME COLON type_parameter_constraint_list ;

type_parameter_constraint_list = type_parameter_constraint
                                 { COMMA type_parameter_constraint } ;

type_parameter_constraint = "class" | "struct" | "new" LPAREN RPAREN | type ;
```

**Nullable types:**

```
nullable_type = type QUESTION ;
```

`int?` is syntactic sugar for `Nullable<int>`. The `?` suffix creates a nullable value type.

**Iterators:**

```
yield_return_statement = "yield" "return" expression SEMICOLON ;
yield_break_statement = "yield" "break" SEMICOLON ;
```

**Anonymous methods:**

```
anonymous_method_expression = "delegate" [ LPAREN [ formal_parameter_list ] RPAREN ] block ;
```

**Partial types:**

```
# "partial" modifier on class/struct/interface declarations
partial_class_declaration = { attribute_section } { class_modifier } "partial" "class" NAME
                            [ type_parameters ] [ ":" class_base_list ]
                            { type_parameter_constraints }
                            class_body [ SEMICOLON ] ;
```

#### Grammar Rules Changed

- `type` gains `QUESTION` suffix for nullable types
- `class_declaration`, `struct_declaration`, `interface_declaration` gain type parameters
- `method_declaration` gains type parameters (generic methods)
- `delegate_declaration` gains type parameters
- `primary` gains `anonymous_method_expression`
- `statement` gains `yield_return_statement` and `yield_break_statement`
- `type` used in qualified positions gains `type_arguments`
- `class_declaration` gains `"partial"` context keyword in modifier position
- Method/class declarations gain `type_parameter_constraints` clauses

---

### C# 3.0 (2007) -- LINQ and Lambdas

C# 3.0 is the third transformative release. LINQ (Language Integrated Query) brought
SQL-like query syntax directly into the language. Lambda expressions replaced anonymous
methods with concise arrow syntax. Extension methods enabled adding methods to existing
types without subclassing. `var` for local variable type inference reduced verbosity.
Anonymous types created lightweight unnamed classes for projections. Object and collection
initializers eliminated boilerplate constructor calls.

Together, these features made C# a multi-paradigm language -- equally comfortable with OOP
and functional programming. LINQ in particular was revolutionary: no other mainstream language
had SQL-like query syntax integrated into the type system.

Reference: ECMA-334 4th edition (2007), C# Language Specification 3.0.

#### Tokens Added (delta from C# 2.0)

**New operator:**

```
ARROW = "=>"     # lambda arrow
```

The `=>` token serves as the lambda arrow. Note: C# already had `->` for unsafe pointer
member access. The `=>` was chosen specifically to avoid collision.

**Context keywords:**

```
context_keywords:
  var
  from
  where      # (already context keyword for generics, now also used in LINQ)
  select
  group
  into
  orderby
  ascending
  descending
  join
  on
  equals
  let
  by
```

These LINQ query keywords are context-sensitive -- they are only special inside query
expressions. `var` is special only in local variable declaration position.

#### Grammar Rules Added

**Lambda expressions:**

```
lambda_expression = lambda_parameters ARROW lambda_body ;

lambda_parameters = NAME
                  | LPAREN [ lambda_parameter_list ] RPAREN ;

lambda_parameter_list = lambda_parameter { COMMA lambda_parameter } ;

lambda_parameter = [ type ] NAME ;

lambda_body = expression | block ;
```

**LINQ query expressions:**

```
query_expression = from_clause query_body ;

from_clause = "from" [ type ] NAME "in" expression ;

query_body = { query_body_clause } select_or_group_clause [ query_continuation ] ;

query_body_clause = from_clause
                  | let_clause
                  | where_clause
                  | join_clause
                  | orderby_clause ;

let_clause = "let" NAME EQUALS expression ;

where_clause = "where" expression ;

join_clause = "join" [ type ] NAME "in" expression
              "on" expression "equals" expression
              [ "into" NAME ] ;

orderby_clause = "orderby" ordering { COMMA ordering } ;

ordering = expression [ "ascending" | "descending" ] ;

select_or_group_clause = "select" expression
                       | "group" expression "by" expression ;

query_continuation = "into" NAME query_body ;
```

**Object and collection initializers:**

```
object_initializer = LBRACE [ member_initializer { COMMA member_initializer } [ COMMA ] ] RBRACE ;

member_initializer = NAME EQUALS initializer_value ;

initializer_value = expression | object_initializer | collection_initializer ;

collection_initializer = LBRACE element_initializer { COMMA element_initializer } [ COMMA ] RBRACE ;

element_initializer = expression
                    | LBRACE expression_list RBRACE ;
```

**Anonymous types:**

```
anonymous_type_creation = "new" LBRACE [ anonymous_type_member { COMMA anonymous_type_member }
                          [ COMMA ] ] RBRACE ;

anonymous_type_member = [ NAME EQUALS ] expression ;
```

**Extension methods (grammar-level change):**

```
# The "this" modifier on the first parameter of a static method
extension_parameter = "this" type NAME ;
```

#### Grammar Rules Changed

- `assignment_expression` gains `lambda_expression` as an alternative
- `primary` gains `query_expression` and `anonymous_type_creation`
- `class_instance_creation` gains optional `object_initializer` or `collection_initializer`
- `local_variable_declaration` gains `"var"` as alternative to explicit type
- `fixed_parameter` gains `"this"` modifier (extension methods)
- `array_creation` gains optional `collection_initializer`

---

### C# 4.0 (2010) -- Dynamic and Named Parameters

C# 4.0 is a smaller release focused on interoperability. The `dynamic` type enables
late-binding for COM interop, reflection, and dynamic languages on the DLR (Dynamic
Language Runtime). Named and optional parameters improve API usability. Generic variance
(`in`/`out` on type parameters) enables natural assignment compatibility for generic
interfaces and delegates.

Reference: C# Language Specification 4.0.

#### Tokens Added (delta from C# 3.0)

**New keyword:**

```
dynamic
```

`dynamic` is a context keyword -- it's only a type name, and can still be used as a variable
name in other positions.

#### Grammar Rules Added

**Named arguments:**

```
named_argument = NAME COLON expression ;
```

**Generic variance:**

```
variance_annotation = "in" | "out" ;

# Added to type_parameter in interface and delegate declarations
variant_type_parameter = [ variance_annotation ] NAME ;
```

#### Grammar Rules Changed

- `argument` gains named argument form: `NAME COLON expression`
- `fixed_parameter` gains default value: `[ EQUALS expression ]` (already in 1.0 grammar
  above for forward compatibility, but semantically new in 4.0)
- `type_parameter` gains `variance_annotation` in interface and delegate type parameter lists
- `type` gains `"dynamic"` as a type name

---

### C# 5.0 (2012) -- Async/Await

C# 5.0 added asynchronous programming with `async`/`await`. This was groundbreaking --
C# was the first mainstream language to integrate async/await into the type system. JavaScript
would adopt the same pattern in ES2017 (five years later), and Python in 3.5 (three years
later).

The `async` modifier on methods/lambdas enables the `await` keyword inside them. `await`
suspends execution until a `Task` completes, without blocking the thread. The compiler
rewrites the method into a state machine.

Reference: C# Language Specification 5.0.

#### Tokens Added (delta from C# 4.0)

**Context keywords:**

```
context_keywords:
  async
  await
```

Both are context keywords. `async` is only meaningful as a method/lambda modifier. `await`
is only meaningful inside an `async` method. This means existing code using `async` or
`await` as variable names continues to compile.

#### Grammar Rules Added

**Async methods:**

```
# async modifier on method, lambda, and anonymous method declarations
async_method_declaration = { method_modifier } "async" return_type NAME
                           LPAREN [ formal_parameter_list ] RPAREN ( block | SEMICOLON ) ;
```

**Await expression:**

```
await_expression = "await" unary_expression ;
```

**Caller info attributes (semantic, not syntactic):** The `[CallerMemberName]`,
`[CallerFilePath]`, and `[CallerLineNumber]` attributes are regular attributes -- no grammar
change needed.

#### Grammar Rules Changed

- `method_declaration` gains `"async"` modifier
- `lambda_expression` gains `"async"` prefix
- `anonymous_method_expression` gains `"async"` prefix
- `unary_expression` gains `await_expression` as an alternative

---

### C# 6.0 (2015) -- Expression-Bodied Members and String Interpolation

C# 6.0 was the first version built with the Roslyn compiler. It focused on reducing
boilerplate and improving expressiveness. Expression-bodied members (`=>`) let single-
expression methods/properties skip the braces and return. String interpolation (`$"..."`)
replaced `String.Format()`. Null-conditional operators (`?.` and `?[]`) eliminated null
check cascades. `nameof()` provided compile-time string representations of identifiers.

Reference: C# 6.0 Language Specification.

#### Tokens Added (delta from C# 5.0)

**New operators:**

```
NULL_CONDITIONAL_DOT     = "?."     # null-conditional member access
NULL_CONDITIONAL_BRACKET = "?["     # null-conditional element access
```

**String interpolation tokens:**

```
INTERPOLATED_STRING_BEGIN = /\$"/
INTERPOLATED_STRING_MID   = /\}/...\{/   # between interpolation holes
INTERPOLATED_STRING_END   = /\}[^{]*"/
```

String interpolation requires special lexer support: `$"Hello, {name}!"` must tokenize the
`{name}` part as a code expression, not a string character.

**Context keywords:**

```
context_keywords:
  nameof
  when       # (exception filter: catch ... when ...)
```

#### Grammar Rules Added

**Expression-bodied members:**

```
# Methods and properties can use => instead of a block
expression_body = ARROW expression SEMICOLON ;
```

**String interpolation:**

```
interpolated_string = DOLLAR_QUOTE { interpolated_string_part } QUOTE ;

interpolated_string_part = interpolated_text | interpolation ;

interpolation = LBRACE expression [ COMMA expression ] [ COLON format_string ] RBRACE ;
```

**Null-conditional operators (integrated into primary_suffix):**

```
null_conditional_access = QUESTION DOT NAME
                        | QUESTION LBRACKET expression_list RBRACKET ;
```

**Exception filters:**

```
catch_clause = "catch" [ LPAREN type [ NAME ] RPAREN ] [ "when" LPAREN expression RPAREN ] block ;
```

**`nameof` expression:**

```
nameof_expression = "nameof" LPAREN expression RPAREN ;
```

#### Grammar Rules Changed

- `method_declaration` body gains `expression_body` alternative (`=>` form)
- `property_declaration` gains `expression_body` alternative
- `primary_suffix` gains null-conditional operators
- `catch_clause` gains `"when"` filter
- `primary` gains `nameof_expression` and `interpolated_string`
- `property_declaration` gains auto-property initializers: `= expression ;`

---

### C# 7.0 (2017) -- Tuples, Patterns, and Local Functions

C# 7.0 introduced a cluster of features that moved C# further toward multi-paradigm
programming. Tuples with named elements replaced `Tuple<T1,T2>` with `(int x, int y)`.
Pattern matching in `is` expressions and `switch` statements enabled type-safe decomposition.
Local functions allowed helper methods inside methods. Out variables eliminated pre-declaration
of `out` parameters. Ref returns and ref locals enabled performance optimization.

Reference: C# 7.0 Language Specification.

#### Tokens Added (delta from C# 6.0)

No new operators or punctuation. The tuple syntax uses existing `(`, `)`, `,` tokens.

**New context keywords:**

```
context_keywords:
  _ (discard)
  var (in pattern context)
```

#### Grammar Rules Added

**Tuple types and expressions:**

```
tuple_type = LPAREN tuple_element COMMA tuple_element { COMMA tuple_element } RPAREN ;

tuple_element = type [ NAME ] ;

tuple_expression = LPAREN expression COMMA expression { COMMA expression } RPAREN ;
```

**Pattern matching:**

```
pattern = type_pattern | constant_pattern | var_pattern ;

type_pattern = type NAME ;

constant_pattern = expression ;

var_pattern = "var" NAME ;

# is-expression with pattern
is_pattern_expression = expression "is" pattern ;

# switch statement with patterns
switch_pattern_label = "case" pattern [ "when" expression ] COLON ;
```

**Local functions:**

```
local_function_declaration = [ "async" ] return_type NAME [ type_parameters ]
                             LPAREN [ formal_parameter_list ] RPAREN
                             { type_parameter_constraints }
                             ( block | expression_body ) ;
```

**Out variables:**

```
# out parameter with inline declaration
out_var_argument = "out" ( type | "var" ) NAME ;
```

**Ref returns and ref locals:**

```
ref_return_type = "ref" [ "readonly" ] type ;

ref_local_declaration = "ref" [ "readonly" ] type NAME EQUALS "ref" expression ;
```

**Binary literals and digit separators (token-level):**

```
BINARY_NUMBER = /0[bB][01]([01_]*[01])?([uU]?[lL]?|[lL]?[uU]?)/ -> NUMBER
```

Underscores in all numeric literals: `1_000_000`, `0xFF_FF`, `0b1010_0101`.

#### Grammar Rules Changed

- `relational_expression` gains `"is" pattern` alternative
- `switch_label` gains `switch_pattern_label` with pattern and optional `when` guard
- `statement` gains `local_function_declaration`
- `argument` gains `out_var_argument` (inline out variable declaration)
- `return_type` gains `"ref"` prefix option
- `local_variable_declaration` gains `"ref"` option
- Numeric literal patterns updated for binary literals and digit separators

---

### C# 8.0 (2019) -- Nullable References, Ranges, and Switch Expressions

C# 8.0 is a significant release. Nullable reference types add null-safety to the type system.
Ranges and indices (`..`, `^`) enable Python-like slicing. Switch expressions make `switch`
usable in expression position. Async streams combine `async` with `IAsyncEnumerable`.
Default interface methods allow adding methods to interfaces without breaking implementers.

Reference: C# 8.0 Language Specification.

#### Tokens Added (delta from C# 7.0)

**New operators:**

```
DOT_DOT  = ".."     # range operator
CARET    = "^"      # index-from-end (already existed as XOR, now overloaded)
```

Wait -- `^` was already the XOR operator. In C# 8.0, `^` gains a second meaning as the
index-from-end operator when used as a unary prefix in an indexing context. The grammar
disambiguates by position.

```
NULL_COALESCING_EQUALS = "??="     # null-coalescing assignment
```

**New keyword:**

None fully new.

**Context keywords:**

```
context_keywords:
  notnull      # generic constraint
  unmanaged    # generic constraint
```

#### Grammar Rules Added

**Switch expressions:**

```
switch_expression = expression "switch" LBRACE switch_expression_arms RBRACE ;

switch_expression_arms = switch_expression_arm { COMMA switch_expression_arm } [ COMMA ] ;

switch_expression_arm = pattern [ "when" expression ] ARROW expression ;
```

**Range expressions:**

```
range_expression = [ expression ] DOT_DOT [ expression ] ;
```

**Index-from-end:**

```
index_expression = CARET expression ;
```

**Async streams:**

```
await_foreach_statement = "await" "foreach" LPAREN type NAME "in" expression RPAREN statement ;

async_iterator_method = "async" return_type NAME LPAREN [ formal_parameter_list ] RPAREN block ;
```

**Using declarations (without braces):**

```
using_declaration = "using" [ "readonly" ] type variable_declarators SEMICOLON ;
```

**Property patterns:**

```
property_pattern = type [ NAME ] LBRACE { property_subpattern COMMA } RBRACE ;

property_subpattern = NAME COLON pattern ;
```

**Positional patterns:**

```
positional_pattern = type LPAREN [ pattern { COMMA pattern } ] RPAREN ;
```

**Default interface methods (grammar change in interface members):**

```
# interface methods can now have bodies
interface_method_declaration = { method_modifier } return_type NAME
                               LPAREN [ formal_parameter_list ] RPAREN
                               ( block | SEMICOLON ) ;
```

#### Grammar Rules Changed

- `primary` gains `switch_expression`
- `primary` gains `range_expression`
- `unary_expression` gains `index_expression` (`^` prefix)
- `assignment_operator` gains `??=`
- `foreach_statement` gains `"await"` prefix
- `pattern` gains property, positional, and tuple patterns
- `interface_member_declaration` allows method bodies (default methods)
- `statement` gains `using_declaration` (braceless `using`)

---

### C# 9.0 (2020) -- Records, Init-Only, and Top-Level Statements

C# 9.0 introduced records (immutable data classes with value semantics), init-only setters
(`init` accessor), top-level statements (no explicit `Main` method needed), `with` expressions
for non-destructive mutation, and several pattern matching enhancements.

Records are C#'s answer to the same problem Java solved with records in Java 14: eliminating
the boilerplate of data classes. But C# records are more flexible -- they can be mutable,
can inherit from other records, and use `with` expressions for copying with changes.

Reference: C# 9.0 Language Specification.

#### Tokens Added (delta from C# 8.0)

**Context keywords:**

```
context_keywords:
  record
  init
  with
  and       # pattern combinator
  or        # pattern combinator
  not       # pattern combinator
```

#### Grammar Rules Added

**Records:**

```
record_declaration = { attribute_section } { class_modifier } "record" NAME
                     [ type_parameters ] [ LPAREN [ record_parameter_list ] RPAREN ]
                     [ ":" class_base_list ]
                     { type_parameter_constraints }
                     ( record_body | SEMICOLON ) ;

record_parameter_list = record_parameter { COMMA record_parameter } ;

record_parameter = { attribute_section } type NAME [ EQUALS expression ] ;

record_body = LBRACE { class_member_declaration } RBRACE ;
```

**Init-only setter:**

```
init_accessor = { attribute_section } [ accessor_modifier ] "init" ( block | SEMICOLON ) ;
```

**Top-level statements:**

```
compilation_unit = { using_directive } { global_statement } { namespace_member_declaration } ;

global_statement = statement ;
```

**With expression:**

```
with_expression = expression "with" LBRACE [ member_initializer_list ] RBRACE ;
```

**Pattern combinators:**

```
pattern = disjunctive_pattern ;

disjunctive_pattern = conjunctive_pattern { "or" conjunctive_pattern } ;

conjunctive_pattern = negated_pattern { "and" negated_pattern } ;

negated_pattern = "not" negated_pattern | primary_pattern ;

relational_pattern = LESS_THAN expression
                   | GREATER_THAN expression
                   | LESS_EQUALS expression
                   | GREATER_EQUALS expression ;
```

**Covariant return types (semantic change, minimal grammar impact).**

#### Grammar Rules Changed

- `type_declaration` gains `record_declaration`
- `accessor_declarations` gains `init_accessor` as alternative to `set_accessor`
- `compilation_unit` gains `global_statement` for top-level statements
- `primary` gains `with_expression`
- `pattern` gains combinators (`and`, `or`, `not`) and relational patterns

---

### C# 10.0 (2021) -- Global Usings and File-Scoped Namespaces

C# 10.0 is a smaller release focused on reducing ceremony. Global using directives apply
imports to the entire project. File-scoped namespace declarations eliminate one level of
nesting. Record structs extend the record concept to value types. Constant interpolated
strings allow `const string s = $"..."` when all holes are constant.

Reference: C# 10.0 Language Specification.

#### Tokens Added (delta from C# 9.0)

**Context keywords:**

```
context_keywords:
  global    # global using directive
  file      # file-scoped type (C# 11, but "file" becomes a modifier)
```

#### Grammar Rules Added

**Global using:**

```
global_using_directive = "global" "using" [ NAME EQUALS ] qualified_name SEMICOLON ;
```

**File-scoped namespace:**

```
file_scoped_namespace = "namespace" qualified_name SEMICOLON ;
```

**Record structs:**

```
record_struct_declaration = { attribute_section } { struct_modifier } "record" "struct" NAME
                            [ type_parameters ] [ LPAREN [ record_parameter_list ] RPAREN ]
                            [ ":" interface_type_list ]
                            { type_parameter_constraints }
                            ( struct_body | SEMICOLON ) ;
```

#### Grammar Rules Changed

- `compilation_unit` gains `global_using_directive` at the top
- `namespace_declaration` gains `file_scoped_namespace` alternative (no braces)
- `type_declaration` gains `record_struct_declaration`

---

### C# 11.0 (2022) -- Raw Strings, Required Members, and List Patterns

C# 11.0 added raw string literals (multi-line, no escaping needed), required members
(constructor-like initialization requirements without constructors), list patterns (matching
array/list contents), UTF-8 string literals, unsigned right shift (`>>>`), and more.

Reference: C# 11.0 Language Specification.

#### Tokens Added (delta from C# 10.0)

**New operators:**

```
UNSIGNED_RIGHT_SHIFT        = ">>>"
UNSIGNED_RIGHT_SHIFT_EQUALS = ">>>="
```

C# finally gets unsigned right shift, 20+ years after Java. Previously, C# handled
unsigned shifting via unsigned types (`uint >>`) but lacked the explicit `>>>` operator.

**Raw string literal:**

```
RAW_STRING = /"""[^"]*"""/ -> STRING
```

Raw strings use three or more quote characters: `"""..."""`. No escape processing. If the
content contains `"""`, use more quotes: `""""...""""`. They support interpolation with
matching `$` signs: `$$"""..{expression}..."""`.

**UTF-8 string literal suffix:**

```
UTF8_STRING = /"([^"\\]|\\.)*"u8/
```

**Context keywords:**

```
context_keywords:
  required
  scoped
  file
```

#### Grammar Rules Added

**Required members:**

```
# required modifier on fields and properties
required_modifier = "required" ;
```

**List patterns:**

```
list_pattern = LBRACKET [ list_pattern_element { COMMA list_pattern_element } ] RBRACKET ;

list_pattern_element = pattern | slice_pattern ;

slice_pattern = DOT_DOT [ pattern ] ;
```

#### Grammar Rules Changed

- `field_modifier` gains `"required"`
- `property_modifier` gains `"required"`
- `shift_expression` gains `>>>` operator
- `assignment_operator` gains `>>>=`
- `pattern` gains `list_pattern`

---

### C# 12.0 (2023) -- Primary Constructors, Collection Expressions, and Inline Arrays

C# 12.0 added primary constructors for classes and structs (previously only for records),
collection expressions (`[1, 2, 3]` syntax), inline arrays, and optional lambda parameters.

Reference: C# 12.0 Language Specification.

#### Tokens Added (delta from C# 11.0)

No new operators or literal formats.

#### Grammar Rules Added

**Primary constructors:**

```
# class and struct declarations gain an optional parameter list after the name
primary_constructor_class = { attribute_section } { class_modifier } "class" NAME
                            LPAREN [ formal_parameter_list ] RPAREN
                            [ ":" class_base_list ]
                            class_body [ SEMICOLON ] ;

primary_constructor_struct = { attribute_section } { struct_modifier } "struct" NAME
                             LPAREN [ formal_parameter_list ] RPAREN
                             [ ":" interface_type_list ]
                             struct_body [ SEMICOLON ] ;
```

**Collection expressions:**

```
collection_expression = LBRACKET [ collection_element { COMMA collection_element } ] RBRACKET ;

collection_element = expression
                   | DOT_DOT expression ;     # spread element
```

**Inline arrays:**

```
# [InlineArray(N)] attribute on struct -- semantic, not syntactic
# Grammar change is minimal: the attribute is a regular attribute
```

**Optional lambda parameters:**

```
# Lambda parameters can now have default values
lambda_parameter = [ type ] NAME [ EQUALS expression ] ;
```

#### Grammar Rules Changed

- `class_declaration` gains optional `LPAREN formal_parameter_list RPAREN` after NAME
- `struct_declaration` gains optional `LPAREN formal_parameter_list RPAREN` after NAME
- `primary` gains `collection_expression`
- `lambda_parameter` gains optional default value

---

## 3. Cross-Version Summary

### Keywords by Version

| Keyword | Introduced | Notes |
|---------|------------|-------|
| `abstract` | 1.0 | Class/method modifier |
| `as` | 1.0 | Type cast (safe) |
| `base` | 1.0 | Base class reference |
| `bool` | 1.0 | Primitive type |
| `break` | 1.0 | Loop/switch control |
| `byte` | 1.0 | Primitive type (unsigned 8-bit) |
| `case` | 1.0 | Switch label; pattern label (7.0) |
| `catch` | 1.0 | Exception handling; filter (6.0) |
| `char` | 1.0 | Primitive type |
| `checked` | 1.0 | Overflow checking |
| `class` | 1.0 | Type declaration; constraint (2.0) |
| `const` | 1.0 | Compile-time constant |
| `continue` | 1.0 | Loop control |
| `decimal` | 1.0 | 128-bit decimal type |
| `default` | 1.0 | Switch label; default value expression (2.0) |
| `delegate` | 1.0 | Delegate type; anonymous method (2.0) |
| `do` | 1.0 | Loop |
| `double` | 1.0 | Primitive type |
| `else` | 1.0 | Conditional |
| `enum` | 1.0 | Enumeration type |
| `event` | 1.0 | Event declaration |
| `explicit` | 1.0 | Explicit conversion operator |
| `extern` | 1.0 | External method |
| `false` | 1.0 | Boolean literal; operator overload |
| `finally` | 1.0 | Exception handling |
| `fixed` | 1.0 | Pin in memory (unsafe) |
| `float` | 1.0 | Primitive type |
| `for` | 1.0 | Loop |
| `foreach` | 1.0 | Collection iteration |
| `goto` | 1.0 | Jump statement (unlike Java, actually works) |
| `if` | 1.0 | Conditional |
| `implicit` | 1.0 | Implicit conversion operator |
| `in` | 1.0 | foreach; generic variance (4.0); ref readonly param (7.2) |
| `int` | 1.0 | Primitive type |
| `interface` | 1.0 | Type declaration |
| `internal` | 1.0 | Assembly access modifier |
| `is` | 1.0 | Type test; pattern matching (7.0) |
| `lock` | 1.0 | Thread synchronization |
| `long` | 1.0 | Primitive type |
| `namespace` | 1.0 | Namespace declaration; file-scoped (10.0) |
| `new` | 1.0 | Object creation; modifier; constraint (2.0) |
| `null` | 1.0 | Null literal |
| `object` | 1.0 | Base type alias |
| `operator` | 1.0 | Operator overloading |
| `out` | 1.0 | Parameter modifier; generic variance (4.0); out var (7.0) |
| `override` | 1.0 | Method override |
| `params` | 1.0 | Variable-length parameters |
| `private` | 1.0 | Access modifier |
| `protected` | 1.0 | Access modifier |
| `public` | 1.0 | Access modifier |
| `readonly` | 1.0 | Immutable field; readonly struct (7.2); readonly ref (8.0) |
| `ref` | 1.0 | Parameter modifier; ref returns (7.0); ref struct (7.2) |
| `return` | 1.0 | Method return |
| `sbyte` | 1.0 | Signed 8-bit type |
| `sealed` | 1.0 | Prevent inheritance |
| `short` | 1.0 | Primitive type |
| `sizeof` | 1.0 | Size of value type |
| `stackalloc` | 1.0 | Stack allocation |
| `static` | 1.0 | Modifier; static using (6.0); static local function (8.0) |
| `string` | 1.0 | String type alias |
| `struct` | 1.0 | Value type; constraint (2.0); record struct (10.0) |
| `switch` | 1.0 | Switch statement; switch expression (8.0) |
| `this` | 1.0 | Self-reference; extension method (3.0); indexer |
| `throw` | 1.0 | Exception throw; throw expression (7.0) |
| `true` | 1.0 | Boolean literal; operator overload |
| `try` | 1.0 | Exception handling |
| `typeof` | 1.0 | Type object expression |
| `uint` | 1.0 | Unsigned 32-bit type |
| `ulong` | 1.0 | Unsigned 64-bit type |
| `unchecked` | 1.0 | Disable overflow checking |
| `unsafe` | 1.0 | Enable pointer operations |
| `ushort` | 1.0 | Unsigned 16-bit type |
| `using` | 1.0 | Import directive; resource statement; using declaration (8.0) |
| `virtual` | 1.0 | Mark method as overridable |
| `void` | 1.0 | Return type |
| `volatile` | 1.0 | Thread modifier |
| `while` | 1.0 | Loop |

### Context Keywords by Version

| Context Keyword | Introduced | Context |
|-----------------|------------|---------|
| `partial` | 2.0 | Partial type declaration |
| `yield` | 2.0 | Iterator return/break |
| `where` | 2.0 | Generic constraint; LINQ clause (3.0) |
| `var` | 3.0 | Local variable type inference; var pattern (7.0) |
| `from` | 3.0 | LINQ query source |
| `select` | 3.0 | LINQ projection |
| `group` | 3.0 | LINQ grouping |
| `into` | 3.0 | LINQ continuation; join into |
| `orderby` | 3.0 | LINQ ordering |
| `ascending` | 3.0 | LINQ order direction |
| `descending` | 3.0 | LINQ order direction |
| `join` | 3.0 | LINQ join |
| `on` | 3.0 | LINQ join key |
| `equals` | 3.0 | LINQ join equality |
| `let` | 3.0 | LINQ local variable |
| `by` | 3.0 | LINQ group-by |
| `dynamic` | 4.0 | Late-bound type |
| `async` | 5.0 | Async method/lambda modifier |
| `await` | 5.0 | Async suspension |
| `nameof` | 6.0 | Name-of expression |
| `when` | 6.0 | Exception filter; pattern guard (7.0) |
| `record` | 9.0 | Record type declaration |
| `init` | 9.0 | Init-only setter |
| `with` | 9.0 | Non-destructive mutation |
| `and` | 9.0 | Pattern combinator |
| `or` | 9.0 | Pattern combinator |
| `not` | 9.0 | Pattern negation |
| `global` | 10.0 | Global using directive |
| `file` | 11.0 | File-scoped type modifier |
| `required` | 11.0 | Required member modifier |
| `scoped` | 11.0 | Scoped ref modifier |

### Operators by Version

| Operator | Introduced | Purpose |
|----------|------------|---------|
| `->` | 1.0 | Unsafe pointer member access |
| `??` | 2.0 | Null coalescing |
| `=>` | 3.0 | Lambda arrow; expression body (6.0) |
| `?.` | 6.0 | Null-conditional member access |
| `?[` | 6.0 | Null-conditional element access |
| `..` | 8.0 | Range |
| `??=` | 8.0 | Null-coalescing assignment |
| `>>>` | 11.0 | Unsigned right shift |
| `>>>=` | 11.0 | Unsigned right shift assignment |

### Literal Formats by Version

| Format | Introduced | Example |
|--------|------------|---------|
| Decimal int | 1.0 | `42`, `42U`, `42L`, `42UL` |
| Hex int | 1.0 | `0xFF`, `0xFFUL` |
| Float/double | 1.0 | `3.14`, `3.14f`, `1e10` |
| Decimal (128-bit) | 1.0 | `3.14m`, `100M` |
| Char | 1.0 | `'a'`, `'\n'` |
| String | 1.0 | `"hello"` |
| Verbatim string | 1.0 | `@"C:\path"` |
| Binary int | 7.0 | `0b1010` |
| Digit separators | 7.0 | `1_000_000` |
| Interpolated string | 6.0 | `$"Hello, {name}"` |
| Raw string | 11.0 | `"""raw"""` |
| UTF-8 string | 11.0 | `"hello"u8` |

---

## 4. Implementation Notes

### Generics and Angle Brackets (C# 2.0+)

Same ambiguity as Java: `<` and `>` are overloaded as comparison operators and generic
delimiters. Same resolution strategy: syntactic predicates with bracket depth tracking.
C# also has the `>>` issue: `List<List<string>>` requires splitting `>>` into two `>` tokens.

### Nullable Value Types vs Nullable Reference Types

C# has TWO nullable systems:
- **Nullable value types** (C# 2.0): `int?` is `Nullable<int>`. The `?` creates a wrapper type.
- **Nullable reference types** (C# 8.0): `string?` is an annotation. `string` becomes non-null
  by default, and `string?` opts back into nullability. This is purely a compiler analysis --
  no runtime type change.

The grammar is the same (`type QUESTION`) but the semantics differ based on whether the
underlying type is a value type or reference type.

### The `=>` Ambiguity

`=>` serves two roles:
- Lambda arrow: `x => x + 1`
- Expression body: `int Foo() => 42;`

These are never syntactically ambiguous because expression bodies appear only in member
declaration position (after a method/property signature), while lambdas appear in expression
position.

### Context Keywords and Backward Compatibility

C# uses context keywords extensively (30+ by C# 12.0) to avoid breaking existing code.
The strategy: a word is only special in specific syntactic positions and remains a valid
identifier everywhere else. The parser must check position context to determine if a NAME
token should be treated as a keyword.

### Preprocessor Directives

C# preprocessor directives are processed in a separate phase before parsing. They affect
which tokens are seen by the parser (via conditional compilation `#if`/`#endif`). Our
grammar treats them as skip tokens and does not model conditional compilation.

---

## 5. Testing Strategy

Each grammar version should be tested with programs that:

1. **Parse valid code** -- programs using only features available in that version
2. **Reject future features** -- e.g., lambda syntax must be a parse error in C# 2.0 grammar
3. **Handle edge cases** -- generic brackets, nullable annotation vs nullable type,
   pattern matching in deeply nested switch expressions
4. **Match ECMA-334 examples** -- use code examples from the corresponding specification

### Test file naming:

```
code/grammars/csharp/tests/
  csharp1.0_valid.cs
  csharp1.0_invalid.cs
  csharp2.0_generics.cs
  csharp3.0_linq.cs
  csharp5.0_async.cs
  csharp7.0_patterns.cs
  csharp8.0_switch_expressions.cs
  csharp9.0_records.cs
  csharp12.0_primary_constructors.cs
```

---

## 6. Open Questions

1. **Preprocessor completeness:** Should the grammar model `#if`/`#endif` conditional
   compilation, or treat preprocessor directives as skip tokens? Modeling them would require
   a two-phase parse, which adds complexity.

2. **Unsafe code:** Should `unsafe` blocks and pointer types be included in every version's
   grammar, or should we have separate "safe" and "full" grammar variants? Unsafe code is
   rarely used but is part of the language specification.

3. **Minor version features:** C# 7.1, 7.2, 7.3 added features (default literals, ref
   structs, pattern-based `fixed`). Should we include sub-versions, or fold their features
   into C# 7.0?

4. **Preview features:** Like Java, C# has preview features that may change. Should we
   track them in the version where they preview or only when finalized?

5. **Global attributes and extern aliases:** C# 1.0 supports `extern alias` directives and
   global attributes (`[assembly: ...]`). These are rarely used. Should they be in the grammar
   or deferred to a later pass?
