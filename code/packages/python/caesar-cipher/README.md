# caesar-cipher

**Caesar cipher -- the oldest substitution cipher, with brute-force and frequency analysis.**

Part of the [coding-adventures](https://github.com/adhithyan15/coding-adventures)
monorepo: a ground-up implementation of the computing stack from transistors
to operating systems.

---

## What Is the Caesar Cipher?

The Caesar cipher is the simplest and most widely known encryption technique.
It is a *substitution cipher* in which each letter in the plaintext is replaced
by a letter a fixed number of positions down the alphabet.

For example, with a shift of 3:

```
Plaintext:  A B C D E F G H I J K L M N O P Q R S T U V W X Y Z
Ciphertext: D E F G H I J K L M N O P Q R S T U V W X Y Z A B C
```

The cipher wraps around: after Z comes A.  Non-alphabetic characters (digits,
spaces, punctuation) pass through unchanged.

## Historical Context

The cipher is named after Julius Caesar (100--44 BCE), who reportedly used it
to communicate with his generals.  According to the Roman historian Suetonius,
Caesar used a shift of 3 in his private correspondence.

While laughably weak by modern standards, the Caesar cipher was effective in
an era when most adversaries were illiterate.  It is the ancestor of all
substitution ciphers and remains a cornerstone of cryptography education.

The Caesar cipher belongs to the family of *monoalphabetic substitution ciphers*,
meaning each plaintext letter always maps to the same ciphertext letter.  This
property is precisely what makes it vulnerable to frequency analysis (more on
that below).

## How It Works

### The Mathematics

Given a letter at position `p` in the alphabet (A=0, B=1, ..., Z=25) and a
shift `s`, the encryption formula is:

```
encrypted = (p + s) mod 26
```

Decryption reverses the shift:

```
decrypted = (p - s) mod 26
```

The `mod 26` operation ensures the result wraps around within the 26-letter
alphabet.

### Worked Example: Encrypting "HELLO" with Shift 3

Let's trace through each letter:

```
Letter | Position | + Shift | mod 26 | Result
-------|----------|---------|--------|-------
H      | 7        | 10      | 10     | K
E      | 4        | 7       | 7      | H
L      | 11       | 14      | 14     | O
L      | 11       | 14      | 14     | O
O      | 14       | 17      | 17     | R
```

Result: **KHOOR**

To decrypt KHOOR back to HELLO, subtract 3 from each position (or equivalently,
add 23, since -3 mod 26 = 23).

### ROT13: The Self-Inverse Special Case

When the shift is exactly 13 (half the alphabet), applying the cipher twice
returns the original text:

```
ROT13(ROT13(text)) = text
```

This works because 13 + 13 = 26, and shifting by 26 is the same as no shift.
ROT13 was historically used on Usenet forums to hide spoilers and punchlines.

```
Input:  A B C D E F G H I J K L M  N O P Q R S T U V W X Y Z
Output: N O P Q R S T U V W X Y Z  A B C D E F G H I J K L M
```

## Why It Is Insecure

The Caesar cipher has two fatal weaknesses:

### 1. Tiny Key Space (Brute Force)

There are only 25 non-trivial shifts (1 through 25).  An attacker can simply
try all of them and read the results.  A computer can do this in microseconds.
Even by hand, checking 25 possibilities takes only a few minutes.

### 2. Preserved Frequency Distribution (Frequency Analysis)

Because the cipher shifts *all* letters by the same amount, the relative
frequency of letters in the ciphertext is identical to the plaintext -- just
shifted.  English has a very distinctive frequency distribution:

```
Most common:  E (12.7%)  T (9.1%)  A (8.2%)  O (7.5%)  I (7.0%)
Least common: Z (0.07%)  Q (0.10%)  X (0.15%)  J (0.15%)
```

An attacker counts the letter frequencies in the ciphertext and compares them
to the expected English frequencies using the *chi-squared statistic*.  The
shift that produces the best match is almost certainly the correct one.

The chi-squared formula:

```
chi^2 = SUM( (observed_i - expected_i)^2 / expected_i )
```

A lower chi-squared value means the observed distribution is closer to
English.  This technique works reliably for texts as short as 50 characters
and almost perfectly for longer texts.

## API Reference

### `encrypt(text: str, shift: int) -> str`

Encrypt plaintext using the Caesar cipher with the given shift.

```python
from caesar_cipher import encrypt

encrypt("HELLO", 3)       # "KHOOR"
encrypt("attack at dawn", 13)  # "nggnpx ng qnja"
encrypt("abc XYZ 123!", 1)    # "bcd YZA 123!"
```

- Preserves case (uppercase stays uppercase, lowercase stays lowercase).
- Non-alphabetic characters pass through unchanged.
- Shift can be any integer; values outside 0-25 wrap via modular arithmetic.

### `decrypt(text: str, shift: int) -> str`

Decrypt ciphertext by reversing the Caesar shift.

```python
from caesar_cipher import decrypt

decrypt("KHOOR", 3)  # "HELLO"
```

Internally, `decrypt(text, shift)` is equivalent to `encrypt(text, -shift)`.

### `rot13(text: str) -> str`

Apply ROT13 (shift of 13).  Self-inverse: `rot13(rot13(text)) == text`.

```python
from caesar_cipher import rot13

rot13("Hello, World!")         # "Uryyb, Jbeyq!"
rot13(rot13("Hello, World!"))  # "Hello, World!"
```

### `brute_force(ciphertext: str) -> list[tuple[int, str]]`

Try all 25 non-trivial shifts and return every possible decryption.

```python
from caesar_cipher import brute_force

results = brute_force("KHOOR")
for shift, plaintext in results:
    print(f"Shift {shift:2d}: {plaintext}")
# Shift  1: JGNNQ
# Shift  2: IFMMP
# Shift  3: HELLO  <-- correct!
# ...
```

### `frequency_analysis(ciphertext: str) -> tuple[int, str]`

Statistically determine the most likely shift using chi-squared comparison
against English letter frequencies.

```python
from caesar_cipher import encrypt, frequency_analysis

ciphertext = encrypt("the quick brown fox jumps over the lazy dog", 7)
shift, plaintext = frequency_analysis(ciphertext)
print(f"Detected shift: {shift}")   # 7
print(f"Decrypted: {plaintext}")    # "the quick brown fox jumps over the lazy dog"
```

### `ENGLISH_FREQUENCIES: dict[str, float]`

The expected frequency distribution of letters in English text.  Keys are
lowercase letters a-z, values are proportions summing to approximately 1.0.

```python
from caesar_cipher import ENGLISH_FREQUENCIES

ENGLISH_FREQUENCIES["e"]  # 0.12702 (most common)
ENGLISH_FREQUENCIES["z"]  # 0.00074 (least common)
```

## Where This Fits

This package sits in the **cryptography** layer of the coding-adventures stack.
The Caesar cipher is the simplest possible cipher, introducing the core concepts
of encryption, decryption, key space, and cryptanalysis.  It serves as the
foundation before moving to more sophisticated ciphers like Vigenere, substitution
ciphers with mixed alphabets, and eventually modern symmetric encryption.

## Development

```bash
# Run tests
bash BUILD
```

## License

MIT
