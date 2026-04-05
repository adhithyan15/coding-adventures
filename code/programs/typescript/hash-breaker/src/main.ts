/**
 * hash-breaker — Demonstrating why MD5 is cryptographically broken.
 *
 * This program runs three attacks against MD5:
 *
 *   1. Known Collision Pairs — two different byte sequences with the same MD5
 *   2. Length Extension Attack — forge a valid hash without knowing the secret
 *   3. Birthday Attack — find a collision on a truncated hash via birthday paradox
 */

import { md5, md5Hex } from "@coding-adventures/md5";

// ============================================================================
// Utility: hex string to Uint8Array
// ============================================================================

function hexToBytes(hex: string): Uint8Array {
  const bytes = new Uint8Array(hex.length / 2);
  for (let i = 0; i < hex.length; i += 2) {
    bytes[i / 2] = parseInt(hex.substring(i, i + 2), 16);
  }
  return bytes;
}

function bytesToHex(bytes: Uint8Array): string {
  return Array.from(bytes)
    .map((b) => b.toString(16).padStart(2, "0"))
    .join("");
}

function hexDump(data: Uint8Array): string {
  const lines: string[] = [];
  for (let i = 0; i < data.length; i += 16) {
    lines.push("  " + bytesToHex(data.slice(i, i + 16)));
  }
  return lines.join("\n");
}

// ============================================================================
// ATTACK 1: Known MD5 Collision Pairs (Wang & Yu, 2004)
// ============================================================================
//
// Two 128-byte messages that produce the SAME MD5 hash. The canonical Wang/Yu
// collision pair from 2004 — the breakthrough that proved MD5 is broken.

const COLLISION_A = hexToBytes(
  "d131dd02c5e6eec4693d9a0698aff95c" +
    "2fcab58712467eab4004583eb8fb7f89" +
    "55ad340609f4b30283e488832571415a" +
    "085125e8f7cdc99fd91dbdf280373c5b" +
    "d8823e3156348f5bae6dacd436c919c6" +
    "dd53e2b487da03fd02396306d248cda0" +
    "e99f33420f577ee8ce54b67080a80d1e" +
    "c69821bcb6a8839396f9652b6ff72a70",
);

const COLLISION_B = hexToBytes(
  "d131dd02c5e6eec4693d9a0698aff95c" +
    "2fcab50712467eab4004583eb8fb7f89" +
    "55ad340609f4b30283e4888325f1415a" +
    "085125e8f7cdc99fd91dbd7280373c5b" +
    "d8823e3156348f5bae6dacd436c919c6" +
    "dd53e23487da03fd02396306d248cda0" +
    "e99f33420f577ee8ce54b67080280d1e" +
    "c69821bcb6a8839396f965ab6ff72a70",
);

function attack1(): void {
  console.log("=".repeat(72));
  console.log("ATTACK 1: Known MD5 Collision Pair (Wang & Yu, 2004)");
  console.log("=".repeat(72));
  console.log();
  console.log(
    "Two different 128-byte messages that produce the SAME MD5 hash.",
  );
  console.log(
    "This was the breakthrough that proved MD5 is broken for security.",
  );
  console.log();

  console.log("Block A (hex):");
  console.log(hexDump(COLLISION_A));
  console.log();
  console.log("Block B (hex):");
  console.log(hexDump(COLLISION_B));
  console.log();

  const diffs: number[] = [];
  for (let i = 0; i < COLLISION_A.length; i++) {
    if (COLLISION_A[i] !== COLLISION_B[i]) diffs.push(i);
  }
  console.log(
    `Blocks differ at ${diffs.length} byte positions: [${diffs.join(", ")}]`,
  );
  for (const pos of diffs) {
    console.log(
      `  Byte ${pos}: A=0x${COLLISION_A[pos].toString(16).padStart(2, "0")}  B=0x${COLLISION_B[pos].toString(16).padStart(2, "0")}`,
    );
  }
  console.log();

  const hashA = md5Hex(COLLISION_A);
  const hashB = md5Hex(COLLISION_B);
  console.log(`MD5(A) = ${hashA}`);
  console.log(`MD5(B) = ${hashB}`);
  console.log(`Match?   ${hashA === hashB ? "YES — COLLISION!" : "No (unexpected)"}`);
  console.log();
  console.log(
    "Lesson: MD5 collisions are REAL. Never use MD5 for integrity or auth.",
  );
  console.log();
}

// ============================================================================
// ATTACK 2: Length Extension Attack
// ============================================================================
//
// Given md5(secret || message) and len(secret || message), forge
// md5(secret || message || padding || evil_data) WITHOUT knowing the secret.

// MD5 T-table constants (sine-derived).
const T_TABLE = new Uint32Array(64);
for (let i = 0; i < 64; i++) {
  T_TABLE[i] = (Math.floor(Math.abs(Math.sin(i + 1)) * 0x100000000) >>> 0);
}

// MD5 per-round shift amounts.
const SHIFTS = [
  7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 5, 9, 14, 20,
  5, 9, 14, 20, 5, 9, 14, 20, 5, 9, 14, 20, 4, 11, 16, 23, 4, 11, 16, 23, 4,
  11, 16, 23, 4, 11, 16, 23, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21, 6,
  10, 15, 21,
];

