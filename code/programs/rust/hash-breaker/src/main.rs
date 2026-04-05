//! hash-breaker — Demonstrating why MD5 is cryptographically broken.
//!
//! Three attacks against MD5:
//!   1. Known Collision Pairs (Wang & Yu, 2004)
//!   2. Length Extension Attack (forge hash without secret)
//!   3. Birthday Attack on truncated hash (birthday paradox)

use coding_adventures_md5::{hex_string, sum_md5};
use std::collections::HashMap;

// ============================================================================
// Utility: decode hex string to bytes
// ============================================================================

fn hex_to_bytes(hex: &str) -> Vec<u8> {
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
        .collect()
}

fn bytes_to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

fn hex_dump(data: &[u8]) -> String {
    data.chunks(16)
        .map(|chunk| format!("  {}", bytes_to_hex(chunk)))
        .collect::<Vec<_>>()
        .join("\n")
}

// ============================================================================
// ATTACK 1: Known MD5 Collision Pairs (Wang & Yu, 2004)
// ============================================================================
//
// Two 128-byte messages that produce the SAME MD5 hash. The canonical Wang/Yu
// collision pair — the breakthrough that proved MD5 is broken for security.

fn attack_1() {
    let collision_a = hex_to_bytes(
        "d131dd02c5e6eec4693d9a0698aff95c\
         2fcab58712467eab4004583eb8fb7f89\
         55ad340609f4b30283e488832571415a\
         085125e8f7cdc99fd91dbdf280373c5b\
         d8823e3156348f5bae6dacd436c919c6\
         dd53e2b487da03fd02396306d248cda0\
         e99f33420f577ee8ce54b67080a80d1e\
         c69821bcb6a8839396f9652b6ff72a70",
    );
    let collision_b = hex_to_bytes(
        "d131dd02c5e6eec4693d9a0698aff95c\
         2fcab50712467eab4004583eb8fb7f89\
         55ad340609f4b30283e4888325f1415a\
         085125e8f7cdc99fd91dbd7280373c5b\
         d8823e3156348f5bae6dacd436c919c6\
         dd53e23487da03fd02396306d248cda0\
         e99f33420f577ee8ce54b67080280d1e\
         c69821bcb6a8839396f965ab6ff72a70",
    );

    println!("{}", "=".repeat(72));
    println!("ATTACK 1: Known MD5 Collision Pair (Wang & Yu, 2004)");
    println!("{}", "=".repeat(72));
    println!();
    println!("Two different 128-byte messages that produce the SAME MD5 hash.");
    println!("This was the breakthrough that proved MD5 is broken for security.");
    println!();

    println!("Block A (hex):");
    println!("{}", hex_dump(&collision_a));
    println!();
    println!("Block B (hex):");
    println!("{}", hex_dump(&collision_b));
    println!();

    let diffs: Vec<usize> = collision_a
        .iter()
        .zip(collision_b.iter())
        .enumerate()
        .filter(|(_, (a, b))| a != b)
        .map(|(i, _)| i)
        .collect();

    println!("Blocks differ at {} byte positions: {:?}", diffs.len(), diffs);
    for &pos in &diffs {
        println!(
            "  Byte {}: A=0x{:02x}  B=0x{:02x}",
            pos, collision_a[pos], collision_b[pos]
        );
    }
    println!();

    let hash_a = hex_string(&collision_a);
    let hash_b = hex_string(&collision_b);
    println!("MD5(A) = {}", hash_a);
    println!("MD5(B) = {}", hash_b);
    println!(
        "Match?   {}",
        if hash_a == hash_b {
            "YES — COLLISION!"
        } else {
            "No (unexpected)"
        }
    );
    println!();
    println!("Lesson: MD5 collisions are REAL. Never use MD5 for integrity or auth.");
    println!();
}

// ============================================================================
// ATTACK 2: Length Extension Attack
// ============================================================================
//
// Given md5(secret || message) and len(secret || message), forge
// md5(secret || message || padding || evil_data) WITHOUT knowing the secret.

/// MD5 T-table constants derived from sine.
fn t_table() -> [u32; 64] {
    let mut t = [0u32; 64];
    for i in 0..64 {
        t[i] = ((i as f64 + 1.0).sin().abs() * (1u64 << 32) as f64).floor() as u32;
    }
    t
}

/// MD5 per-round shift amounts.
const SHIFTS: [u32; 64] = [
    7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 5, 9, 14, 20, 5, 9, 14, 20, 5,
    9, 14, 20, 5, 9, 14, 20, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 6, 10,
    15, 21, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21,
];

