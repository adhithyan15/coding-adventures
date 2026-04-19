// blake2b -- BLAKE2b cryptographic hash function (RFC 7693), from scratch.
//
// BLAKE2b is a modern hash faster than MD5 on 64-bit hardware and as secure
// as SHA-3 against known attacks.  Output 1..64 bytes, optional key, salt,
// and personalization.  This crate is sequential-mode only; tree hashing,
// BLAKE2s, BLAKE2bp, BLAKE2sp, BLAKE2Xb, and BLAKE3 are out of scope
// (see `code/specs/HF06-blake2b.md`).
//
// Rust has native `u64` with `wrapping_add` and `rotate_right`, so the
// implementation reads a one-to-one transliteration of the RFC with no
// masking tricks.
//
// Key invariant and classic BLAKE2 off-by-one: the *last real block* is the
// one flagged `final`.  For messages whose byte length is an exact multiple
// of 128, we do NOT emit a spurious all-zero final block -- streaming flushes
// only when the buffer *strictly exceeds* 128 bytes, so at `digest()` time
// there is always something in the buffer to flag final (even if it's a
// full block).

#![forbid(unsafe_code)]

/// BLAKE2b block size in bytes.
pub const BLOCK_SIZE: usize = 128;
/// BLAKE2b maximum digest size in bytes.
pub const MAX_DIGEST: usize = 64;
/// BLAKE2b maximum key length in bytes.
pub const MAX_KEY: usize = 64;

// Initial Hash Values -- identical to SHA-512 (fractional parts of the square
// roots of the first eight primes).
const IV: [u64; 8] = [
    0x6A09_E667_F3BC_C908,
    0xBB67_AE85_84CA_A73B,
    0x3C6E_F372_FE94_F82B,
    0xA54F_F53A_5F1D_36F1,
    0x510E_527F_ADE6_82D1,
    0x9B05_688C_2B3E_6C1F,
    0x1F83_D9AB_FB41_BD6B,
    0x5BE0_CD19_137E_2179,
];