function rotl32(x: number, n: number): number {
  return ((x << n) | (x >>> (32 - n))) >>> 0;
}

// Inline MD5 compression for length extension.
function md5Compress(
  state: [number, number, number, number],
  block: Uint8Array,
): [number, number, number, number] {
  const view = new DataView(block.buffer, block.byteOffset, 64);
  const m = new Uint32Array(16);
  for (let i = 0; i < 16; i++) {
    m[i] = view.getUint32(i * 4, true); // little-endian
  }

  let [a, b, c, d] = state;
  const [a0, b0, c0, d0] = [a, b, c, d];

  for (let i = 0; i < 64; i++) {
    let f: number;
    let g: number;
    if (i < 16) {
      f = ((b & c) | (~b & d)) >>> 0;
      g = i;
    } else if (i < 32) {
      f = ((d & b) | (~d & c)) >>> 0;
      g = (5 * i + 1) % 16;
    } else if (i < 48) {
      f = (b ^ c ^ d) >>> 0;
      g = (3 * i + 5) % 16;
    } else {
      f = (c ^ (b | ~d)) >>> 0;
      g = (7 * i) % 16;
    }
    const temp = d;
    d = c;
    c = b;
    b = (b + rotl32(((a + f + T_TABLE[i] + m[g]) >>> 0), SHIFTS[i])) >>> 0;
    a = temp;
  }

  return [
    (a0 + a) >>> 0,
    (b0 + b) >>> 0,
    (c0 + c) >>> 0,
    (d0 + d) >>> 0,
  ];
}

function md5Padding(messageLen: number): Uint8Array {
  const remainder = messageLen % 64;
  let padLen = (55 - remainder) % 64;
  if (padLen < 0) padLen += 64;
  const padding = new Uint8Array(1 + padLen + 8);
  padding[0] = 0x80;
  // 64-bit little-endian bit length
  const bitLen = messageLen * 8;
  const view = new DataView(padding.buffer);
  // JavaScript can't do 64-bit ints natively, but for our message sizes 32-bit is enough
  view.setUint32(1 + padLen, bitLen & 0xffffffff, true);
  view.setUint32(1 + padLen + 4, 0, true); // high 32 bits = 0 for small messages
  return padding;
}

function concatBytes(...arrays: Uint8Array[]): Uint8Array {
  const totalLen = arrays.reduce((sum, a) => sum + a.length, 0);
  const result = new Uint8Array(totalLen);
  let offset = 0;
  for (const arr of arrays) {
    result.set(arr, offset);
    offset += arr.length;
  }
  return result;
}

function attack2(): void {
  console.log("=".repeat(72));
  console.log("ATTACK 2: Length Extension Attack");
  console.log("=".repeat(72));
  console.log();
  console.log(
    "Given md5(secret + message) and len(secret + message), we can forge",
  );
  console.log(
    "md5(secret + message + padding + evil_data) WITHOUT knowing the secret!",
  );
  console.log();

  const encoder = new TextEncoder();
  const secret = encoder.encode("supersecretkey!!");
  const message = encoder.encode("amount=100&to=alice");
  const originalData = concatBytes(secret, message);
  const originalHash = md5(originalData);
  const originalHex = bytesToHex(originalHash);

  console.log(`Secret (unknown to attacker): "supersecretkey!!"`);
  console.log(`Message:                      "amount=100&to=alice"`);
  console.log(`MAC = md5(secret || message): ${originalHex}`);
  console.log(`Length of (secret || message): ${originalData.length} bytes`);
  console.log();

  const evilData = encoder.encode("&amount=1000000&to=mallory");
  console.log(`Evil data to append: "&amount=1000000&to=mallory"`);
  console.log();

  // Step 1: Extract state from hash
  const hashView = new DataView(
    originalHash.buffer,
    originalHash.byteOffset,
    16,
  );
  const a = hashView.getUint32(0, true);
  const b = hashView.getUint32(4, true);
  const c = hashView.getUint32(8, true);
  const d = hashView.getUint32(12, true);

  console.log("Step 1: Extract MD5 internal state from the hash");
  console.log(
    `  A = 0x${a.toString(16).padStart(8, "0")}, B = 0x${b.toString(16).padStart(8, "0")}, C = 0x${c.toString(16).padStart(8, "0")}, D = 0x${d.toString(16).padStart(8, "0")}`,
  );
  console.log();

  // Step 2: Compute padding
  const padding = md5Padding(originalData.length);
  console.log("Step 2: Compute MD5 padding for the original message");
  console.log(`  Padding (${padding.length} bytes): ${bytesToHex(padding)}`);
  console.log();

  const processedLen = originalData.length + padding.length;
  console.log(`Step 3: Total bytes processed so far: ${processedLen}`);
  console.log();

  // Step 4: Forge
  const forgedInput = concatBytes(
    evilData,
    md5Padding(processedLen + evilData.length),
  );
  let state: [number, number, number, number] = [a, b, c, d];
  for (let i = 0; i + 64 <= forgedInput.length; i += 64) {
    state = md5Compress(state, forgedInput.slice(i, i + 64));
  }

  const forgedHash = new Uint8Array(16);
  const forgedView = new DataView(forgedHash.buffer);
  forgedView.setUint32(0, state[0], true);
  forgedView.setUint32(4, state[1], true);
  forgedView.setUint32(8, state[2], true);
  forgedView.setUint32(12, state[3], true);
  const forgedHex = bytesToHex(forgedHash);

  console.log("Step 4: Initialize hasher with extracted state, feed evil_data");
  console.log(`  Forged hash: ${forgedHex}`);
  console.log();

  // Step 5: Verify
  const actualFull = concatBytes(originalData, padding, evilData);
  const actualHex = md5Hex(actualFull);

  console.log(
    "Step 5: Verify — compute actual md5(secret || message || padding || evil_data)",
  );
  console.log(`  Actual hash: ${actualHex}`);
  console.log(
    `  Match?       ${forgedHex === actualHex ? "YES — FORGED!" : "No (bug)"}`,
  );
  console.log();
  console.log("The attacker forged a valid MAC without knowing the secret!");
  console.log();
  console.log("Why HMAC fixes this:");
  console.log(
    "  HMAC = md5(key XOR opad || md5(key XOR ipad || message))",
  );
  console.log("  The outer hash prevents length extension because the attacker");
  console.log("  cannot extend past the outer md5() boundary.");
  console.log();
}

