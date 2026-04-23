# MSG-Signal — The Signal Protocol

## Overview

Most encrypted messaging systems encrypt data **in transit** — the message is
scrambled between your phone and the server, but the server decrypts it,
stores it, and re-encrypts it when sending to the recipient. If the server is
compromised, or compelled by a government, all historical messages are
readable.

Signal's goal is different: the server should learn **as little as possible**,
including nothing about message content, and ideally nothing about who is
talking to whom.

To achieve this, Signal is built on three cryptographic building blocks:

1. **X3DH** (Extended Triple Diffie-Hellman) — establishes a shared secret
   key with someone who might be offline, using prekeys published to the server.
2. **Double Ratchet** — derives a fresh encryption key for every single message,
   so that compromising one message's key does not compromise past or future
   messages.
3. **Sealed Sender** — hides the sender's identity from Signal's own servers.

**Analogy:** Traditional encryption is like a lockbox — you put a message
inside, lock it, and the courier company (the server) delivers it. The courier
cannot read the message, but they hold a master key if needed. Signal is more
like each party minting a unique one-time padlock for every letter, with no
master key in existence — not even held by Signal.

**What makes the key distribution problem hard:** Cryptography is easy once
both parties share a secret key. The hard part is: how do Alice and Bob agree
on a secret key without ever having met, without the server learning the key,
and without requiring Bob to be online when Alice wants to start talking?

X3DH solves exactly this.

```
What Signal protects against
══════════════════════════════════════════════════════════════════════════

  Threat                         Protection
  ─────────────────────────────────────────────────────────────────────
  Server reads messages          End-to-end encryption (X3DH + DR)
  Server breach exposes history  Forward secrecy (DR symmetric ratchet)
  Server breach exposes future   Break-in recovery (DR DH ratchet)
  Key substitution MITM          Safety Numbers (manual verification)
  Server learns who talks to     Sealed Sender + private contact
   whom                           discovery
  Replay attacks                 One-time prekeys, message numbers
  Group message overhead         Sender Keys (one ciphertext per msg)
  Contact list exposure          SGX enclaves / Private Set Intersection
```

## Architecture

```
Signal Protocol Stack
══════════════════════════════════════════════════════════════════════════

  ┌────────────────────────────────────────────────────────────────────┐
  │  Application Layer                                                  │
  │  ┌─────────────────────┐  ┌──────────────────┐  ┌──────────────┐ │
  │  │  1:1 Messaging      │  │  Group Messaging  │  │  Calls       │ │
  │  │  (SignalMessage)    │  │  (SenderKey)      │  │  (SRTP)      │ │
  │  └─────────────────────┘  └──────────────────┘  └──────────────┘ │
  ├────────────────────────────────────────────────────────────────────┤
  │  Sealed Sender                                                      │
  │  Encrypts sender identity inside the ciphertext.                   │
  │  Server receives UnidentifiedDelivery — cannot see the sender.     │
  ├────────────────────────────────────────────────────────────────────┤
  │  Double Ratchet Algorithm                                           │
  │  Derives a fresh symmetric key for every message.                  │
  │  Provides forward secrecy AND break-in recovery.                   │
  ├────────────────────────────────────────────────────────────────────┤
  │  X3DH Key Agreement                                                 │
  │  One-time setup: establishes the root key for Double Ratchet.      │
  │  Allows session initiation when recipient is offline.              │
  ├────────────────────────────────────────────────────────────────────┤
  │  Cryptographic Primitives                                           │
  │  Curve25519 (DH), XSalsa20-Poly1305 (AEAD), HKDF-SHA256          │
  │  Ed25519 (signatures), AES-256-CBC (older clients, legacy)         │
  └────────────────────────────────────────────────────────────────────┘

  Signal Server (what it sees)
  ┌────────────────────────────────────────────────────────────────────┐
  │  - Stores encrypted prekey bundles per user (no secret keys)       │
  │  - Delivers encrypted envelopes (cannot decrypt content)           │
  │  - With Sealed Sender: cannot see sender's identity                │
  │  - Stores no message history (messages deleted after delivery)     │
  └────────────────────────────────────────────────────────────────────┘
```

## X3DH: Extended Triple Diffie-Hellman

### The Problem X3DH Solves

Diffie-Hellman key exchange (DH) lets two parties compute a shared secret
without transmitting it. But basic DH requires both parties to be online
simultaneously to exchange public keys.

X3DH solves the offline problem: Bob publishes a **prekey bundle** to
Signal's server. Alice fetches it and computes a shared secret. Bob can be
completely offline. When Bob comes online later, he can derive the same shared
secret from Alice's initial message.

**Analogy:** Imagine Bob installs a locked dropbox outside his house before
going on vacation. Alice can drop a sealed letter in the box. When Bob returns
and opens the box, the letter is waiting — and the letter establishes a secret
channel between them that nobody else (including the postal service) could read,
even if they watched every step.

### Key Types

Each user maintains three categories of keys, all on Curve25519:

```
Key Types in X3DH
══════════════════════════════════════════════════════════════════════════

  IK — Identity Key (permanent, one per device)
  ┌──────────────────────────────────────────────────────────────────┐
  │  A long-term Curve25519 keypair. Generated once when the app is  │
  │  installed. The public key is registered with Signal's server.   │
  │  This is the closest thing to a "user identity" in Signal.       │
  │  Changing it is equivalent to resetting your account.            │
  │  Doubles as an Ed25519 key for signing SPKs.                     │
  └──────────────────────────────────────────────────────────────────┘

  SPK — Signed Prekey (medium-term, rotated monthly)
  ┌──────────────────────────────────────────────────────────────────┐
  │  A Curve25519 keypair generated by the device. Its public key is │
  │  signed by the IK private key so Alice can verify it was         │
  │  published by Bob (not an impostor or the server).               │
  │  Rotated roughly every 30 days. Old SPKs are retained for a      │
  │  short time to allow decryption of in-flight messages.           │
  └──────────────────────────────────────────────────────────────────┘

  OPK — One-Time Prekey (single-use)
  ┌──────────────────────────────────────────────────────────────────┐
  │  A batch of Curve25519 keypairs (e.g., 100 at a time). Each OPK  │
  │  is used exactly once and then deleted. Its purpose is to prevent │
  │  replay attacks and improve forward secrecy of the initial        │
  │  handshake. Signal's server hands out one OPK per new session     │
  │  initiation and tracks which are used.                            │
  │  If the OPK list runs out, X3DH proceeds without OPK (DH4 is     │
  │  omitted). This weakens replay protection for that session.       │
  └──────────────────────────────────────────────────────────────────┘

  EK — Ephemeral Key (per-session, sender only)
  ┌──────────────────────────────────────────────────────────────────┐
  │  Generated fresh by Alice for each X3DH handshake. Never stored. │
  │  Its public key is sent to Bob in the initial message.            │
  └──────────────────────────────────────────────────────────────────┘
```

### The Prekey Bundle

Signal's server stores this for each registered user:

