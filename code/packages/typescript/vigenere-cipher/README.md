# @coding-adventures/vigenere-cipher

Vigenere cipher -- polyalphabetic substitution cipher with full cryptanalysis.

## What is the Vigenere Cipher?

The Vigenere cipher (1553) applies a repeating keyword to shift each letter by a different amount, creating a polyalphabetic substitution that resisted cryptanalysis for 300 years. It was broken by Kasiski (1863) and Friedman (1920s) using statistical methods.

## API

```typescript
import { encrypt, decrypt, findKeyLength, findKey, breakCipher } from "@coding-adventures/vigenere-cipher";

// Encrypt and decrypt
encrypt("ATTACKATDAWN", "LEMON");  // "LXFOPVEFRNHR"
decrypt("LXFOPVEFRNHR", "LEMON"); // "ATTACKATDAWN"

// Mixed case and punctuation preserved
encrypt("Hello, World!", "key");   // "Rijvs, Uyvjn!"

// Cryptanalysis (requires ~200+ chars of English)
const keyLen = findKeyLength(ciphertext);       // IC analysis
const key = findKey(ciphertext, keyLen);         // chi-squared
const result = breakCipher(ciphertext);          // automatic
// result.key, result.plaintext
```

## How It Works

- **encrypt/decrypt**: Shift each letter forward/backward by the corresponding key letter's position (A=0..Z=25). Non-alpha characters pass through unchanged; key advances only on alpha characters.
- **findKeyLength**: Uses Index of Coincidence (IC) to detect periodicity. English text has IC ~0.0667; random text ~0.0385.
- **findKey**: For each key position, tries all 26 shifts and picks the one with the lowest chi-squared against English letter frequencies.
- **breakCipher**: Combines findKeyLength + findKey + decrypt.

## Part of coding-adventures

This is CR03 in the cryptography layer. See `code/specs/CR03-vigenere-cipher.md` for the full specification.
