# scytale-cipher — Kotlin

The Scytale cipher: the ancient Spartan transposition cipher. Messages are written into a grid and read column-by-column — the key is the number of columns.

The implementation follows [CR02](../../../specs/CR02-scytale-cipher.md). Grid cells and key bounds use Unicode scalar values rather than UTF-16 code units, uneven ciphertext columns are reconstructed explicitly, and only trailing U+0020 padding is removed. Brute force rejects inputs above 4096 scalars before allocating quadratic candidate output. Production code is deterministic pure computation with no OS capabilities.

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
gradle test jacocoTestCoverageVerification
```

The suite covers encryption, decryption, roundtrip, padding, input validation,
and brute force, with an 80% line-coverage gate.

## Part of the Coding Adventures series

Kotlin counterpart to the Python, Rust, Go, TypeScript, and Java implementations.