// Ten message-schedule permutations.  Rounds 10 and 11 reuse rows 0 and 1.
const SIGMA: [[usize; 16]; 10] = [
    [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
    [14, 10, 4, 8, 9, 15, 13, 6, 1, 12, 0, 2, 11, 7, 5, 3],
    [11, 8, 12, 0, 5, 2, 15, 13, 10, 14, 3, 6, 7, 1, 9, 4],
    [7, 9, 3, 1, 13, 12, 11, 14, 2, 6, 5, 10, 4, 0, 15, 8],
    [9, 0, 5, 7, 2, 4, 10, 15, 14, 1, 11, 12, 6, 8, 3, 13],
    [2, 12, 6, 10, 0, 11, 8, 3, 4, 13, 7, 5, 15, 14, 1, 9],
    [12, 5, 1, 15, 14, 13, 4, 10, 0, 7, 6, 3, 9, 2, 8, 11],
    [13, 11, 7, 14, 12, 1, 3, 9, 5, 0, 15, 4, 8, 6, 2, 10],
    [6, 15, 14, 9, 11, 3, 0, 8, 12, 2, 13, 7, 1, 4, 10, 5],
    [10, 2, 8, 4, 7, 6, 1, 5, 15, 11, 9, 14, 3, 12, 13, 0],
];

// BLAKE2b quarter-round G.  Rotation constants (R1..R4) = (32, 24, 16, 63).
// All adds are wrapping -- the algorithm is defined over Z/2^64.
#[inline(always)]
fn mix(v: &mut [u64; 16], a: usize, b: usize, c: usize, d: usize, x: u64, y: u64) {
    v[a] = v[a].wrapping_add(v[b]).wrapping_add(x);
    v[d] = (v[d] ^ v[a]).rotate_right(32);
    v[c] = v[c].wrapping_add(v[d]);
    v[b] = (v[b] ^ v[c]).rotate_right(24);
    v[a] = v[a].wrapping_add(v[b]).wrapping_add(y);
    v[d] = (v[d] ^ v[a]).rotate_right(16);
    v[c] = v[c].wrapping_add(v[d]);
    v[b] = (v[b] ^ v[c]).rotate_right(63);
}

// Parse a 128-byte block as sixteen little-endian u64 words.
fn parse_block(block: &[u8; BLOCK_SIZE]) -> [u64; 16] {
    let mut m = [0u64; 16];
    for (i, word) in m.iter_mut().enumerate() {
        let start = i * 8;
        let bytes: [u8; 8] = block[start..start + 8].try_into().unwrap();
        *word = u64::from_le_bytes(bytes);
    }
    m
}

// Compression function F.  `t` is the 128-bit total byte count so far
// (including the bytes of the current block).  `is_final` must be true iff
// this is the last compression call; that triggers the v[14] inversion used
// to domain-separate the final block.
fn compress(h: &mut [u64; 8], block: &[u8; BLOCK_SIZE], t: u128, is_final: bool) {
    let m = parse_block(block);
    let mut v = [0u64; 16];
    v[..8].copy_from_slice(h);
    v[8..].copy_from_slice(&IV);

    // Fold the 128-bit counter into v[12..14].  Messages > 2^64 bytes are
    // not supported in practice, so the upper half usually stays zero.
    v[12] ^= t as u64;
    v[13] ^= (t >> 64) as u64;
    if is_final {
        v[14] ^= u64::MAX;
    }

    for i in 0..12 {
        let s = &SIGMA[i % 10];
        mix(&mut v, 0, 4, 8, 12, m[s[0]], m[s[1]]);
        mix(&mut v, 1, 5, 9, 13, m[s[2]], m[s[3]]);
        mix(&mut v, 2, 6, 10, 14, m[s[4]], m[s[5]]);
        mix(&mut v, 3, 7, 11, 15, m[s[6]], m[s[7]]);
        mix(&mut v, 0, 5, 10, 15, m[s[8]], m[s[9]]);
        mix(&mut v, 1, 6, 11, 12, m[s[10]], m[s[11]]);
        mix(&mut v, 2, 7, 8, 13, m[s[12]], m[s[13]]);
        mix(&mut v, 3, 4, 9, 14, m[s[14]], m[s[15]]);
    }

    // Davies-Meyer feed-forward: XOR both halves of v into the state.
    for i in 0..8 {
        h[i] ^= v[i] ^ v[i + 8];
    }
}

// Build the parameter-block-XOR-ed starting state.  Sequential mode only
// (fanout=1, depth=1).
fn initial_state(digest_size: u8, key_len: u8, salt: &[u8], personal: &[u8]) -> [u64; 8] {
    let mut p = [0u8; 64];
    p[0] = digest_size;
    p[1] = key_len;
    p[2] = 1; // fanout
    p[3] = 1; // depth
    if !salt.is_empty() {
        p[32..48].copy_from_slice(salt);
    }
    if !personal.is_empty() {
        p[48..64].copy_from_slice(personal);
    }

    let mut state = IV;
    for (i, word) in state.iter_mut().enumerate() {
        let bytes: [u8; 8] = p[i * 8..i * 8 + 8].try_into().unwrap();
        *word ^= u64::from_le_bytes(bytes);
    }
    state
}

/// Errors returned when options fail validation.
#[derive(Debug, PartialEq, Eq)]
pub enum Blake2bError {
    /// `digest_size` must be in 1..=64.
    InvalidDigestSize(usize),
    /// `key` length must be in 0..=64.
    KeyTooLong(usize),
    /// `salt` must be empty or exactly 16 bytes.
    InvalidSaltLen(usize),
    /// `personal` must be empty or exactly 16 bytes.
    InvalidPersonalLen(usize),
}

impl core::fmt::Display for Blake2bError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidDigestSize(n) => write!(f, "digest_size must be in [1, 64], got {n}"),
            Self::KeyTooLong(n) => write!(f, "key length must be in [0, 64], got {n}"),
            Self::InvalidSaltLen(n) => write!(f, "salt must be exactly 16 bytes (or empty), got {n}"),
            Self::InvalidPersonalLen(n) => {
                write!(f, "personal must be exactly 16 bytes (or empty), got {n}")
            }
        }
    }
}

