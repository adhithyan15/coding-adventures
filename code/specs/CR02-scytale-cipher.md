# CR02 — Scytale Cipher

## Overview

The Scytale (pronounced "SKIT-ah-lee") cipher is one of the earliest known
transposition ciphers, used by the Spartans around 700 BCE. Unlike
substitution ciphers (CR00 Caesar, CR01 Atbash) which replace characters,
transposition ciphers rearrange the positions of characters in the message.

The Scytale was a physical device: a wooden rod of specific diameter around
which a strip of leather or parchment was wrapped. The sender wrote the
message along the length of the rod, then unwrapped the strip. The
seemingly random letters on the unwrapped strip could only be read by
wrapping it around a rod of the same diameter (the key).

### How It Works

The Scytale cipher is equivalent to a columnar transposition:

1. **Encrypt**: Write the plaintext row-by-row into a grid with `key` columns.
   Pad the last row with spaces if needed. Then read the ciphertext
   column-by-column (top to bottom, left to right).

2. **Decrypt**: Calculate the number of rows as `ceil(len / key)`. Write the
   ciphertext into columns of that length (top to bottom, left to right).
   Then read row-by-row and strip any trailing padding spaces.

### Historical Context

The Scytale appears in Plutarch's "Life of Lysander" as a tool of Spartan
military communication. Ephors (Spartan magistrates) would share rods of
matching diameter with their generals. A strip of leather wrapped around the
wrong-sized rod would produce gibberish.

### Why After Atbash?

The Atbash cipher (CR01) is a substitution cipher with no key. The Scytale
introduces two new concepts:

1. **Transposition vs. Substitution**: Characters are not replaced but
   rearranged. This is a fundamentally different approach to encryption.

2. **Key-based security**: The rod diameter (number of columns) acts as a
   numeric key. Different keys produce different ciphertexts, unlike Atbash.

3. **Brute-force vulnerability**: With a small key space (2 to len/2),
   the cipher is trivially brute-forced, making it an excellent teaching
   example for why key space size matters.

## Worked Example

**Encrypt "HELLO WORLD" with key=3:**

Step 1: Write row-by-row into 3 columns, padding with spaces:

```
Row 0: H E L
Row 1: L O
Row 2: W O R
Row 3: L D
```

Wait — let's be more precise. The text is 11 characters, key is 3 columns,
so we need ceil(11/3) = 4 rows. We pad to 4 * 3 = 12 characters:

```
"HELLO WORLD" + " " (one space of padding) = "HELLO WORLD "
```

Grid (4 rows x 3 columns):
```
     col0  col1  col2
row0:  H     E     L
row1:  L     O     (space)
row2:  W     O     R
row3:  L     D     (space)
```

Step 2: Read column-by-column:
```
col0: H L W L
col1: E O O D
col2: L   R
```

Result: `"HLWL" + "EOOD" + "L R "` = `"HLWLEOODL R "`

**Decrypt "HLWLEOODL R " with key=3:**

Step 1: Calculate rows = ceil(12/3) = 4.

Step 2: Write ciphertext into columns of length 4:
```
col0 (chars 0-3):  H L W L
col1 (chars 4-7):  E O O D
col2 (chars 8-11): L   R
```

Step 3: Read row-by-row:
```
row0: H E L
row1: L O
row2: W O R
row3: L D
```

Result (before stripping): `"HELLO WORLD "`
Result (after stripping trailing pad): `"HELLO WORLD"`

## Interface Contract

### Functions

| Function | Signature | Description |
|----------|-----------|-------------|
| `encrypt` | `(text: string, key: int) -> string` | Encrypt text using Scytale transposition with given key (number of columns) |
| `decrypt` | `(text: string, key: int) -> string` | Decrypt ciphertext using Scytale transposition with given key |
| `brute_force` | `(text: string) -> list of {key, text}` | Try all keys from 2 to len/2, return list of results |

### Character Handling Rules

1. **ALL characters are preserved** — spaces, punctuation, digits, and
   letters are all transposed. Unlike substitution ciphers, no character
   is "special" in a transposition cipher.

2. **Padding**: When the text length is not evenly divisible by the key,
   the last row is padded with space characters to fill the grid.

3. **Empty strings** return empty strings.

4. **Key validation**: Key must be >= 2 (a key of 1 is the identity).
   Key must be <= len(text). Invalid keys should raise an error or
   return the input unchanged, depending on language convention.

### Brute Force

The `brute_force` function tries every possible key from 2 to `len(text) / 2`
(inclusive). For each key, it decrypts the ciphertext and returns a list of
`{key, decrypted_text}` pairs. This demonstrates that the Scytale has a very
small key space (roughly `n/2` possibilities), making it trivially breakable.

## Package Matrix

| Language | Package Directory | Module/Namespace |
|----------|-------------------|------------------|
| Python | `code/packages/python/scytale-cipher/` | `scytale_cipher` |
| Go | `code/packages/go/scytale-cipher/` | `scytalecipher` |
| Ruby | `code/packages/ruby/scytale_cipher/` | `CodingAdventures::ScytaleCipher` |
| TypeScript | `code/packages/typescript/scytale-cipher/` | `@coding-adventures/scytale-cipher` |
| Rust | `code/packages/rust/scytale-cipher/` | `scytale_cipher` |
| Elixir | `code/packages/elixir/scytale_cipher/` | `CodingAdventures.ScytaleCipher` |
| Lua | `code/packages/lua/scytale_cipher/` | `coding_adventures.scytale_cipher` |
| Perl | `code/packages/perl/scytale-cipher/` | `CodingAdventures::ScytaleCipher` |
| Swift | `code/packages/swift/scytale-cipher/` | `ScytaleCipher` |

**Dependencies:** None. Standalone foundation package.
