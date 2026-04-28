# atbash-cipher — Kotlin

The Atbash cipher: an ancient Hebrew substitution cipher that mirrors the alphabet. A↔Z, B↔Y, C↔X, and so on.

## Usage

```kotlin
import com.codingadventures.atbashcipher.AtbashCipher

AtbashCipher.encrypt("Hello, World!")   // → "Svool, Dliow!"
AtbashCipher.decrypt("Svool, Dliow!")   // → "Hello, World!"
AtbashCipher.encrypt("ABCXYZ")          // → "ZYXCBA"
```

## How it works

Every letter is replaced by its mirror image: position `i` (0 = A) maps to position `25 - i` (Z).

```
A B C D E F G H I J K L M N O P Q R S T U V W X Y Z
↕ ↕ ↕ ↕ ↕ ↕ ↕ ↕ ↕ ↕ ↕ ↕ ↕ ↕ ↕ ↕ ↕ ↕ ↕ ↕ ↕ ↕ ↕ ↕ ↕ ↕
Z Y X W V U T S R Q P O N M L K J I H G F E D C B A
```

Non-alphabetic characters (digits, spaces, punctuation) pass through unchanged. Case is preserved.

## Self-inverse property

Applying Atbash twice returns the original text — so `encrypt` and `decrypt` are the same function:

```kotlin
AtbashCipher.encrypt(AtbashCipher.encrypt("HELLO"))  // → "HELLO"
```

## Running Tests

```bash
gradle test
```

24 tests covering individual letters, full alphabet, case preservation, non-alpha pass-through, and roundtrip self-inverse verification.

## Part of the Coding Adventures series

Kotlin counterpart to the Python, Rust, Go, TypeScript, and Java implementations.
