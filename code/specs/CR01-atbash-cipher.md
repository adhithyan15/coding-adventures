# CR01 — Atbash Cipher

## Overview

The Atbash cipher is a substitution cipher originally used with the Hebrew
alphabet. Each letter maps to its reverse counterpart: A↔Z, B↔Y, C↔X, and so
on. Unlike the Caesar cipher (CR00), Atbash has no key — the substitution table
is fixed.

### Substitution Table

```
Plain:  A B C D E F G H I J K L M N O P Q R S T U V W X Y Z
Cipher: Z Y X W V U T S R Q P O N M L K J I H G F E D C B A
```

The mapping formula is:

```
encrypted_char = 25 - char_position
```

Where `char_position` is 0-indexed (A=0, B=1, ..., Z=25).

### Key Properties

1. **Self-inverse.** Encrypting twice returns the original text:
   `atbash(atbash(text)) == text`. This is because reversing a reversal is the
   identity. Atbash shares this property with ROT13 (CR00).

2. **No key.** There is exactly one Atbash substitution. This makes it even
   less secure than Caesar — there is nothing to brute-force.

3. **Fixed-point at midpoint.** N (position 13) maps to M (position 12) and
   vice versa. No letter maps to itself — every letter changes.

### Historical Context

The name "Atbash" comes from the Hebrew alphabet: Aleph-Tav-Beth-Shin — the
first, last, second, and second-to-last letters. The cipher appears in the
Hebrew Bible. In the Book of Jeremiah (25:26, 51:41), "Sheshach" (שׁשׁך) is the
Atbash encoding of "Babel" (בבל).

### Why After Caesar?

Atbash is even simpler than Caesar — it removes the concept of a key entirely.
Including it in the CR series after Caesar reinforces the substitution concept
while showing that a cipher with no key provides zero security. It also
introduces the idea of involutions (self-inverse functions), which reappear in
Enigma's reflector (CR02) and in XOR-based ciphers (CR03).

## Worked Example

**Encrypt "HELLO":**

```
H (7)  → 25 - 7  = 18 → S
E (4)  → 25 - 4  = 21 → V
L (11) → 25 - 11 = 14 → O
L (11) → 25 - 11 = 14 → O
O (14) → 25 - 14 = 11 → L
```

Result: `SVOOL`

**Decrypt "SVOOL"** (same operation):

```
S (18) → 25 - 18 = 7  → H
V (21) → 25 - 21 = 4  → E
O (14) → 25 - 14 = 11 → L
O (14) → 25 - 14 = 11 → L
L (11) → 25 - 11 = 14 → O
```

Result: `HELLO`

## Interface Contract

### Functions

| Function | Signature | Description |
|----------|-----------|-------------|
| `encrypt` | `(text: string) → string` | Apply Atbash substitution |
| `decrypt` | `(text: string) → string` | Same as encrypt (self-inverse) |

### Character Handling Rules

Same as CR00 (Caesar cipher):
1. Alphabetic characters (A-Z, a-z) are substituted. Case is preserved.
2. Non-alphabetic characters pass through unchanged.
3. Empty strings return empty strings.

### Why No `brute_force` or `frequency_analysis`?

Atbash has no key, so brute force is meaningless — there is only one possible
decryption. Frequency analysis still works (the letter distribution is merely
reversed), but it is already implemented in CR00. Including it again would be
redundant.

## Package Matrix

| Language | Package Directory | Module/Namespace |
|----------|-------------------|------------------|
| Python | `code/packages/python/atbash-cipher/` | `atbash_cipher` |
| Go | `code/packages/go/atbash-cipher/` | `atbashcipher` |
| Ruby | `code/packages/ruby/atbash_cipher/` | `CodingAdventures::AtbashCipher` |
| TypeScript | `code/packages/typescript/atbash-cipher/` | `@coding-adventures/atbash-cipher` |
| Rust | `code/packages/rust/atbash-cipher/` | `atbash_cipher` |
| Elixir | `code/packages/elixir/atbash_cipher/` | `CodingAdventures.AtbashCipher` |
| Lua | `code/packages/lua/atbash-cipher/` | `coding_adventures.atbash_cipher` |
| Perl | `code/packages/perl/atbash-cipher/` | `CodingAdventures::AtbashCipher` |
| Swift | `code/packages/swift/atbash-cipher/` | `AtbashCipher` |

**Dependencies:** None. Standalone foundation package.