/// Inline MD5 compression for the length extension attack.
fn md5_compress(state: [u32; 4], block: &[u8]) -> [u32; 4] {
    let t = t_table();
    let mut m = [0u32; 16];
    for i in 0..16 {
        m[i] = u32::from_le_bytes([
            block[i * 4],
            block[i * 4 + 1],
            block[i * 4 + 2],
            block[i * 4 + 3],
        ]);
    }

    let [a0, b0, c0, d0] = state;
    let (mut a, mut b, mut c, mut d) = (a0, b0, c0, d0);

    for i in 0..64 {
        let (f, g) = if i < 16 {
            ((b & c) | (!b & d), i)
        } else if i < 32 {
            ((d & b) | (!d & c), (5 * i + 1) % 16)
        } else if i < 48 {
            (b ^ c ^ d, (3 * i + 5) % 16)
        } else {
            (c ^ (b | !d), (7 * i) % 16)
        };

        let temp = d;
        d = c;
        c = b;
        b = b.wrapping_add(
            a.wrapping_add(f)
                .wrapping_add(t[i])
                .wrapping_add(m[g])
                .rotate_left(SHIFTS[i]),
        );
        a = temp;
    }

    [
        a0.wrapping_add(a),
        b0.wrapping_add(b),
        c0.wrapping_add(c),
        d0.wrapping_add(d),
    ]
}

fn md5_padding(message_len: usize) -> Vec<u8> {
    let remainder = message_len % 64;
    let pad_len = (55usize.wrapping_sub(remainder)) % 64;
    let mut padding = vec![0u8; 1 + pad_len + 8];
    padding[0] = 0x80;
    let bit_len = (message_len as u64) * 8;
    padding[1 + pad_len..1 + pad_len + 8].copy_from_slice(&bit_len.to_le_bytes());
    padding
}

fn attack_2() {
    println!("{}", "=".repeat(72));
    println!("ATTACK 2: Length Extension Attack");
    println!("{}", "=".repeat(72));
    println!();
    println!("Given md5(secret + message) and len(secret + message), we can forge");
    println!("md5(secret + message + padding + evil_data) WITHOUT knowing the secret!");
    println!();

    let secret = b"supersecretkey!!";
    let message = b"amount=100&to=alice";
    let mut original_data = Vec::new();
    original_data.extend_from_slice(secret);
    original_data.extend_from_slice(message);
    let original_hash = sum_md5(&original_data);
    let original_hex = bytes_to_hex(&original_hash);

    println!(
        "Secret (unknown to attacker): {:?}",
        String::from_utf8_lossy(secret)
    );
    println!(
        "Message:                      {:?}",
        String::from_utf8_lossy(message)
    );
    println!("MAC = md5(secret || message): {}", original_hex);
    println!(
        "Length of (secret || message): {} bytes",
        original_data.len()
    );
    println!();

    let evil_data = b"&amount=1000000&to=mallory";
    println!(
        "Evil data to append: {:?}",
        String::from_utf8_lossy(evil_data)
    );
    println!();

    // Step 1: Extract state from hash
    let a = u32::from_le_bytes([original_hash[0], original_hash[1], original_hash[2], original_hash[3]]);
    let b = u32::from_le_bytes([original_hash[4], original_hash[5], original_hash[6], original_hash[7]]);
    let c = u32::from_le_bytes([original_hash[8], original_hash[9], original_hash[10], original_hash[11]]);
    let d = u32::from_le_bytes([original_hash[12], original_hash[13], original_hash[14], original_hash[15]]);

    println!("Step 1: Extract MD5 internal state from the hash");
    println!(
        "  A = 0x{:08x}, B = 0x{:08x}, C = 0x{:08x}, D = 0x{:08x}",
        a, b, c, d
    );
    println!();

    // Step 2: Compute padding
    let padding = md5_padding(original_data.len());
    println!("Step 2: Compute MD5 padding for the original message");
    println!(
        "  Padding ({} bytes): {}",
        padding.len(),
        bytes_to_hex(&padding)
    );
    println!();

    let processed_len = original_data.len() + padding.len();
    println!("Step 3: Total bytes processed so far: {}", processed_len);
    println!();

    // Step 4: Forge
    let mut forged_input = Vec::new();
    forged_input.extend_from_slice(evil_data);
    forged_input.extend_from_slice(&md5_padding(processed_len + evil_data.len()));

    let mut state = [a, b, c, d];
    for chunk in forged_input.chunks(64) {
        if chunk.len() == 64 {
            state = md5_compress(state, chunk);
        }
    }

    let mut forged_hash = [0u8; 16];
    forged_hash[0..4].copy_from_slice(&state[0].to_le_bytes());
    forged_hash[4..8].copy_from_slice(&state[1].to_le_bytes());
    forged_hash[8..12].copy_from_slice(&state[2].to_le_bytes());
    forged_hash[12..16].copy_from_slice(&state[3].to_le_bytes());
    let forged_hex = bytes_to_hex(&forged_hash);

    println!("Step 4: Initialize hasher with extracted state, feed evil_data");
    println!("  Forged hash: {}", forged_hex);
    println!();

    // Step 5: Verify
    let mut actual_full = Vec::new();
    actual_full.extend_from_slice(&original_data);
    actual_full.extend_from_slice(&padding);
    actual_full.extend_from_slice(evil_data);
    let actual_hex = hex_string(&actual_full);

    println!("Step 5: Verify — compute actual md5(secret || message || padding || evil_data)");
    println!("  Actual hash: {}", actual_hex);
    println!(
        "  Match?       {}",
        if forged_hex == actual_hex {
            "YES — FORGED!"
        } else {
            "No (bug)"
        }
    );
    println!();
    println!("The attacker forged a valid MAC without knowing the secret!");
    println!();
    println!("Why HMAC fixes this:");
    println!("  HMAC = md5(key XOR opad || md5(key XOR ipad || message))");
    println!("  The outer hash prevents length extension because the attacker");
    println!("  cannot extend past the outer md5() boundary.");
    println!();
}

