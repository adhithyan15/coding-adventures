# Caesar Cipher

A pure-Elixir implementation of the Caesar cipher with encryption, decryption,
ROT13, brute-force attack, and chi-squared frequency analysis. This package is
part of the **coding-adventures** monorepo, which builds the computing stack
from transistors to operating systems. The Caesar cipher sits at the
cryptography layer — the first cipher you should learn before moving on to
Vigenere, substitution ciphers, and modern block ciphers like AES.

## What Is the Caesar Cipher?

The Caesar cipher is a **substitution cipher** where each letter in the
plaintext is replaced by a letter a fixed number of positions further down the
alphabet. It is named after Julius Caesar, who reportedly used it with a shift
of 3 to encrypt military dispatches around 58 BC.

### The Algorithm

Given a shift value `s`, each letter is transformed using modular arithmetic:

```
encrypt(letter, s) = (letter + s) mod 26
decrypt(letter, s) = (letter - s) mod 26
```

Where letters are numbered A=0, B=1, ..., Z=25.

For example, with shift 3:

```
Plaintext alphabet:  A B C D E F G H I J K L M N O P Q R S T U V W X Y Z
Ciphertext alphabet: D E F G H I J K L M N O P Q R S T U V W X Y Z A B C
```

### Worked Example

Let us encrypt "HELLO" with shift 3, step by step:

```
H → position  7 → (7  + 3) mod 26 = 10 → K
E → position  4 → (4  + 3) mod 26 =  7 → H
L → position 11 → (11 + 3) mod 26 = 14 → O
L → position 11 → (11 + 3) mod 26 = 14 → O
O → position 14 → (14 + 3) mod 26 = 17 → R

Ciphertext: "KHOOR"
```

To decrypt, we subtract instead of adding:

```
K → position 10 → (10 - 3) mod 26 =  7 → H
H → position  7 → (7  - 3) mod 26 =  4 → E
O → position 14 → (14 - 3) mod 26 = 11 → L
O → position 14 → (14 - 3) mod 26 = 11 → L
R → position 17 → (17 - 3) mod 26 = 14 → O

Plaintext: "HELLO"
```

## Usage

### Basic Encryption and Decryption

```elixir
alias CodingAdventures.CaesarCipher

# Encrypt with shift 3
CaesarCipher.encrypt("HELLO", 3)
# => "KHOOR"

# Decrypt by providing the same shift
CaesarCipher.decrypt("KHOOR", 3)
# => "HELLO"

# Case is preserved, non-alpha characters pass through
CaesarCipher.encrypt("Hello, World!", 5)
# => "Mjqqt, Btwqi!"
```

### ROT13

ROT13 is a special case of the Caesar cipher with shift 13. Because 13 is
exactly half of 26, ROT13 is its own inverse — applying it twice returns the
original text:

```elixir
CaesarCipher.rot13("Hello")
# => "Uryyb"

CaesarCipher.rot13("Uryyb")
# => "Hello"
```

ROT13 was widely used on Usenet in the 1980s and 1990s to hide spoilers and
punchlines. It provides no real security — everyone knows the "key" — but it
prevents accidentally reading content you did not want to see.

### Brute Force Attack

Since there are only 26 possible shifts (0 through 25), we can try them all:

```elixir
results = CaesarCipher.brute_force("KHOOR")
# => [{1, "JGNNQ"}, {2, "IFMMP"}, {3, "HELLO"}, ...]

# Find the readable one
Enum.find(results, fn {_shift, text} -> text == "HELLO" end)
# => {3, "HELLO"}
```

This demonstrates a fundamental weakness of the Caesar cipher: the key space
is tiny. With only 25 non-trivial shifts to try, any ciphertext can be broken
in microseconds. Modern ciphers like AES-256 have a key space of 2^256, making
brute force computationally infeasible.

### Frequency Analysis

For longer texts, we can use statistics to automatically find the correct
shift without trying each one manually. The `frequency_analysis/1` function
compares the letter distribution of each possible decryption against known
English letter frequencies using the chi-squared statistic:

