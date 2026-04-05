# hash-breaker (Ruby)

Demonstrates three attacks proving MD5 is cryptographically broken:

1. **Known Collision Pairs** - Uses the Wang/Yu (2004) collision to show two different byte sequences producing the same MD5 hash
2. **Length Extension Attack** - Forges a valid MD5-based MAC without knowing the secret key
3. **Birthday Attack** - Finds a collision on truncated MD5 (32-bit) using the birthday paradox

## Usage

```bash
ruby -I../../../packages/ruby/md5/lib main.rb
```

## Dependencies

- `coding_adventures_md5` - Our from-scratch MD5 implementation
