//! # Argon2i — data-independent memory-hard password hashing (RFC 9106)
//!
//! Argon2i picks reference blocks from a deterministic pseudo-random stream
//! that does *not* depend on the password or any memory contents. The memory
//! access pattern is constant across secrets, which defeats side-channel
//! observers at the cost of making the variant the easiest for GPUs/ASICs
//! to parallelise. For password hashing prefer [`argon2id`](../argon2id).
//!
//! Reference: <https://datatracker.ietf.org/doc/html/rfc9106>
//! Spec: `code/specs/KD03-argon2.md`

#![forbid(unsafe_code)]

use coding_adventures_blake2b::{blake2b, Blake2bOptions};

const MASK32: u64 = 0xFFFF_FFFF;
const BLOCK_SIZE: usize = 1024;
const BLOCK_WORDS: usize = BLOCK_SIZE / 8;
const SYNC_POINTS: usize = 4;
const ADDRESSES_PER_BLOCK: usize = BLOCK_WORDS;

pub const VERSION: u32 = 0x13;
const TYPE_I: u32 = 1;

#[derive(Debug, Clone)]
pub enum Argon2Error {
    PasswordTooLong(usize),
    SaltTooShort(usize),
    SaltTooLong(usize),
    KeyTooLong(usize),
    AssociatedDataTooLong(usize),
    TagLengthTooSmall(usize),
    TagLengthTooLarge(usize),
    InvalidParallelism(u32),
    MemoryTooSmall { got: u32, min: u32 },
    TimeCostZero,
    UnsupportedVersion(u32),
    Blake2b(String),
}

impl std::fmt::Display for Argon2Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PasswordTooLong(n) => write!(f, "password length must fit in 32 bits, got {}", n),
            Self::SaltTooShort(n) => write!(f, "salt must be at least 8 bytes, got {}", n),
            Self::SaltTooLong(n) => write!(f, "salt length must fit in 32 bits, got {}", n),
            Self::KeyTooLong(n) => write!(f, "key length must fit in 32 bits, got {}", n),
            Self::AssociatedDataTooLong(n) => write!(f, "associated data length must fit in 32 bits, got {}", n),
            Self::TagLengthTooSmall(n) => write!(f, "tagLength must be >= 4, got {}", n),
            Self::TagLengthTooLarge(n) => write!(f, "tagLength must fit in 32 bits, got {}", n),
            Self::InvalidParallelism(n) => write!(f, "parallelism must be in [1, 2^24-1], got {}", n),
            Self::MemoryTooSmall { got, min } => write!(f, "memoryCost must be >= 8*parallelism ({}), got {}", min, got),
            Self::TimeCostZero => write!(f, "timeCost must be >= 1"),
            Self::UnsupportedVersion(v) => write!(f, "only Argon2 v1.3 (0x13) is supported; got 0x{:02x}", v),
            Self::Blake2b(s) => write!(f, "blake2b error: {}", s),
        }
    }
}

impl std::error::Error for Argon2Error {}

#[derive(Debug, Clone, Default)]
pub struct Options<'a> {
    pub key: Option<&'a [u8]>,
    pub associated_data: Option<&'a [u8]>,
    pub version: Option<u32>,
}

#[inline(always)]
fn rotr64(x: u64, n: u32) -> u64 {
    x.rotate_right(n)
}

fn g_b(v: &mut [u64], a: usize, b: usize, c: usize, d: usize) {
    let (mut va, mut vb, mut vc, mut vd) = (v[a], v[b], v[c], v[d]);

    va = va.wrapping_add(vb).wrapping_add(2u64.wrapping_mul(va & MASK32).wrapping_mul(vb & MASK32));
    vd = rotr64(vd ^ va, 32);
    vc = vc.wrapping_add(vd).wrapping_add(2u64.wrapping_mul(vc & MASK32).wrapping_mul(vd & MASK32));
    vb = rotr64(vb ^ vc, 24);
    va = va.wrapping_add(vb).wrapping_add(2u64.wrapping_mul(va & MASK32).wrapping_mul(vb & MASK32));
    vd = rotr64(vd ^ va, 16);
    vc = vc.wrapping_add(vd).wrapping_add(2u64.wrapping_mul(vc & MASK32).wrapping_mul(vd & MASK32));
    vb = rotr64(vb ^ vc, 63);

    v[a] = va;
    v[b] = vb;
    v[c] = vc;
    v[d] = vd;
}