impl std::error::Error for Blake2bError {}

/// BLAKE2b options builder.  All fields are optional; defaults are 64-byte
/// digest, unkeyed, no salt, no personalization.
#[derive(Clone, Default)]
pub struct Blake2bOptions<'a> {
    pub digest_size: Option<usize>,
    pub key: Option<&'a [u8]>,
    pub salt: Option<&'a [u8]>,
    pub personal: Option<&'a [u8]>,
}

impl<'a> Blake2bOptions<'a> {
    /// Zero-configured options equivalent to `Default::default()`.
    pub fn new() -> Self {
        Self::default()
    }
    pub fn digest_size(mut self, n: usize) -> Self {
        self.digest_size = Some(n);
        self
    }
    pub fn key(mut self, k: &'a [u8]) -> Self {
        self.key = Some(k);
        self
    }
    pub fn salt(mut self, s: &'a [u8]) -> Self {
        self.salt = Some(s);
        self
    }
    pub fn personal(mut self, p: &'a [u8]) -> Self {
        self.personal = Some(p);
        self
    }
}

fn validate(
    digest_size: usize,
    key: &[u8],
    salt: &[u8],
    personal: &[u8],
) -> Result<(), Blake2bError> {
    if digest_size < 1 || digest_size > MAX_DIGEST {
        return Err(Blake2bError::InvalidDigestSize(digest_size));
    }
    if key.len() > MAX_KEY {
        return Err(Blake2bError::KeyTooLong(key.len()));
    }
    if !salt.is_empty() && salt.len() != 16 {
        return Err(Blake2bError::InvalidSaltLen(salt.len()));
    }
    if !personal.is_empty() && personal.len() != 16 {
        return Err(Blake2bError::InvalidPersonalLen(personal.len()));
    }
    Ok(())
}

/// Streaming BLAKE2b hasher.  `digest()` is non-destructive; repeated calls
/// return the same bytes and the hasher remains usable for further `update`.
#[derive(Clone, Debug)]
pub struct Blake2bHasher {
    state: [u64; 8],
    buffer: Vec<u8>,
    byte_count: u128,
    digest_size: usize,
}

impl Blake2bHasher {
    /// Construct a hasher with the given options.
    pub fn new(opts: &Blake2bOptions) -> Result<Self, Blake2bError> {
        let digest_size = opts.digest_size.unwrap_or(MAX_DIGEST);
        let key = opts.key.unwrap_or(&[]);
        let salt = opts.salt.unwrap_or(&[]);
        let personal = opts.personal.unwrap_or(&[]);
        validate(digest_size, key, salt, personal)?;

        let state = initial_state(digest_size as u8, key.len() as u8, salt, personal);

        // Keyed mode: prepend the key zero-padded to one full block.
        let buffer = if key.is_empty() {
            Vec::with_capacity(BLOCK_SIZE)
        } else {
            let mut b = vec![0u8; BLOCK_SIZE];
            b[..key.len()].copy_from_slice(key);
            b
        };

        Ok(Self {
            state,
            buffer,
            byte_count: 0,
            digest_size,
        })
    }

    /// Append bytes to the message.  Flushes full blocks except the latest:
    /// we always keep at least one byte in the buffer until `digest` so the
    /// final compression is the one flagged final.
    pub fn update(&mut self, data: &[u8]) -> &mut Self {
        self.buffer.extend_from_slice(data);
        while self.buffer.len() > BLOCK_SIZE {
            self.byte_count = self.byte_count.wrapping_add(BLOCK_SIZE as u128);
            let mut block = [0u8; BLOCK_SIZE];
            block.copy_from_slice(&self.buffer[..BLOCK_SIZE]);
            compress(&mut self.state, &block, self.byte_count, false);
            self.buffer.drain(..BLOCK_SIZE);
        }
        self
    }