// ============================================================================
// ATTACK 3: Birthday Attack (Truncated Hash)
// ============================================================================

// Simple seeded PRNG (xorshift32) for reproducible results.
function makeRng(seed: number): () => number {
  let state = seed >>> 0;
  return () => {
    state ^= state << 13;
    state ^= state >>> 17;
    state ^= state << 5;
    return state >>> 0;
  };
}

function attack3(): void {
  console.log("=".repeat(72));
  console.log("ATTACK 3: Birthday Attack on Truncated MD5 (32-bit)");
  console.log("=".repeat(72));
  console.log();
  console.log(
    "The birthday paradox: with N possible hash values, expect a collision",
  );
  console.log(
    "after ~sqrt(N) random inputs. For 32-bit hash: sqrt(2^32) = 2^16 = 65536.",
  );
  console.log();

  const rng = makeRng(42);
  const seen = new Map<string, Uint8Array>();

  for (let attempts = 1; ; attempts++) {
    const msg = new Uint8Array(8);
    for (let i = 0; i < 8; i++) msg[i] = rng() & 0xff;

    const fullHash = md5(msg);
    const truncated = bytesToHex(fullHash.slice(0, 4));

    if (seen.has(truncated)) {
      const other = seen.get(truncated)!;
      if (bytesToHex(other) !== bytesToHex(msg)) {
        console.log(`COLLISION FOUND after ${attempts} attempts!`);
        console.log();
        console.log(`  Message 1: ${bytesToHex(other)}`);
        console.log(`  Message 2: ${bytesToHex(msg)}`);
        console.log(`  Truncated MD5 (4 bytes): ${truncated}`);
        console.log(`  Full MD5 of msg1: ${md5Hex(other)}`);
        console.log(`  Full MD5 of msg2: ${md5Hex(msg)}`);
        console.log();
        console.log(
          `  Expected ~65536 attempts (2^16), got ${attempts}`,
        );
        console.log(
          `  Ratio: ${(attempts / 65536).toFixed(2)}x the theoretical expectation`,
        );
        break;
      }
    } else {
      seen.set(truncated, msg.slice());
    }
  }

  console.log();
  console.log("This is a GENERIC attack — it works against any hash function.");
  console.log("The defense is a longer hash: SHA-256 has 2^128 birthday bound,");
  console.log(
    "while MD5 has only 2^64 (and dedicated attacks are even faster).",
  );
  console.log();
}

// ============================================================================
// Main
// ============================================================================

function main(): void {
  console.log();
  console.log("======================================================================");
  console.log("           MD5 HASH BREAKER — Why MD5 Is Broken");
  console.log("======================================================================");
  console.log("  Three attacks showing MD5 must NEVER be used for security:");
  console.log("    1. Known collision pairs (Wang & Yu, 2004)");
  console.log("    2. Length extension attack (forge MAC without secret)");
  console.log("    3. Birthday attack on truncated hash (birthday paradox)");
  console.log("======================================================================");
  console.log();

  attack1();
  attack2();
  attack3();

  console.log("=".repeat(72));
  console.log("CONCLUSION");
  console.log("=".repeat(72));
  console.log();
  console.log("MD5 is broken in three distinct ways:");
  console.log(
    "  1. COLLISION RESISTANCE: known pairs exist (and can be generated)",
  );
  console.log(
    "  2. LENGTH EXTENSION: Merkle-Damgard structure leaks internal state",
  );
  console.log(
    "  3. BIRTHDAY BOUND: only 2^64 (and dedicated attacks beat even that)",
  );
  console.log();
  console.log(
    "Use SHA-256 or SHA-3 for security. Use HMAC (not raw hash) for MACs.",
  );
  console.log();
}

main();
