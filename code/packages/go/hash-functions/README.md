# hash-functions

Pure non-cryptographic hash functions implemented from scratch in Go.

The package provides FNV-1a, DJB2, polynomial rolling hash, MurmurHash3, and
small deterministic analysis helpers. These hashes are useful for learning,
hash tables, Bloom filters, and other data structures. They are not password
hashes and should not be treated as cryptographic primitives.

## Usage

```go
package main

import hashfunctions "github.com/adhithyan15/coding-adventures/code/packages/go/hash-functions"

func main() {
    _ = hashfunctions.Fnv1a32([]byte("hello"))
    _ = hashfunctions.Murmur3_32([]byte("abc"))
}
```

## Development

```bash
bash BUILD
```