    /// Finalize and return the digest.  Non-destructive: the hasher keeps
    /// its state, so `digest` can be called again or followed by more
    /// `update` calls.
    pub fn digest(&self) -> Vec<u8> {
        let mut state = self.state;
        let mut final_block = [0u8; BLOCK_SIZE];
        final_block[..self.buffer.len()].copy_from_slice(&self.buffer);
        let total = self.byte_count + self.buffer.len() as u128;
        compress(&mut state, &final_block, total, true);

        let mut out = [0u8; 64];
        for (i, word) in state.iter().enumerate() {
            out[i * 8..i * 8 + 8].copy_from_slice(&word.to_le_bytes());
        }
        out[..self.digest_size].to_vec()
    }

    /// Finalize and return the digest as lowercase hex.
    pub fn hex_digest(&self) -> String {
        bytes_to_hex(&self.digest())
    }

    /// Return an independent clone at this point in the stream.
    pub fn copy(&self) -> Self {
        self.clone()
    }
}

fn bytes_to_hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{:02x}", b));
    }
    s
}

/// One-shot BLAKE2b.  Returns raw bytes of length `digest_size`.
pub fn blake2b(data: &[u8], opts: &Blake2bOptions) -> Result<Vec<u8>, Blake2bError> {
    let mut h = Blake2bHasher::new(opts)?;
    h.update(data);
    Ok(h.digest())
}

