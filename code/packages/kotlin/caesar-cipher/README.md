# caesar-cipher — Kotlin

The Caesar cipher: the classical shift cipher used by Julius Caesar himself, plus ROT13, brute-force, and frequency analysis.

## Usage

```kotlin
import com.codingadventures.caesarcipher.CaesarCipher

CaesarCipher.encrypt("Hello, World!", 3)  // → "Khoor, Zruog!"
CaesarCipher.decrypt("Khoor, Zruog!", 3)  // → "Hello, World!"
CaesarCipher.rot13("Hello")               // → "Uryyb"

// Brute force: try all 25 shifts
CaesarCipher.bruteForce("KHOOR").forEach { (shift, text) ->
    println("shift $shift: $text")
}

// Frequency analysis: automatic attack
val (shift, plaintext) = CaesarCipher.frequencyAnalysis("KHOOR...")
println("Likely shift: $shift, plaintext: $plaintext")
```

## Running Tests

```bash
gradle test
```

26 tests covering shift arithmetic, ROT13, brute-force, and frequency analysis on long English texts.

## Part of the Coding Adventures series

Kotlin counterpart to the Python, Rust, Go, TypeScript, and Java implementations.
