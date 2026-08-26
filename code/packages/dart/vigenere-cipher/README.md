# coding_adventures_vigenere_cipher

A pure Dart implementation of the [CR03 Vigenère cipher](../../../specs/CR03-vigenere-cipher.md)
and deterministic classical frequency-analysis helpers.

## Usage

```dart
import 'package:coding_adventures_vigenere_cipher/vigenere_cipher.dart';

final ciphertext = encrypt('ATTACKATDAWN', 'LEMON'); // LXFOPVEFRNHR
final plaintext = decrypt(ciphertext, 'LEMON');       // ATTACKATDAWN
final result = breakCipher(aLongEnglishCiphertext);
```

Keys must contain one or more ASCII letters. Only ASCII letters are shifted or
consume a key position; case is preserved and every other Unicode scalar passes
through unchanged. `findKeyLength`, `findKey`, and `breakCipher` use index of
coincidence and chi-squared scoring. Analysis is statistical and becomes useful
only on sufficiently long English text; analysis key lengths are bounded at 40.

Vigenère and its automated breaker are educational. Neither is suitable for
protecting data.

## Authority and development

Production code is deterministic pure computation with immutable embedded
frequencies and no filesystem, network, process, environment, clock, entropy,
console, FFI, or native authority. Run:

```text
dart pub get
dart format --output=none --set-exit-if-changed lib test
dart analyze --fatal-infos
dart run coverage:test_with_coverage --branch-coverage --function-coverage --fail-under=90
```
