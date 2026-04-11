use std::convert::TryInto;

pub const DJB2_OFFSET_BASIS: u64 = 5_381;
pub const FNV32_OFFSET_BASIS: u32 = 0x811C_9DC5;
pub const FNV32_PRIME: u32 = 0x0100_0193;
pub const FNV64_OFFSET_BASIS: u64 = 0xCBF2_9CE4_8422_2325;
pub const FNV64_PRIME: u64 = 0x0000_0100_0000_01B3;
pub const POLYNOMIAL_ROLLING_DEFAULT_BASE: u64 = 31;
pub const POLYNOMIAL_ROLLING_DEFAULT_MODULUS: u64 = (1u64 << 61) - 1;

const MURMUR3_C1: u32 = 0xCC9E_2D51;
const MURMUR3_C2: u32 = 0x1B87_3593;

const SIPHASH_V0: u64 = 0x736F_6D65_7073_6575;
const SIPHASH_V1: u64 = 0x646F_7261_6E64_6F6D;
const SIPHASH_V2: u64 = 0x6C79_6765_6E65_7261;
const SIPHASH_V3: u64 = 0x7465_6462_7974_6573;

/// Common interface for DT17 hash strategies.
pub trait HashFunction {
    fn hash(&self, data: &[u8]) -> u64;
    fn output_bits(&self) -> u32;
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Fnv1a32;

impl HashFunction for Fnv1a32 {
    fn hash(&self, data: &[u8]) -> u64 {
        fnv1a_32(data) as u64
    }