fn permutation_p(v: &mut [u64]) {
    g_b(v, 0, 4, 8, 12);
    g_b(v, 1, 5, 9, 13);
    g_b(v, 2, 6, 10, 14);
    g_b(v, 3, 7, 11, 15);
    g_b(v, 0, 5, 10, 15);
    g_b(v, 1, 6, 11, 12);
    g_b(v, 2, 7, 8, 13);
    g_b(v, 3, 4, 9, 14);
}

fn compress(x: &[u64], y: &[u64]) -> Vec<u64> {
    let mut r = vec![0u64; BLOCK_WORDS];
    for i in 0..BLOCK_WORDS {
        r[i] = x[i] ^ y[i];
    }
    let mut q = r.clone();

    for i in 0..8 {
        permutation_p(&mut q[i * 16..(i + 1) * 16]);
    }

    let mut col = vec![0u64; 16];
    for c in 0..8 {
        for row in 0..8 {
            col[2 * row] = q[row * 16 + 2 * c];
            col[2 * row + 1] = q[row * 16 + 2 * c + 1];
        }
        permutation_p(&mut col);
        for row in 0..8 {
            q[row * 16 + 2 * c] = col[2 * row];
            q[row * 16 + 2 * c + 1] = col[2 * row + 1];
        }
    }

    let mut out = vec![0u64; BLOCK_WORDS];
    for i in 0..BLOCK_WORDS {
        out[i] = r[i] ^ q[i];
    }
    out
}

fn block_to_bytes(block: &[u64]) -> Vec<u8> {
    let mut out = Vec::with_capacity(BLOCK_SIZE);
    for w in block {
        out.extend_from_slice(&w.to_le_bytes());
    }
    out
}

fn bytes_to_block(data: &[u8]) -> Vec<u64> {
    let mut out = vec![0u64; BLOCK_WORDS];
    for i in 0..BLOCK_WORDS {
        out[i] = u64::from_le_bytes(data[i * 8..i * 8 + 8].try_into().unwrap());
    }
    out
}

#[inline(always)]
fn le32(n: u32) -> [u8; 4] {
    n.to_le_bytes()
}

fn blake2b_long(t: usize, x: &[u8]) -> Result<Vec<u8>, Argon2Error> {
    if t == 0 {
        return Err(Argon2Error::Blake2b("H' output length must be positive".into()));
    }
    let prefix = (t as u32).to_le_bytes();
    let mut input = Vec::with_capacity(4 + x.len());
    input.extend_from_slice(&prefix);
    input.extend_from_slice(x);

    if t <= 64 {
        return blake2b(&input, &Blake2bOptions::new().digest_size(t))
            .map_err(|e| Argon2Error::Blake2b(format!("{:?}", e)));
    }

    let r = (t + 31) / 32 - 2;
    let mut v = blake2b(&input, &Blake2bOptions::new().digest_size(64))
        .map_err(|e| Argon2Error::Blake2b(format!("{:?}", e)))?;
    let mut out = Vec::with_capacity(t);
    out.extend_from_slice(&v[..32]);
    for _ in 1..r {
        v = blake2b(&v, &Blake2bOptions::new().digest_size(64))
            .map_err(|e| Argon2Error::Blake2b(format!("{:?}", e)))?;
        out.extend_from_slice(&v[..32]);
    }
    let final_size = t - 32 * r;
    let last = blake2b(&v, &Blake2bOptions::new().digest_size(final_size))
        .map_err(|e| Argon2Error::Blake2b(format!("{:?}", e)))?;
    out.extend_from_slice(&last);
    Ok(out)
}

fn index_alpha(j1: u64, r: usize, sl: usize, c: usize, same_lane: bool, q: usize, sl_len: usize) -> usize {
    let (w, start): (usize, usize);
    if r == 0 && sl == 0 {
        w = c - 1;
        start = 0;
    } else if r == 0 {
        w = if same_lane { sl * sl_len + c - 1 }
            else if c == 0 { sl * sl_len - 1 }
            else { sl * sl_len };
        start = 0;
    } else {
        w = if same_lane { q - sl_len + c - 1 }
            else if c == 0 { q - sl_len - 1 }
            else { q - sl_len };
        start = ((sl + 1) * sl_len) % q;
    }

    let x = (j1.wrapping_mul(j1)) >> 32;
    let y = ((w as u64).wrapping_mul(x)) >> 32;
    let rel = (w as i64) - 1 - (y as i64);

    ((start as i64 + rel).rem_euclid(q as i64)) as usize
}