```
Prekey Bundle (what Alice fetches for Bob)
══════════════════════════════════════════════════════════════════════════

  ┌──────────────────┬────────────────────────────────────────────────┐
  │ Field            │ Description                                    │
  ├──────────────────┼────────────────────────────────────────────────┤
  │ IKb              │ Bob's identity public key (Curve25519).        │
  │                  │ 32 bytes.                                       │
  ├──────────────────┼────────────────────────────────────────────────┤
  │ SPKb             │ Bob's signed prekey public key. 32 bytes.      │
  ├──────────────────┼────────────────────────────────────────────────┤
  │ SPKb_sig         │ IKb's Ed25519 signature over SPKb. 64 bytes.  │
  │                  │ Alice verifies this before proceeding.         │
  ├──────────────────┼────────────────────────────────────────────────┤
  │ SPKb_id          │ Identifier for this SPK. Alice sends it back   │
  │                  │ in the initial message so Bob knows which SPK  │
  │                  │ to use for decryption.                         │
  ├──────────────────┼────────────────────────────────────────────────┤
  │ OPKb             │ One-time prekey public key. 32 bytes.          │
  │                  │ May be absent if supply is exhausted.          │
  ├──────────────────┼────────────────────────────────────────────────┤
  │ OPKb_id          │ Identifier for this OPK. Also sent back in the │
  │                  │ initial message.                                │
  └──────────────────┴────────────────────────────────────────────────┘
```

### The X3DH Calculation

Alice computes four Diffie-Hellman operations:

```
X3DH Key Agreement — The Four DH Operations
══════════════════════════════════════════════════════════════════════════

  Alice has:
    IKa (identity keypair)       — her long-term key
    EKa (ephemeral keypair)      — generated fresh for this session

  Alice fetches from server:
    IKb.pub                      — Bob's identity public key
    SPKb.pub                     — Bob's signed prekey (verified with sig)
    OPKb.pub                     — Bob's one-time prekey (may be absent)

  ┌──────────────────────────────────────────────────────────────────┐
  │  DH1 = DH(IKa.priv, SPKb.pub)                                   │
  │                                                                  │
  │  Why: Binds Alice's long-term identity to Bob's signed prekey.   │
  │  Bob will compute DH(SPKb.priv, IKa.pub) and get the same result.│
  ├──────────────────────────────────────────────────────────────────┤
  │  DH2 = DH(EKa.priv, IKb.pub)                                    │
  │                                                                  │
  │  Why: Binds Alice's ephemeral key to Bob's long-term identity.   │
  │  Provides forward secrecy: if IKa is later compromised, DH2's   │
  │  freshness (from EKa) protects past sessions.                    │
  ├──────────────────────────────────────────────────────────────────┤
  │  DH3 = DH(EKa.priv, SPKb.pub)                                   │
  │                                                                  │
  │  Why: Binds Alice's ephemeral key to Bob's signed prekey.        │
  │  Provides freshness from Alice's side and links to Bob's medium- │
  │  term key.                                                        │
  ├──────────────────────────────────────────────────────────────────┤
  │  DH4 = DH(EKa.priv, OPKb.pub)   [if OPK available]             │
  │                                                                  │
  │  Why: The one-time prekey makes each X3DH session unique, even   │
  │  if Alice uses the same EKa for two sessions (which she should   │
  │  not, but DH4 provides defense-in-depth). Prevents replay: if   │
  │  an attacker captures Alice's initial message and replays it,    │
  │  the server will not issue the same OPK again, so decryption     │
  │  fails.                                                           │
  └──────────────────────────────────────────────────────────────────┘

  master_secret = HKDF(
      ikm = 0x00..00 || DH1 || DH2 || DH3 || [DH4],
      salt = 0x00..00 (32 bytes),
      info = "WhisperText" (the original Signal protocol name),
      length = 32 bytes
  )

  The leading 0x00..00 padding (32 zero bytes matching Curve25519 output
  size) prevents the ikm from being all-zero if all DH values collide —
  a theoretical edge case.
```

**Formula summary:**

```
Alice computes:          Bob computes:
DH(IKa, SPKb)     ←→   DH(SPKb, IKa)
DH(EKa,  IKb)     ←→   DH( IKb, EKa)
DH(EKa, SPKb)     ←→   DH(SPKb, EKa)
DH(EKa, OPKb)     ←→   DH(OPKb, EKa)

All four pairs produce the same 32-byte value due to DH commutativity:
  DH(a, B) == DH(b, A) when B = aG and A = bG (elliptic curve group law)
```

### The Initial Message

When Alice initiates, she sends Bob:

```
Initial X3DH Message (what Alice sends to Signal's server, for Bob)
══════════════════════════════════════════════════════════════════════════

  ┌──────────────────┬────────────────────────────────────────────────┐
  │ Field            │ Description                                    │
  ├──────────────────┼────────────────────────────────────────────────┤
  │ IKa.pub          │ Alice's identity public key. Bob needs this    │
  │                  │ to compute DH2 and DH1 on his side.            │
  ├──────────────────┼────────────────────────────────────────────────┤
  │ EKa.pub          │ Alice's ephemeral public key. Bob needs this   │
  │                  │ to compute DH2, DH3, DH4.                      │
  ├──────────────────┼────────────────────────────────────────────────┤
  │ SPKb_id          │ Which of Bob's SPKs Alice used. Bob may have   │
  │                  │ recently rotated; this tells him which private  │
  │                  │ key to use.                                     │
  ├──────────────────┼────────────────────────────────────────────────┤
  │ OPKb_id          │ Which one-time prekey Alice used (if any). Bob │
  │                  │ needs the matching private key for DH4.        │
  ├──────────────────┼────────────────────────────────────────────────┤
  │ ciphertext       │ Alice's first message, encrypted with a key    │
  │                  │ derived from master_secret. Bob can decrypt     │
  │                  │ as soon as he reconstructs master_secret.      │
  └──────────────────┴────────────────────────────────────────────────┘
```

The ciphertext uses a key derived from master_secret via HKDF:

```
(root_key, chain_key) = HKDF(master_secret, info="WhisperRatchet")

first_msg_key = HKDF(chain_key, info="WhisperMessageKeys")
ciphertext = AES256-CBC-HMAC-SHA256(first_msg_key, plaintext)
```

This "initial message key" is used exactly once. After Bob receives the
message and the Double Ratchet begins, the root_key and chain_key seed the
ratchet.

### Signed Prekey Rotation Policy

- SPK rotation: every 30 days (or whenever the app is launched after 30 days).
- On rotation: generate new SPKb, sign with IKb, upload to server.
- Retain old SPKb private keys for approximately 30 days after rotation
  to decrypt messages encrypted with the old SPK (in-flight during rotation).
- OPK replenishment: the client uploads 100 OPKs at registration. When the
  server's supply drops below ~10, Signal requests more. The client generates
  and uploads another batch.

## Double Ratchet Algorithm

### Why Two Ratchets?

A single symmetric-key ratchet provides forward secrecy: each message key is
derived from the previous one via a one-way hash function. If an attacker
captures message key 5, they cannot derive message keys 1-4 (forward secrecy)
because the function is one-way. But they CAN derive message keys 6, 7, 8...
(no break-in recovery).

The DH ratchet adds break-in recovery: each time a new DH public key arrives
from the other party, both sides perform a new DH operation and inject fresh
randomness into the chain. An attacker who steals the current symmetric key
will be locked out again after the next DH ratchet step.

**Analogy:** The symmetric ratchet is like a combination lock where each new
combination is derived from the previous one using a one-way hash. The DH
ratchet is like periodically replacing the entire lock with a new one agreed
upon by secret handshake. Even if a burglar learned the current combination,
they will be locked out after the handshake.

