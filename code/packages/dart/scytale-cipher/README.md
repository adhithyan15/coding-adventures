# coding_adventures_scytale_cipher

A pure Dart implementation of the historical [CR02 Scytale cipher](../../../specs/CR02-scytale-cipher.md).
It writes Unicode scalar values across a fixed-width grid and reads the grid by
columns.

## Usage

```dart
import 'package:coding_adventures_scytale_cipher/scytale_cipher.dart';

final ciphertext = encrypt('HELLO WORLD', 3); // "HLWLEOODL R "
final plaintext = decrypt(ciphertext, 3);     // HELLO WORLD
final candidates = bruteForce(ciphertext);
```

For non-empty text, the key must be between 2 and the Unicode-scalar length.
Empty text returns empty text before key validation. Encryption pads with
literal U+0020 spaces; decryption removes every trailing U+0020, so source text
that already ends in spaces is intentionally not recoverable exactly.

`bruteForce` returns keys 2 through half the scalar length. Because that list
contains quadratic total text, inputs above `maxBruteForceTextLength` (4096
scalars) are rejected before candidates are allocated. Scytale is educational
history, not secure cryptography.

## Authority and development

Production code is deterministic pure computation with no filesystem, network,
process, environment, clock, entropy, console, FFI, or native authority. Run:

```text
dart pub get
dart format --output=none --set-exit-if-changed lib test
dart analyze --fatal-infos
dart run coverage:test_with_coverage --branch-coverage --function-coverage --fail-under=90
```
