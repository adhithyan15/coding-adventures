# MSG-MTPROTO — MTProto 2.0 (Telegram)

## Overview

MTProto is the custom binary protocol that powers every message, photo, video,
and voice call in Telegram. It was designed from scratch in 2013 by Nikolai
Durov (one of Telegram's founders) because no existing protocol was good enough
for Telegram's requirements.

**Why not just use HTTPS?**

Most chat apps in 2013 layered their messages on top of HTTPS — the same
protocol your browser uses. This worked, but had problems:

1. **Overhead** — TLS + HTTP adds hundreds of bytes of headers per message.
   On a slow mobile network, this matters.
2. **Latency** — TLS handshakes require 1–2 extra round trips before any
   data can flow. On a 3G network with 300ms latency, this is painfully slow.
3. **Encryption** — Most chat apps using HTTPS relied on the server to hold
   decryption keys. Telegram wanted server-side encryption (cloud chats) AND
   end-to-end encryption (secret chats) in the same system.
4. **Mobile efficiency** — Phones frequently switch between networks (WiFi →
   4G → WiFi). Protocols need to handle session resumption gracefully.

**Analogy:** Imagine sending a letter through an existing courier company
(HTTPS). Their envelope is large and wasteful for a three-word note. You have
to fill out their standard forms even when you are sending one word. The courier
reads every letter at each sorting office (the server holds your keys). Telegram
instead built its own courier company with custom envelopes perfectly sized for
messages, sorting offices that cannot open sealed envelopes (for secret chats),
and special fast lanes for returning customers.

**MTProto vs MTProto 2.0:**

The original MTProto (v1) was introduced in 2013 and used SHA-1 for message
authentication keys and AES-CBC for encryption. In 2017, Telegram upgraded to
MTProto 2.0:
- SHA-256 instead of SHA-1 for computing `msg_key` (stronger hash)
- Different padding scheme (minimum 12 bytes, aligned to 16 bytes)
- Explicit randomized padding to frustrate traffic analysis
- All new clients use 2.0; the spec here covers 2.0 exclusively

## Architecture

MTProto is built from three layers that each solve a different problem:

```
┌─────────────────────────────────────────────────────────────────────┐
│                    Application Layer (TL API)                       │
│  messages.sendMessage, auth.signIn, updates.getState, ...           │
│  Defined in TL schema. Every API call has a CRC32 constructor ID.   │
├─────────────────────────────────────────────────────────────────────┤
│                    Encryption Layer                                  │
│  - Derives AES key from auth_key + msg_key                          │
│  - Encrypts/decrypts with IGE mode                                  │
│  - Computes msg_key = SHA-256(auth_key_fragment + plaintext)        │
│  - Wire: auth_key_id (8) | msg_key (16) | encrypted_data (N)       │
├─────────────────────────────────────────────────────────────────────┤
│                  Authorization Layer                                 │
│  - Bootstraps auth_key via DH key exchange                         │
│  - Uses RSA to authenticate the server during bootstrap             │
│  - Manages sessions, session_id, salts                              │
│  - Message IDs (timestamp-derived), sequence numbers                │
├─────────────────────────────────────────────────────────────────────┤
│                    Transport Layer                                   │
│  - TCP full / abridged / intermediate                               │
│  - HTTP, WebSocket, UDP (deprecated)                                │
│  - MTProto proxy (obfuscation for censorship circumvention)         │
│  - DC (Data Center) routing                                         │
└─────────────────────────────────────────────────────────────────────┘
```

The layers compose like a stack of envelopes: the application puts a TL-encoded
API call in the innermost envelope, the authorization layer adds a message header
and salt, the encryption layer seals the whole thing with AES-IGE, and the
transport layer wraps it in the appropriate framing for TCP or HTTP.

```
Client                                      Telegram Server
  │                                               │
  │  Application: messages.sendMessage            │
  │    ↓ TL serialize                            │
  │  Auth: add salt, session_id, msg_id, seq_no  │
  │    ↓ AES-IGE encrypt with auth_key           │
  │  Transport: TCP abridged frame               │
  │─────────────────────────────────────────────▶│
  │                                               │  Transport: unwrap frame
  │                                               │  Decrypt with auth_key
  │                                               │  Verify msg_key
  │                                               │  Dispatch TL call
  │◀─────────────────────────────────────────────│
  │  Transport: TCP abridged frame               │
  │    ↑ AES-IGE decrypt                        │
  │  Auth: verify sequence number               │
  │  Application: receive RPC result            │
  │                                               │
```

## Key Concepts

### TL (Type Language) Schema

Telegram invented its own schema language called TL (Type Language) to describe
every data structure and remote procedure call in the API. Think of it as
Telegram's answer to Protobuf or Thrift — but predating both in Telegram's stack.

**Analogy:** TL is like a blueprint for an envelope. The blueprint says "this
envelope has a 'recipient' field of type Address, a 'message' field of type
Text, and a 'timestamp' field of type Date." When you fold the paper according
to the blueprint, you get a physical envelope. When you unfold it, you get the
fields back. TL does this for binary data.

**TL Syntax:**

```
constructorName#hexCrc32 field:Type field:Type = ReturnType;

Examples:
  inputPeer#9c95f7bb user_id:int access_hash:long = InputPeer;
  message#85d6cbe2 id:int from_id:Peer peer_id:Peer date:int message:string = Message;
  auth.signIn#bcd51581 phone_number:string phone_code_hash:string phone_code:string = auth.Authorization;
```

- **`constructorName`** — a human-readable name (lowercase, camelCase)
- **`#hexCrc32`** — the CRC32 of the TL declaration string, written in hex.
  This 4-byte value is the **constructor ID**, the type identifier on the wire.
- **`field:Type`** — fields and their types
- **`= ReturnType;`** — the abstract type this constructor belongs to

**Constructor ID:** The 4-byte constructor ID is computed as CRC32 of the
declaration with all whitespace normalized. This makes the schema self-versioning
— changing any field name or type changes the ID, immediately breaking
compatibility and forcing an upgrade. It is a clever design that avoids needing
a separate version field.

```
TL declaration (before CRC):
  "inputPeer user_id:int access_hash:long = InputPeer"
CRC32 of above string: 0x9C95F7BB
Wire representation: BB F7 95 9C (little-endian)
```

**Bare vs Boxed Types:**

A **boxed** type includes its constructor ID on the wire. A **bare** type omits
it (the receiver knows the type from context). Primitive types (`int`, `long`,
`string`, `bytes`) are bare. Compound types are boxed by default.

```
Boxed int:  BB F7 95 9C | 07 00 00 00    (constructor ID + value)
Bare int:                  07 00 00 00    (just the value, 4 bytes LE)
```

**Serialization Rules:**

```
Type        Wire encoding
──────────────────────────────────────────────────────────────────────
int         4 bytes, little-endian signed. Example: 42 → 2A 00 00 00
long        8 bytes, little-endian signed. Example: 0xDEAD → AD DE 00...
double      8 bytes, IEEE 754 double, little-endian
bool        True  → 4 bytes: B5 75 72 99  (CRC of "boolTrue")
            False → 4 bytes: 37 97 79 BC  (CRC of "boolFalse")
string      If len < 254:  1 byte length + bytes + padding to 4-byte align
            If len >= 254: 0xFE + 3-byte length (LE) + bytes + padding
bytes       Same as string
vector<T>   Constructor ID (15 C4 B5 1C) + 4-byte count + N serialized T
──────────────────────────────────────────────────────────────────────
```

**String encoding example** — the string "Hello" (5 bytes):

```
05           ← length byte (5)
48 65 6C 6C  ← 'H' 'e' 'l' 'l'
6F 00 00     ← 'o' + 2 padding bytes (total length 5+1=6, pad to 8)

TL strings are padded so that the next field starts on a 4-byte boundary.
The length byte itself counts as 1 byte, so 1 + 5 = 6, rounded up to 8.
```

### The Three MTProto Layers in Detail

#### Layer 1: Transport

The transport layer delivers raw bytes between client and server. MTProto
supports several transports — the client picks one at connection time:

```
Transport Comparison
════════════════════

┌───────────────────┬────────────┬─────────────┬───────────────────────┐
│ Transport         │ Overhead   │ Format      │ Use case              │
├───────────────────┼────────────┼─────────────┼───────────────────────┤
│ TCP Full          │ 8 bytes    │ len+seq+crc │ Reference impl        │
│ TCP Abridged      │ 1–4 bytes  │ variable    │ Default, most clients │
│ TCP Intermediate  │ 4 bytes    │ len only    │ Some MTProto proxies  │
│ HTTP              │ large      │ POST        │ Browsers, fallback    │
│ WebSocket         │ 2–14 bytes │ WS frame    │ Web clients           │
│ UDP               │ small      │ datagram    │ Deprecated            │
└───────────────────┴────────────┴─────────────┴───────────────────────┘
```

**TCP Full Transport** — the original, reference format:

```
┌────────────┬──────────────┬────────────────────────────┬────────────┐
│ Length     │ Seq No       │ Payload (encrypted message) │ CRC32      │
│ 4 bytes LE │ 4 bytes LE   │ variable                    │ 4 bytes LE │
└────────────┴──────────────┴────────────────────────────┴────────────┘

Length   = total packet length including this 4-byte field
Seq No   = sequential packet counter (separate from MTProto seq_no)
CRC32    = checksum of length + seq_no + payload
```

**TCP Abridged Transport** — used by virtually all production clients. Identified
by sending a single byte `0xEF` as the first byte of the connection:

```
Case 1: payload_length / 4 < 127
┌────────────────────────────────┐
│ 1 byte: (payload_len / 4)      │  ← length in 4-byte units, fits 1 byte
│ N bytes: payload               │
└────────────────────────────────┘

Case 2: payload_length / 4 >= 127
┌──────────────────────────────────────────────┐
│ 0x7F                                          │  ← marker byte
│ 3 bytes LE: (payload_len / 4)                │  ← length in 4-byte units
│ N bytes: payload                             │
└──────────────────────────────────────────────┘
```

Why divide by 4? MTProto messages are always a multiple of 4 bytes (due to TL
alignment). By storing length/4, we save 2 bits per length byte and the 1-byte
prefix covers messages up to 127 * 4 = 508 bytes (covering ~90% of messages).

**TCP Intermediate Transport** — simpler than abridged, no CRC or seq:

```
┌────────────┬────────────────────────────┐
│ Length     │ Payload (encrypted message) │
│ 4 bytes LE │ variable                    │
└────────────┴────────────────────────────┘
Identified by sending 4 bytes: EE EE EE EE at connection start.
```

**HTTP Transport** — each MTProto message is sent as the body of an HTTP POST
request to `https://DC_IP/api`. The response is the reply message body. Used
by Telegram Web and as a fallback when TCP is blocked.

#### Layer 2: Authorization

The authorization layer manages long-lived identity (the auth key) and
per-connection state (sessions, salts, message IDs).

**Auth Key** — a 2048-bit shared secret established once per device via
Diffie-Hellman key exchange. Stored on the device indefinitely. The same auth
key is used for all messages on that device. Different devices have different
auth keys, so Telegram's servers know which device each message comes from.

**Session** — a temporary context within an auth key. Each client connection
has a `session_id` (64 random bits). Sessions track which messages have been
acknowledged. Multiple sessions can share the same auth key (e.g., one for
background push, one for the foreground app).

**Message ID** — a 64-bit value that encodes a timestamp:

```
msg_id = unix_time_seconds * 2^32 + fractional_seconds * 2^32

Example: 2024-01-15 12:00:00.500 UTC
  seconds: 1705320000
  fraction: 0.500 → 0.5 * 2^32 = 2147483648 = 0x80000000

  msg_id = (1705320000 << 32) | 0x80000000
         = 0x65A57B8080000000

Wire (8 bytes LE): 00 00 00 80 80 7B A5 65
```

The server enforces that client message IDs are within ±30 seconds of server
time. If a client's clock is wrong, the server replies with `bad_msg_notification`
containing the correct server time. The client adjusts its time offset and retries.

The lower 2 bits of msg_id encode the direction:
- `0b00` — message from client
- `0b01` — reserved
- `0b10` — reserved  
- `0b11` — message from server

**seq_no** — a 32-bit sequence number. Incremented by 2 for content-bearing
messages, by 1 for acknowledgment-only messages. The lsb = 1 means "this
message requires an acknowledgment."

**Salt** — a 64-bit value provided by the server, included in every message.
It changes periodically (the server provides future salts in advance). This
prevents replay attacks: a replayed message would have a stale salt and be
rejected.

#### Layer 3: Encryption

Every message is encrypted with AES in IGE (Infinite Garble Extension) mode
using a per-message key derived from the auth key. This layer has no knowledge
of what the message contains — it just wraps bytes in a cryptographic seal.

## Auth Key Generation — The DH Bootstrap

When a fresh Telegram client starts, it has no shared secret with the server.
It needs to establish one. But it cannot use TLS — the chicken-and-egg problem:
TLS requires a certificate, and Telegram's servers are authenticated by their
RSA public keys hardcoded into the app.

**Analogy:** Imagine you want to set up a secret code with a friend in a room
full of eavesdroppers. You both shout colors, mix them with your private colors,
and exchange the mixtures. Neither the eavesdroppers nor even the intermediary
(Telegram's servers, playing the role of the room) can figure out your private
color from the public mixture. This is Diffie-Hellman key exchange, and it is
exactly what MTProto uses.

The auth key generation is a 6-step protocol:

```
Auth Key Generation: Full Sequence
═══════════════════════════════════

Client                                              Server
  │                                                   │
  │  Step 1: req_pq                                   │
  │  nonce = random 128 bits                          │
  │─── req_pq { nonce } ──────────────────────────▶  │
  │                                                   │
  │  Step 2: res_pq                                   │
  │◀── res_pq { nonce, server_nonce, pq,              │
  │             server_public_key_fingerprints } ─────│
  │                                                   │
  │  Step 3: Client factors pq                        │
  │  pq = p * q (p < q, both prime, ~32 bits each)   │
  │  Pick p and q                                     │
  │                                                   │
  │  Step 4: req_DH_params                            │
  │  Construct P_Q_inner_data:                        │
  │    { pq, p, q, nonce, server_nonce,               │
  │      new_nonce (256 random bits) }                │
  │  Encrypt with server's RSA public key (OAEP)      │
  │─── req_DH_params { nonce, server_nonce,           │
  │         p, q, fingerprint, encrypted_data } ────▶ │
  │                                                   │  Verify p*q == pq
  │                                                   │  Decrypt inner data
  │                                                   │  Generate g, dh_prime, a
  │                                                   │  Compute g_a = g^a mod dh_prime
  │                                                   │  Derive temp AES key from nonces
  │                                                   │  Encrypt server_DH_inner_data
  │                                                   │
  │  Step 5: server_DH_params_ok                      │
  │◀── server_DH_params_ok { nonce, server_nonce,     │
  │              encrypted_answer } ─────────────────│
  │                                                   │
  │  Decrypt encrypted_answer with temp AES key       │
  │  Verify nonce, server_nonce                       │
  │  Extract g, dh_prime, g_a                         │
  │  Generate b (random 2048-bit)                     │
  │  Compute g_b = g^b mod dh_prime                   │
  │  Compute auth_key = g_a^b mod dh_prime            │
  │  Derive temp AES key (same method as server)      │
  │  Encrypt client_DH_inner_data                     │
  │                                                   │
  │  Step 6: set_client_DH_params                     │
  │─── set_client_DH_params { nonce, server_nonce,    │
  │              encrypted_data } ──────────────────▶ │
  │                                                   │  Compute auth_key = g_b^a mod dh_prime
  │                                                   │  = g^(ab) mod dh_prime = same key!
  │                                                   │  Compute auth_key_hash
  │                                                   │
  │  dh_gen_ok                                        │
  │◀── dh_gen_ok { nonce, server_nonce,               │
  │                new_nonce_hash1 } ────────────────│
  │                                                   │
  │  Verify new_nonce_hash1                           │
  │  Store auth_key (2048 bits)                       │
  │  Compute auth_key_id = lower 64 bits of SHA-1(auth_key)
  │                                                   │
```

### Step-by-Step Wire Formats

**Step 1: req_pq**

Sent **unencrypted** (no auth key yet). All unencrypted MTProto messages have
a special header with `auth_key_id = 0`:

```
req_pq wire format (unencrypted)
══════════════════════════════════

┌─────────────────┬──────────────────────────────────────────────────┐
│ Field           │ Value / Description                              │
├─────────────────┼──────────────────────────────────────────────────┤
│ auth_key_id     │ 00 00 00 00 00 00 00 00 (8 bytes, all zero)      │
│                 │ Zero means "no encryption, this is a handshake"  │
├─────────────────┼──────────────────────────────────────────────────┤
│ message_id      │ 8 bytes LE timestamp-based ID                    │
├─────────────────┼──────────────────────────────────────────────────┤
│ message_length  │ 4 bytes LE length of message body                │
├─────────────────┼──────────────────────────────────────────────────┤
│ constructor_id  │ 60 46 9B 78  (req_pq#78969b60, LE)              │
├─────────────────┼──────────────────────────────────────────────────┤
│ nonce           │ 16 bytes of random data                          │
│                 │ Example: 3E 05 49 82 8C CA 27 E9 66 B3 01 19    │
│                 │          F9 70 DE 4D                             │
└─────────────────┴──────────────────────────────────────────────────┘

Hex example (28 bytes total):
  auth_key_id:    00 00 00 00 00 00 00 00
  message_id:     51 B8 2B D3 28 4A 57 61
  message_length: 14 00 00 00
  body:           60 46 9B 78              ← req_pq constructor
                  3E 05 49 82 8C CA 27 E9  ← nonce bytes 0–7
                  66 B3 01 19 F9 70 DE 4D  ← nonce bytes 8–15
```

**Step 2: res_pq**

The server's reply, also unencrypted:

```
res_pq fields
══════════════

┌─────────────────────┬──────────────────────────────────────────────┐
│ Field               │ Description                                  │
├─────────────────────┼──────────────────────────────────────────────┤
│ nonce               │ 16 bytes — echoed from client's req_pq       │
│                     │ (client verifies this matches what it sent)  │
├─────────────────────┼──────────────────────────────────────────────┤
│ server_nonce        │ 16 bytes — random, generated by server       │
│                     │ Used in nonce-chain for key derivation       │
├─────────────────────┼──────────────────────────────────────────────┤
│ pq                  │ TL bytes — the product of two primes p and q │
│                     │ Typically 8 bytes (64-bit product)           │
│                     │ Example: 17 ED 48 94 1A 08 F9 81 (big-endian│
│                     │ bytes encoding the integer)                  │
├─────────────────────┼──────────────────────────────────────────────┤
│ server_public_key   │ vector<long> — fingerprints (lower 64 bits   │
│ _fingerprints       │ of SHA-1) of server RSA public keys the      │
│                     │ client can use. Client picks one it knows.   │
└─────────────────────┴──────────────────────────────────────────────┘
```

**Step 3: Client factors pq**

The pq value is the product of exactly two 64-bit primes p and q. Because
pq is only ~64 bits (versus RSA's 2048-bit moduli), factoring it is fast —
a laptop can factor a 64-bit semiprime in milliseconds using Pollard's rho
algorithm. The client finds p and q such that pq = p × q and p < q.

```
Example factoring:
  pq = 0x17ED48941A08F981
     = 1722718525659109761 (decimal)
  p  = 0x494C553B  = 1230439739
  q  = 0x53911073  = 1401691251
  Verify: 1230439739 × 1401691251 = 1722718525659109761 ✓
```

**Step 4: P_Q_inner_data and req_DH_params**

The client constructs `P_Q_inner_data`, a TL structure that includes both nonces
and the factored p and q:

```
P_Q_inner_data (TL)
════════════════════

┌──────────────────┬─────────────────────────────────────────────────┐
│ Field            │ Description                                     │
├──────────────────┼─────────────────────────────────────────────────┤
│ pq               │ bytes — the original pq from server             │
├──────────────────┼─────────────────────────────────────────────────┤
│ p                │ bytes — the smaller prime factor                │
├──────────────────┼─────────────────────────────────────────────────┤
│ q                │ bytes — the larger prime factor                 │
├──────────────────┼─────────────────────────────────────────────────┤
│ nonce            │ 16 bytes — client's original nonce              │
├──────────────────┼─────────────────────────────────────────────────┤
│ server_nonce     │ 16 bytes — server's nonce from res_pq           │
├──────────────────┼─────────────────────────────────────────────────┤
│ new_nonce        │ 32 bytes — new random nonce from client         │
│                  │ CRITICAL: included in auth_key derivation.      │
│                  │ Prevents a compromised server from replaying    │
│                  │ old sessions.                                   │
└──────────────────┴─────────────────────────────────────────────────┘
```

This structure is serialized to TL bytes, then SHA-1 hashed, then the hash
and data are formatted as an RSAES-OAEP message and encrypted with the server's
2048-bit RSA public key. The encrypted blob is sent as `req_DH_params`.

**Step 5: server_DH_params_ok**

The server decrypts the RSA blob, verifies p × q == pq, generates the DH
parameters, and responds:

```
server_DH_inner_data (TL, decrypted from encrypted_answer)
═══════════════════════════════════════════════════════════

┌──────────────────┬─────────────────────────────────────────────────┐
│ Field            │ Description                                     │
├──────────────────┼─────────────────────────────────────────────────┤
│ nonce            │ 16 bytes — echoed from client                   │
├──────────────────┼─────────────────────────────────────────────────┤
│ server_nonce     │ 16 bytes — echoed                               │
├──────────────────┼─────────────────────────────────────────────────┤
│ g                │ int — the DH generator (small prime, e.g., 2)   │
├──────────────────┼─────────────────────────────────────────────────┤
│ dh_prime         │ bytes — 2048-bit safe prime (Sophie Germain)    │
│                  │ (p such that (p-1)/2 is also prime)             │
├──────────────────┼─────────────────────────────────────────────────┤
│ g_a              │ bytes — g^a mod dh_prime (server's DH pubkey)   │
├──────────────────┼─────────────────────────────────────────────────┤
│ server_time      │ int — server's Unix timestamp                   │
│                  │ Client uses this to correct its clock offset    │
└──────────────────┴──────────────────────────────────────────────────┘
```

The `encrypted_answer` is encrypted with a temporary AES key derived from the
two nonces:

```
Temporary AES key derivation (for server_DH_params exchange):
  tmp_aes_key = SHA-1(new_nonce + server_nonce) +
                SHA-1(server_nonce + new_nonce)[0:12]
              → 32 bytes total

  tmp_aes_iv  = SHA-1(server_nonce + new_nonce)[12:20] +
                SHA-1(new_nonce + new_nonce) +
                new_nonce[0:4]
              → 32 bytes total
```

**Step 6: set_client_DH_params → dh_gen_ok**

```
client_DH_inner_data (TL)
══════════════════════════

┌──────────────────┬─────────────────────────────────────────────────┐
│ Field            │ Description                                     │
├──────────────────┼─────────────────────────────────────────────────┤
│ nonce            │ 16 bytes — client's nonce                       │
├──────────────────┼─────────────────────────────────────────────────┤
│ server_nonce     │ 16 bytes — server's nonce                       │
├──────────────────┼─────────────────────────────────────────────────┤
│ retry_id         │ long — 0 for first attempt, previous            │
│                  │ auth_key_hash if retrying                       │
├──────────────────┼─────────────────────────────────────────────────┤
│ g_b              │ bytes — g^b mod dh_prime (client's DH pubkey)   │
└──────────────────┴──────────────────────────────────────────────────┘

Both sides now have:
  Server: g_b (from client), a (private) → auth_key = g_b^a mod dh_prime
  Client: g_a (from server), b (private) → auth_key = g_a^b mod dh_prime
  g_b^a = g^(ab) = g_a^b = the same value! ← Diffie-Hellman magic
```

**auth_key_id derivation:**

```
auth_key         = g^(ab) mod dh_prime   (2048-bit = 256 bytes)
auth_key_sha1    = SHA-1(auth_key)       (20 bytes)
auth_key_id      = auth_key_sha1[12:20]  (8 bytes, lower 64 bits)

The auth_key_id appears in every encrypted message header so the server
knows which of the millions of stored auth keys to use for decryption.
```

## Message Encryption (Cloud Chats)

Every encrypted MTProto message uses AES-256 in IGE mode. Understanding IGE
requires first understanding CBC, then seeing what IGE changes.

### AES-CBC vs AES-IGE

**CBC (Cipher Block Chaining)** — the well-known mode:

```
CBC Encryption:
                IV
                │
  Plaintext₁ ──⊕──▶ AES_Encrypt ──▶ Ciphertext₁
                                          │
  Plaintext₂ ──⊕──────────────────────▶  AES_Encrypt ──▶ Ciphertext₂
                                                               │
  Plaintext₃ ──⊕──────────────────────────────────────▶  AES_Encrypt ──▶ Ciphertext₃

Each plaintext block is XORed with the previous CIPHERTEXT before encryption.
```

**CBC Weakness:** If one ciphertext block is corrupted (say, Ciphertext₂ is
flipped), it garbles Plaintext₂ during decryption AND XORs predictable garbage
into Plaintext₃. Only 2 blocks are affected. This "controlled corruption" of
adjacent blocks is a useful property for certain attacks.

**IGE (Infinite Garble Extension)** — Telegram's chosen mode:

```
IGE Encryption:
  IV = (iv_first_half, iv_second_half) — two 16-byte halves

  x₁ = Plaintext₁ XOR iv_second_half    ← XOR with PREVIOUS CIPHERTEXT (initially IV second half)
  Ciphertext₁ = AES_Encrypt(x₁) XOR iv_first_half  ← XOR result with PREVIOUS PLAINTEXT (initially IV first half)

  x₂ = Plaintext₂ XOR Ciphertext₁       ← XOR with PREVIOUS CIPHERTEXT
  Ciphertext₂ = AES_Encrypt(x₂) XOR Plaintext₁     ← XOR with PREVIOUS PLAINTEXT

  x₃ = Plaintext₃ XOR Ciphertext₂
  Ciphertext₃ = AES_Encrypt(x₃) XOR Plaintext₂

In general:
  xᵢ      = Plaintextᵢ XOR Ciphertextᵢ₋₁
  Ciphertextᵢ = AES_Encrypt(xᵢ) XOR Plaintextᵢ₋₁

IGE Decryption:
  xᵢ       = AES_Decrypt(Ciphertextᵢ) XOR Plaintextᵢ₋₁
  Plaintextᵢ = xᵢ XOR Ciphertextᵢ₋₁
```

**Why IGE?** If a single ciphertext block is corrupted, it affects ALL
subsequent blocks during decryption — the garble propagates forward infinitely
(hence "Infinite Garble Extension"). This means an active attacker cannot
surgically flip one block without corrupting the entire rest of the message,
making chosen-ciphertext attacks harder. Telegram chose IGE because it was a
well-studied but underdeployed mode with this desirable forward-propagation
property.

**Note:** IGE alone does not provide authentication. MTProto uses the msg_key
as a MAC to verify integrity before decrypting.

### msg_key Derivation (MTProto 2.0)

The `msg_key` is 128 bits (16 bytes). It both identifies which AES key/IV to
use AND acts as a message authentication code (MAC):

```
MTProto 2.0 msg_key derivation
════════════════════════════════

For client→server messages:
  msg_key_large = SHA-256(auth_key[88:88+32] + plaintext)
  msg_key = msg_key_large[8:24]   ← middle 16 bytes

For server→client messages:
  msg_key_large = SHA-256(auth_key[96:96+32] + plaintext)
  msg_key = msg_key_large[8:24]

(MTProto 1.0 used auth_key[0:32] and SHA-1 instead)
```

The auth_key fragments at offsets 88 and 96 ensure that client-to-server and
server-to-client messages use different message key derivation paths, preventing
cross-directional replay.

### AES Key and IV Derivation

Given `auth_key` (256 bytes) and `msg_key` (16 bytes):

```
AES key/IV derivation from msg_key (MTProto 2.0)
═════════════════════════════════════════════════

x = 0 for client→server, 8 for server→client

sha256_a = SHA-256(msg_key + auth_key[x:x+36])
sha256_b = SHA-256(auth_key[x+40:x+76] + msg_key)

aes_key = sha256_a[0:8] + sha256_b[8:24] + sha256_a[24:32]  ← 32 bytes
aes_iv  = sha256_b[0:8] + sha256_a[8:24] + sha256_b[24:32]  ← 32 bytes

The AES key and IV are derived independently for each message.
Changing any byte of the payload changes msg_key, which changes
aes_key and aes_iv completely (SHA-256's avalanche effect).
```

### Plaintext (Inner) Message Structure

Before encryption, the plaintext has this structure:

```
MTProto Plaintext Message Layout
══════════════════════════════════

┌───────────────────┬──────────────────────────────────────────────────┐
│ Field             │ Description                                      │
├───────────────────┼──────────────────────────────────────────────────┤
│ salt              │ 8 bytes — server salt (provided by server,       │
│                   │ changes periodically to prevent replay)          │
├───────────────────┼──────────────────────────────────────────────────┤
│ session_id        │ 8 bytes — random ID for this connection session  │
│                   │ Different for each TCP connection                │
├───────────────────┼──────────────────────────────────────────────────┤
│ message_id        │ 8 bytes LE — timestamp-derived ID (see above)    │
├───────────────────┼──────────────────────────────────────────────────┤
│ seq_no            │ 4 bytes LE — sequence number                     │
│                   │ Odd for content messages, even for pure-ack msgs │
├───────────────────┼──────────────────────────────────────────────────┤
│ message_data_len  │ 4 bytes LE — byte length of message_data         │
├───────────────────┼──────────────────────────────────────────────────┤
│ message_data      │ N bytes — TL-serialized API call or response     │
├───────────────────┼──────────────────────────────────────────────────┤
│ padding           │ 12–1024 random bytes, total length multiple of 16│
│                   │ MTProto 2.0: minimum 12 bytes padding            │
└───────────────────┴──────────────────────────────────────────────────┘

Total plaintext size: 32 + message_data_len + padding_len
                      must be a multiple of 16 (AES block size)
```

### Wire Format of an Encrypted Message

After encryption, the message is sent as:

```
MTProto Encrypted Message Wire Format
═══════════════════════════════════════

┌───────────────────┬──────────────────────────────────────────────────┐
│ Field             │ Bytes │ Description                              │
├───────────────────┼───────┼──────────────────────────────────────────┤
│ auth_key_id       │  8    │ Lower 64 bits of SHA-1(auth_key)         │
│                   │       │ Tells server which key to use            │
├───────────────────┼───────┼──────────────────────────────────────────┤
│ msg_key           │ 16    │ Middle 16 bytes of SHA-256(auth_key +    │
│                   │       │ plaintext). Acts as MAC + IV seed.       │
├───────────────────┼───────┼──────────────────────────────────────────┤
│ encrypted_data    │  N    │ AES-IGE encrypted plaintext              │
│                   │       │ N must be multiple of 16                 │
└───────────────────┴───────┴──────────────────────────────────────────┘

Total header overhead: 8 + 16 = 24 bytes per message.
```

**Concrete byte-level example** — a tiny "ping" message:

```
Example: client sends ping (ping_id = 0x1234567890ABCDEF)

Plaintext (before encryption):
  Bytes 0–7:   salt          = 12 5F 7D 8E 9A BC DE F0  (current server salt)
  Bytes 8–15:  session_id    = A1 B2 C3 D4 E5 F6 07 18
  Bytes 16–23: message_id    = 00 00 00 80 80 7B A5 65  (timestamp)
  Bytes 24–27: seq_no        = 01 00 00 00              (content msg = odd)
  Bytes 28–31: data_len      = 0C 00 00 00              (12 bytes)
  Bytes 32–35: constructor   = DA 9B 3F 7C              (ping#7c3f9bda LE)
  Bytes 36–43: ping_id       = EF CD AB 90 78 56 34 12  (LE)
  Bytes 44–59: padding       = 16 random bytes to reach multiple of 16

Total plaintext: 60 bytes (multiple of 16? 60 = 4×15... no, add 4 more bytes padding)
Actually: 32 header + 12 data + 12 min padding + 4 alignment = 60 bytes → need 64
Padding: 16 random bytes → total = 60... let's make it 64 with 20 bytes padding.

After AES-IGE encryption (64 bytes → 64 bytes ciphertext)
Wire output:
  auth_key_id:     [8 bytes from SHA-1 of auth_key]
  msg_key:         [16 bytes from SHA-256 of auth_key fragment + plaintext]
  encrypted_data:  [64 bytes of AES-IGE ciphertext]
  Total:           88 bytes on wire
```

### Verification on Receipt

```
Message verification steps:
═══════════════════════════

1. Read auth_key_id from first 8 bytes
2. Look up auth_key in server's key store
3. Read msg_key from next 16 bytes
4. Derive aes_key and aes_iv from auth_key + msg_key
5. Decrypt encrypted_data with AES-IGE using aes_key, aes_iv
6. Recompute expected_msg_key = SHA-256(auth_key[88:120] + plaintext)[8:24]
7. Verify: expected_msg_key == msg_key  ← REJECT if mismatch (tampering!)
8. Verify salt matches current or recent server salt
9. Verify session_id matches open session
10. Verify message_id within ±30 seconds of server time
11. Verify seq_no is expected (not replayed)
12. Dispatch message_data as TL-encoded API call
```

## Session Management and Message Containers

### msg_container — Batching Multiple Messages

To reduce round trips, MTProto allows bundling multiple messages into a single
packet using `msg_container`:

```
msg_container#73f1f8dc messages:vector<message> = MessageContainer;

Each bundled message:
  msg_id:     8 bytes LE
  seqno:      4 bytes LE
  bytes:      4 bytes LE (length of body)
  body:       N bytes (TL-encoded message)

msg_container wire format:
  73 F1 F8 DC               ← constructor ID
  [count: 4 bytes LE]       ← number of messages
  [message1][message2]...   ← each message as above
```

**Analogy:** Instead of sending 5 separate envelopes, stuff them all into one
large envelope. The postal system delivers one envelope; the recipient opens
it and finds 5 smaller ones inside. This is especially valuable for Telegram's
update batching — when the server has 20 new messages for you, it sends one
TCP frame containing all 20 instead of 20 separate frames.

### Message Acknowledgment

The receiver must acknowledge every content-bearing message. Acknowledgments
are sent via `msgs_ack`:

```
msgs_ack#62d6b459 msg_ids:vector<long> = MsgsAck;

Wire: 59 B4 D6 62               ← constructor
      [count: 4 bytes LE]       ← number of message IDs being acked
      [msg_id₁][msg_id₂]...     ← 8 bytes each
```

Acknowledgments can be piggy-backed on outgoing content messages (included in a
`msg_container` alongside the real API call) to avoid extra round trips.

### Ping / Pong

Long-lived TCP connections need a keepalive mechanism. MTProto uses ping/pong:

```
ping#7abe77ec ping_id:long = Pong;  (client sends ping)
pong#347773c5 msg_id:long ping_id:long = Pong;  (server replies)
```

The `ping_id` is a random 64-bit value chosen by the client. The server echoes
it back. The client verifies the pong's ping_id matches what it sent, proving
the connection is still alive end-to-end.

### Future Salts

The salt in every message is a 64-bit value provided by the server. Salts
rotate periodically. To avoid interruption when a salt expires, the server
proactively sends upcoming salts:

```
future_salts#ae500895
  req_msg_id: long
  now:        int
  salts:      vector<future_salt>

future_salt:
  valid_since: int   ← Unix timestamp when this salt becomes valid
  valid_until: int   ← Unix timestamp when this salt expires
  salt:        long  ← the salt value

The client stores future salts and switches to the next one
automatically when valid_since is reached.
```

## Secret Chats vs Cloud Chats

Telegram offers two types of conversations with fundamentally different
security guarantees:

```
Cloud Chats vs Secret Chats
════════════════════════════

┌─────────────────────────┬─────────────────┬────────────────────────┐
│ Property                │ Cloud Chat      │ Secret Chat            │
├─────────────────────────┼─────────────────┼────────────────────────┤
│ Encryption              │ MTProto to DCs  │ End-to-end (client↔    │
│                         │ (server holds   │ client, server never   │
│                         │ keys)           │ sees plaintext)        │
├─────────────────────────┼─────────────────┼────────────────────────┤
│ Server access           │ Yes (for backup,│ No — server only       │
│                         │ multi-device)   │ relays opaque bytes    │
├─────────────────────────┼─────────────────┼────────────────────────┤
│ Multi-device sync       │ Yes             │ No — one device pair   │
├─────────────────────────┼─────────────────┼────────────────────────┤
│ Message storage         │ On Telegram DCs │ Local device only      │
├─────────────────────────┼─────────────────┼────────────────────────┤
│ Self-destruct timers    │ No              │ Yes                    │
├─────────────────────────┼─────────────────┼────────────────────────┤
│ Perfect Forward Secrecy │ Per-session     │ Yes, with rekeying     │
├─────────────────────────┼─────────────────┼────────────────────────┤
│ Voice/video calls       │ N/A             │ Yes (E2E encrypted)    │
└─────────────────────────┴─────────────────┴────────────────────────┘
```

### Secret Chat Key Exchange

Secret chats use a second DH key exchange, this time between two client
devices (mediated by Telegram's servers):

```
Secret Chat Establishment
══════════════════════════

Alice's Device                  Telegram Server        Bob's Device
     │                                │                     │
     │  Generate a (random 2048-bit)  │                     │
     │  Compute g_a = g^a mod p       │                     │
     │                                │                     │
     │── messages.requestEncryption ─▶│                     │
     │   { user_id: Bob, g_a: g^a }   │                     │
     │                                │── updateEncryption ▶│
     │                                │   { g_a }           │
     │                                │                     │  Generate b
     │                                │                     │  g_b = g^b mod p
     │                                │                     │  key = g_a^b = g^(ab)
     │                                │◀── messages.acceptEncryption ─│
     │                                │    { g_b }          │
     │◀── updateEncryption ───────────│                     │
     │    { g_b }                     │                     │
     │  key = g_b^a = g^(ab)         │                     │
     │  Both sides now have key!      │                     │
     │                                │                     │
```

The server only sees g_a and g_b — public values from which computing g^(ab)
requires solving the discrete logarithm problem, which is computationally
infeasible for 2048-bit primes.

### Visualization Keys (Key Fingerprints as Emoji)

After establishing a secret chat key, Telegram displays a fingerprint as a 4×4
grid of emoji icons. Both users should see the same pattern — if they do not,
the secret chat may have been intercepted:

```
Key fingerprint → emoji visualization:
  1. Take SHA-256 of the secret chat key → 32 bytes
  2. Interpret as a sequence of 16 values in range 0–255
  3. Map each value to one of 512 emoji icons (using a fixed mapping)
  4. Display as a 4×4 grid

  Example fingerprint display:
    🦊 🌈 🎭 🎪
    🚂 🏔️  🎸 🐙
    🎯 🌺 🦁 🎡
    🔮 🎨 🌊 🎃

If Alice and Bob both see this grid, their secret chat keys match.
If they differ, someone performed a man-in-the-middle attack.
```

### Perfect Forward Secrecy and Rekeying

Secret chats support rekeying: after exchanging 100 messages or 7 days,
the two devices perform a new DH exchange to establish a fresh key. This is
**perfect forward secrecy**: even if an attacker records all ciphertext today
and later compromises one device, they cannot decrypt past messages because
the old keys are deleted.

```
Rekeying protocol:
  1. Alice generates new_a, computes new_g_a = g^new_a mod p
  2. Alice sends: messages.requestEncryption with new_g_a
  3. Bob generates new_b, computes new_g_b = g^new_b mod p
  4. Bob sends: messages.acceptEncryption with new_g_b
  5. Both compute new_key = SHA-256(g^(new_a * new_b) mod p)
  6. Old key is securely erased from both devices
```

### Self-Destruct Timers

Telegram implements self-destruct via server-enforced message expiry:

```
Self-destruct implementation:
  - Sender sets a ttl (time-to-live) on the message
  - Server stores the expiry timestamp
  - After ttl seconds from first read, the server:
    1. Marks the message as expired
    2. Stops delivering it to any future sessions of the recipient
    3. Sends a service message to both parties to delete locally
  - On secret chats: the encrypted blob is also deleted from server
  - The timer starts only when the recipient reads the message
    (open receipt + timestamp sent back to server)
```

## Channels and the Updates System

### TL Object Hierarchy

Telegram's data model is built around TL-defined objects. The key entities:

```
Telegram Object Hierarchy (TL types)
═════════════════════════════════════

Peer (abstract, who is the recipient?)
  ├── peerUser#9db1bc6d user_id:int
  ├── peerChat#bad0e5bb chat_id:int
  └── peerChannel#bddde532 channel_id:int

Chat (abstract, any kind of chat)
  ├── chat#3bda1bde id:int title:string photo:ChatPhoto
  │     participants_count:int date:int ...
  └── channel#d31a961e id:int title:string username:string
        photo:ChatPhoto participants_count:int ...

Message (abstract)
  └── message#85d6cbe2
        id:int
        from_id:Peer          ← who sent it (null for channel posts)
        peer_id:Peer          ← which chat/channel/user it belongs to
        date:int              ← Unix timestamp
        message:string        ← text content
        media:MessageMedia    ← attached photo/video/file (optional)
        reply_to:MessageReplyHeader (optional)
        fwd_from:MessageFwdHeader (optional)
        ...
```

### pts: Persistent Timestamp (Update Sequence Counter)

Telegram's server maintains a monotonically increasing counter called `pts`
(persistent timestamp — confusingly named; it is not a timestamp but a sequence
number). Every change to a user's message state increments `pts`.

```
pts lifecycle:
  - User starts with pts = 0
  - Each event (new message, edit, deletion) increments pts by N
    (N = number of messages affected; usually 1)
  - Client always knows its current pts
  - Client can fetch updates since a given pts:
      updates.getDifference { pts: 1234, date: ..., qts: ... }
  - Server replies with all events from pts=1234 to current

pts is per-user for direct messages.
pts is per-channel for channel messages (separate counter per channel).
```

**Analogy:** `pts` is like a bank account transaction number. Every time money
moves (a message is sent/edited/deleted), the transaction count increments.
At any point, you can call the bank and ask "give me all transactions since
transaction #1234." The bank sends exactly the transactions you missed.

### The Updates Object Hierarchy

When connected, the server pushes updates to clients in real time:

```
Updates (abstract, returned by long-poll or server push)
══════════════════════════════════════════════════════════

updatesTooLong
  "You are too far behind, call updates.getDifference to resync"

updateShortMessage#2296d2c8
  id:int flags:# from_id:int message:string pts:int pts_count:int
  (Optimization: for simple text messages to/from a user, avoid
   full Update envelope overhead. Saves ~60 bytes per message.)

updateShort#78d4dec1
  update:Update date:int
  (One update that doesn't need full chat/user context objects)

updatesCombined#725b04c3
  updates:vector<Update>   ← the actual update objects
  users:vector<User>       ← User objects referenced by updates
  chats:vector<Chat>       ← Chat/Channel objects referenced
  date:int
  seq_start:int            ← beginning of gap-free range
  seq:int                  ← end of gap-free range

updates#74ae4240
  updates:vector<Update>   ← same as updatesCombined but:
  users:vector<User>
  chats:vector<Chat>
  date:int
  seq:int                  ← just the current seq, no gap tracking
```

**Why separate users and chats from updates?** The `updates` list may contain
many references to the same User (e.g., 100 messages from the same person).
Embedding the full User object in each update would be wasteful. Instead,
users and chats are deduped into separate lists and updates refer to them by ID.

### Channel-Specific Updates

Channels have their own independent pts counter. When a channel has new messages:

```
Update flow for channel messages:
  1. Server pushes:
       updateNewChannelMessage { message: Message, pts: int, pts_count: int }
  2. Client checks: is our stored channel_pts == (pts - pts_count)?
     - Yes: apply update, set channel_pts = pts
     - No: we missed some updates, call:
         updates.getChannelDifference {
           channel: InputChannel,
           filter: ChannelMessagesFilterEmpty,
           pts: our_pts,
           limit: 100
         }
```

## MTProto Proxy

### Why Proxies Exist

Telegram has been blocked in various countries (Russia 2018–2020, Iran, China,
etc.). MTProto proxies allow users to circumvent these blocks. The proxy
connects to Telegram's DCs on the user's behalf, and the traffic between the
user and the proxy is obfuscated to resist deep packet inspection (DPI).

**Analogy:** Imagine a country has blocked all direct phone calls to a foreign
number. A proxy is like a friend in another country: you call your friend,
your friend calls the blocked number, and relays everything. The phone company
only sees you calling your friend's local number.

### The Obfuscation Layer

The first 64 bytes sent by the client look like random data. This prevents
DPI systems from identifying MTProto connections by their header pattern:

```
Obfuscated MTProto connection initiation
═════════════════════════════════════════

Client generates 64 random bytes: r[0..63]
  - r[0..3]  must not be  EF EF EF EF  (intermediate init)
  - r[0..3]  must not be  EE EE EE EE  (abridged init with EE)
  - r[0..3]  must not be  DD DD DD DD
  - r[0..1]  must not be  47 45       ("GE" — HTTP GET)
  - r[0..1]  must not be  50 4F       ("PO" — HTTP POST)
  - r[0..3]  must not be  16 03 01/02 (TLS ClientHello)
  (retry with new random bytes if any of the above match)

Key extraction from the 64-byte nonce:
  encrypt_key = r[8:40]   ← 32 bytes
  encrypt_iv  = r[40:56]  ← 16 bytes
  decrypt_key = r[56:64] + [reverse of r[8:40]][0:24]  ← 32 bytes
  decrypt_iv  = [reverse of r[40:56]]  ← 16 bytes

These keys initialize AES-256-CTR streams in both directions.
All subsequent bytes (including the transport init byte 0xEF) are
XOR'd through these CTR streams.

The 64-byte nonce is sent to the server (or proxy) in plaintext,
and the server uses the same extraction to set up its CTR streams.
```

### Secret-Based Proxy Configuration

Users configure proxies with a hex "secret" — typically 32 hex characters (16
bytes) or 66 characters (1 byte prefix + 32 bytes + domain for fake-TLS mode):

```
Proxy secret formats:
  dd<32 hex>  ← "dd" prefix = fake TLS mode. The 32 hex bytes are the
               secret key, and an additional domain is encoded after.
  <32 hex>    ← Plain mode. Just 16 random bytes.

In fake-TLS mode, the client's traffic mimics TLS ClientHello with
the specified domain name (e.g., "cdn.cloudflare.com"), making it
appear as a legitimate HTTPS connection to a CDN.

The proxy verifies the client knows the secret by checking a field
in the obfuscated handshake. Clients without the correct secret
cannot use the proxy — this prevents the proxy from being abused.
```

### DC (Data Center) Discovery

Telegram operates data centers worldwide. The client needs to know which DC
to connect to:

```
DC ID → IP address mapping (hardcoded in app):
  DC 1:  149.154.175.50   (USA, Miami)
  DC 2:  149.154.167.51   (Netherlands)
  DC 3:  149.154.175.100  (USA, Miami)
  DC 4:  149.154.167.91   (Netherlands)
  DC 5:  91.108.56.130    (Singapore)

Initial connection: client connects to DC 2 (main DC for new users)
After auth: server may redirect client with:
  auth.exportAuthorization { dc_id: 4 }  ← "go connect to DC 4 instead"
  
Client exports its auth, connects to the new DC, imports auth there.
The auth_key may differ per DC (client runs DH separately with each DC).
```

## Algorithms

### Pseudocode: Auth Key Generation

```
function generate_auth_key(server_host, server_port):
  # Step 1: req_pq
  nonce = random_bytes(16)
  send(unencrypted_message(req_pq { nonce }))

  # Step 2: res_pq
  res = recv_unencrypted_message()
  assert res.nonce == nonce
  server_nonce = res.server_nonce
  pq = res.pq  # big-endian bytes encoding an integer

  # Step 3: factor pq
  p, q = pollard_rho_factor(bytes_to_int(pq))
  assert p < q
  assert p * q == bytes_to_int(pq)

  # Step 4: req_DH_params
  new_nonce = random_bytes(32)
  inner = P_Q_inner_data {
    pq: pq, p: int_to_bytes(p), q: int_to_bytes(q),
    nonce: nonce, server_nonce: server_nonce, new_nonce: new_nonce
  }
  inner_bytes = tl_serialize(inner)
  # RSA-OAEP encrypt:
  # pad inner_bytes with SHA-1 hash, random padding to 255 bytes
  encrypted_data = rsa_oaep_encrypt(server_public_key, inner_bytes)
  send(unencrypted_message(req_DH_params {
    nonce, server_nonce,
    p: int_to_bytes(p), q: int_to_bytes(q),
    public_key_fingerprint: chosen_fingerprint,
    encrypted_data
  }))

  # Step 5: server_DH_params_ok
  resp = recv_unencrypted_message()
  assert resp.nonce == nonce
  assert resp.server_nonce == server_nonce

  # Derive temp AES key from nonces
  tmp_key = sha1(new_nonce + server_nonce) +
            sha1(server_nonce + new_nonce)[0:12]
  tmp_iv  = sha1(server_nonce + new_nonce)[12:20] +
            sha1(new_nonce + new_nonce) +
            new_nonce[0:4]

  # Decrypt server's DH params (AES-IGE with tmp_key, tmp_iv)
  decrypted = aes_ige_decrypt(resp.encrypted_answer, tmp_key, tmp_iv)
  # decrypted = SHA-1 (20 bytes) + TL-serialized server_DH_inner_data
  hash = decrypted[0:20]
  inner_data_bytes = decrypted[20:]
  assert sha1(inner_data_bytes) == hash  # integrity check

  server_dh = tl_deserialize(server_dh_inner_data, inner_data_bytes)
  assert server_dh.nonce == nonce
  assert server_dh.server_nonce == server_nonce
  g = server_dh.g          # small prime, e.g., 2
  dh_prime = bytes_to_int(server_dh.dh_prime)
  g_a = bytes_to_int(server_dh.g_a)

  # Step 6: set_client_DH_params
  b = random_int(2048_bits)
  g_b = pow(g, b, dh_prime)     # g^b mod dh_prime

  client_inner = client_DH_inner_data {
    nonce, server_nonce, retry_id: 0, g_b: int_to_bytes(g_b)
  }
  client_inner_bytes = tl_serialize(client_inner)
  # Pad: SHA-1(client_inner_bytes) + client_inner_bytes + random_pad
  # (pad to multiple of 16)
  encrypted = aes_ige_encrypt(sha1(client_inner_bytes) + client_inner_bytes
              + padding, tmp_key, tmp_iv)

  send(unencrypted_message(set_client_DH_params {
    nonce, server_nonce, encrypted_data: encrypted
  }))

  # dh_gen_ok
  result = recv_unencrypted_message()
  assert result.type == dh_gen_ok
  assert result.nonce == nonce
  assert result.server_nonce == server_nonce

  # Compute auth_key
  auth_key_int = pow(g_a, b, dh_prime)     # g_a^b = g^(ab)
  auth_key = int_to_bytes_2048(auth_key_int)  # always pad to 256 bytes

  # Verify new_nonce_hash1
  auth_key_aux = sha1(auth_key)[0:8]
  expected_hash1 = sha1(new_nonce + bytes([1]) + auth_key_aux)[4:20]
  assert result.new_nonce_hash1 == expected_hash1

  auth_key_id = sha1(auth_key)[12:20]  # lower 8 bytes
  return auth_key, auth_key_id
```

### Pseudocode: AES-IGE Encrypt

```
function aes_ige_encrypt(plaintext, key, iv):
  # iv is 32 bytes: first 16 = iv_first_half, next 16 = iv_second_half
  assert len(plaintext) % 16 == 0
  iv_prev_ciphertext = iv[0:16]   # "previous ciphertext" starts as iv first half
  iv_prev_plaintext  = iv[16:32]  # "previous plaintext" starts as iv second half
  ciphertext = []

  for block in chunks_of_16(plaintext):
    xored = xor_bytes(block, iv_prev_ciphertext)
    encrypted = aes_ecb_encrypt_block(xored, key)
    cipher_block = xor_bytes(encrypted, iv_prev_plaintext)

    iv_prev_plaintext  = block         # update: prev plaintext = current plaintext
    iv_prev_ciphertext = cipher_block  # update: prev ciphertext = current ciphertext
    ciphertext.append(cipher_block)

  return concat(ciphertext)


function aes_ige_decrypt(ciphertext, key, iv):
  assert len(ciphertext) % 16 == 0
  iv_prev_ciphertext = iv[0:16]
  iv_prev_plaintext  = iv[16:32]
  plaintext = []

  for cipher_block in chunks_of_16(ciphertext):
    xored = xor_bytes(cipher_block, iv_prev_plaintext)
    decrypted = aes_ecb_decrypt_block(xored, key)
    plain_block = xor_bytes(decrypted, iv_prev_ciphertext)

    iv_prev_plaintext  = plain_block   # update: prev plaintext = current plaintext
    iv_prev_ciphertext = cipher_block  # update: prev ciphertext = current ciphertext
    plaintext.append(plain_block)

  return concat(plaintext)
```

### Pseudocode: Encrypt MTProto Message

```
function encrypt_message(auth_key, salt, session_id, seq_no, message_data):
  # Build plaintext
  padding_len = compute_padding(len(message_data))
  # MTProto 2.0: minimum 12 bytes padding, total length % 16 == 0
  padding = random_bytes(padding_len)

  plaintext = (
    salt            +   # 8 bytes
    session_id      +   # 8 bytes
    message_id      +   # 8 bytes (timestamp-derived)
    to_le_u32(seq_no) + # 4 bytes
    to_le_u32(len(message_data)) +   # 4 bytes
    message_data    +   # N bytes
    padding             # P bytes (minimum 12, multiple of 16 total)
  )

  # Derive msg_key (direction: client→server uses offset 88)
  msg_key_large = sha256(auth_key[88:120] + plaintext)
  msg_key = msg_key_large[8:24]  # middle 16 bytes

  # Derive AES key and IV
  sha256_a = sha256(msg_key + auth_key[0:36])
  sha256_b = sha256(auth_key[40:76] + msg_key)
  aes_key  = sha256_a[0:8]  + sha256_b[8:24] + sha256_a[24:32]
  aes_iv   = sha256_b[0:8]  + sha256_a[8:24] + sha256_b[24:32]

  encrypted_data = aes_ige_encrypt(plaintext, aes_key, aes_iv)

  auth_key_id = sha1(auth_key)[12:20]  # lower 8 bytes

  return auth_key_id + msg_key + encrypted_data


function decrypt_message(auth_key, data):
  auth_key_id    = data[0:8]
  msg_key        = data[8:24]
  encrypted_data = data[24:]

  # Derive AES key/IV (server→client uses offset 96 for key extraction)
  sha256_a = sha256(msg_key + auth_key[8:44])
  sha256_b = sha256(auth_key[48:84] + msg_key)
  aes_key  = sha256_a[0:8]  + sha256_b[8:24] + sha256_a[24:32]
  aes_iv   = sha256_b[0:8]  + sha256_a[8:24] + sha256_b[24:32]

  plaintext = aes_ige_decrypt(encrypted_data, aes_key, aes_iv)

  # Verify msg_key
  expected_msg_key_large = sha256(auth_key[96:128] + plaintext)
  expected_msg_key       = expected_msg_key_large[8:24]
  assert msg_key == expected_msg_key  # reject if mismatch!

  # Parse plaintext fields
  salt          = plaintext[0:8]
  session_id    = plaintext[8:16]
  message_id    = plaintext[16:24]
  seq_no        = from_le_u32(plaintext[24:28])
  data_len      = from_le_u32(plaintext[28:32])
  message_data  = plaintext[32:32+data_len]

  return Message(salt, session_id, message_id, seq_no, message_data)
```

### Pseudocode: Pollard's Rho Factoring

The client must factor `pq` in step 3 of auth key generation:

```
function pollard_rho_factor(n):
  # n = p * q where p and q are 32-bit primes
  # Pollard's rho finds a factor in O(n^(1/4)) time
  # For 64-bit n, this is O(2^16) — milliseconds

  if n % 2 == 0:
    return 2, n // 2

  x = random_int(2, n - 1)
  c = random_int(1, n - 1)
  y = x
  d = 1

  while d == 1:
    x = (x * x + c) % n
    y = (y * y + c) % n
    y = (y * y + c) % n   # y advances twice as fast as x
    d = gcd(abs(x - y), n)

  if d != n:
    p = d
    q = n // d
    return (min(p, q), max(p, q))
  else:
    # Cycle detected without finding factor, retry with new c
    return pollard_rho_factor(n)
```

### Pseudocode: TL Serializer/Deserializer

```
function tl_serialize_string(s: bytes) -> bytes:
  n = len(s)
  if n < 254:
    length_prefix = bytes([n])
    padding_len = (4 - (1 + n) % 4) % 4
  else:
    length_prefix = bytes([0xFE]) + to_le_u24(n)
    padding_len = (4 - n % 4) % 4
  return length_prefix + s + bytes(padding_len)

function tl_deserialize_string(data: bytes, offset: int) -> (bytes, int):
  first = data[offset]
  if first < 254:
    n = first
    start = offset + 1
    padding_len = (4 - (1 + n) % 4) % 4
  else:
    n = from_le_u24(data[offset+1:offset+4])
    start = offset + 4
    padding_len = (4 - n % 4) % 4
  s = data[start:start+n]
  return s, start + n + padding_len

function tl_serialize_vector(items, serialize_fn) -> bytes:
  header = bytes([0x1C, 0xB5, 0xC4, 0x15])  # vector constructor 0x1cb5c415
  count = to_le_u32(len(items))
  body = concat(serialize_fn(item) for item in items)
  return header + count + body
```

## Test Strategy

### Unit Tests

**TL Serialization:**

```
test: serialize_bare_int_42
  input:  42
  expect: 2A 00 00 00

test: serialize_string_hello
  input:  "Hello" (5 bytes)
  expect: 05 48 65 6C 6C 6F 00 00
          ↑  ↑──────────────┘↑──↑
          len   H e l l o   padding to 4-byte boundary

test: serialize_long_negative
  input:  -1 (as int64)
  expect: FF FF FF FF FF FF FF FF

test: serialize_bool_true
  input:  True
  expect: B5 75 72 99  (CRC32 of "boolTrue")

test: serialize_vector_of_int
  input:  [1, 2, 3]
  expect: 1C B5 C4 15     ← vector constructor
          03 00 00 00     ← count = 3
          01 00 00 00     ← 1
          02 00 00 00     ← 2
          03 00 00 00     ← 3

test: constructor_id_correctness
  input:  "inputPeer user_id:int access_hash:long = InputPeer"
  expect: CRC32 = 0x9C95F7BB
```

**AES-IGE:**

```
test: ige_encrypt_one_block
  key:  0101010101010101010101010101010101010101010101010101010101010101
  iv:   00000000000000000000000000000000 00000000000000000000000000000000
  pt:   00000000000000000000000000000000  (16 zero bytes)
  ct:   (AES_ECB_encrypt(pt XOR iv_second_half) XOR iv_first_half)
        = AES_ECB_encrypt(00...00) XOR 00...00
        = AES_ECB_encrypt(00...00)
        = 7D F7 6B 0C 1A B8 99 B3 3E 42 F0 47 B9 1B 54 6F

test: ige_decrypt_is_inverse_of_encrypt
  For any key, iv, plaintext:
    aes_ige_decrypt(aes_ige_encrypt(pt, key, iv), key, iv) == pt

test: ige_error_propagation
  Corrupt byte 5 of ciphertext:
    Decrypted block 0: garbled (uses corrupted ciphertext in XOR)
    Decrypted block 1: ALL subsequent blocks: also garbled
    This is the "infinite garble" property — verify it holds.
```

**msg_key and Key Derivation:**

```
test: msg_key_derivation_client_to_server
  auth_key: (256 bytes of 0x01)
  plaintext: (64 bytes of 0x00)
  # Client→server uses auth_key[88:120]
  msg_key_large = SHA-256(auth_key[88:120] + plaintext)
  msg_key = msg_key_large[8:24]
  # Verify against known-good reference implementation

test: aes_key_derivation_deterministic
  Same auth_key + msg_key must always produce same aes_key and aes_iv.
  Verify with two independent computations.
```

**Message ID:**

```
test: message_id_from_timestamp
  unix_time = 1705320000 (2024-01-15 12:00:00 UTC)
  fraction  = 0          (exactly on the second)
  expected  = 1705320000 * 2^32 = 0x65A57B8000000000
  wire (LE) = 00 00 00 00 80 7B A5 65

test: message_id_lower_bits_zero_for_client
  client message_id & 0b11 == 0b00  ← must be true for client messages

test: message_id_lower_bits_three_for_server
  server message_id & 0b11 == 0b11  ← must be true for server messages
```

**TCP Abridged Framing:**

```
test: abridged_short_payload
  payload: 12 bytes (3 four-byte units)
  length_byte: 0x03  (3 * 4 = 12 bytes, no 0x7F marker needed)
  wire output: 03 [12 bytes payload]

test: abridged_long_payload
  payload: 512 bytes (128 four-byte units, >= 127 → need 0x7F marker)
  wire output: 7F 80 00 00 [512 bytes payload]
               ↑  ↑──────┘
               marker  128 as 3-byte LE

test: connection_init_byte
  First byte sent by client for abridged transport: EF
  Server must see this before any framed message.
```

### Integration Tests

**Full Auth Key Generation (against Telegram test DCs):**

```
test: auth_key_gen_roundtrip
  1. Connect to Telegram's test server (DC 2 test: 149.154.167.40:443)
  2. Run full 6-step DH exchange
  3. Verify: dh_gen_ok received, auth_key stored
  4. Send a ping message with the new auth_key
  5. Verify: pong received with correct ping_id

test: req_pq_nonce_echoed
  Send req_pq with nonce = AA BB CC DD EE FF 00 11 22 33 44 55 66 77 88 99
  Verify: res_pq.nonce == AA BB CC DD EE FF 00 11 22 33 44 55 66 77 88 99
```

**Message Encryption Roundtrip:**

```
test: encrypt_decrypt_roundtrip
  1. Generate auth_key (or use a fixed test key)
  2. Construct a plaintext message with known salt, session_id, etc.
  3. encrypt_message(auth_key, ...)
  4. decrypt_message(auth_key, result)
  5. Verify all fields match original plaintext

test: tampered_message_rejected
  1. encrypt_message(auth_key, plaintext)
  2. Flip one byte in the middle of encrypted_data
  3. decrypt_message must raise integrity error
     (because msg_key will not match recomputed value)
```

**RESP-like Integration (for testing without live servers):**

```
test: mock_server_ping_pong
  Spin up a mock server that:
    1. Accepts TCP connections
    2. Responds to req_pq with fake res_pq (no real DH, use test keys)
    3. Responds to set_client_DH_params with dh_gen_ok
    4. Decrypts and re-encrypts messages to simulate Telegram DC

  Client sends ping, mock server sends pong.
  Verify end-to-end encryption works.
```

**Secret Chat Key Establishment:**

```
test: secret_chat_dh_exchange
  Simulate Alice and Bob as two local objects sharing a mock server.
  1. Alice generates a, sends g_a
  2. Bob generates b, sends g_b
  3. Alice computes key_A = g_b^a mod dh_prime
  4. Bob computes key_B = g_a^b mod dh_prime
  5. Assert key_A == key_B

test: key_visualization_deterministic
  Same key bytes must always produce the same emoji grid.
  Test with a fixed 32-byte key and a reference emoji sequence.
```

### Property-Based Tests

```
test: ige_decrypt_inverse_of_encrypt (property test)
  For all (key: bytes[32], iv: bytes[32], pt: bytes[multiple of 16]):
    decrypt(encrypt(pt, key, iv), key, iv) == pt

test: tl_roundtrip (property test)
  For all valid TL values:
    deserialize(serialize(v)) == v

test: message_id_monotonic (property test)
  Successive calls to generate_message_id() must be strictly increasing.
  (Enforced by wall clock or a local counter if two IDs would be identical.)

test: padding_correctness (property test)
  For all message_data lengths:
    total plaintext length is always a multiple of 16
    padding length >= 12 (MTProto 2.0 requirement)
```

## Security Properties

### What MTProto 2.0 Provides

```
Property               MTProto 2.0 Mechanism
══════════════════════════════════════════════════════════════════════
Confidentiality        AES-256-IGE with per-message keys derived
                       from auth_key and msg_key. 256-bit key space.

Integrity              msg_key = SHA-256(auth_key_fragment + plaintext)
                       Server rejects messages where recomputed msg_key
                       differs from the transmitted msg_key.

Replay protection      salt (server-rotated), message_id (timestamp),
                       seq_no (monotonic). Three independent barriers.

Server authentication  Server's DH parameters signed by hardcoded RSA
                       public key. MitM cannot present false g_a without
                       the server's RSA private key.

Forward secrecy        Per-session (not per-message): auth_key stays
(cloud chats)          constant but session_id changes. Full PFS
                       requires secret chats.

End-to-end             Secret chats only. Server never holds the E2E key.
encryption

Forward secrecy        Secret chats: rekeying protocol after 100 messages
(secret chats)         or 7 days. Old keys are securely erased.
```

### Known Criticisms and Responses

**Criticism 1: Telegram invented its own crypto instead of using TLS.**

Response: The auth key bootstrap MUST use custom crypto — there is no TLS
certificate to bootstrap from. Once auth_key is established, all traffic is
AES-256 encrypted. The use of IGE rather than GCM is the main academic
critique; IGE lacks authentication natively, but MTProto adds integrity via
msg_key checks.

**Criticism 2: IGE is an unusual mode.**

Response: IGE was chosen because it is Encrypt-then-MAC when combined with
msg_key verification. A chosen-ciphertext attacker cannot surgically modify
one block — the entire message decrypts to garbage if any ciphertext bit is
changed. The msg_key check is the MAC, computed before decryption.

**Criticism 3: Cloud chats are not end-to-end encrypted.**

Response: True by design. Cloud chats allow multi-device sync, message backup,
and searchability. Users who want E2E can use secret chats. Telegram publishes
an explicit security analysis distinguishing the two models.

## Implementation Notes

### Language-Specific Considerations

**Big Integer Arithmetic:**
The DH key exchange requires exponentiation with 2048-bit numbers. Most
languages need a bignum library:
- Python: built-in `int` handles arbitrary precision
- Rust: `num-bigint` crate
- Go: `math/big` standard library
- Ruby: built-in `Integer` handles arbitrary precision
- Java/Kotlin: `java.math.BigInteger`

**Constant-Time Operations:**
All comparisons involving cryptographic material (msg_key comparison,
auth_key_id lookup) must be constant-time to prevent timing side-channels.
Use `hmac.compare_digest()` (Python), `subtle.ConstantTimeCompare()` (Go),
or equivalent in other languages.

**Secure Random Number Generation:**
All random bytes must come from a cryptographically secure source:
- `os.urandom()` in Python
- `rand::rngs::OsRng` in Rust
- `crypto/rand` in Go
- `SecureRandom` in Java

**AES Implementation:**
Do NOT implement AES from scratch. Use platform-provided implementations
that use hardware AES instructions (AES-NI on x86):
- Python: `cryptography` library (`cryptography.hazmat.primitives.ciphers`)
- Rust: `aes` crate (uses AES-NI automatically)
- Go: `crypto/aes` standard library

**Timing of RSA Operations:**
RSA decryption is expensive (~50ms). It only happens once per device bootstrap.
After auth_key is established, all messages use AES which is fast (~1 GB/s).

### Wire Format Summary

```
Complete wire-format reference
════════════════════════════════

Unencrypted (handshake only):
  [ 8 bytes auth_key_id = 0 ]
  [ 8 bytes message_id     ]
  [ 4 bytes message_length ]
  [ message_length bytes   ]

Encrypted (all post-auth messages):
  [ 8  bytes auth_key_id   ]
  [ 16 bytes msg_key       ]
  [ N  bytes encrypted     ]
    where encrypted = AES-IGE(
      [ 8 bytes salt         ]
      [ 8 bytes session_id   ]
      [ 8 bytes message_id   ]
      [ 4 bytes seq_no       ]
      [ 4 bytes data_length  ]
      [ data_length bytes    ]
      [ padding (≥12 bytes)  ]
    )

TCP Abridged framing around the above:
  [ EF ] (sent once as connection init byte)
  [ 1 or 4 bytes length prefix ]
  [ message bytes ]
```

### Error Handling

```
Common MTProto errors and their meaning:
═════════════════════════════════════════

bad_msg_notification (bad_msg_notification#a7eff811)
  bad_msg_id:    long   ← the message_id that was rejected
  error_code:    int    ← reason code

Error codes:
  16 = msg_id too low (server time in the future relative to client)
  17 = msg_id too high (client time in the future relative to server)
  18 = incorrect two lower order msg_id bits (not 0b00 for client)
  19 = container msg_id is the same as or less than the previous one
  20 = message too old (more than 300 seconds old)
  32 = seq_no too low (possible replay)
  33 = seq_no too high (message missed)
  48 = incorrect server salt — check future salts
  64 = invalid container

On error code 16 or 17: client reads server_time from the notification
and adjusts its local time offset. Then retransmit with corrected msg_id.
On error code 48: client has stale salt; use salt from bad_server_salt message.
```

## Appendix: TL Type Reference for Core MTProto

```
# Authorization layer (unencrypted layer)
req_pq#60469b60 nonce:int128 = ResPQ;
res_pq#5162463 nonce:int128 server_nonce:int128 pq:bytes server_public_key_fingerprints:Vector long = ResPQ;
p_q_inner_data#83c95aec pq:bytes p:bytes q:bytes nonce:int128 server_nonce:int128 new_nonce:int256 = P_Q_inner_data;
req_DH_params#d712e4be nonce:int128 server_nonce:int128 p:bytes q:bytes public_key_fingerprint:long encrypted_data:bytes = Server_DH_Params;
server_DH_params_ok#d0e8075c nonce:int128 server_nonce:int128 encrypted_answer:bytes = Server_DH_Params;
server_DH_inner_data#b5890dba nonce:int128 server_nonce:int128 g:int dh_prime:bytes g_a:bytes server_time:int = Server_DH_inner_data;
client_DH_inner_data#6643b654 nonce:int128 server_nonce:int128 retry_id:long g_b:bytes = Client_DH_Inner_Data;
set_client_DH_params#f5045f1f nonce:int128 server_nonce:int128 encrypted_data:bytes = Set_client_DH_params_answer;
dh_gen_ok#3bcbf734 nonce:int128 server_nonce:int128 new_nonce_hash1:int128 = Set_client_DH_params_answer;

# Session / service messages
ping#7abe77ec ping_id:long = Pong;
pong#347773c5 msg_id:long ping_id:long = Pong;
ping_delay_disconnect#f3427b8c ping_id:long disconnect_delay:int = Pong;
msgs_ack#62d6b459 msg_ids:Vector long = MsgsAck;
msg_container#73f1f8dc messages:vector message = MessageContainer;
future_salts#ae500895 req_msg_id:long now:int salts:vector<FutureSalt> = FutureSalts;
future_salt#0949d9dc valid_since:int valid_until:int salt:long = FutureSalt;
bad_msg_notification#a7eff811 bad_msg_id:long bad_msg_seqno:int error_code:int = BadMsgNotification;
bad_server_salt#edab447b bad_msg_id:long bad_msg_seqno:int error_code:int new_server_salt:long = BadMsgNotification;
```

---

This specification covers MTProto 2.0 in the depth needed to implement a
complete client from scratch. The auth key generation, message encryption, and
transport framing are the three independent subsystems — implement them in that
order and test each independently before integrating. The TL serializer is a
prerequisite for everything else. Start there.