/// Stateful generator of (J1, J2) pairs for Argon2i's data-independent
/// addressing (RFC 9106 §3.4.2). Each call yields the next u64 from a
/// 1024-byte pseudorandom block regenerated every `ADDRESSES_PER_BLOCK` calls.
struct AddressStream {
    r: u64,
    lane: u64,
    sl: u64,
    m_prime: u64,
    t: u64,
    type_i: u64,
    counter: u64,
    buf: Vec<u64>,
    idx: usize,
}

impl AddressStream {
    fn new(r: usize, lane: usize, sl: usize, m_prime: usize, t: usize) -> Self {
        Self {
            r: r as u64,
            lane: lane as u64,
            sl: sl as u64,
            m_prime: m_prime as u64,
            t: t as u64,
            type_i: TYPE_I as u64,
            counter: 0,
            buf: vec![0u64; BLOCK_WORDS],
            idx: ADDRESSES_PER_BLOCK, // force a refill on first call
        }
    }

    fn next(&mut self) -> u64 {
        if self.idx >= ADDRESSES_PER_BLOCK {
            self.counter += 1;
            let zero = vec![0u64; BLOCK_WORDS];
            let mut input_block = vec![0u64; BLOCK_WORDS];
            input_block[0] = self.r;
            input_block[1] = self.lane;
            input_block[2] = self.sl;
            input_block[3] = self.m_prime;
            input_block[4] = self.t;
            input_block[5] = self.type_i;
            input_block[6] = self.counter;
            let once = compress(&zero, &input_block);
            self.buf = compress(&zero, &once);
            self.idx = 0;
        }
        let v = self.buf[self.idx];
        self.idx += 1;
        v
    }
}

fn fill_segment(
    memory: &mut [Vec<Vec<u64>>],
    r: usize,
    lane: usize,
    sl: usize,
    q: usize,
    sl_len: usize,
    p: usize,
    m_prime: usize,
    t: usize,
) {
    let starting_c = if r == 0 && sl == 0 { 2 } else { 0 };

    let mut addr = AddressStream::new(r, lane, sl, m_prime, t);
    for _ in 0..starting_c {
        addr.next();
    }

    for i in starting_c..sl_len {
        let col = sl * sl_len + i;
        let prev_col = if col == 0 { q - 1 } else { col - 1 };
        let prev_block = memory[lane][prev_col].clone();

        let pseudo_rand = addr.next();
        let j1 = pseudo_rand & MASK32;
        let j2 = (pseudo_rand >> 32) & MASK32;

        let l_prime = if r == 0 && sl == 0 {
            lane
        } else {
            (j2 % p as u64) as usize
        };
        let z_prime = index_alpha(j1, r, sl, i, l_prime == lane, q, sl_len);
        let ref_block = memory[l_prime][z_prime].clone();

        let new_block = compress(&prev_block, &ref_block);
        if r == 0 {
            memory[lane][col] = new_block;
        } else {
            let existing = &memory[lane][col];
            let merged: Vec<u64> = existing
                .iter()
                .zip(new_block.iter())
                .map(|(a, b)| a ^ b)
                .collect();
            memory[lane][col] = merged;
        }
    }
}

fn validate(
    password: &[u8],
    salt: &[u8],
    time_cost: u32,
    memory_cost: u32,
    parallelism: u32,
    tag_length: u32,
    key: &[u8],
    ad: &[u8],
    version: u32,
) -> Result<(), Argon2Error> {
    if password.len() as u64 > 0xFFFF_FFFF {
        return Err(Argon2Error::PasswordTooLong(password.len()));
    }
    if salt.len() < 8 {
        return Err(Argon2Error::SaltTooShort(salt.len()));
    }
    if salt.len() as u64 > 0xFFFF_FFFF {
        return Err(Argon2Error::SaltTooLong(salt.len()));
    }
    if key.len() as u64 > 0xFFFF_FFFF {
        return Err(Argon2Error::KeyTooLong(key.len()));
    }
    if ad.len() as u64 > 0xFFFF_FFFF {
        return Err(Argon2Error::AssociatedDataTooLong(ad.len()));
    }
    if tag_length < 4 {
        return Err(Argon2Error::TagLengthTooSmall(tag_length as usize));
    }
    if parallelism < 1 || parallelism > 0xFF_FFFF {
        return Err(Argon2Error::InvalidParallelism(parallelism));
    }
    if memory_cost < 8 * parallelism {
        return Err(Argon2Error::MemoryTooSmall { got: memory_cost, min: 8 * parallelism });
    }
    if time_cost < 1 {
        return Err(Argon2Error::TimeCostZero);
    }
    if version != VERSION {
        return Err(Argon2Error::UnsupportedVersion(version));
    }
    Ok(())
}