/// One-shot BLAKE2b returning lowercase hex.
pub fn blake2b_hex(data: &[u8], opts: &Blake2bOptions) -> Result<String, Blake2bError> {
    Ok(bytes_to_hex(&blake2b(data, opts)?))
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- Helpers ---

    fn bytes_from_range(start: u16, end: u16) -> Vec<u8> {
        (start..end).map(|i| (i & 0xff) as u8).collect()
    }

    fn hex(b: &[u8]) -> String {
        bytes_to_hex(b)
    }

    fn one_shot_hex(data: &[u8]) -> String {
        blake2b_hex(data, &Blake2bOptions::new()).unwrap()
    }

    // --- Canonical vectors (Python hashlib.blake2b oracle) ---

    #[test]
    fn empty_default() {
        assert_eq!(
            one_shot_hex(&[]),
            "786a02f742015903c6c6fd852552d272912f4740e15847618a86e217f71f5419d25e1031afee585313896444934eb04b903a685b1448b755d56f701afe9be2ce"
        );
    }

    #[test]
    fn abc() {
        assert_eq!(
            one_shot_hex(b"abc"),
            "ba80a53f981c4d0d6a2797b69f12f6e94c212f14685ac4b74b12bb6fdbffa2d17d87c5392aab792dc252d5de4533cc9518d38aa8dbf1925ab92386edd4009923"
        );
    }

    #[test]
    fn fox() {
        assert_eq!(
            one_shot_hex(b"The quick brown fox jumps over the lazy dog"),
            "a8add4bdddfd93e4877d2746e62817b116364a1fa7bc148d95090bc7333b3673f82401cf7aa2e4cb1ecd90296e3f14cb5413f8ed77be73045b13914cdcd6a918"
        );
    }

    #[test]
    fn truncated_digest_size_32() {
        let got = blake2b_hex(&[], &Blake2bOptions::new().digest_size(32)).unwrap();
        assert_eq!(
            got,
            "0e5751c026e543b2e8ab2eb06099daa1d1e5df47778f7787faab45cdf12fe3a8"
        );
    }

    #[test]
    fn keyed_long_vector() {
        let key = bytes_from_range(1, 65);
        let data = bytes_from_range(0, 256);
        let got = blake2b_hex(&data, &Blake2bOptions::new().key(&key)).unwrap();
        assert_eq!(
            got,
            "402fa70e35f026c9bfc1202805e931b995647fe479e1701ad8b7203cddad5927ee7950b898a5a8229443d93963e4f6f27136b2b56f6845ab18f59bc130db8bf3"
        );
    }

    // --- Block-boundary sizes (mirrors the TS/Go/Python test matrix) ---

    #[test]
    fn block_boundary_sizes() {
        let cases: &[(usize, &str)] = &[
            (0, "786a02f742015903c6c6fd852552d272912f4740e15847618a86e217f71f5419d25e1031afee585313896444934eb04b903a685b1448b755d56f701afe9be2ce"),
            (1, "4fe4da61bcc756071b226843361d74944c72245d23e8245ea678c13fdcd7fe2ae529cf999ad99cc24f7a73416a18ba53e76c0afef83b16a568b12fbfc1a2674d"),
            (63, "70b2a0e6daecac22c7a2df82c06e3fc0b4c66bd5ef8098e4ed54e723b393d79ef3bceba079a01a14c6ef2ae2ed1171df1662cd14ef38e6f77b01c7f48144dd09"),
            (64, "3db7bb5c40745f0c975ac6bb8578f590e2cd2cc1fc6d13533ef725325c9fddff5cca24e7a591a0f6032a24fad0e09f6df873c4ff314628391f78df7f09cb7ed7"),
            (65, "149c114a3e8c6e06bafee27c9d0de0e39ef28294fa0d9f81876dcceb10bb41101e256593587e46b844819ed7ded90d56c0843df06c95d1695c3de635cd7a888e"),
            (127, "71546bbf9110ad184cc60f2eb120fcfd9b4dbbca7a7f1270045b8a23a6a4f4330f65c1f030dd2f5fabc6c57617242c37cf427bd90407fac5b9deffd3ae888c39"),
            (128, "2d9e329f42afa3601d646692b81c13e87fcaff5bf15972e9813d7373cb6d181f9599f4d513d4af4fd6ebd37497aceb29aba5ee23ed764d8510b552bd088814fb"),
            (129, "47889df9eb4d717afc5019df5c6a83df00a0b8677395e078cd5778ace0f338a618e68b7d9afb065d9e6a01ccd31d109447e7fae771c3ee3e105709194122ba2b"),
            (255, "1a5199ac66a00e8a87ad1c7fbad30b33137dd8312bf6d98602dacf8f40ea2cb623a7fbc63e5a6bfa434d337ae7da5ca1a52502a215a3fe0297a151be85d88789"),
            (256, "91019c558584980249ca43eceed27e19f1c3c24161b93eed1eee2a6a774f60bf8a81b43750870bee1698feac9c5336ae4d5c842e7ead159bf3916387e8ded9ae"),
            (257, "9f1975efca45e7b74b020975d4d2c22802906ed8bfefca51ac497bd23147fc8f303890d8e5471ab6caaa02362e831a9e8d3435279912ccd4842c7806b096c348"),
            (1024, "eddc3f3af9392eff065b359ce5f2b28f71e9f3a3a50e60ec27787b9fa623094d17b046c1dfce89bc5cdfc951b95a9a9c05fb8cc2361c905db01dd237fe56efb3"),
            (4096, "31404c9c7ed64c59112579f300f2afef181ee6283c3918bf026c4ed4bcde0697a7834f3a3410396622ef3d4f432602528a689498141c184cc2063554ba688dc7"),
            (9999, "b4a5808e65d7424b517bde11e04075a09b1343148e3ab2c8b13ff35c542e0a2beff6309ecc54b59ac046f6d65a9e3680c6372a033607709c95d5fd8070be6069"),
        ];
        for (size, want) in cases {
            let data: Vec<u8> = (0..*size).map(|i| ((i * 7 + 3) & 0xff) as u8).collect();
            assert_eq!(one_shot_hex(&data), *want, "size {}", size);
        }
    }

    // --- Variable digest sizes ---

    #[test]
    fn variable_digest_sizes() {
        let data = b"The quick brown fox jumps over the lazy dog";
        let cases: &[(usize, &str)] = &[
            (1, "b5"),
            (16, "249df9a49f517ddcd37f5c897620ec73"),
            (20, "3c523ed102ab45a37d54f5610d5a983162fde84f"),
            (32, "01718cec35cd3d796dd00020e0bfecb473ad23457d063b75eff29c0ffa2e58a9"),
            (48, "b7c81b228b6bd912930e8f0b5387989691c1cee1e65aade4da3b86a3c9f678fc8018f6ed9e2906720c8d2a3aeda9c03d"),
            (64, "a8add4bdddfd93e4877d2746e62817b116364a1fa7bc148d95090bc7333b3673f82401cf7aa2e4cb1ecd90296e3f14cb5413f8ed77be73045b13914cdcd6a918"),
        ];
        for (ds, want) in cases {
            let out = blake2b(data, &Blake2bOptions::new().digest_size(*ds)).unwrap();
            assert_eq!(out.len(), *ds);
            assert_eq!(hex(&out), *want, "digest_size {}", ds);
        }
    }

    // --- Keyed across multiple key lengths ---

    #[test]
    fn keyed_variants() {
        let data = b"secret message body";
        let cases: &[(u16, &str)] = &[
            (1, "affd4e429aa2fb18da276f6ecff16f7d048769cacefe1a7ac75184448e082422"),
            (16, "5f8510d05dac42e8b6fc542af93f349d41ae4ebaf5cecae4af43fae54c7ca618"),
            (32, "88a78036d5890e91b5e3d70ba4738d2be302b76e0857d8ee029dc56dfa04fe67"),
            (64, "df7eab2ec9135ab8c58f48c288cdc873bac245a7fa46ca9f047cab672bd1eabb"),
        ];
        for (klen, want) in cases {
            let key = bytes_from_range(1, klen + 1);
            let got = blake2b_hex(
                data,
                &Blake2bOptions::new().key(&key).digest_size(32),
            )
            .unwrap();
            assert_eq!(got, *want, "keyLen {}", klen);
        }
    }

    #[test]
    fn salt_and_personal() {
        let salt = bytes_from_range(0, 16);
        let personal = bytes_from_range(16, 32);
        let got = blake2b_hex(
            b"parameterized hash",
            &Blake2bOptions::new().salt(&salt).personal(&personal),
        )
        .unwrap();
        assert_eq!(
            got,
            "a2185d648fc63f3d363871a76360330c9b238af5466a20f94bb64d363289b95da0453438eea300cd6f31521274ec001011fa29e91a603fabf00f2b454e30bf3d"
        );
    }

    // --- Streaming ---

    #[test]
    fn streaming_single_chunk_matches_one_shot() {
        let mut h = Blake2bHasher::new(&Blake2bOptions::new()).unwrap();
        h.update(b"hello world");
        assert_eq!(hex(&h.digest()), one_shot_hex(b"hello world"));
    }

    #[test]
    fn streaming_byte_by_byte_matches_one_shot() {
        let data = bytes_from_range(0, 200);
        let mut h = Blake2bHasher::new(&Blake2bOptions::new().digest_size(32)).unwrap();
        for b in &data {
            h.update(&[*b]);
        }
        let want = blake2b(&data, &Blake2bOptions::new().digest_size(32)).unwrap();
        assert_eq!(hex(&h.digest()), hex(&want));
    }

    #[test]
    fn streaming_chunks_across_block_boundary() {
        let data = bytes_from_range(0, 129);
        let mut h = Blake2bHasher::new(&Blake2bOptions::new()).unwrap();
        h.update(&data[..127]);
        h.update(&data[127..]);
        assert_eq!(hex(&h.digest()), one_shot_hex(&data));
    }

    #[test]
    fn streaming_exact_block_then_more_off_by_one() {
        // 128 bytes exact, then 4 more.  The 128-byte block must NOT be
        // flagged final while more data is still coming.
        let data: Vec<u8> = (0..132u16).map(|i| (i & 0xff) as u8).collect();
        let mut h = Blake2bHasher::new(&Blake2bOptions::new()).unwrap();
        h.update(&data[..128]);
        h.update(&data[128..]);
        assert_eq!(hex(&h.digest()), one_shot_hex(&data));
    }

    #[test]
    fn digest_is_idempotent() {
        let mut h = Blake2bHasher::new(&Blake2bOptions::new()).unwrap();
        h.update(b"hello");
        let a = h.hex_digest();
        let b = h.hex_digest();
        assert_eq!(a, b);
    }

    #[test]
    fn update_after_digest_continues_stream() {
        let mut h = Blake2bHasher::new(&Blake2bOptions::new().digest_size(32)).unwrap();
        h.update(b"hello ");
        let _ = h.digest();
        h.update(b"world");
        let want = blake2b_hex(b"hello world", &Blake2bOptions::new().digest_size(32)).unwrap();
        assert_eq!(h.hex_digest(), want);
    }

    #[test]
    fn copy_is_independent() {
        let mut h = Blake2bHasher::new(&Blake2bOptions::new()).unwrap();
        h.update(b"prefix ");
        let mut clone = h.copy();
        h.update(b"path A");
        clone.update(b"path B");
        assert_eq!(hex(&h.digest()), one_shot_hex(b"prefix path A"));
        assert_eq!(hex(&clone.digest()), one_shot_hex(b"prefix path B"));
    }

    // --- Validation ---

    #[test]
    fn rejects_digest_size_zero() {
        let err = Blake2bHasher::new(&Blake2bOptions::new().digest_size(0)).unwrap_err();
        assert_eq!(err, Blake2bError::InvalidDigestSize(0));
    }

    #[test]
    fn rejects_digest_size_65() {
        let err = Blake2bHasher::new(&Blake2bOptions::new().digest_size(65)).unwrap_err();
        assert_eq!(err, Blake2bError::InvalidDigestSize(65));
    }

    #[test]
    fn rejects_key_too_long() {
        let key = [0u8; 65];
        let err = Blake2bHasher::new(&Blake2bOptions::new().key(&key)).unwrap_err();
        assert_eq!(err, Blake2bError::KeyTooLong(65));
    }

    #[test]
    fn rejects_wrong_salt_length() {
        let salt = [0u8; 8];
        let err = Blake2bHasher::new(&Blake2bOptions::new().salt(&salt)).unwrap_err();
        assert_eq!(err, Blake2bError::InvalidSaltLen(8));
    }

    #[test]
    fn rejects_wrong_personal_length() {
        let personal = [0u8; 20];
        let err = Blake2bHasher::new(&Blake2bOptions::new().personal(&personal)).unwrap_err();
        assert_eq!(err, Blake2bError::InvalidPersonalLen(20));
    }

    #[test]
    fn accepts_max_64_byte_key() {
        let key = [0x41u8; 64];
        let got = blake2b(b"x", &Blake2bOptions::new().key(&key));
        assert!(got.is_ok());
    }

    #[test]
    fn error_display() {
        // All variants have human-readable messages.
        for e in &[
            Blake2bError::InvalidDigestSize(0),
            Blake2bError::KeyTooLong(65),
            Blake2bError::InvalidSaltLen(8),
            Blake2bError::InvalidPersonalLen(20),
        ] {
            assert!(!format!("{e}").is_empty());
        }
    }

    #[test]
    fn hex_digest_matches_bytes_to_hex_of_digest() {
        let mut h = Blake2bHasher::new(&Blake2bOptions::new().digest_size(32)).unwrap();
        h.update(b"hex check");
        assert_eq!(h.hex_digest(), hex(&h.digest()));
    }
}
