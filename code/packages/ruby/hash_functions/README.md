# coding_adventures_hash_functions

Pure non-cryptographic hash functions implemented from scratch in Ruby.

The package includes FNV-1a, DJB2, polynomial rolling hash, MurmurHash3, and
small deterministic analysis helpers. These functions are teaching and data
structure utilities, not cryptographic password hashes.

## Usage

```ruby
require "coding_adventures_hash_functions"

CodingAdventures::HashFunctions.fnv1a32("hello")
# => 1335831723

CodingAdventures::HashFunctions.murmur3_32("abc")
# => 3017643002
```

## Development

```bash
bash BUILD
```