pub fn argon2i(
    password: &[u8],
    salt: &[u8],
    time_cost: u32,
    memory_cost: u32,
    parallelism: u32,
    tag_length: u32,
    opts: &Options,
) -> Result<Vec<u8>, Argon2Error> {
    let key = opts.key.unwrap_or(&[]);
    let ad = opts.associated_data.unwrap_or(&[]);
    let version = opts.version.unwrap_or(VERSION);

    validate(password, salt, time_cost, memory_cost, parallelism, tag_length, key, ad, version)?;

    let p = parallelism as usize;
    let t = time_cost as usize;
    let segment_length = (memory_cost / (SYNC_POINTS as u32 * parallelism)) as usize;
    let m_prime = segment_length * SYNC_POINTS * p;
    let q = m_prime / p;
    let sl_len = segment_length;

    let mut h0_in: Vec<u8> = Vec::new();
    h0_in.extend_from_slice(&le32(p as u32));
    h0_in.extend_from_slice(&le32(tag_length));
    h0_in.extend_from_slice(&le32(memory_cost));
    h0_in.extend_from_slice(&le32(t as u32));
    h0_in.extend_from_slice(&le32(version));
    h0_in.extend_from_slice(&le32(TYPE_I));
    h0_in.extend_from_slice(&le32(password.len() as u32));
    h0_in.extend_from_slice(password);
    h0_in.extend_from_slice(&le32(salt.len() as u32));
    h0_in.extend_from_slice(salt);
    h0_in.extend_from_slice(&le32(key.len() as u32));
    h0_in.extend_from_slice(key);
    h0_in.extend_from_slice(&le32(ad.len() as u32));
    h0_in.extend_from_slice(ad);

    let h0 = blake2b(&h0_in, &Blake2bOptions::new().digest_size(64))
        .map_err(|e| Argon2Error::Blake2b(format!("{:?}", e)))?;

    let mut memory: Vec<Vec<Vec<u64>>> = (0..p)
        .map(|_| (0..q).map(|_| vec![0u64; BLOCK_WORDS]).collect())
        .collect();

    for i in 0..p {
        let mut in0 = Vec::with_capacity(h0.len() + 8);
        in0.extend_from_slice(&h0);
        in0.extend_from_slice(&le32(0));
        in0.extend_from_slice(&le32(i as u32));
        let b0 = blake2b_long(BLOCK_SIZE, &in0)?;

        let mut in1 = Vec::with_capacity(h0.len() + 8);
        in1.extend_from_slice(&h0);
        in1.extend_from_slice(&le32(1));
        in1.extend_from_slice(&le32(i as u32));
        let b1 = blake2b_long(BLOCK_SIZE, &in1)?;

        memory[i][0] = bytes_to_block(&b0);
        memory[i][1] = bytes_to_block(&b1);
    }

    for r in 0..t {
        for sl in 0..SYNC_POINTS {
            for lane in 0..p {
                fill_segment(&mut memory, r, lane, sl, q, sl_len, p, m_prime, t);
            }
        }
    }

    let mut final_block = memory[0][q - 1].clone();
    for lane in 1..p {
        for k in 0..BLOCK_WORDS {
            final_block[k] ^= memory[lane][q - 1][k];
        }
    }

    blake2b_long(tag_length as usize, &block_to_bytes(&final_block))
}

