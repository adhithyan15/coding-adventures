# Caesar Cipher

**Caesar cipher -- the oldest substitution cipher, with brute-force and frequency analysis.**

Part of the [coding-adventures](https://github.com/adhithyan15/coding-adventures) project.

---

## What Is a Caesar Cipher?

The Caesar cipher is the simplest and oldest known substitution cipher. It is named after Julius Caesar, who reportedly used it to communicate with his generals around 58 BC. The Roman historian Suetonius recorded that Caesar would shift each letter in his messages by three positions forward in the alphabet: A became D, B became E, C became F, and so on. When the shift reached the end of the alphabet, it wrapped around -- X became A, Y became B, Z became C.

In modern terms, the Caesar cipher is a **monoalphabetic substitution cipher**. Every occurrence of a given letter always maps to the same replacement letter throughout the entire message. The cipher is defined by a single parameter: the **shift** (also called the **key**), an integer between 0 and 25.

### The Mathematics

If we assign each letter a number (A=0, B=1, C=2, ..., Z=25), then encryption and decryption are simple modular arithmetic:

```
encrypt(letter, shift) = (letter + shift) mod 26
decrypt(letter, shift) = (letter - shift) mod 26
```

Non-alphabetic characters (digits, spaces, punctuation) pass through unchanged. Letter case is preserved: uppercase input produces uppercase output.

### A Worked Example

Let us encrypt "HELLO WORLD" with a shift of 3:

```
Plaintext:  H  E  L  L  O     W  O  R  L  D
Numeric:    7  4  11 11 14    22 14 17 11 3
+3:         10 7  14 14 17    25 17 20 14 6
Ciphertext: K  H  O  O  R     Z  R  U  O  G
```

The space passes through unchanged. To decrypt, subtract 3 instead of adding it.

---

## Installation

### Via LuaRocks

```bash
luarocks install coding-adventures-caesar-cipher
```

### From Source

Clone the repository and add the `src/` directory to your Lua module path:

```bash
git clone https://github.com/adhithyan15/coding-adventures.git
cd coding-adventures/code/packages/lua/caesar_cipher
```

Then either set `LUA_PATH` or adjust `package.path` in your script:

```lua
package.path = "src/?.lua;src/?/init.lua;" .. package.path
local caesar = require("coding_adventures.caesar_cipher")
```

---

## Usage

### Basic Encryption and Decryption

```lua
local caesar = require("coding_adventures.caesar_cipher")

-- Encrypt with shift=3 (the classic Caesar shift)
local ciphertext = caesar.encrypt("Hello, World!", 3)
print(ciphertext)  --> "Khoor, Zruog!"

-- Decrypt by providing the same shift
local plaintext = caesar.decrypt(ciphertext, 3)
print(plaintext)  --> "Hello, World!"
```

The `encrypt` function shifts each letter forward by the given amount. The `decrypt` function reverses the operation by shifting backward. Non-alphabetic characters (spaces, commas, exclamation marks, digits) pass through unchanged. Letter case is preserved.

### Negative and Large Shifts

Shifts are normalized modulo 26, so negative shifts and shifts larger than 26 work correctly:

```lua
-- Negative shift: shift left by 3 (same as shifting right by 23)
print(caesar.encrypt("HELLO", -3))  --> "EBIIL"

-- Large shift: 29 mod 26 = 3
print(caesar.encrypt("HELLO", 29))  --> "KHOOR"
```

### ROT13

ROT13 is a special case of the Caesar cipher with shift=13. Because the English alphabet has 26 letters and 13 is exactly half, applying ROT13 twice returns the original text. This self-inverse property made ROT13 popular on Usenet in the 1980s for hiding spoilers and punchlines.

```lua
local secret = caesar.rot13("Hello, World!")
print(secret)  --> "Uryyb, Jbeyq!"

-- Apply again to recover the original
local original = caesar.rot13(secret)
print(original)  --> "Hello, World!"

-- The self-inverse property: rot13(rot13(x)) == x
assert(caesar.rot13(caesar.rot13("any text")) == "any text")
```

### Brute-Force Attack

Since there are only 25 possible non-trivial shifts, the Caesar cipher can be broken trivially by trying all of them. The `brute_force` function returns a table of all 25 candidates:

```lua
local results = caesar.brute_force("Khoor, Zruog!")

-- Each result is a table { shift = N, plaintext = "..." }
for _, entry in ipairs(results) do
    print(string.format("Shift %2d: %s", entry.shift, entry.plaintext))
end

-- Output includes:
-- Shift  1: Jgnnq, Yqtnf!
-- Shift  2: Ifmmp, Xpsme!
-- Shift  3: Hello, World!   <-- the correct plaintext
-- Shift  4: Gdkkn, Vnqkc!
-- ...
```

A human can immediately spot which result is meaningful English.

### Frequency Analysis

For longer ciphertexts, frequency analysis can automatically identify the most likely shift without human inspection. The function compares the letter frequency distribution of each candidate decryption against known English letter frequencies using the chi-squared statistic.

```lua
local ciphertext = caesar.encrypt(
    "IN CRYPTOGRAPHY A CAESAR CIPHER IS ONE OF THE SIMPLEST " ..
    "AND MOST WIDELY KNOWN ENCRYPTION TECHNIQUES",
    7
)

local best = caesar.frequency_analysis(ciphertext)
print("Most likely shift:", best.shift)       --> 7
print("Decrypted text:",    best.plaintext)   --> original text
```

The chi-squared statistic measures how closely the observed letter frequencies match the expected English frequencies. The candidate shift that produces the lowest chi-squared value is returned as the best guess.

Frequency analysis works best on longer texts (50+ characters of alphabetic content). On very short texts, the statistical signal may be too weak to identify the correct shift reliably.

### English Frequency Table

The module exports the English letter frequency table used by `frequency_analysis`:

```lua
for letter, freq in pairs(caesar.ENGLISH_FREQUENCIES) do
    print(string.format("%s: %.3f%%", letter, freq))
end
-- E: 12.702%
-- T: 9.056%
-- A: 8.167%
-- ...
```

These frequencies are based on large samples of English text and represent the expected percentage of each letter in typical prose.

---

## API Reference

### `caesar.encrypt(text, shift)`

Encrypts `text` using the Caesar cipher with the given `shift`. Letters are shifted forward; non-alphabetic characters pass through unchanged. Case is preserved. The shift is normalized modulo 26.

**Parameters:**
- `text` (string) -- the plaintext to encrypt
- `shift` (number) -- positions to shift each letter (can be negative)

**Returns:** (string) -- the ciphertext

### `caesar.decrypt(text, shift)`

Decrypts `text` that was encrypted with the given `shift`. This is equivalent to `encrypt(text, -shift)`.

**Parameters:**
- `text` (string) -- the ciphertext to decrypt
- `shift` (number) -- the shift that was used to encrypt

**Returns:** (string) -- the plaintext

### `caesar.rot13(text)`

Applies ROT13 (shift=13). Self-inverse: `rot13(rot13(text)) == text`.

**Parameters:**
- `text` (string) -- text to transform

**Returns:** (string) -- the ROT13-transformed text

### `caesar.brute_force(ciphertext)`

Tries all 25 non-trivial shifts (1-25) and returns the results.

**Parameters:**
- `ciphertext` (string) -- the encrypted text

**Returns:** (table) -- a sequence of 25 tables, each with `{ shift = N, plaintext = "..." }`

### `caesar.frequency_analysis(ciphertext)`

Uses chi-squared frequency analysis against English letter frequencies to find the most likely Caesar shift.

**Parameters:**
- `ciphertext` (string) -- the encrypted text to analyze

**Returns:** (table) -- `{ shift = N, plaintext = "..." }` for the best candidate

### `caesar.ENGLISH_FREQUENCIES`

A table mapping uppercase letters (A-Z) to their expected frequency in English text as percentages.

### `caesar.VERSION`

The version string for this module (`"0.1.0"`).

---

## Why Study the Caesar Cipher?

The Caesar cipher is not secure by any modern standard. With only 25 possible keys, it falls to brute force in milliseconds. Even without brute force, frequency analysis breaks it on any reasonably long text. So why study it?

1. **Foundation of cryptography.** The Caesar cipher introduces the core concept of substitution ciphers. Understanding it is the first step toward understanding more complex ciphers like Vigenere, Enigma, and modern AES.

2. **Modular arithmetic.** The shift-and-wrap operation is modular arithmetic in action. This same mathematical structure appears throughout computer science: hash functions, checksums, circular buffers, and clock arithmetic.

3. **Frequency analysis.** Breaking the Caesar cipher with frequency analysis introduces one of the most important ideas in cryptanalysis: that the statistical properties of a language leak through simple encryption schemes. This idea extends to breaking far more complex ciphers.

4. **Programming fundamentals.** Implementing the Caesar cipher exercises string manipulation, character encoding (ASCII), modular arithmetic, table construction, and iteration -- fundamental skills in any language.

---

## Development

### Running Tests

Tests use the [Busted](https://lunarmodules.github.io/busted/) testing framework:

```bash
cd tests
busted . --verbose --pattern=test_
```

### Project Structure

```
caesar_cipher/
  BUILD                    -- Build/test command
  BUILD_windows            -- Windows build/test command
  CHANGELOG.md             -- Version history
  README.md                -- This file
  required_capabilities.json
  coding-adventures-caesar-cipher-0.1.0-1.rockspec
  src/
    coding_adventures/
      caesar_cipher/
        init.lua           -- Module implementation
  tests/
    test_caesar_cipher.lua -- Test suite
```

---

## License

MIT