```
The Two Ratchets
══════════════════════════════════════════════════════════════════════════

  ┌────────────────────────────────────────────────────────────────────┐
  │  DH Ratchet (outer loop)                                           │
  │                                                                    │
  │  Triggered by: receiving a new DH public key from the other party. │
  │  Effect: completely replaces the root key using fresh DH material. │
  │  Provides: break-in recovery ("future secrecy").                   │
  │                                                                    │
  │  ┌─────────────┐    new DH pub    ┌──────────────┐                │
  │  │  Root Key   │ ────────────────►│  New Root Key│                │
  │  │  (32 bytes) │  KDF(RK, DH_out) │  + Chain Key │                │
  │  └─────────────┘                  └──────────────┘                │
  │                                                                    │
  ├────────────────────────────────────────────────────────────────────┤
  │  Symmetric-Key Ratchet (inner loop, runs per-message)              │
  │                                                                    │
  │  Triggered by: every message sent or received.                     │
  │  Effect: derives a per-message key, advances the chain key.        │
  │  Provides: forward secrecy (old keys are deleted).                 │
  │                                                                    │
  │  Chain Key[n] ──KDF──► Chain Key[n+1]                             │
  │       │                                                            │
  │       └──────────────► Message Key[n]   (used to encrypt msg n)   │
  │                                                                    │
  │  Message keys are DELETED immediately after use.                   │
  └────────────────────────────────────────────────────────────────────┘
```

### State Machine Fields

The Double Ratchet state (stored per-session, per-device):

```
Double Ratchet State
══════════════════════════════════════════════════════════════════════════

  ┌──────────────────────┬──────────────────────────────────────────────┐
  │ Field                │ Description                                  │
  ├──────────────────────┼──────────────────────────────────────────────┤
  │ RK                   │ Root Key (32 bytes). Seeded from X3DH        │
  │                      │ master_secret. Updated on each DH ratchet    │
  │                      │ step.                                         │
  ├──────────────────────┼──────────────────────────────────────────────┤
  │ CKs                  │ Sending Chain Key (32 bytes). Derives        │
  │                      │ sending message keys one at a time.          │
  ├──────────────────────┼──────────────────────────────────────────────┤
  │ CKr                  │ Receiving Chain Key (32 bytes). Derives      │
  │                      │ receiving message keys.                       │
  ├──────────────────────┼──────────────────────────────────────────────┤
  │ DHs                  │ Sending Ratchet Keypair (Curve25519).        │
  │                      │ The public key is included in every message  │
  │                      │ header so the receiver can ratchet forward.  │
  ├──────────────────────┼──────────────────────────────────────────────┤
  │ DHr                  │ Receiving Ratchet Public Key. The last DH    │
  │                      │ public key received from the other party.    │
  ├──────────────────────┼──────────────────────────────────────────────┤
  │ Ns                   │ Sending message number (starts at 0,         │
  │                      │ increments per message). Included in the     │
  │                      │ message header.                               │
  ├──────────────────────┼──────────────────────────────────────────────┤
  │ Nr                   │ Receiving message number. Expected next       │
  │                      │ message number from the other party.         │
  ├──────────────────────┼──────────────────────────────────────────────┤
  │ PN                   │ Previous Sending Chain Length. The number of │
  │                      │ messages sent in the previous sending chain. │
  │                      │ Included in headers so the receiver can      │
  │                      │ compute skipped keys.                         │
  ├──────────────────────┼──────────────────────────────────────────────┤
  │ MKSKIPPED            │ Map of (DHr_pub, msg_number) → message_key. │
  │                      │ Stores keys for out-of-order messages.       │
  │                      │ Bounded (e.g., max 2000 entries) to prevent  │
  │                      │ memory exhaustion attacks.                    │
  └──────────────────────┴──────────────────────────────────────────────┘
```

### KDF Chains

The KDF chains use HKDF-SHA256 in a specific way:

```
Chain Key KDF (symmetric ratchet step)
═════════════════════════════════════════

  Given current chain key CK:

  message_key   = HMAC-SHA256(CK, 0x01)   ← constant byte 0x01
  next_chain_key = HMAC-SHA256(CK, 0x02)  ← constant byte 0x02

  The message_key is 32 bytes used for AEAD encryption.
  The next_chain_key replaces CK for the next message.
  The message_key is deleted after use.

Root Key KDF (DH ratchet step)
═════════════════════════════════

  Given current root key RK and DH output dh_out:

  (new_RK, new_CK) = HKDF(
      ikm   = dh_out,
      salt  = RK,
      info  = "WhisperRatchet",
      length = 64   ← 32 for new RK, 32 for new CK
  )

  The old RK is replaced by new_RK.
  The new_CK becomes either CKs or CKr depending on direction.
```

### Message Encryption (AEAD)

Each message key expands into three subkeys for authenticated encryption:

```
Message Key Expansion
═════════════════════

  Given message_key (32 bytes from the chain KDF):

  (aes_key, hmac_key, iv) = HKDF(
      ikm   = message_key,
      salt  = 0x00..00 (32 bytes),
      info  = "WhisperMessageKeys",
      length = 80   ← 32 aes + 32 hmac + 16 iv
  )

  Encryption:
    ciphertext = AES-256-CBC(key=aes_key, iv=iv, plaintext=padded_plaintext)
    mac        = HMAC-SHA256(key=hmac_key, data=header || ciphertext)[0:8]

  The 8-byte MAC truncation is intentional — it is sufficient to detect
  tampering while minimizing ciphertext overhead.

  Note: Signal's modern clients use AES-256-GCM instead of CBC+HMAC,
  but CBC+HMAC is described here for pedagogical completeness as it
  matches the published protocol specification (Signal Protocol docs).
```

### Message Header

Every Double Ratchet message carries a header (in addition to the ciphertext):

```
Message Header Structure
════════════════════════════════════════════════════════════════════════

  ┌──────────────────┬────────────────────────────────────────────────┐
  │ Field            │ Description                                    │
  ├──────────────────┼────────────────────────────────────────────────┤
  │ DHs.pub          │ Sender's current ratchet public key (32 bytes).│
  │                  │ The receiver uses this to advance their        │
  │                  │ receiving ratchet when a new key appears.      │
  ├──────────────────┼────────────────────────────────────────────────┤
  │ PN               │ Previous chain length: how many messages were  │
  │                  │ sent in the previous sending chain before this │
  │                  │ DH ratchet step. Allows the receiver to cache  │
  │                  │ skipped message keys from the previous chain.  │
  ├──────────────────┼────────────────────────────────────────────────┤
  │ Ns               │ Message number within the current sending      │
  │                  │ chain (starts at 0). Together with DHs.pub,    │
  │                  │ uniquely identifies the message key.           │
  └──────────────────┴────────────────────────────────────────────────┘

Header encryption (optional, used by Signal):
  The header itself is encrypted using a separate header key derived
  during the DH ratchet step:

  header_key = HKDF(dh_out, RK, info="WhisperHeaderKeys", length=32)

  This hides DHs.pub, PN, and Ns from a passive observer, preventing
  them from using ratchet key changes to infer the conversation's
  reply pattern or detect message reordering.
```

### Out-of-Order Messages (Skipped Keys)

Messages may arrive out of order due to network reordering. When Alice sends
messages 1, 2, 3 and only message 3 arrives first, the receiver must cache
the message keys for 1 and 2 without having received them yet.

