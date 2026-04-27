# scytale-cipher — Java

The Scytale cipher: the ancient Spartan transposition cipher. Messages are written into a grid and read column-by-column — the key is the number of columns.

## Usage

```java
import com.codingadventures.scytalecipher.ScytaleCipher;

ScytaleCipher.encrypt("HELLOSPARTANS", 4)   // → "HORSEST LPA LAN "
ScytaleCipher.decrypt("HORSEST LPA LAN ", 4)  // → "HELLOSPARTANS"

// Brute force all possible keys
for (ScytaleCipher.BruteForceResult r : ScytaleCipher.bruteForce(ciphertext)) {
    System.out.println("key " + r.key + ": " + r.text);
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

To decrypt: reverse the process and strip trailing spaces.

## Running Tests

```bash
gradle test
```

22 tests covering encryption, decryption, roundtrip, padding, input validation, and brute force.

## Part of the Coding Adventures series

Java counterpart to the Python, Rust, Go, TypeScript, and Kotlin implementations.
