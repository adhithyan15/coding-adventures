# sha512 (Rust)

SHA-512 cryptographic hash function (FIPS 180-4) implemented from scratch in Rust.

## What It Does

Computes 64-byte (512-bit) digests using the SHA-2 family algorithm with native 64-bit word operations. SHA-512 processes 128-byte blocks through 80 rounds of compression.

## How It Works

SHA-512 is structurally identical to SHA-256 but uses 64-bit words. Rust's native `u64` type and `wrapping_add`/`rotate_right` methods make this implementation clean and efficient. The `rotate_right` call compiles to a single `ror` instruction on x86-64.

## Usage

```rust
use coding_adventures_sha512::{sum512, hex_string, Digest};

// One-shot hashing
let digest = sum512(b"hello");           // [u8; 64]
let hex = hex_string(b"hello");          // 128-char String

// Streaming (for large data)
let mut h = Digest::new();
h.update(b"hello ");
h.update(b"world");
assert_eq!(h.sum512(), sum512(b"hello world"));
```

## API

| Function | Returns | Description |
|----------|---------|-------------|
| `sum512(data)` | `[u8; 64]` | 64-byte digest |
| `hex_string(data)` | `String` | 128-char lowercase hex |
| `Digest::new()` | `Digest` | Streaming hasher |
| `.update(data)` | `()` | Feed bytes |
| `.sum512()` | `[u8; 64]` | Get 64-byte digest |
| `.hex_digest()` | `String` | Get 128-char hex |
| `.clone_digest()` | `Digest` | Independent copy |

## Dependencies

None. Implemented from scratch.