```
Skipped Message Key Handling
════════════════════════════════════════════════════════════════════════

  When receiving message (DHr_new, PN, Ns, ciphertext):

  If DHr_new != current DHr:
    # New DH ratchet key — need to advance the DH ratchet
    # But first, cache all skipped message keys in the CURRENT chain:
    for n in range(Nr, PN):
        MKSKIPPED[(DHr, n)] = derive_message_key(CKr, n)
        advance CKr

    # Now perform the DH ratchet step:
    (RK, CKr) = KDF_RK(RK, DH(DHs.priv, DHr_new))
    DHr = DHr_new
    Nr = 0

  # Cache any skipped message keys in the NEW chain:
  for n in range(Nr, Ns):
      MKSKIPPED[(DHr, n)] = derive_message_key(CKr, n)
      advance CKr
      Nr += 1

  # Decrypt with the current message key:
  mk = derive_message_key(CKr, Ns)
  advance CKr
  Nr += 1
  plaintext = decrypt(mk, ciphertext)

Security note on MKSKIPPED:
  - Max size: Signal limits to 2000 skipped keys. If a sender sends
    more than 2000 messages without the receiver acknowledging any,
    decryption fails. This prevents memory exhaustion.
  - Keys in MKSKIPPED are "stale" — they cannot be re-derived if lost.
    Implementation must persist MKSKIPPED to disk.
  - Keys in MKSKIPPED are NOT protected by forward secrecy. Once cached,
    they protect confidentiality of past messages only until they are used
    or expire.
```

### Step-by-Step Trace: Alice and Bob Exchange 5 Messages

```
Initial state (after X3DH):
  RK = 0xABCD...  (from master_secret)
  CKs = 0x1111...
  CKr = undefined (Alice hasn't received anything yet)
  DHs = Alice's first ratchet keypair (A1)
  DHr = Bob's ratchet public key (B0) — received in X3DH

══════════════════════════════════════════════════════════════════════════

Step 1: Alice sends message "Hello"
  Symmetric ratchet:
    mk1 = HMAC(CKs, 0x01)   → message key for msg 1
    CKs = HMAC(CKs, 0x02)   → advance sending chain

  Header: {DHs=A1.pub, PN=0, Ns=0}
  Ciphertext: encrypt(mk1, "Hello")
  Alice sends: [header | ciphertext]

  Alice state: Ns=1, CKs=CKs_new

──────────────────────────────────────────────────────────────────────────

Step 2: Alice sends message "How are you?"
  (Same chain, no new DH key from Bob yet)
    mk2 = HMAC(CKs, 0x01)
    CKs = HMAC(CKs, 0x02)

  Header: {DHs=A1.pub, PN=0, Ns=1}  ← same DHs, Ns incremented
  Ciphertext: encrypt(mk2, "How are you?")

  Alice state: Ns=2

──────────────────────────────────────────────────────────────────────────

Step 3: Bob receives msg 1 ("Hello") — first message ever from Alice
  Bob sees DHs=A1.pub (new — different from B0 baseline)
  Bob does DH ratchet:
    dh_out = DH(Bob's_ratchet_priv_B0, A1.pub)
    (RK, CKr) = KDF_RK(RK, dh_out)
    DHr = A1.pub
    Nr = 0

  Symmetric ratchet (receive):
    mk1 = HMAC(CKr, 0x01)
    CKr = HMAC(CKr, 0x02)
    Nr = 1

  Decrypt: decrypt(mk1, ciphertext) = "Hello"

──────────────────────────────────────────────────────────────────────────

Step 4: Bob replies "I'm good!"
  Bob generates new ratchet keypair B1 (DH ratchet — Bob's turn)
  Bob does DH ratchet (sending direction):
    dh_out = DH(B1.priv, A1.pub)
    (RK, CKs) = KDF_RK(RK, dh_out)
    DHs = B1

  Symmetric ratchet (send):
    mk_b1 = HMAC(CKs, 0x01)
    CKs = HMAC(CKs, 0x02)
    Ns = 0

  Header: {DHs=B1.pub, PN=0, Ns=0}  ← new DHs key!
  Ciphertext: encrypt(mk_b1, "I'm good!")

──────────────────────────────────────────────────────────────────────────

Step 5: Alice receives Bob's reply "I'm good!"
  Alice sees DHs=B1.pub (new key from Bob)
  Alice does DH ratchet:
    dh_out = DH(A1.priv, B1.pub)
    (RK, CKr) = KDF_RK(RK, dh_out)
    DHr = B1.pub
    Nr = 0

  (Note: PN=0 in Bob's header, so no skipped keys to cache)

  Symmetric ratchet:
    mk_b1 = HMAC(CKr, 0x01)
    CKr = HMAC(CKr, 0x02)
    Nr = 1

  Decrypt: "I'm good!"

  Alice generates A2 for her next message:
  (On next send, Alice will include A2.pub in the header,
   triggering another DH ratchet step for Bob)

Key evolution summary after 5 steps:
  Root key:  RK₀ → RK₁ (after Bob's first reply) → RK₂ (after Alice's next)
  Message keys used: mk1, mk2, mk_b1
  All used message keys: deleted immediately after decryption
  Forward secrecy: mk1, mk2 cannot be rederived even if mk_b1 is compromised
```

## Sealed Sender

### The Problem

Even with end-to-end encryption, Signal's server could observe metadata:
"At 14:32, phone number +14085551234 sent a message to phone number
+14085559876." This metadata, accumulated over time, reveals social graphs,
relationships, and behavioral patterns — even without knowing message content.

### The Solution

Sealed Sender encrypts the sender's identity into the ciphertext using the
**recipient's** certificate. The server receives a blob it can route (because
the destination is encrypted to the server in an outer envelope) but cannot
identify the sender.

```
Sealed Sender Architecture
══════════════════════════════════════════════════════════════════════════

  Traditional Signal delivery:
  ┌──────────────────────────────────────────────────────────────────┐
  │  Server receives:  FROM: Alice, TO: Bob, CONTENT: <encrypted>   │
  │  Server knows: Alice sent something to Bob at time T.           │
  └──────────────────────────────────────────────────────────────────┘

  Sealed Sender delivery:
  ┌──────────────────────────────────────────────────────────────────┐
  │  Server receives:  TO: Bob, CONTENT: <outer encrypted blob>     │
  │  Server knows: Someone sent something to Bob at time T.         │
  │  Server does NOT know: Who the sender is.                       │
  └──────────────────────────────────────────────────────────────────┘

Sealed Sender Encryption Steps:
  1. Alice's SignalMessage (ciphertext + DR header) is wrapped in a
     SenderCertificate:
     {
       sender_uuid: alice's UUID,
       sender_device_id: 1,
       sender_public_key: Alice's identity public key,
       expiration: unix timestamp,
       certificate_signature: Signal-server-signed blob
     }

  2. Alice encrypts [SenderCertificate || SignalMessage] using an
     ephemeral X25519 key agreement with Bob's identity public key:
     outer_key = DH(ephemeral_priv, Bob.IK.pub)
     sealed_blob = XSalsa20-Poly1305(outer_key, sender_cert || msg)

  3. Alice sends to Signal server:
     { destination: Bob's registration ID,
       content: sealed_blob }
     (No "from" field in the HTTP request.)

Bob's decryption:
  1. Bob decrypts the outer layer using his IK private key.
  2. Bob extracts the SenderCertificate: verifies the server's signature.
  3. Bob now knows Alice is the sender (from the SenderCertificate).
  4. Bob processes the SignalMessage normally with the Double Ratchet.
```

### The Server-Side Certificate

