# CodingAdventures::CaesarCipher

A pure-Ruby implementation of the Caesar cipher -- the oldest known substitution cipher -- with both encryption/decryption and two automated cryptanalysis techniques (brute force and chi-squared frequency analysis). Written in literate programming style so that reading the source code teaches you the underlying mathematics and history.

## What is the Caesar cipher?

The Caesar cipher is named after Julius Caesar, who used it to protect his military correspondence around 50 BCE. The idea is disarmingly simple: pick a number (the "shift" or "key"), and replace every letter in your message with the letter that is *shift* positions later in the alphabet, wrapping around from Z back to A.

For example, with a shift of 3:

```
Plain alphabet:  A B C D E F G H I J K L M N O P Q R S T U V W X Y Z
Cipher alphabet: D E F G H I J K L M N O P Q R S T U V W X Y Z A B C
```

So "HELLO" becomes "KHOOR", and "ATTACK AT DAWN" becomes "DWWDFN DW GDZQ".

The Caesar cipher is a special case of the more general *monoalphabetic substitution cipher*, where each letter maps to exactly one other letter. What makes the Caesar cipher special is that the mapping is completely determined by a single number -- the shift. This means there are only 25 possible keys (shifts 1 through 25; shift 0 leaves the text unchanged), making it trivially easy to break by modern standards.

Despite its weakness, the Caesar cipher is a wonderful teaching tool. It introduces core concepts that appear in every area of cryptography: keys, encryption, decryption, brute-force attacks, and statistical cryptanalysis.

## How it fits in the coding-adventures stack

This package lives in the `code/packages/ruby/` directory of the coding-adventures monorepo. It is a standalone gem with no runtime dependencies -- only minitest and rake for development. It serves as both a cryptography teaching tool and a reference implementation for the literate programming style used throughout the project.

The package demonstrates several Ruby idioms: `chars.map { ... }.join` for character-level transformations, `ord` and `chr` for ASCII arithmetic, `Hash.new(0)` for counting, and frozen string literals for immutability.

## Installation

Add to your Gemfile:

```ruby
gem "coding_adventures_caesar_cipher", path: "code/packages/ruby/caesar_cipher"
```

Then run:

```bash
bundle install
```

Or install directly from the gemspec:

```bash
cd code/packages/ruby/caesar_cipher
gem build coding_adventures_caesar_cipher.gemspec
gem install coding_adventures_caesar_cipher-0.1.0.gem
```

## Usage

### Basic encryption and decryption

```ruby
require "coding_adventures/caesar_cipher"

CC = CodingAdventures::CaesarCipher

# Encrypt with shift 3 (Caesar's own key)
CC.encrypt("HELLO", 3)          #=> "KHOOR"
CC.encrypt("Hello, World!", 3)  #=> "Khoor, Zruog!"

# Decrypt by providing the same shift
CC.decrypt("KHOOR", 3)          #=> "HELLO"
CC.decrypt("Khoor, Zruog!", 3)  #=> "Hello, World!"

# Negative shifts work too
CC.encrypt("ABC", -1)           #=> "ZAB"
```

Key behaviors:
- **Case is preserved**: uppercase letters stay uppercase, lowercase stays lowercase.
- **Non-alphabetic characters pass through unchanged**: digits, spaces, punctuation, and Unicode characters are not modified.
- **Any integer shift works**: negative shifts, shifts larger than 26, and shift 0 (identity) are all handled correctly via modular arithmetic.

### ROT13

ROT13 is the Caesar cipher with shift 13. Because the English alphabet has 26 letters and 13 is exactly half, applying ROT13 twice returns the original text -- the same function both encrypts and decrypts.

```ruby
CC.rot13("HELLO")               #=> "URYYB"
CC.rot13("URYYB")               #=> "HELLO"
CC.rot13(CC.rot13("anything"))  #=> "anything"
```

ROT13 was widely used on Usenet in the 1980s and 1990s to hide spoilers and punchlines. It provides no real security -- just enough obfuscation that you cannot accidentally read something you did not intend to.

### Brute-force attack

Since there are only 25 possible non-trivial shifts, we can simply try all of them and look for readable text:

```ruby
results = CC.brute_force("KHOOR")
# Returns 25 pairs: [[1, "JGNNQ"], [2, "IFMMP"], [3, "HELLO"], ...]

results.each do |shift, text|
  puts "Shift #{shift}: #{text}"
end
```

This is the simplest possible attack. A human can scan the output in seconds to find the readable English. For automated cracking, use frequency analysis.

### Frequency analysis

For longer texts, we can automatically determine the shift by comparing the letter frequency distribution of the ciphertext against the known distribution of English:

```ruby
ciphertext = CC.encrypt("the quick brown fox jumps over the lazy dog and then rests", 7)
shift, plaintext = CC.frequency_analysis(ciphertext)
shift      #=> 7
plaintext  #=> "the quick brown fox jumps over the lazy dog and then rests"
```

The algorithm works by trying each of the 26 possible shifts, decrypting with that shift, counting letter frequencies in the result, and computing the chi-squared statistic against the expected English frequencies. The shift that produces the lowest chi-squared value (closest match to English) is selected as the answer.

This technique works reliably on texts of roughly 50 characters or more. For very short texts (a few words), the statistical signal may be too weak, and brute force with human inspection is more reliable.

## The mathematics behind frequency analysis

Every natural language has a characteristic distribution of letter frequencies. In English, the letter E appears about 12.7 percent of the time, T about 9.1 percent, A about 8.2 percent, and so on down to Z at about 0.07 percent. The classic mnemonic for the most common letters is "ETAOIN SHRDLU".

When we encrypt English text with a Caesar cipher (shift s), the distribution does not change shape -- it just slides over by s positions. The letter that was E is now at position E+s. So if we can figure out how far the distribution has been shifted, we know the key.

The chi-squared statistic gives us a precise way to measure how well an observed distribution matches an expected one:

```
chi^2 = SUM over all letters: (observed_count - expected_count)^2 / expected_count
```

A lower chi-squared value means a closer match. We compute this for all 26 possible shifts and pick the one with the minimum value.

This technique was first described by the Arab scholar Al-Kindi in the 9th century, making frequency analysis one of the oldest known cryptanalytic techniques -- predating the Caesar cipher's formal description by centuries (though Caesar himself used the cipher much earlier, the mathematical framework for breaking it came later).

## API reference

| Method | Signature | Description |
|--------|-----------|-------------|
| `encrypt` | `(String, Integer) -> String` | Encrypt text with a given shift |
| `decrypt` | `(String, Integer) -> String` | Decrypt text with a given shift |
| `rot13` | `(String) -> String` | Apply ROT13 (shift 13, self-inverse) |
| `brute_force` | `(String) -> Array<[Integer, String]>` | Try all 25 shifts, return pairs |
| `frequency_analysis` | `(String) -> [Integer, String]` | Find best shift via chi-squared |

### Constants

| Constant | Type | Description |
|----------|------|-------------|
| `VERSION` | `String` | Gem version ("0.1.0") |
| `ALPHABET_SIZE` | `Integer` | 26 |
| `ENGLISH_FREQUENCIES` | `Hash<String, Float>` | Letter frequencies for a-z |

## Development

```bash
cd code/packages/ruby/caesar_cipher
bundle install
bundle exec rake test
```

## License

MIT
