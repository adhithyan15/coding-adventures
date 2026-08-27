# coding_adventures_atbash_cipher

A pure Dart implementation of the fixed [CR01 Atbash cipher](../../../specs/CR01-atbash-cipher.md).
It mirrors ASCII letters, preserves case, and passes every other character
through unchanged.

## Usage

```dart
import 'package:coding_adventures_atbash_cipher/atbash_cipher.dart';

final ciphertext = encrypt('Hello, World!'); // Svool, Dliow!
final plaintext = decrypt(ciphertext);       // Hello, World!
assert(encrypt(encrypt(plaintext)) == plaintext);
```

Atbash is an educational historical cipher, not secure cryptography. It has no
key and therefore offers no secrecy against a modern attacker.

## Authority and development

Production code is deterministic pure computation with no filesystem, network,
process, environment, clock, entropy, console, FFI, or native authority. Run:

```text
dart pub get
dart format --output=none --set-exit-if-changed lib test
dart analyze --fatal-infos
dart run coverage:test_with_coverage --branch-coverage --function-coverage --fail-under=90
```
## Language-neutral conformance

The test suite executes all six normative `atbash-transform` objects from the
`classical-ciphers-v1` fixture. Generated dependency-free test source pins the
corpus digest and exact case roster; production code does not read the fixture
or gain filesystem or JSON-parser authority.