```elixir
ciphertext = CaesarCipher.encrypt("THE QUICK BROWN FOX JUMPS OVER THE LAZY DOG", 7)
# => "AOL XBPJR IYVDU MVE QBTWZ VCLY AOL SHGF KVN"

{shift, plaintext} = CaesarCipher.frequency_analysis(ciphertext)
# => {7, "THE QUICK BROWN FOX JUMPS OVER THE LAZY DOG"}
```

#### How Chi-Squared Frequency Analysis Works

In English, letters do not appear with equal frequency. The letter 'E' appears
about 12.7% of the time, while 'Z' appears only 0.074% of the time. This
uneven distribution is the fingerprint that frequency analysis exploits.

For each candidate shift (0 through 25):

1. Decrypt the ciphertext with that shift.
2. Count how often each letter appears.
3. Compare to expected English frequencies using chi-squared:

```
chi_squared = sum over all letters of:
    (observed_count - expected_count)^2 / expected_count
```

4. The shift that produces the lowest chi-squared value (closest to English)
   is the most likely correct decryption.

This works well for texts longer than about 25-30 characters. For very short
texts, there may not be enough letters for the statistical signal to emerge.

## Design Decisions

### Why Character Lists Instead of Graphemes?

Elixir strings are UTF-8 binaries. We convert to charlists (`String.to_charlist/1`)
for character-level arithmetic because:

1. Caesar cipher operates on individual characters as numbers.
2. Charlist elements are integers, so we can do `char + shift` directly.
3. Pattern matching on integer ranges (`?A..?Z`) is clean and efficient.
4. Converting back with `List.to_string/1` is straightforward.

Using `String.graphemes/1` would give us single-character strings, requiring
extra conversions for the arithmetic. Charlists are the natural fit here.

### Why `Integer.mod/2` Instead of `rem/2`?

Elixir's `rem/2` preserves the sign of the dividend, which gives negative
results for negative shifts:

```elixir
rem(-1, 26)   # => -1  (wrong for our purposes)
Integer.mod(-1, 26)  # => 25  (correct wrap-around)
```

For modular arithmetic where we always want a result in `0..25`, we need
`Integer.mod/2`, which always returns a non-negative result.

### Decryption as Negated Encryption

Instead of writing separate decryption logic, we define `decrypt/2` as
`encrypt(text, -shift)`. This is correct because:

```
encrypt(x, s)  = (x + s) mod 26
encrypt(x, -s) = (x - s) mod 26 = decrypt(x, s)
```

One function, two uses. This reduces code duplication and the chance of bugs.

## Historical Context

The Caesar cipher is one of the earliest known encryption techniques. Suetonius
records that Caesar used a shift of 3 in his personal correspondence. His
nephew Augustus reportedly used a shift of 1. The cipher was considered secure
in its time because most of Caesar's enemies were illiterate.

The technique remained in use for centuries. As late as the 19th century,
Russian military commanders used Caesar ciphers because they believed their
troops could not handle anything more complex. It was finally rendered
completely obsolete by Kasiski's 1863 publication on breaking polyalphabetic
ciphers, which made even Vigenere ciphers insecure — let alone simple
substitution ciphers like Caesar.

Today, the Caesar cipher is used exclusively for education. It teaches the
core concepts of symmetric encryption, key spaces, brute force, and frequency
analysis — all of which are essential for understanding modern cryptography.

## Running Tests

```bash
bash BUILD
```

Or directly:

```bash
mix test
```

## API Reference

| Function | Description |
|----------|-------------|
| `encrypt(text, shift)` | Shift each letter forward by `shift` positions |
| `decrypt(text, shift)` | Reverse a Caesar cipher encryption |
| `rot13(text)` | Apply ROT13 (shift 13, self-inverse) |
| `brute_force(ciphertext)` | Try all 25 shifts, return list of tuples |
| `frequency_analysis(ciphertext)` | Chi-squared analysis to find best shift |
