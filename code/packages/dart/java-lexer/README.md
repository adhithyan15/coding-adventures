# Java Lexer for Dart

Pure Dart, grammar-driven implementation of the Java lexical analyzer conforming to canonical Java Language Specifications across 10 distinct language editions.

## Supported Editions

| Edition | Identifier | Grammar File | Highlights |
|---|---|---|---|
| Java 1.0 | `"1.0"` | `java1.0.tokens` | Initial JDK release: basic types, statements, classes |
| Java 1.1 | `"1.1"` | `java1.1.tokens` | Inner classes, transient, volatile |
| Java 1.4 | `"1.4"` | `java1.4.tokens` | `assert` statement keyword |
| Java 5 | `"5"` | `java5.tokens` | Generics, `enum`, annotations (`@`) |
| Java 7 | `"7"` | `java7.tokens` | Binary literals, underscores in numbers, diamond operator |
| Java 8 | `"8"` | `java8.tokens` | Lambda expressions (`->`), method references (`::`) |
| Java 10 | `"10"` | `java10.tokens` | Local variable type inference (`var`) |
| Java 14 | `"14"` | `java14.tokens` | Switch expressions (`yield`) |
| Java 17 | `"17"` | `java17.tokens` | Sealed classes (`sealed`, `permits`), text blocks |
| Java 21 | `"21"` (Default) | `java21.tokens` | Pattern matching, records, string templates |

## Usage

```dart
import 'package:coding_adventures_java_lexer/java_lexer.dart';

// Tokenize modern Java (defaults to Java 21)
final tokens = tokenizeJava('public class Hello { public static void main(String[] args) {} }');

// Tokenize edition-specific Java (e.g. Java 8)
final java8Tokens = tokenizeJava('Runnable r = () -> System.out.println("Hello");', version: '8');

// Use configured lexer instance
final lexer = createJavaLexer('var list = List.of(1, 2, 3);', version: '10');
final result = lexer.tokenize();
```

## 5-Class Lexer Security Resilience Showcase

| Class | Vulnerability / Threat | Architectural Defense & Mitigation |
|---|---|---|
| **Class 1: Oversized Input & Resource Exhaustion** | CWE-400 Memory amplification / unconstrained input DoS | Strict source ceiling of 1,048,576 UTF-16 code units (`maxSourceLength = 1048576`, same unit as the Swift lane) enforced before tokenization or regex execution. Single-pass bounded fixture loading (`maxFixtureBytes = 2097152`). |
| **Class 2: Catastrophic Backtracking Immunity** | CWE-1333 ReDoS on pathological literals / comments | Linear-time tokenization with bounded lookahead and pre-compiled DFA/regexes; tested against pathological nested comments and malformed numeric literals. |
| **Class 3: Lexical Smuggling & Surrogate Defense** | Unicode spoofing, lone surrogate corruption, null-byte injection | Explicit UTF-16 surrogate validation rejects lone high/low surrogates with `InvalidJavaSourceException` prior to regex compilation. Null bytes and non-ASCII controls rejected gracefully. |
| **Class 4: Edition-Downgrade Keyword Confusion** | Dialect confusion (e.g. `assert`/`enum`/`yield` as identifiers vs keywords) | Exact edition isolation across all 10 grammars: `assert` is an identifier in Java 1.1, keyword in 1.4+; `enum` is an identifier in 1.4, keyword in 5+; `yield` is an identifier in 10, keyword in 14+. |
| **Class 5: Payload-Blind Security** | CWE-209 Sensitive source code / token reflection in diagnostics | Closed `JavaLexerErrorCode` enum and sealed exceptions guarantee static error messages without reflection of caller source code or version arguments. |
