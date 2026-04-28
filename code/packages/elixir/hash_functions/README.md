# hash_functions

Pure non-cryptographic hash functions implemented from scratch in Elixir.

The package includes FNV-1a, DJB2, polynomial rolling hash, MurmurHash3, and
small deterministic analysis helpers. These hashes are useful for teaching and
data structures, not password storage or signatures.

## Usage

```elixir
CodingAdventures.HashFunctions.fnv1a32("hello")
# => 1335831723

CodingAdventures.HashFunctions.murmur3_32("abc")
# => 3017643002
```

## Development

```bash
mix deps.get && mix test --cover
```
