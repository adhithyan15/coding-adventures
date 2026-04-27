# caesar-cipher — Java

The Caesar cipher: the classical shift cipher used by Julius Caesar himself, plus ROT13, brute-force, and frequency analysis.

## Usage

```java
import com.codingadventures.caesarcipher.CaesarCipher;

CaesarCipher.encrypt("Hello, World!", 3)   // → "Khoor, Zruog!"
CaesarCipher.decrypt("Khoor, Zruog!", 3)   // → "Hello, World!"
CaesarCipher.rot13("Hello")               // → "Uryyb"

// Brute force: try all 25 shifts
for (CaesarCipher.BruteForceResult r : CaesarCipher.bruteForce("KHOOR")) {
    System.out.println("shift " + r.shift + ": " + r.text);
}

// Frequency analysis: automatic attack
CaesarCipher.FrequencyResult result = CaesarCipher.frequencyAnalysis("KHOOR...");
System.out.println("Likely shift: " + result.shift);
System.out.println("Plaintext: " + result.text);
```

## Running Tests

```bash
gradle test
```

26 tests covering shift arithmetic, ROT13, brute-force, and frequency analysis on long English texts.

## Part of the Coding Adventures series

Java counterpart to the Python, Rust, Go, TypeScript, and Kotlin implementations.