To use Sealed Sender, each registered user receives a **SenderCertificate**
from Signal's server — a server-signed blob containing the user's identity
public key. This certificate is time-limited (e.g., 24 hours):

```
SenderCertificate Structure
══════════════════════════════════════════════════════════════════════════

  ┌──────────────────┬────────────────────────────────────────────────┐
  │ Field            │ Description                                    │
  ├──────────────────┼────────────────────────────────────────────────┤
  │ sender_uuid      │ Sender's UUID (not phone number).              │
  ├──────────────────┼────────────────────────────────────────────────┤
  │ sender_e164      │ Optional: sender's phone number (E.164 format).│
  │                  │ Can be omitted for extra privacy.              │
  ├──────────────────┼────────────────────────────────────────────────┤
  │ sender_device_id │ Which of the sender's devices sent this.       │
  ├──────────────────┼────────────────────────────────────────────────┤
  │ identity_key     │ Sender's Curve25519 identity public key.       │
  ├──────────────────┼────────────────────────────────────────────────┤
  │ expiration       │ Unix timestamp. Bob checks this before         │
  │                  │ trusting the certificate.                       │
  ├──────────────────┼────────────────────────────────────────────────┤
  │ signature        │ Signal server's Ed25519 signature over the     │
  │                  │ above fields. Bob verifies this against the     │
  │                  │ server's known certificate.                     │
  └──────────────────┴────────────────────────────────────────────────┘
```

### Spam and Rate Limiting: Zero-Knowledge Credentials

Without a sender, the server cannot attribute spam or abuse. Signal uses
**anonymous credentials** (based on Ristretto255 group signatures) to enforce
rate limits without learning the sender's identity:

```
ZKSK Anonymous Credentials Flow
══════════════════════════════════════════════════════════════════════════

  Registration time:
    Signal server issues Alice a credential (privacy-preserving token)
    proving she is a valid registered user. Issuing does not reveal
    which future message the credential will be used for.

  Send time:
    Alice presents a zero-knowledge proof that she holds a valid
    credential (without revealing which credential, i.e., without
    revealing her identity). The server verifies the proof and
    accepts the message without learning the sender.

  If Alice attempts to send 1000 messages/minute:
    The rate-limit token is spent; Alice cannot prove she has more
    tokens without fetching more from the server (which reveals her
    identity at the replenishment step, not the send step).
```

## Sealed Groups: Group Messaging

### Why Not Double Ratchet for Groups?

If a group has N members and Alice wants to send a message, she would need to
maintain N Double Ratchet sessions and encrypt the message N times (once per
member). For a group of 50 people sending 1000 messages per day, that is
50,000 AEAD operations per member per day. Impractical.

**Solution: Sender Keys.** Each member of the group distributes a single
SenderKey to every other member. When Alice sends a message, she encrypts it
once using her SenderKey. Every recipient holds Alice's SenderKey and can
decrypt that single ciphertext.

```
Sender Key vs Double Ratchet for Group N=50
══════════════════════════════════════════════════════════════════════════

  Double Ratchet (naive):
    Alice sends message → 49 separate ciphertexts (one per member)
    Cost: O(N) ciphertexts per message, O(N²) sessions total

  Sender Key:
    Alice sends message → 1 ciphertext (one for all)
    Cost: O(1) ciphertexts per message, O(N) sessions (one per member's key)

  Tradeoff:
    Double Ratchet: full forward + future secrecy per-message
    Sender Key: forward secrecy (the key chain advances one-way)
                but NO future secrecy between ratchet steps
                (if Carol's device is compromised, attacker can
                 decrypt all future Alice messages until Alice
                 performs a new key distribution)
```

### Sender Key State

```
SenderKey State (Alice's, stored per group per device)
══════════════════════════════════════════════════════════════════════════

  ┌──────────────────────┬──────────────────────────────────────────────┐
  │ Field                │ Description                                  │
  ├──────────────────────┼──────────────────────────────────────────────┤
  │ sender_chain_key     │ 32-byte chain key. Advances with each        │
  │                      │ message sent. Provides forward secrecy.      │
  ├──────────────────────┼──────────────────────────────────────────────┤
  │ signature_key        │ Ed25519 keypair. Every SenderKeyMessage is   │
  │                      │ signed so recipients can verify authorship   │
  │                      │ even if the distribution message was forged. │
  ├──────────────────────┼──────────────────────────────────────────────┤
  │ iteration            │ Message counter (starts at 0). Included in  │
  │                      │ the message for forward secrecy: recipients  │
  │                      │ can fast-forward their cached key to         │
  │                      │ iteration N without receiving intervening    │
  │                      │ messages.                                     │
  └──────────────────────┴──────────────────────────────────────────────┘

  When Alice sends group message iteration n:
    message_key = HMAC(sender_chain_key, 0x01)
    sender_chain_key = HMAC(sender_chain_key, 0x02)
    iteration += 1
    ciphertext = encrypt(message_key, plaintext)
    sig = Ed25519_sign(signature_key, ciphertext)
    send SenderKeyMessage { iteration, ciphertext, sig }
```

### SenderKeyDistributionMessage

When Alice joins a group or wants to establish a new key with the group:

```
SenderKeyDistributionMessage
══════════════════════════════════════════════════════════════════════════

  Alice sends this to each group member individually, over their
  established 1:1 Double Ratchet session:

  {
    distribution_id: UUID,          ← identifies this key distribution epoch
    chain_id: 0,                    ← starting iteration (usually 0)
    iteration: 0,
    chain_key: alice_chain_key,     ← the starting chain key (32 bytes)
    signature_key: alice_sig_key.pub ← Ed25519 public key for verification
  }

  Each recipient stores (alice_uuid, distribution_id) → SenderKeyState.
  When Alice sends a SenderKeyMessage, Bob looks up Alice's state and:
    1. Verifies the Ed25519 signature using the stored signature_key.
    2. Fast-forwards the chain to the message's iteration if needed.
    3. Decrypts the ciphertext.
```

### Group Membership Changes

When a member leaves (or is removed) from a group, all remaining members
must perform a fresh SenderKeyDistributionMessage exchange. The departed
member held everyone's current chain keys and can decrypt future messages
encrypted with those keys.

This is called **group key rotation**. Signal does it automatically when
membership changes. The UX consequence: the "Sender key reset" or "This
person joined/left — security number changed" notifications in Signal
group chats.

## Private Contact Discovery

### The Problem

To show which of your phone contacts use Signal, the app must check your
contact list against Signal's user database. But uploading your entire contact
list to Signal's servers is a massive privacy leak — Signal would learn every
person in your life.

### Solution: SGX Enclaves

Signal's original solution used Intel SGX (Software Guard Extensions):
processor-level isolated execution environments where code and data are
protected even from the operating system and the hardware owner.

```
SGX Private Contact Discovery
══════════════════════════════════════════════════════════════════════════

  Client (your phone)           SGX Enclave (Signal's server)
        │                              │
        │  Establish remote            │
        │  attestation (verify         │
        │  enclave is genuine Intel    │
        │  SGX, running Signal's code) │
        │◄─────────────────────────── │
        │                              │
        │  Send encrypted contact list │
        │  (encrypted to enclave's key)│
        │ ────────────────────────────►│
        │                              │ Enclave decrypts contact list
        │                              │ Checks against phone number DB
        │                              │ Result: set intersection
        │                              │ Enclave encrypts result
        │                              │ (Never visible to Signal admins)
        │◄─────────────────────────── │
        │  Encrypted intersection      │
        │  (only your contacts who     │
        │   use Signal)                │

Why SGX isn't perfect:
  - Side-channel attacks: memory access patterns observable via cache timing.
  - Firmware vulnerabilities: SGX has had multiple high-severity CVEs
    (Foreshadow/L1TF, SGAxe, etc.).
  - Hardware supply chain trust: SGX roots trust in Intel as a manufacturer.

Signal's move: migrating to Oblivious RAM (ORAM) and Private Information
Retrieval (PIR) schemes that provide contact discovery without trusting
hardware enclaves. These are based on cryptographic assumptions rather than
hardware isolation guarantees.
```

