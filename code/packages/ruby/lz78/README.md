# lz78 — LZ78 Lossless Compression Algorithm (Ruby)

LZ78 (Lempel & Ziv, 1978) explicit-dictionary compression. Part of the CMP compression series.

## Usage

```ruby
require "coding_adventures_lz78"

data = "hello hello hello world"
compressed = CodingAdventures::LZ78.compress(data)
original   = CodingAdventures::LZ78.decompress(compressed)
# original == "hello hello hello world".b

# Token-level API
tokens = CodingAdventures::LZ78.encode(data)
decoded = CodingAdventures::LZ78.decode(tokens, original_length: data.bytesize)
```

## Development

```bash
bundle install
bundle exec rake test
```
