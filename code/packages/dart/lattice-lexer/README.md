# coding_adventures_lattice_lexer

A pure Dart grammar-driven lexical analyzer for Lattice stylesheets that produces
flat token streams using pre-compiled grammar definitions.

## Features

- **Grammar-Driven Architecture**: Uses pre-compiled token patterns from `lattice.tokens`.
- **CSS Superset**: Full support for standard CSS tokens plus Lattice extensions (`$var`, `==`, `!=`, `>=`, `<=`, `!default`, `!global`).
- **Comment Handling**: Automatically skips line comments (`// ...`) and block comments (`/* ... */`).
- **Resource & Encoding Bounds**: Enforces `maxSourceLength = 1048576` (1 MB) and validates UTF-16 surrogates atomically before tokenization.
- **Standing Rule #20**: Zero raw string literals in operational code, sealed exception hierarchies, and payload-blind diagnostics.

## 5-Class Lattice Lexer Security Resilience Showcase

| Class | Threat | Defense |
|---|---|---|
| **1: CWE-400 Source Length DoS** | Memory exhaustion via oversized payload strings | `maxSourceLength = 1048576` (1 MB) ceiling enforced before version resolution or tokenization. |
| **2: Surrogate Smuggling Attacks** | Lone or reversed UTF-16 surrogates causing decoding confusion | Source scanned for invalid surrogates (`0xD800`..`0xDFFF`) atomically before processing. |
| **3: Token Domain Integrity** | CSS token collision or missed Lattice-specific operators | Fully distinct token types for `$var` (`VARIABLE`), `==` (`EQUALS_EQUALS`), `!=` (`NOT_EQUALS`), etc. |
| **4: Comment & Literal Resilience** | Unclosed block comments or unterminated string literals | Tokenizer detects boundary errors cleanly and reports typed `LatticeLexicalException`. |
| **5: CWE-209 Payload-Blind Confidentiality** | Sensitive stylesheet contents leaked into error diagnostics | All exceptions emit static messages with position indicators only, strictly redacting caller source. |

## Quick Start

```dart
import 'package:coding_adventures_lattice_lexer/coding_adventures_lattice_lexer.dart';

void main() {
  final tokens = tokenizeLattice('\$primary: #4a90d9; h1 { color: \$primary; }');
  print(tokens.map((t) => t.type).toList());
}
```

## Development & Testing

```bash
sh BUILD   # Format, analyze, conformance test, and coverage (>= 95%)
```
