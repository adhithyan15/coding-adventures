# coding_adventures_vhdl_lexer

A pure Dart grammar-driven lexical analyzer for VHDL (IEEE 1076) that produces
flat, case-normalized token streams from pre-compiled edition grammars.

## Features

- **Five Editions**: 1987, 1993, 2002, 2008 (default) and 2019, each embedded
  from `code/grammars/vhdl/vhdl<year>.tokens` (SHA-256 pinned by the shared
  `language-frontends-v1` conformance manifest). Accepts `2008` and `vhdl2008`.
- **Case-Insensitive Identity**: VHDL identifiers are case-insensitive, so every
  `NAME`/`KEYWORD` value is lowercased and retyped `KEYWORD` exactly when the
  lowered text is in the edition's keyword list, matching the TypeScript lane.
  Based literals, bit strings, character literals and attribute ticks pass
  through unchanged.
- **Resource & Encoding Bounds**: Enforces `maxSourceLength = 1048576` (1 MB) and
  validates UTF-16 surrogates before version resolution and tokenization.
- **Standing Rule #20**: Every error id and message lives in
  `lib/src/messages.dart`; exceptions are sealed with closed constructors and
  never echo caller source or version text.

## 5-Class VHDL Lexer Security Resilience Showcase

| Class | Threat | Defense |
|---|---|---|
| **1: CWE-400 Source Length DoS** | Memory exhaustion via oversized source | `maxSourceLength = 1048576` ceiling enforced before version resolution or tokenization. |
| **2: Surrogate Smuggling** | Lone or reversed UTF-16 surrogates causing decoding confusion | Source scanned for invalid surrogates (`0xD800`..`0xDFFF`) before processing; bounds validate before the version is resolved. |
| **3: Edition Domain Isolation** | Unknown or look-alike edition strings selecting the wrong grammar | Closed alias table; anything else throws `InvalidVhdlVersionException` without echoing the input. |
| **4: Case-Fold Integrity** | `ENTITY`/`Entity`/`entity` producing different tokens | One normalization pass retypes by the edition keyword set; verified on every edition. |
| **5: CWE-209 Payload-Blind Confidentiality** | Sensitive design contents leaked into diagnostics | Static messages with 1-based positions only; caller source is never reflected. |

## Quick Start

```dart
import 'package:coding_adventures_vhdl_lexer/coding_adventures_vhdl_lexer.dart';

void main() {
  final tokens = tokenizeVhdl('ENTITY Foo IS END;', version: '2008');
  print(tokens.map((t) => '${t.type}:${t.value}').toList());
  // [KEYWORD:entity, NAME:foo, KEYWORD:is, KEYWORD:end, SEMICOLON:;, EOF:]
}
```

## Development & Testing

```bash
sh BUILD   # Format, analyze, conformance test, and coverage (>= 95%)
```
