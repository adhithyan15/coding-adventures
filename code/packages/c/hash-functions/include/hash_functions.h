/*
 * hash_functions.h — non-cryptographic hash functions, pure ISO C17.
 * ==================================================================
 *
 * A faithful port of the Rust `hash-functions` crate (the "DT17" family): a
 * grab-bag of well-known *non-cryptographic* hashes, implemented from scratch,
 * plus two quality-analysis helpers.
 *
 *   FNV-1a (32 & 64-bit) ... multiply-xor, tiny and fast, great for hash tables
 *   DJB2 ................... Bernstein's `hash*33 + c`, the classic string hash
 *   polynomial rolling .... Rabin-Karp style `sum(c_i * base^i) mod m`
 *   Murmur3 (32-bit) ...... good avalanche, the workhorse of many hash maps
 *   SipHash-2-4 ........... keyed PRF, resists hash-flooding (still not a MAC-
 *                           grade primitive - for that use the crypto hashes)
 *
 * These are DISTINCT from the cryptographic digests (sha256/sha1/md5/hmac) this
 * repo also ports: those are collision-resistant; these are fast table hashes.
 * Don't use them for security.
 *
 * Every function is deterministic integer arithmetic - unsigned overflow is the
 * intended `wrapping_*` behaviour and is fully defined in C. The one place Rust
 * reached for `u128` (the modular multiply inside polynomial rolling) is done
 * here with an exact overflow-safe `mulmod`, so results match bit-for-bit for
 * *any* 64-bit modulus.
 *
 * Pure ISO C17 - no <math.h>, no compiler extensions, no 128-bit integers.
 */
#ifndef HASH_FUNCTIONS_H
#define HASH_FUNCTIONS_H

#include <stddef.h> /* size_t */
#include <stdint.h> /* uint8_t, uint32_t, uint64_t */

#ifdef __cplusplus
extern "C" {
#endif

/* ── Named constants (mirroring the Rust `pub const`s) ─────────────────────*/

#define HF_DJB2_OFFSET_BASIS ((uint64_t)5381)
#define HF_FNV32_OFFSET_BASIS ((uint32_t)0x811C9DC5u)
#define HF_FNV32_PRIME ((uint32_t)0x01000193u)
#define HF_FNV64_OFFSET_BASIS ((uint64_t)0xCBF29CE484222325ull)
#define HF_FNV64_PRIME ((uint64_t)0x00000100000001B3ull)
#define HF_POLYNOMIAL_ROLLING_DEFAULT_BASE ((uint64_t)31)
#define HF_POLYNOMIAL_ROLLING_DEFAULT_MODULUS (((uint64_t)1 << 61) - 1)

/* ── Free functions (hash a byte range) ────────────────────────────────────*/

uint32_t hf_fnv1a_32(const uint8_t *data, size_t len);
uint64_t hf_fnv1a_64(const uint8_t *data, size_t len);
uint64_t hf_djb2(const uint8_t *data, size_t len);

/* Rabin-Karp polynomial hash with the crate's default base (31) and modulus
 * (2^61 - 1). */
uint64_t hf_polynomial_rolling(const uint8_t *data, size_t len);
/* Same, with a caller-chosen base and modulus. `modulus` must be > 0 (a zero
 * modulus returns 0, matching the crate's `assert` contract defensively). */
uint64_t hf_polynomial_rolling_with_params(const uint8_t *data, size_t len,
                                           uint64_t base, uint64_t modulus);

uint32_t hf_murmur3_32(const uint8_t *data, size_t len);
uint32_t hf_murmur3_32_with_seed(const uint8_t *data, size_t len, uint32_t seed);

/* SipHash-2-4 keyed by a 16-byte key (read little-endian as two u64 words). */
uint64_t hf_siphash_2_4(const uint8_t *data, size_t len, const uint8_t key[16]);

/* Convenience helpers over NUL-terminated strings (bytes up to, not incl, \0). */
uint32_t hf_hash_str_fnv1a_32(const char *s);
uint64_t hf_hash_str_siphash(const char *s, const uint8_t key[16]);

/* ── HashFunction: the trait, as a tagged value ────────────────────────────*/

/* Rust models these as a trait with one zero/small struct per algorithm; C
 * folds that into a single tagged struct dispatched by `hf_hash`. */
typedef enum {
    HF_KIND_FNV1A_32,
    HF_KIND_FNV1A_64,
    HF_KIND_DJB2,
    HF_KIND_POLYNOMIAL_ROLLING,
    HF_KIND_MURMUR3_32,
    HF_KIND_SIPHASH_2_4
} HfKind;

typedef struct {
    HfKind kind;
    uint64_t base;    /* polynomial rolling */
    uint64_t modulus; /* polynomial rolling */
    uint32_t seed;    /* murmur3 */
    uint8_t key[16];  /* siphash */
} HfHashFunction;

HfHashFunction hf_new_fnv1a_32(void);
HfHashFunction hf_new_fnv1a_64(void);
HfHashFunction hf_new_djb2(void);
HfHashFunction hf_new_polynomial_rolling(void); /* default base/modulus */
HfHashFunction hf_new_polynomial_rolling_with(uint64_t base, uint64_t modulus);
HfHashFunction hf_new_murmur3_32(void); /* seed 0 */
HfHashFunction hf_new_murmur3_32_with_seed(uint32_t seed);
HfHashFunction hf_new_siphash_2_4(const uint8_t key[16]);

/* Dispatch: hash `data` with `hf`, widened to u64 (32-bit hashes zero-extend,
 * matching the Rust trait's `-> u64`). */
uint64_t hf_hash(const HfHashFunction *hf, const uint8_t *data, size_t len);
/* The hash's natural output width in bits (32 or 64). */
uint32_t hf_output_bits(const HfHashFunction *hf);

/* ── Analysis helpers ──────────────────────────────────────────────────────*/

/* A hash callback for the analysis routines: hash `len` bytes at `data`,
 * returning the value widened to u64. `ctx` carries any state. */
typedef uint64_t (*HfHashCb)(const uint8_t *data, size_t len, void *ctx);

/* A byte-source callback: fill `len` bytes at `buf`. Used by avalanche scoring
 * in place of Rust's `getrandom` (which is OS entropy / FFI and has no pure-ISO
 * equivalent) - the caller supplies the randomness, exactly mirroring the
 * crate's internal `avalanche_score_with_source`. */
typedef void (*HfFillCb)(uint8_t *buf, size_t len, void *ctx);

/* Estimate the average fraction of the `output_bits` output bits that flip when
 * a single input bit (of an 8-byte input) is toggled. `sample_size` must be > 0
 * and `output_bits` in 1..=64. Returns 0.0 on a contract violation. */
double hf_avalanche_score(HfHashCb hash, void *hash_ctx, uint32_t output_bits,
                          size_t sample_size, HfFillCb fill, void *fill_ctx);

/* One input to `hf_distribution_test`. */
typedef struct {
    const uint8_t *data;
    size_t len;
} HfInput;

/* Chi-square statistic of how evenly the hash spreads `inputs` across
 * `num_buckets` buckets (0.0 = perfectly uniform). `num_buckets` must be > 0 and
 * `n_inputs` > 0. Returns a negative value (-1.0) on a contract violation or
 * allocation failure. */
double hf_distribution_test(HfHashCb hash, void *hash_ctx,
                            const HfInput *inputs, size_t n_inputs,
                            size_t num_buckets);

#ifdef __cplusplus
}
#endif

#endif /* HASH_FUNCTIONS_H */