## Safety Numbers

Safety Numbers are Signal's mechanism for catching man-in-the-middle attacks.
If an attacker substitutes their own public key for Alice's when Bob registers,
Bob would encrypt messages to the attacker's key thinking it's Alice's.

Safety Numbers give Alice and Bob a human-verifiable fingerprint of their
shared key agreement. If the numbers match when compared in-person or via
a trusted channel, no MITM occurred.

```
Safety Number Computation
══════════════════════════════════════════════════════════════════════════

  Given:
    IKa.pub = Alice's 32-byte identity public key
    IKb.pub = Bob's 32-byte identity public key

  fingerprint_a = SHA-512(iteration_count=5200 || IKa.pub || alice_number)
  fingerprint_b = SHA-512(iteration_count=5200 || IKb.pub || bob_number)

  Where iteration_count is a 32-bit big-endian integer, applied in a
  5200-round chain:
    hash = IKa.pub
    for i in range(5200):
        hash = SHA-512(hash || IKa.pub)
    fingerprint_a = hash

  combined = sort_and_concat(fingerprint_a, fingerprint_b)

  Display: convert combined to 60 decimal digits, grouped as 12 × 5:
    e.g., "05156 87657 23423 98765 12345 67890
           09876 54321 11111 22222 33333 44444"

  QR code: encodes the same data for camera scanning instead of
  reading digits aloud.
```

**Why 5200 rounds?** To make brute-force collision attacks expensive. An
attacker trying to find two key pairs with matching safety numbers must
compute 5200 SHA-512 operations per candidate pair. With modern GPUs, this
still takes days per attempt — far too slow for a practical MITM.

**Why sort IKa and IKb?** The Safety Number must be the same whether Alice
views it from her perspective or Bob views it from his. Sorting ensures the
combined fingerprint is independent of who computes it.

## Wire Format

Signal uses Protocol Buffers (protobuf) for all message serialization.
Protobuf provides compact binary encoding, schema evolution (adding new
fields without breaking old clients), and efficient parsing.

### Envelope

Every message delivered by Signal's server is wrapped in an `Envelope`:

```
Signal Envelope (Protobuf schema)
══════════════════════════════════════════════════════════════════════════

  message Envelope {
    enum Type {
      UNKNOWN              = 0;
      CIPHERTEXT           = 1;  // Double Ratchet SignalMessage
      KEY_EXCHANGE         = 2;  // Legacy: X3DH without prekeys
      PREKEY_BUNDLE        = 3;  // PreKeySignalMessage (X3DH initiation)
      RECEIPT              = 5;  // Delivery / read receipt
      UNIDENTIFIED_SENDER  = 6;  // Sealed Sender (hides sender identity)
      PLAINTEXT_CONTENT    = 8;  // Unencrypted (internal / sync)
    }

    optional Type   type            = 1;  // envelope type
    optional string source_uuid     = 11; // sender UUID (absent for sealed sender)
    optional uint32 source_device   = 7;  // sender's device ID
    optional string destination_uuid= 13; // recipient UUID
    optional uint64 timestamp       = 5;  // milliseconds since epoch
    optional bytes  content         = 8;  // encrypted content
    optional bytes  server_guid     = 9;  // server-assigned message ID
    optional uint64 server_timestamp= 10; // when server received this
  }
```

### SignalMessage (Double Ratchet ciphertext)

```
  message SignalMessage {
    optional bytes  ratchet_key       = 1;  // DHs.pub: sender's ratchet key
    optional uint32 counter            = 2;  // Ns: message number in chain
    optional uint32 previous_counter   = 3;  // PN: previous chain length
    optional bytes  ciphertext         = 4;  // AEAD ciphertext
  }

  Wire overhead: ~74 bytes (32 byte ratchet key + counters + framing)
  plus 8-byte MAC appended to ciphertext.
```

### PreKeySignalMessage (X3DH initiation)

```
  message PreKeySignalMessage {
    optional uint32 registration_id    = 5;  // sender's registration ID
    optional uint32 pre_key_id         = 1;  // which OPK was used (if any)
    optional uint32 signed_pre_key_id  = 6;  // which SPK was used
    optional bytes  base_key           = 2;  // EKa.pub: sender's ephemeral key
    optional bytes  identity_key       = 3;  // IKa.pub: sender's identity key
    optional bytes  message            = 4;  // SignalMessage (encrypted)
  }

  This is the initial message Alice sends to Bob when establishing a new
  session. It contains everything Bob needs to perform his side of X3DH:
    - EKa.pub → computes DH2, DH3, DH4
    - IKa.pub → computes DH1
    - signed_pre_key_id → identifies which SPKb.priv to use
    - pre_key_id → identifies which OPKb.priv to use (if any)
```

### Content (Decrypted Payload)

After decryption, the content is another protobuf:

```
  message Content {
    optional DataMessage        data_message          = 1;  // text, media
    optional SyncMessage        sync_message          = 2;  // multi-device sync
    optional CallMessage        call_message          = 3;  // voice/video calls
    optional NullMessage        null_message          = 4;  // sealed sender ping
    optional ReceiptMessage     receipt_message       = 5;  // read/delivery ack
    optional TypingMessage      typing_message        = 6;  // is-typing indicator
    optional StoryMessage       story_message         = 7;  // Stories (Instagram-like)
    optional DecryptionErrorMessage
                                decryption_error_message = 8; // undecryptable msg
  }

  message DataMessage {
    optional string   body              = 1;   // message text
    repeated AttachmentPointer
                      attachments       = 2;   // images, videos, files
    optional QuoteMessage
                      quote             = 6;   // replied-to message
    optional uint64   timestamp         = 5;   // client send time (ms epoch)
    optional uint32   expiration_start_time = 10;  // disappearing messages
    optional uint32   expire_timer      = 12;  // TTL in seconds
    repeated Reaction  reactions        = 23;  // emoji reactions
  }
```

### Worked Example: Alice Sends "Hello" to Bob