// ============================================================================
// ATTACK 3: Birthday Attack (Truncated Hash)
// ============================================================================
//
// Birthday paradox: with N values, expect collision after ~sqrt(N) samples.
// 32-bit truncated hash: expect collision after ~2^16 = 65536 attempts.

/// Simple xorshift32 PRNG for reproducible results.
struct Xorshift32 {
    state: u32,
}

impl Xorshift32 {
    fn new(seed: u32) -> Self {
        Self { state: seed }
    }

    fn next(&mut self) -> u32 {
        self.state ^= self.state << 13;
        self.state ^= self.state >> 17;
        self.state ^= self.state << 5;
        self.state
    }
}

fn attack_3() {
    println!("{}", "=".repeat(72));
    println!("ATTACK 3: Birthday Attack on Truncated MD5 (32-bit)");
    println!("{}", "=".repeat(72));
    println!();
    println!("The birthday paradox: with N possible hash values, expect a collision");
    println!("after ~sqrt(N) random inputs. For 32-bit hash: sqrt(2^32) = 2^16 = 65536.");
    println!();

    let mut rng = Xorshift32::new(42);
    let mut seen: HashMap<[u8; 4], Vec<u8>> = HashMap::new();

    for attempts in 1.. {
        let mut msg = [0u8; 8];
        for byte in &mut msg {
            *byte = (rng.next() & 0xff) as u8;
        }

        let hash = sum_md5(&msg);
        let truncated: [u8; 4] = [hash[0], hash[1], hash[2], hash[3]];

        if let Some(other) = seen.get(&truncated) {
            if other.as_slice() != &msg[..] {
                println!("COLLISION FOUND after {} attempts!", attempts);
                println!();
                println!("  Message 1: {}", bytes_to_hex(other));
                println!("  Message 2: {}", bytes_to_hex(&msg));
                println!("  Truncated MD5 (4 bytes): {}", bytes_to_hex(&truncated));
                println!("  Full MD5 of msg1: {}", hex_string(other));
                println!("  Full MD5 of msg2: {}", hex_string(&msg));
                println!();
                println!("  Expected ~65536 attempts (2^16), got {}", attempts);
                println!(
                    "  Ratio: {:.2}x the theoretical expectation",
                    attempts as f64 / 65536.0
                );
                break;
            }
        } else {
            seen.insert(truncated, msg.to_vec());
        }
    }

    println!();
    println!("This is a GENERIC attack — it works against any hash function.");
    println!("The defense is a longer hash: SHA-256 has 2^128 birthday bound,");
    println!("while MD5 has only 2^64 (and dedicated attacks are even faster).");
    println!();
}

fn main() {
    println!();
    println!("======================================================================");
    println!("           MD5 HASH BREAKER — Why MD5 Is Broken");
    println!("======================================================================");
    println!("  Three attacks showing MD5 must NEVER be used for security:");
    println!("    1. Known collision pairs (Wang & Yu, 2004)");
    println!("    2. Length extension attack (forge MAC without secret)");
    println!("    3. Birthday attack on truncated hash (birthday paradox)");
    println!("======================================================================");
    println!();

    attack_1();
    attack_2();
    attack_3();

    println!("{}", "=".repeat(72));
    println!("CONCLUSION");
    println!("{}", "=".repeat(72));
    println!();
    println!("MD5 is broken in three distinct ways:");
    println!("  1. COLLISION RESISTANCE: known pairs exist (and can be generated)");
    println!("  2. LENGTH EXTENSION: Merkle-Damgard structure leaks internal state");
    println!("  3. BIRTHDAY BOUND: only 2^64 (and dedicated attacks beat even that)");
    println!();
    println!("Use SHA-256 or SHA-3 for security. Use HMAC (not raw hash) for MACs.");
    println!();
}