pub fn argon2i_hex(
    password: &[u8],
    salt: &[u8],
    time_cost: u32,
    memory_cost: u32,
    parallelism: u32,
    tag_length: u32,
    opts: &Options,
) -> Result<String, Argon2Error> {
    let tag = argon2i(password, salt, time_cost, memory_cost, parallelism, tag_length, opts)?;
    let mut s = String::with_capacity(tag.len() * 2);
    for b in &tag {
        s.push_str(&format!("{:02x}", b));
    }
    Ok(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn filled(n: usize, b: u8) -> Vec<u8> {
        vec![b; n]
    }

    fn hex_decode(s: &str) -> Vec<u8> {
        (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap()).collect()
    }

    #[test]
    fn rfc9106_vector_5_2() {
        let password = filled(32, 0x01);
        let salt = filled(16, 0x02);
        let key = filled(8, 0x03);
        let ad = filled(12, 0x04);
        let expected = hex_decode("c814d9d1dc7f37aa13f0d77f2494bda1c8de6b016dd388d29952a4c4672b6ce8");

        let tag = argon2i(
            &password, &salt, 3, 32, 4, 32,
            &Options { key: Some(&key), associated_data: Some(&ad), version: None },
        ).unwrap();
        assert_eq!(tag, expected);
    }

    #[test]
    fn hex_matches_bytes() {
        let tag = argon2i(b"password", b"saltsalt", 1, 8, 1, 32, &Options::default()).unwrap();
        let h = argon2i_hex(b"password", b"saltsalt", 1, 8, 1, 32, &Options::default()).unwrap();
        assert_eq!(h, tag.iter().map(|b| format!("{:02x}", b)).collect::<String>());
    }

    #[test]
    fn short_salt_rejected() { assert!(argon2i(b"pw", b"short", 1, 8, 1, 32, &Options::default()).is_err()); }

    #[test]
    fn tag_length_too_small_rejected() { assert!(argon2i(b"pw", b"saltsalt", 1, 8, 1, 3, &Options::default()).is_err()); }

    #[test]
    fn memory_below_minimum_rejected() { assert!(argon2i(b"pw", b"saltsalt", 1, 1, 1, 32, &Options::default()).is_err()); }

    #[test]
    fn zero_time_cost_rejected() { assert!(argon2i(b"pw", b"saltsalt", 0, 8, 1, 32, &Options::default()).is_err()); }

    #[test]
    fn zero_parallelism_rejected() { assert!(argon2i(b"pw", b"saltsalt", 1, 8, 0, 32, &Options::default()).is_err()); }

    #[test]
    fn unsupported_version_rejected() {
        assert!(argon2i(b"pw", b"saltsalt", 1, 8, 1, 32,
            &Options { key: None, associated_data: None, version: Some(0x10) }).is_err());
    }

    #[test]
    fn deterministic() {
        let a = argon2i(b"password", b"somesalt", 1, 8, 1, 32, &Options::default()).unwrap();
        let b = argon2i(b"password", b"somesalt", 1, 8, 1, 32, &Options::default()).unwrap();
        assert_eq!(a, b);
        assert_eq!(a.len(), 32);
    }

    #[test]
    fn different_passwords_differ() {
        let a = argon2i(b"password1", b"somesalt", 1, 8, 1, 32, &Options::default()).unwrap();
        let b = argon2i(b"password2", b"somesalt", 1, 8, 1, 32, &Options::default()).unwrap();
        assert_ne!(a, b);
    }

    #[test]
    fn different_salts_differ() {
        let a = argon2i(b"password", b"saltsalt", 1, 8, 1, 32, &Options::default()).unwrap();
        let b = argon2i(b"password", b"saltsal2", 1, 8, 1, 32, &Options::default()).unwrap();
        assert_ne!(a, b);
    }

    #[test]
    fn key_binds() {
        let a = argon2i(b"password", b"saltsalt", 1, 8, 1, 32, &Options::default()).unwrap();
        let b = argon2i(b"password", b"saltsalt", 1, 8, 1, 32,
            &Options { key: Some(b"secret!!"), associated_data: None, version: None }).unwrap();
        assert_ne!(a, b);
    }

    #[test]
    fn associated_data_binds() {
        let a = argon2i(b"password", b"saltsalt", 1, 8, 1, 32, &Options::default()).unwrap();
        let b = argon2i(b"password", b"saltsalt", 1, 8, 1, 32,
            &Options { key: None, associated_data: Some(b"ad"), version: None }).unwrap();
        assert_ne!(a, b);
    }

    #[test]
    fn tag_length_variants() {
        for t in [4, 16, 32, 64, 65, 128] {
            let tag = argon2i(b"password", b"saltsalt", 1, 8, 1, t, &Options::default()).unwrap();
            assert_eq!(tag.len(), t as usize);
        }
    }

    #[test]
    fn multi_lane_parameters() {
        let tag = argon2i(&filled(32, 0x01), &filled(16, 0x02), 3, 32, 4, 32, &Options::default()).unwrap();
        assert_eq!(tag.len(), 32);
    }

    #[test]
    fn multiple_passes() {
        let t1 = argon2i(b"password", b"saltsalt", 1, 8, 1, 32, &Options::default()).unwrap();
        let t2 = argon2i(b"password", b"saltsalt", 2, 8, 1, 32, &Options::default()).unwrap();
        let t3 = argon2i(b"password", b"saltsalt", 3, 8, 1, 32, &Options::default()).unwrap();
        assert_ne!(t1, t2);
        assert_ne!(t2, t3);
        assert_ne!(t1, t3);
    }
}
