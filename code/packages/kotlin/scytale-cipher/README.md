# scytale-cipher — Kotlin

The Scytale cipher: the ancient Spartan transposition cipher. Messages are written into a grid and read column-by-column — the key is the number of columns.

## Usage

```kotlin
import com.codingadventures.scytalecipher.ScytaleCipher

ScytaleCipher.encrypt("HELLOSPARTANS", 4)     // → "HORSEST LPA LAN "
ScytaleCipher.decrypt("HORSEST LPA LAN ", 4)  // → "HELLOSPARTANS"

// Brute force all possible keys
ScytaleCipher.bruteForce(ciphertext).forEach { (key, text) ->
    println("key $key: $text")
}
```

## How it works

Write the plaintext row-by-row into a grid of `key` columns (padding the last row with spaces), then read column-by-column:

```
Input: "HELLOSPARTANS"   key=4

Grid:  H E L L
       O S P A
       R T A N
       S _ _ _

Output (read by columns): "HORSEST LPA LAN "
```

## Running Tests

```bash
gradle test
```

17 tests covering encryption, decryption, roundtrip, padding, input validation, and brute force.

## Part of the Coding Adventures series

Kotlin counterpart to the Python, Rust, Go, TypeScript, and Java implementations.