    fn output_bits(&self) -> u32 {
        32
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Fnv1a64;

impl HashFunction for Fnv1a64 {
    fn hash(&self, data: &[u8]) -> u64 {
        fnv1a_64(data)
    }

    fn output_bits(&self) -> u32 {
        64
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Djb2;

impl HashFunction for Djb2 {
    fn hash(&self, data: &[u8]) -> u64 {
        djb2(data)
    }

    fn output_bits(&self) -> u32 {
        64
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PolynomialRolling {
    pub base: u64,
    pub modulus: u64,
}

impl Default for PolynomialRolling {
    fn default() -> Self {
        Self {
            base: POLYNOMIAL_ROLLING_DEFAULT_BASE,
            modulus: POLYNOMIAL_ROLLING_DEFAULT_MODULUS,
        }
    }
}

impl HashFunction for PolynomialRolling {
    fn hash(&self, data: &[u8]) -> u64 {
        polynomial_rolling_with_params(data, self.base, self.modulus)
    }

    fn output_bits(&self) -> u32 {
        64
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Murmur3_32 {
    pub seed: u32,
}

impl Default for Murmur3_32 {
    fn default() -> Self {
        Self { seed: 0 }
    }
}

impl HashFunction for Murmur3_32 {
    fn hash(&self, data: &[u8]) -> u64 {
        murmur3_32_with_seed(data, self.seed) as u64
    }

    fn output_bits(&self) -> u32 {
        32
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SipHash24 {
    pub key: [u8; 16],
}

impl SipHash24 {
    pub fn new(key: [u8; 16]) -> Self {
        Self { key }
    }
}

impl HashFunction for SipHash24 {
    fn hash(&self, data: &[u8]) -> u64 {
        siphash_2_4(data, &self.key)
    }

    fn output_bits(&self) -> u32 {
        64
    }
}

pub fn fnv1a_32(data: &[u8]) -> u32 {
    let mut hash = FNV32_OFFSET_BASIS;
    for &byte in data {
        hash ^= u32::from(byte);
        hash = hash.wrapping_mul(FNV32_PRIME);
    }
    hash
}

pub fn fnv1a_64(data: &[u8]) -> u64 {
    let mut hash = FNV64_OFFSET_BASIS;
    for &byte in data {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(FNV64_PRIME);
    }
    hash
}

pub fn djb2(data: &[u8]) -> u64 {
    let mut hash = DJB2_OFFSET_BASIS;
    for &byte in data {
        hash = hash
            .wrapping_shl(5)
            .wrapping_add(hash)
            .wrapping_add(u64::from(byte));
    }
    hash
}

pub fn polynomial_rolling(data: &[u8]) -> u64 {
    polynomial_rolling_with_params(
        data,
        POLYNOMIAL_ROLLING_DEFAULT_BASE,
        POLYNOMIAL_ROLLING_DEFAULT_MODULUS,
    )
}

pub fn polynomial_rolling_with_params(data: &[u8], base: u64, modulus: u64) -> u64 {
    assert!(modulus > 0, "modulus must be positive");
    let mut hash = 0u128;
    let base = u128::from(base);
    let modulus = u128::from(modulus);
    for &byte in data {
        hash = (hash * base + u128::from(byte)) % modulus;
    }
    hash as u64
}

pub fn murmur3_32(data: &[u8]) -> u32 {
    murmur3_32_with_seed(data, 0)
}

pub fn murmur3_32_with_seed(data: &[u8], seed: u32) -> u32 {
    let mut hash = seed;
    let mut blocks = data.chunks_exact(4);

    for block in &mut blocks {
        let mut k = u32::from_le_bytes(block.try_into().expect("chunks_exact produced 4 bytes"));
        k = k.wrapping_mul(MURMUR3_C1);
        k = k.rotate_left(15);
        k = k.wrapping_mul(MURMUR3_C2);

        hash ^= k;
        hash = hash.rotate_left(13);
        hash = hash.wrapping_mul(5).wrapping_add(0xE654_6B64);
    }

    let mut k = 0u32;
    for (index, &byte) in blocks.remainder().iter().enumerate() {
        k ^= u32::from(byte) << (index * 8);
    }
    if !blocks.remainder().is_empty() {
        k = k.wrapping_mul(MURMUR3_C1);
        k = k.rotate_left(15);
        k = k.wrapping_mul(MURMUR3_C2);
        hash ^= k;
    }

    hash ^= data.len() as u32;
    fmix32(hash)
}

pub fn siphash_2_4(data: &[u8], key: &[u8; 16]) -> u64 {
    let k0 = u64::from_le_bytes(key[0..8].try_into().expect("key slice"));
    let k1 = u64::from_le_bytes(key[8..16].try_into().expect("key slice"));

    let mut v0 = SIPHASH_V0 ^ k0;
    let mut v1 = SIPHASH_V1 ^ k1;
    let mut v2 = SIPHASH_V2 ^ k0;
    let mut v3 = SIPHASH_V3 ^ k1;

    let mut blocks = data.chunks_exact(8);
    for block in &mut blocks {
        let m = u64::from_le_bytes(block.try_into().expect("chunks_exact produced 8 bytes"));
        v3 ^= m;
        sipround(&mut v0, &mut v1, &mut v2, &mut v3);
        sipround(&mut v0, &mut v1, &mut v2, &mut v3);
        v0 ^= m;
    }

    let mut last = ((data.len() as u64) & 0xff) << 56;
    for (index, &byte) in blocks.remainder().iter().enumerate() {
        last |= u64::from(byte) << (index * 8);
    }

    v3 ^= last;
    sipround(&mut v0, &mut v1, &mut v2, &mut v3);
    sipround(&mut v0, &mut v1, &mut v2, &mut v3);
    v0 ^= last;

    v2 ^= 0xff;
    sipround(&mut v0, &mut v1, &mut v2, &mut v3);
    sipround(&mut v0, &mut v1, &mut v2, &mut v3);
    sipround(&mut v0, &mut v1, &mut v2, &mut v3);
    sipround(&mut v0, &mut v1, &mut v2, &mut v3);

    v0 ^ v1 ^ v2 ^ v3
}

pub fn hash_str_fnv1a_32(s: &str) -> u32 {
    fnv1a_32(s.as_bytes())
}

pub fn hash_str_siphash(s: &str, key: &[u8; 16]) -> u64 {
    siphash_2_4(s.as_bytes(), key)
}

fn fmix32(mut hash: u32) -> u32 {
    hash ^= hash >> 16;
    hash = hash.wrapping_mul(0x85EB_CA6B);
    hash ^= hash >> 13;
    hash = hash.wrapping_mul(0xC2B2_AE35);
    hash ^= hash >> 16;
    hash
}

fn sipround(v0: &mut u64, v1: &mut u64, v2: &mut u64, v3: &mut u64) {
    *v0 = v0.wrapping_add(*v1);
    *v1 = v1.rotate_left(13);
    *v1 ^= *v0;
    *v0 = v0.rotate_left(32);

    *v2 = v2.wrapping_add(*v3);
    *v3 = v3.rotate_left(16);
    *v3 ^= *v2;

    *v0 = v0.wrapping_add(*v3);
    *v3 = v3.rotate_left(21);
    *v3 ^= *v0;

    *v2 = v2.wrapping_add(*v1);
    *v1 = v1.rotate_left(17);
    *v1 ^= *v2;
    *v2 = v2.rotate_left(32);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fnv1a_32_known_vectors() {
        assert_eq!(fnv1a_32(b""), 0x811C_9DC5);
        assert_eq!(fnv1a_32(b"a"), 0xE40C_292C);
        assert_eq!(fnv1a_32(b"abc"), 0x1A47_E90B);
        assert_eq!(fnv1a_32(b"hello"), 1_335_831_723);
        assert_eq!(fnv1a_32(b"foobar"), 3_214_735_720);
    }

    #[test]
    fn fnv1a_64_known_vectors() {
        assert_eq!(fnv1a_64(b""), 0xCBF2_9CE4_8422_2325);
        assert_eq!(fnv1a_64(b"a"), 0xAF63_DC4C_8601_EC8C);
        assert_eq!(fnv1a_64(b"abc"), 0xE71F_A219_0541_574B);
        assert_eq!(fnv1a_64(b"hello"), 0xA430_D846_80AA_BD0B);
    }

    #[test]
    fn djb2_known_vectors() {
        assert_eq!(djb2(b""), 5_381);
        assert_eq!(djb2(b"a"), 177_670);
        assert_eq!(djb2(b"abc"), 193_485_963);
    }

    #[test]
    fn polynomial_rolling_known_vectors() {
        assert_eq!(polynomial_rolling(b""), 0);
        assert_eq!(polynomial_rolling(b"a"), 97);
        assert_eq!(polynomial_rolling(b"ab"), 3_105);
        assert_eq!(polynomial_rolling(b"abc"), 96_354);
        assert_eq!(
            polynomial_rolling_with_params(b"abc", 37, 1_000_000_007),
            ((97u128 * 37 + 98) * 37 + 99) as u64
        );
    }

    #[test]
    fn murmur3_known_vectors() {
        assert_eq!(murmur3_32(b""), 0);
        assert_eq!(murmur3_32_with_seed(b"", 1), 0x514E_28B7);
        assert_eq!(murmur3_32(b"a"), 0x3C25_69B2);
        assert_eq!(murmur3_32(b"abc"), 0xB3DD_93FA);
        assert_eq!(murmur3_32(b"abcd"), 0x43ED_676A);
    }

    #[test]
    fn siphash_known_vectors() {
        let key = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15];
        assert_eq!(siphash_2_4(b"", &key), 0x726F_DB47_DD0E_0E31);
        assert_eq!(siphash_2_4(b"\x00", &key), 0x74F8_39C5_93DC_67FD);
    }

    #[test]
    fn hash_str_helpers_match_byte_helpers() {
        let key = [0u8; 16];
        assert_eq!(hash_str_fnv1a_32("hello"), fnv1a_32(b"hello"));
        assert_eq!(hash_str_siphash("hello", &key), siphash_2_4(b"hello", &key));
    }

    #[test]
    fn trait_implementations_forward_to_free_functions() {
        let fnv32 = Fnv1a32;
        let fnv64 = Fnv1a64;
        let djb = Djb2;
        let poly = PolynomialRolling::default();
        let murmur = Murmur3_32::default();
        let sip = SipHash24::new([0u8; 16]);

        assert_eq!(fnv32.hash(b"abc"), fnv1a_32(b"abc") as u64);
        assert_eq!(fnv64.hash(b"abc"), fnv1a_64(b"abc"));
        assert_eq!(djb.hash(b"abc"), djb2(b"abc"));
        assert_eq!(poly.hash(b"abc"), polynomial_rolling(b"abc"));
        assert_eq!(murmur.hash(b"abc"), murmur3_32(b"abc") as u64);
        assert_eq!(sip.hash(b"abc"), siphash_2_4(b"abc", &[0u8; 16]));

        assert_eq!(fnv32.output_bits(), 32);
        assert_eq!(fnv64.output_bits(), 64);
        assert_eq!(djb.output_bits(), 64);
        assert_eq!(poly.output_bits(), 64);
        assert_eq!(murmur.output_bits(), 32);
        assert_eq!(sip.output_bits(), 64);
    }
}
