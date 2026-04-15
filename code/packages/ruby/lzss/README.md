# coding_adventures_lzss — LZSS Compression (CMP02)

LZSS (1982) refines LZ77 with flag bits: literals cost 1 byte, matches cost 3 bytes.

## Usage

```ruby
require "coding_adventures_lzss"

data = "hello hello hello"
compressed = CodingAdventures::LZSS.compress(data)
CodingAdventures::LZSS.decompress(compressed) == data  # true
```

## Development

```bash
bundle install && bundle exec rake test
```
