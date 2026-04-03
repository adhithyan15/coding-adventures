# coding_adventures_gf256 (Ruby)

Galois Field GF(2^8) arithmetic. 256-element field for Reed-Solomon,
QR codes, and AES encryption.

## Usage

```ruby
require_relative "lib/gf256"

GF256.add(0x53, 0xCA)      # → 0x99 (XOR)
GF256.multiply(0x53, 0xCA) # → 1    (inverses!)
GF256.inverse(0x53)        # → 0xCA
GF256.power(2, 255)        # → 1    (g^255 = 1)
```

## API

- `add(a, b)`, `subtract(a, b)` — XOR
- `multiply(a, b)`, `divide(a, b)` — via log/antilog tables
- `power(base, exp)` — exponentiation
- `inverse(a)` — multiplicative inverse
- `zero`, `one` — field identities
- `log_table`, `alog_table` — table accessors
