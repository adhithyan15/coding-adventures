# hash-breaker (Python)

Demonstrates three attacks proving MD5 is cryptographically broken:

1. **Known Collision Pairs** - Uses the Wang/Yu (2004) collision to show two different byte sequences producing the same MD5 hash
2. **Length Extension Attack** - Forges a valid MD5-based MAC without knowing the secret key
3. **Birthday Attack** - Finds a collision on truncated MD5 (32-bit) using the birthday paradox

## Usage

```bash
python3 main.py
```

## Dependencies

- `coding-adventures-md5` - Our from-scratch MD5 implementation

## What You'll Learn

- Why MD5 collisions are practical (not just theoretical)
- How Merkle-Damgard construction enables length extension attacks
- Why HMAC exists (to prevent length extension)
- How the birthday paradox determines hash security margins