```
Worked Wire-Level Example
══════════════════════════════════════════════════════════════════════════

Preconditions:
  - Alice and Bob have completed X3DH.
  - master_secret was derived. Double Ratchet state initialized:
      RK  = 0x7a3f... (32 bytes from X3DH HKDF)
      CKs = 0x1b8c... (32 bytes, Alice's sending chain)
      DHs = (A1.priv, A1.pub) where A1.pub = 0x5e2a... (32 bytes)
      DHr = B0.pub = 0x8d4f... (Bob's ratchet pub key from X3DH)
      Ns  = 0

Step 1: Alice's app creates a DataMessage
  plaintext protobuf:
    DataMessage {
      body: "Hello",
      timestamp: 1745316600000
    }
  serialized: 0x0a05 48656c6c6f 10e0d5e3ca9932
  (7 bytes body field + 9 bytes timestamp field = ~18 bytes)

Step 2: Double Ratchet — derive message key
  message_key = HMAC-SHA256(CKs=0x1b8c..., data=0x01)
    → 0x4a7d... (32 bytes)
  next_CKs    = HMAC-SHA256(CKs=0x1b8c..., data=0x02)
    → 0x9c3e... (new sending chain key)

Step 3: Expand message key
  (aes_key, hmac_key, iv) = HKDF(0x4a7d..., salt=0x0000..., info="WhisperMessageKeys", len=80)
    aes_key  = 0xf38b... (32 bytes)
    hmac_key = 0xd12c... (32 bytes)
    iv       = 0xa5e9... (16 bytes)

Step 4: Encrypt
  padded_plaintext = PKCS7_pad(plaintext, block_size=16)
    = 0x0a05 48656c6c6f 10e0d5e3ca9932 0909090909090909  (+ 9 bytes padding)
  ciphertext = AES-256-CBC(aes_key, iv, padded_plaintext)
    = 0x3c88... (32 bytes)
  mac = HMAC-SHA256(hmac_key, version_byte || iv || ciphertext)[0:8]
    = 0xa1b2c3d4e5f60708  (8 bytes)

Step 5: Build SignalMessage protobuf
  SignalMessage {
    ratchet_key:      0x5e2a...  (A1.pub, 32 bytes)
    counter:          0          (Ns)
    previous_counter: 0          (PN)
    ciphertext:       0x3c88... || 0xa1b2c3d4e5f60708  (ciphertext + MAC)
  }
  serialized: ~74 bytes total

Step 6: Prepend version byte
  Wire: 0x33 || SignalMessage_bytes
  (0x33 = version 3 current, version 3 fallback — Signal's versioning byte)

Step 7: (If Sealed Sender) Wrap in SenderCertificate envelope
  outer = XSalsa20-Poly1305(
      key = DH(ephemeral_key, Bob.IK.pub),
      plaintext = SenderCertificate_bytes || SignalMessage_wire_bytes
  )
  Send to server: Envelope { type=UNIDENTIFIED_SENDER, content=outer }

Step 8: Bob receives and decrypts
  1. Unwrap Sealed Sender outer layer using Bob.IK.priv.
  2. Verify SenderCertificate server signature.
  3. Extract SignalMessage.
  4. See DHs=A1.pub (new key) → perform DH ratchet.
  5. Derive message key 0.
  6. Decrypt → "Hello".
```

## Algorithms

### X3DH Send (Alice initiates)

```
procedure x3dh_send(alice_ik, prekey_bundle):
    # Step 1: Verify Bob's signed prekey
    if not ed25519_verify(
        public_key = prekey_bundle.IKb.pub,
        message    = prekey_bundle.SPKb.pub,
        signature  = prekey_bundle.SPKb_sig
    ):
        raise Exception("Invalid SPK signature — possible MITM")

    # Step 2: Generate ephemeral key
    EKa = generate_curve25519_keypair()

    # Step 3: DH calculations
    DH1 = x25519(alice_ik.priv,     prekey_bundle.SPKb.pub)
    DH2 = x25519(EKa.priv,          prekey_bundle.IKb.pub)
    DH3 = x25519(EKa.priv,          prekey_bundle.SPKb.pub)

    if prekey_bundle.OPKb is not None:
        DH4 = x25519(EKa.priv,      prekey_bundle.OPKb.pub)
        ikm = b"\x00" * 32 + DH1 + DH2 + DH3 + DH4
    else:
        ikm = b"\x00" * 32 + DH1 + DH2 + DH3

    # Step 4: Derive master secret
    master_secret = HKDF(ikm, salt=b"\x00"*32,
                         info=b"WhisperText", length=32)

    # Step 5: Initialize Double Ratchet
    (RK, CKs) = HKDF(master_secret,
                      salt=b"\x00"*32,
                      info=b"WhisperRatchet", length=64)
    DHs = generate_curve25519_keypair()

    state = RatchetState(
        RK=RK, CKs=CKs, CKr=None,
        DHs=DHs, DHr=prekey_bundle.SPKb.pub,
        Ns=0, Nr=0, PN=0, MKSKIPPED={}
    )

    # Step 6: Build initial message (includes first ciphertext)
    first_message_bytes = ratchet_encrypt(state, plaintext)

    return PreKeySignalMessage(
        registration_id  = alice_registration_id,
        pre_key_id       = prekey_bundle.OPKb_id,
        signed_pre_key_id = prekey_bundle.SPKb_id,
        base_key         = EKa.pub,
        identity_key     = alice_ik.pub,
        message          = first_message_bytes,
    ), state
```

### X3DH Receive (Bob processes initial message)

```
procedure x3dh_receive(bob_ik, msg: PreKeySignalMessage):
    # Look up SPK and OPK private keys by ID
    SPKb = lookup_spk(msg.signed_pre_key_id)
    OPKb = lookup_opk(msg.pre_key_id) if msg.pre_key_id else None

    # DH calculations (reverse of Alice's)
    DH1 = x25519(SPKb.priv,  msg.identity_key)    # DH(SPKb, IKa)
    DH2 = x25519(bob_ik.priv, msg.base_key)        # DH(IKb, EKa)
    DH3 = x25519(SPKb.priv,  msg.base_key)         # DH(SPKb, EKa)

    if OPKb is not None:
        DH4 = x25519(OPKb.priv, msg.base_key)      # DH(OPKb, EKa)
        ikm = b"\x00" * 32 + DH1 + DH2 + DH3 + DH4
        # Delete OPK private key immediately — single use
        delete_opk(msg.pre_key_id)
    else:
        ikm = b"\x00" * 32 + DH1 + DH2 + DH3

    master_secret = HKDF(ikm, salt=b"\x00"*32,
                         info=b"WhisperText", length=32)

    (RK, CKr) = HKDF(master_secret,
                     salt=b"\x00"*32,
                     info=b"WhisperRatchet", length=64)

    state = RatchetState(
        RK=RK, CKs=None, CKr=CKr,
        DHs=generate_curve25519_keypair(),  # Bob's first ratchet key
        DHr=msg.base_key,                  # Alice's first ratchet key (EKa.pub? No)
        # Note: DHr is set to the ratchet key in the inner SignalMessage header
        Ns=0, Nr=0, PN=0, MKSKIPPED={}
    )

    plaintext = ratchet_decrypt(state, msg.message)
    return plaintext, state
```

### Double Ratchet Encrypt

```
procedure ratchet_encrypt(state, plaintext):
    # Symmetric ratchet step
    (mk, state.CKs) = chain_kdf(state.CKs)
    state.Ns += 1

    # Build header
    header = MessageHeader(
        dh     = state.DHs.pub,
        pn     = state.PN,
        n      = state.Ns - 1,
    )

    # Encrypt
    ciphertext = aead_encrypt(mk, plaintext, associated_data=header)
    return SignalMessage(header, ciphertext)


procedure chain_kdf(ck):
    mk  = HMAC_SHA256(ck, b"\x01")
    new_ck = HMAC_SHA256(ck, b"\x02")
    return mk, new_ck
```

### Double Ratchet Decrypt

