# C# Lexer for Dart

Pure Dart, grammar-driven implementation of the C# lexical analyzer conforming to canonical C# Language Specifications across 12 distinct language editions.

## Supported Editions

| Edition | Identifier | Grammar File | Highlights |
|---|---|---|---|
| C# 1.0 | `'1.0'` | `csharp1.0.tokens` | Initial .NET 1.0 release: basic types, statements, classes, interfaces |
| C# 2.0 | `'2.0'` | `csharp2.0.tokens` | Generics, iterators (`yield`), nullable types, partial types |
| C# 3.0 | `'3.0'` | `csharp3.0.tokens` | LINQ, lambda expressions (`=>`), type inference (`var`), auto-properties |
| C# 4.0 | `'4.0'` | `csharp4.0.tokens` | Dynamic binding (`dynamic`), named/optional parameters, covariance |
| C# 5.0 | `'5.0'` | `csharp5.0.tokens` | Asynchronous programming (`async`, `await`), caller info attributes |
| C# 6.0 | `'6.0'` | `csharp6.0.tokens` | String interpolation (`$""`), null-conditional (`?.`, `?[]`), expression-bodied members |
| C# 7.0 | `'7.0'` | `csharp7.0.tokens` | Tuples, pattern matching (`is`, `switch`), binary literals, digit separators |
| C# 8.0 | `'8.0'` | `csharp8.0.tokens` | Nullable reference types, async streams, ranges (`..`), indices (`^`) |
| C# 9.0 | `'9.0'` | `csharp9.0.tokens` | Records (`record`), init-only setters (`init`), top-level statements |
| C# 10.0 | `'10.0'` | `csharp10.0.tokens` | Global usings (`global using`), file-scoped namespaces, record structs |
| C# 11.0 | `'11.0'` | `csharp11.0.tokens` | Raw string literals (`"""..."""`), required members, list patterns |
| C# 12.0 | `'12.0'` (Default) | `csharp12.0.tokens` | Primary constructors, collection expressions (`[...]`), ref readonly |

## Usage

```dart
import 'package:coding_adventures_csharp_lexer/csharp_lexer.dart';

// Tokenize modern C# (defaults to C# 12.0)
final tokens = tokenizeCSharp('public class Hello { public static void Main(string[] args) {} }');

// Tokenize edition-specific C# (e.g. C# 2.0)
final csharp2Tokens = tokenizeCSharp('yield return 42;', version: '2.0');

// Use configured lexer instance
final lexer = createCSharpLexer('var list = new List<int> { 1, 2, 3 };', version: '3.0');
final result = lexer.tokenize();
```

## 5-Class Lexer Security Resilience Showcase

| Class | Vulnerability / Threat | Architectural Defense & Mitigation |
|---|---|---|
| **Class 1: Oversized Input & Resource Exhaustion** | CWE-400 Memory amplification / unconstrained input DoS | Strict source ceiling of 1,048,576 UTF-16 code units (`maxSourceLength = 1048576`, counted via `source.length`) enforced before tokenization or regex execution. Single-pass bounded fixture loading (`maxFixtureBytes = 2097152`). |
| **Class 2: Catastrophic Backtracking Immunity** | CWE-1333 ReDoS on pathological literals / comments | Linear-time tokenization with bounded lookahead and pre-compiled DFA/regexes; tested against pathological nested comments and malformed numeric literals. |
| **Class 3: Lexical Smuggling & Character Refusal** | Unicode spoofing, lone surrogates, illegal control characters | Lone UTF-16 surrogates (`0xD800`–`0xDFFF`) detected and refused with payload-blind `InvalidCSharpSourceException.invalidSurrogates()`. |
| **Class 4: Edition-Downgrade Keyword Confusion** | Dialect confusion (e.g. `yield`/`async`/`await` as identifiers vs keywords) | Exact edition isolation across all 12 grammars: `yield` is an identifier in C# 1.0, keyword in 2.0+; `async`/`await` are identifiers in C# 4.0, keywords in 5.0+; raw strings valid in 11.0+. |
| **Class 5: Payload-Blind Security** | CWE-209 Sensitive source code / token reflection in diagnostics | Closed `CSharpLexerErrorCode` enum and static errors guarantee zero reflection of caller source code or version arguments. |