```
procedure ratchet_decrypt(state, msg: SignalMessage):
    header = msg.header

    # Check if we have a cached skipped key for this message
    key = (header.dh, header.n)
    if key in state.MKSKIPPED:
        mk = state.MKSKIPPED.pop(key)
        return aead_decrypt(mk, msg.ciphertext, header)

    # New DH ratchet key from sender?
    if header.dh != state.DHr:
        # Cache skipped keys from previous receiving chain
        skip_message_keys(state, until=header.pn)

        # DH ratchet step
        state.PN = state.Ns
        state.Ns = 0
        state.Nr = 0
        dh_out = x25519(state.DHs.priv, header.dh)
        (state.RK, state.CKr) = root_kdf(state.RK, dh_out)
        state.DHr = header.dh
        state.DHs = generate_curve25519_keypair()
        (state.RK, state.CKs) = root_kdf(state.RK,
            x25519(state.DHs.priv, state.DHr))

    # Skip any out-of-order messages in the current chain
    skip_message_keys(state, until=header.n)

    # Derive and use message key
    (mk, state.CKr) = chain_kdf(state.CKr)
    state.Nr += 1

    return aead_decrypt(mk, msg.ciphertext, header)


procedure skip_message_keys(state, until):
    if state.Nr + MAX_SKIP < until:
        raise Exception("Too many skipped messages")
    if state.CKr is not None:
        while state.Nr < until:
            (mk, state.CKr) = chain_kdf(state.CKr)
            state.MKSKIPPED[(state.DHr, state.Nr)] = mk
            state.Nr += 1


procedure root_kdf(rk, dh_out):
    output = HKDF(ikm=dh_out, salt=rk,
                  info=b"WhisperRatchet", length=64)
    return output[:32], output[32:]
```

### Safety Number Computation

```
procedure compute_safety_number(local_ik_pub, local_identifier,
                                 remote_ik_pub, remote_identifier):
    def fingerprint(ik_pub, identifier):
        # identifier = phone number or UUID as bytes
        data = ik_pub + identifier
        h = data
        for _ in range(5200):
            h = SHA512(h + ik_pub)
        return h[:30]   # 30 bytes = 240 bits

    fp_local  = fingerprint(local_ik_pub, local_identifier)
    fp_remote = fingerprint(remote_ik_pub, remote_identifier)

    # Canonical order: sort so both parties see the same number
    combined = sorted([fp_local, fp_remote])
    combined_bytes = combined[0] + combined[1]  # 60 bytes

    # Format as 12 groups of 5 decimal digits
    # Each group: 5 decimal digits from 5 bytes (big-endian int % 100000)
    result = []
    for i in range(12):
        chunk = combined_bytes[i*5 : i*5 + 5]
        n = int.from_bytes(chunk, "big") % 100000
        result.append(f"{n:05d}")
    return " ".join(result)   # "05156 87657 ..."
```

## Test Strategy

### Unit Tests

**Curve25519 / DH**
- `DH(a, B) == DH(b, A)` for random keypairs — verify commutativity.
- Verify known test vectors from RFC 7748 (Curve25519 test vectors).
- Verify that DH output changes when either key changes.

**X3DH**
- Compute X3DH on Alice's side, then on Bob's side with the same inputs;
  verify `master_secret_alice == master_secret_bob`.
- Verify that removing the OPK (setting it to None) still produces a valid
  (different) shared secret.
- Verify that an invalid SPK signature raises an exception before any DH
  computation.
- Verify that the initial ciphertext decrypts correctly on Bob's side.

**KDF Chains**
- `chain_kdf(ck)` returns two different values.
- `chain_kdf(chain_kdf(ck)[1])` produces a different mk than
  `chain_kdf(ck)[0]` — chain advances correctly.
- Known-answer test: verify HKDF output against RFC 5869 test vector.

**Double Ratchet — basic encrypt/decrypt**
- Initialize Alice and Bob with the same initial state (simulating post-X3DH).
- Alice encrypts "Hello"; Bob decrypts; verify plaintext.
- Bob encrypts "World"; Alice decrypts; verify plaintext.
- Verify that the ratchet keys change after a DH ratchet step.

**Double Ratchet — out-of-order messages**
- Alice sends messages 1, 2, 3. Bob receives 3 first, then 1, then 2.
- Verify all three decrypt correctly.
- Verify `MKSKIPPED` contains keys for messages 1 and 2 after receiving 3.
- After receiving 1 and 2, verify `MKSKIPPED` is empty.

**Double Ratchet — skipped key limit**
- Alice sends 2001 messages without Bob receiving any.
- Verify decryption of message 2001 fails with "Too many skipped messages."

**Safety Numbers**
- Compute safety numbers for Alice and Bob from known public keys.
- Verify the result is exactly 12 groups of 5 decimal digits.
- Verify safety numbers are the same when computed from Bob's perspective
  (with Alice's and Bob's roles swapped in the input).
- Verify that a 1-bit change in either public key produces a completely
  different safety number.

### Integration Tests

**Full X3DH + Double Ratchet session**
- Register Bob with 100 OPKs and 1 SPK.
- Alice fetches Bob's prekey bundle.
- Alice sends "Hello" as a PreKeySignalMessage.
- Bob processes the PreKeySignalMessage → decrypts "Hello".
- Bob replies "Hi" as a regular SignalMessage.
- Alice decrypts "Hi".
- Exchange 50 more messages in both directions.
- Verify all decrypt correctly.

**OPK Exhaustion**
- Register Bob with 0 OPKs (exhausted supply).
- Alice fetches bundle without OPK.
- X3DH proceeds without DH4.
- Verify session establishes and messages decrypt.
- Verify master_secret is different from the OPK-present case.

**Sealed Sender round-trip**
- Alice has a SenderCertificate from Signal's server (use test cert).
- Alice sends a Sealed Sender message to Bob.
- Bob decrypts the outer layer, verifies the certificate, decrypts the
  SignalMessage.
- Verify Bob learns Alice's identity from the certificate, not the envelope.

**Sender Key group messaging**
- Alice, Bob, Carol are in a group.
- Alice distributes a SenderKeyDistributionMessage to Bob and Carol.
- Alice sends 5 messages.
- Verify Bob and Carol each decrypt all 5 correctly.
- Bob sends a message; verify Alice and Carol decrypt.
- Carol leaves the group; Alice sends a new SenderKeyDistributionMessage.
- Verify Carol cannot decrypt Alice's new messages (she has no new chain key).

**Multi-device sync**
- Alice has two devices (device 1, device 2).
- Alice sends from device 1.
- Verify device 2 receives a SyncMessage with the sent content.
- Bob replies; verify both devices receive the reply.

### Security Property Tests

**Forward secrecy verification**
- Record the ratchet state at message 5.
- Advance to message 10 (delete all intermediate keys as per protocol).
- Using only the state at message 10, verify that message keys 1–4 cannot
  be recomputed. (They should require CKr values that have been overwritten.)

**Break-in recovery verification**
- Leak the entire Double Ratchet state at step N.
- Continue exchanging messages; verify that after the next DH ratchet step
  (step N+1), the leaked state cannot decrypt step N+2 messages.
  (The attacker lacks the new DH private key generated after step N.)

**Replay attack**
- Alice sends message M1 to Bob.
- Capture the on-wire bytes of M1.
- Bob decrypts M1 successfully.
- Replay M1 to Bob.
- Verify Bob rejects the replayed message (the message number Nr has advanced
  past M1's counter; the key was deleted).

**Key substitution (MITM) detection**
- Alice sends to a modified prekey bundle (attacker's SPK with valid IK sig
  forged by substituting the IK sig with attacker's key).
- Verify `ed25519_verify` fails and the session is not established.
