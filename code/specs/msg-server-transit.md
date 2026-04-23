# MSG-SERVER-TRANSIT — Transit Server Architecture for E2E Messaging

## Overview

A transit server is the opposite of what most people imagine when they hear
"server." In a traditional web service, the server is the brain: it stores
everything, indexes it, queries it, and could hand the entire database over
to an adversary with a warrant. A transit server is designed so that
handing over the entire database is useless. The goal is not policy ("we
promise not to look") but architecture: the server is mathematically incapable
of reading what it stores.

This spec defines the transit server's exact responsibilities, schemas, wire
APIs, and threat model. It answers three questions precisely:

1. What is the server allowed to know? (The minimum necessary to route messages.)
2. What must the server never know? (Message content, sender identity in
   sealed-sender mode, private keys.)
3. How do you build it so that even a full compromise leaks as little as
   possible?

**Analogy:** A transit server is like a locked postal sorting facility.
The facility knows the recipient's address (to route the envelope), knows the
postmark date, knows the size and weight of the envelope, and has a record of
which envelopes arrived for delivery. But the envelope contents are sealed —
the facility cannot read them even if it wanted to. And if police raid the
facility, they get routing records and envelope sizes. They find no message
content, because the facility never had any.

The key insight: the envelope's seal is unbreakable because the recipient holds
the only key. The postal facility never held a copy of that key and could not
have obtained one.

## Threat Model

### The Four Adversaries

```
Adversary 1: Law Enforcement Compulsion
══════════════════════════════════════════════════════════════
  Traditional server: operator hands over plaintext messages, contact graphs,
  message timestamps, sender-to-recipient mappings, full history.

  Transit server: operator hands over UUIDs, public keys, delivery
  timestamps, sealed ciphertexts (unreadable without client keys), and
  "Bob received something at 10pm" (but not from whom, in sealed-sender
  mode). No message content. No contact graph.


Adversary 2: External Attacker (Database Breach)
══════════════════════════════════════════════════════════════
  Traditional server: attacker reads messages, contact lists, message
  history, possibly passwords.

  Transit server: attacker reads sealed ciphertexts (encrypted blobs),
  public keys (already public), push tokens (usable for spam, but not
  for reading messages), and delivery timestamps. No plaintext. No
  sender identity in sealed-sender messages.


Adversary 3: Malicious Insider (Rogue Employee)
══════════════════════════════════════════════════════════════
  Traditional server: employee reads any user's messages, views contact
  relationships, exports conversation history.

  Transit server: employee reads the same as the attacker above.
  The database contains no plaintext to read.


Adversary 4: Malicious Operator (Policy Change or Acquisition)
══════════════════════════════════════════════════════════════
  Traditional server: operator retroactively exports all message history,
  sells to a data broker, or provides to a government without legal process.

  Transit server: operator cannot retroactively decrypt past messages
  (forward secrecy: the ephemeral keys used for those sessions are gone).
  Delivered messages are deleted. What remains in the database cannot be
  decrypted without keys the operator never possessed.
```

### What the Server Legitimately Knows

Some metadata is unavoidable. Routing requires knowing a recipient. Push
delivery requires knowing a push token. This is the server's irreducible
knowledge:

```
  ┌─────────────────────────────────────────────────────────────────────┐
  │ UNAVOIDABLE SERVER KNOWLEDGE                                        │
  ├─────────────────────────────────────────────────────────────────────┤
  │ Account identifiers: UUID per user, device IDs per account          │
  │ Phone numbers: required for SMS-based registration (see §1 for      │
  │   how to minimize even this)                                        │
  │ Push tokens: APNs or FCM tokens — necessary to wake up the app     │
  │ Public keys: identity keys, signed prekeys, one-time prekeys        │
  │   (by definition, these are public — no harm in knowing them)       │
  │ Delivery timestamp: when Bob received something (not from whom)     │
  │ Recipient UUID: who a queued message is for (to route it)           │
  │ Message size: inferred from ciphertext byte count                   │
  │ "Last seen": when Bob's device last fetched messages                │
  └─────────────────────────────────────────────────────────────────────┘
```

### What the Server Must Never Know

```
  ┌─────────────────────────────────────────────────────────────────────┐
  │ FORBIDDEN SERVER KNOWLEDGE                                          │
  ├─────────────────────────────────────────────────────────────────────┤
  │ Message content: the server stores sealed ciphertexts it cannot     │
  │   decrypt. The plaintext never exists on the server.                │
  │ Sender identity (sealed sender): in sealed-sender mode, the server  │
  │   authenticates the HTTP connection (knows account A sent something) │
  │   but does NOT store the sender UUID in the message queue. The      │
  │   database row has no "from" field.                                 │
  │ Private keys: never transmitted; any request with a private key     │
  │   field is rejected (defense in depth).                             │
  │ Message history: delivered messages are deleted. The server is a    │
  │   mailbox, not an archive.                                          │
  │ Contact relationships: no contact list is stored server-side. The   │
  │   server does not know who is in a user's contacts.                 │
  └─────────────────────────────────────────────────────────────────────┘
```

## Architecture

### System Components

```
╔══════════════════════════════════════════════════════════════════════╗
║                      Client-Facing Services                          ║
║                                                                      ║
║  ┌──────────────────┐  ┌──────────────────┐  ┌──────────────────┐  ║
║  │  Account Service │  │  Prekey Service  │  │ Message Service  │  ║
║  │                  │  │                  │  │                  │  ║
║  │  • Registration  │  │  • Store OPKs    │  │  • Receive       │  ║
║  │  • Auth (UUID+   │  │  • Store SPKs    │  │    sealed msgs   │  ║
║  │    password)     │  │  • Fetch prekey  │  │  • Queue for     │  ║
║  │  • Device link   │  │    bundles       │  │    delivery      │  ║
║  │  • Push token    │  │  • Verify SPK    │  │  • Push notify   │  ║
║  │    update        │  │    signatures    │  │  • Serve on      │  ║
║  └────────┬─────────┘  └────────┬─────────┘  │    fetch         │  ║
║           │                    │             └────────┬─────────┘  ║
╚═══════════╪════════════════════╪══════════════════════╪════════════╝
            │                   │                      │
   ┌────────┴────────┐  ┌───────┴────────┐   ┌────────┴────────────┐
   │   User Store    │  │  Prekey Store  │   │   Message Queue     │
   │                 │  │                │   │                     │
   │  uuid           │  │  OPKs (public  │   │  TTL-limited        │
   │  phone_hash     │  │  keys only)    │   │  Sealed ciphertexts │
   │  identity_key   │  │                │   │  No content index   │
   │  devices[]      │  │  SPKs + sigs   │   │  No sender field    │
   │  push_tokens    │  │                │   │  (sealed sender)    │
   └─────────────────┘  └────────────────┘   └────────┬────────────┘
                                                       │
                                            ┌──────────┴──────────────┐
                                            │     Push Gateway        │
                                            │                         │
                                            │  APNs (iOS)             │
                                            │  FCM (Android)          │
                                            │                         │
                                            │  Payload: "come fetch"  │
                                            │  NOT: message content   │
                                            └─────────────────────────┘
```

### Message Flow: Sender to Recipient

```
Alice (sender)                Server                Bob (recipient)
      │                          │                         │
      │   1. Fetch prekeys        │                         │
      │ ─ GET /v2/keys/bob/* ──► │                         │
      │ ◄─ prekey bundle ──────── │                         │
      │                          │                         │
      │   2. Verify SPK signature locally (client-side)    │
      │   3. Run X3DH + establish Double Ratchet session   │
      │   4. Encrypt message                               │
      │   5. Seal sender envelope                          │
      │                          │                         │
      │   6. Send sealed message  │                         │
      │ ─ PUT /v1/messages/bob ► │                         │
      │                          │ 7. Validate:            │
      │                          │   • Bob exists           │
      │                          │   • Size ≤ 64KB          │
      │                          │   • Alice authenticated  │
      │                          │   • NOT: read content    │
      │                          │                         │
      │                          │ 8. Store in queue        │
      │                          │   {recipient: bob,       │
      │                          │    type: 52,             │
      │                          │    content: <sealed>,    │
      │                          │    NO sender field}      │
      │ ◄─ 200 OK ─────────────── │                         │
      │                          │                         │
      │                          │ 9. Push notification     │
      │                          │ ─ APNs/FCM ──────────► Bob's device
      │                          │   {"content-available":1}│
      │                          │                         │ wakes up
      │                          │                         │
      │                          │  10. Bob fetches         │
      │                          │ ◄─ GET /v1/messages ───── │
      │                          │ ─► sealed ciphertext ──── │
      │                          │                         │ 11. Decrypt
      │                          │                         │ 12. Display
      │                          │                         │
      │                          │  13. Acknowledge delivery│
      │                          │ ◄─ DELETE /v1/messages/.. │
      │                          │ 14. Delete from queue    │
      │                          │ ─► 200 OK ─────────────── │
```

## Component 1: Account Service

### Registration

When a new user installs the app and registers, these fields are collected and
stored:

```
Registration Flow
═════════════════

Step 1: Request SMS OTP
  PUT /v1/accounts/sms/code/{e164_phone_number}
  → Server sends OTP via SMS, stores {phone: hash, otp: HMAC(otp, secret), expires: now+10m}

Step 2: Verify OTP + Create Account
  PUT /v1/accounts/code/{verification_code}
  Body: {
    "accountAttributes": {
      "signalingKey":      "base64...",     -- AES-256 + HMAC-SHA256 key for push
      "registrationId":    12345,           -- random 14-bit integer per device
      "fetchesMessages":   true,            -- does this device poll instead of push?
      "unidentifiedAccessKey": "base64...", -- 16-byte key for sealed-sender validation
      "unrestrictedUnidentifiedAccess": false
    },
    "identityKey":   "base64...",           -- Curve25519 public key (32 bytes)
    "signingKey":    "base64...",           -- Ed25519 public key (32 bytes)
    "password":      "base64...",           -- 64 random chars generated client-side
    "preKeys":       [...],                 -- initial batch of OPKs
    "signedPreKey":  {...}                  -- initial SPK with signature
  }

Step 3: Server creates account
  • Assigns UUID (random 128-bit identifier)
  • Stores identity_key and signing_key
  • Stores SHA-256(password + salt) — the plaintext is discarded
  • Stores initial prekeys
  • The phone number is hashed: HMAC-SHA256(phone, server_pepper)
    The plaintext phone number is needed only during this step.
```

### The User Store Schema

```
User Store
══════════

accounts {
  uuid             UUID PRIMARY KEY              -- random 128-bit, server-assigned
  phone_hash       VARCHAR(64) UNIQUE            -- HMAC-SHA256(phone, pepper)
                                                 -- NOT the plaintext phone number
  identity_key     BYTES(32) NOT NULL            -- Curve25519 public key
  signing_key      BYTES(32) NOT NULL            -- Ed25519 public key
  password_hash    VARCHAR(128) NOT NULL         -- SHA-256(password + salt)
  password_salt    BYTES(32) NOT NULL
  created_at       TIMESTAMP NOT NULL
  unidentified_access_key BYTES(16)              -- for sealed-sender MAC check
  unrestricted_unidentified_access BOOLEAN DEFAULT FALSE
}

devices {
  uuid             UUID REFERENCES accounts(uuid)
  device_id        SMALLINT NOT NULL             -- 1 = primary, 2+ = linked
  PRIMARY KEY (uuid, device_id)

  registration_id  UINT16 NOT NULL               -- random 14-bit per device
  push_token       VARCHAR(512)                  -- APNs or FCM token (nullable)
  push_type        ENUM('apns','apns_sandbox','fcm','apns_voip')
  last_seen        TIMESTAMP                     -- updated on each delivery
  user_agent       VARCHAR(128)                  -- "Signal-iOS/6.12 iOS/17.0"

  -- Current signed prekey for this device
  signed_prekey_id    UINT32
  signed_prekey       BYTES(32)
  signed_prekey_sig   BYTES(64)                  -- Ed25519 signature
}
```

What is intentionally absent from this schema:

```
  ✗ contact lists    — the server never stores who talks to whom
  ✗ message history  — delivered messages are deleted from the queue
  ✗ private keys     — never sent to the server; any request including
                       a private key field is rejected with HTTP 400
  ✗ plaintext phone  — only the HMAC hash is stored after verification
  ✗ IP addresses     — not logged (or logged with a very short TTL)
```

### Phone Number Privacy

**Why store phone numbers at all?** Phone-number-based identity (like Signal)
requires verifying ownership of a number. But after verification, the plaintext
number becomes a liability.

```
Better approach: peppered HMAC storage

  phone_hash = HMAC-SHA256(normalize(phone), server_pepper)

  Where:
    normalize(phone): strip spaces, ensure E.164 format (+15551234567)
    server_pepper: a 32-byte secret stored in hardware security module (HSM)
                   NOT in the database

  Properties:
    • You can check "is this phone registered?" by hashing the query
    • You cannot enumerate all phone numbers from the hash column
    • Even with the database, you need the pepper to reverse the hash
    • The pepper is in an HSM — not on any disk the attacker can steal
```

Contact discovery (helping Alice find which of her contacts use the service)
must be done without leaking Alice's entire contact list to the server. Two
approaches:

```
Option 1: SGX Enclave (Intel Software Guard Extensions)
  Alice sends her contact hashes to a remote attestation-verified enclave.
  The enclave runs inside a tamper-resistant CPU region. The server operator
  cannot inspect the enclave's memory.
  Intersection is computed inside the enclave. Only the match set leaves.

Option 2: Private Information Retrieval (PIR)
  A cryptographic protocol where Alice queries "is phone X registered?"
  without the server learning which phone X she is asking about.
  Computationally heavier, but does not require special hardware.
```

### Authentication

After registration, every authenticated request uses HTTP Basic Auth over TLS:

```
Authorization: Basic base64(uuid + ":" + password)

Where:
  uuid     = the UUID assigned during registration (128-bit, hex string)
  password = the 64-char random string generated client-side at registration
             (the server stores SHA-256(password + salt), not the plaintext)

Why Basic Auth and not JWT/OAuth?
  • Stateless: the server verifies every request independently
  • No session tokens to leak
  • No refresh tokens to rotate
  • The password never changes unless explicitly rotated by the user
  • TLS ensures the password is not intercepted in transit
```

### Device Linking

A primary device can authorize additional linked devices (tablets, desktop
apps). The protocol prevents a malicious server from silently adding a device:

```
Device Linking Protocol
═══════════════════════

Step 1: Primary device generates a provisioning key pair (ephemeral Curve25519)
        and displays a QR code:
          signal://v1/linked-device?uuid=provisioning_uuid&pub_key=base64...

Step 2: New device scans QR code. New device generates its own identity key pair.
        New device contacts server:
          PUT /v1/provisioning/{provisioning_uuid}
          Body: {
            "body": encrypt(AES-256-CBC, provisioning_key, {
              "identityKey": new_device_identity_key_pub,
              "signingKey":  new_device_signing_key_pub,
              "number":      phone_number,              -- for verification
              "provisioningCode": code_from_qr
            })
          }

Step 3: Server forwards the encrypted blob to the primary device
        (the server CANNOT read it — it's encrypted with the provisioning key
        which the server does not have).

Step 4: Primary device decrypts, verifies the provisioning code matches,
        signs the new device's identity key with its own signing key,
        and sends a final authorization message.

Result: The server sees that device_id=2 now exists for account uuid=X.
        It does NOT see the new device's private key (never transmitted).
        It cannot have silently inserted a device (the primary device
        must explicitly approve by signing the new device's key).
```

## Component 2: Prekey Service

**What prekeys are:**

The X3DH key agreement protocol (used to establish an initial shared secret
between Alice and Bob before they have ever communicated) requires Bob to
pre-publish a set of public keys. These are "prekeys" — one-time Diffie-Hellman
material that Alice consumes to establish a session.

**Analogy:** Imagine Bob leaves 50 sealed numbered envelopes at the post office,
each containing one half of a unique secret handshake. When Alice wants to talk
to Bob for the first time, she picks one envelope, uses it to derive a shared
secret, and mails her first message. Bob then opens the matching envelope on his
side and derives the same secret. Once an envelope is used, it is discarded.
The post office (server) sees the envelopes but cannot see inside them (private
keys were never in the envelopes — only the public halves).

### Key Types

```
One-Time Prekeys (OPKs)
────────────────────────
  • Curve25519 key pairs
  • Each OPK is used exactly once, then deleted from the server
  • Provides forward secrecy at the session-establishment level:
    if an OPK private key is later compromised, only one session
    is affected (the one that used that OPK)
  • Uploaded in batches (e.g., 100 at a time)
  • The server deletes the OPK from its store when it is claimed
    by an initiating sender

Signed Prekeys (SPKs)
──────────────────────
  • Curve25519 key pairs
  • Signed by the account's Ed25519 signing key
    (proves the SPK came from the legitimate owner, not a server MITM)
  • Rotated every 1–4 weeks
  • Used as fallback when OPKs are exhausted
  • Only the public key + signature are stored on the server

Identity Key
─────────────
  • Curve25519 key pair
  • The user's permanent public key
  • Rarely changes (only if the user reinstalls or switches devices)
  • Used in the X3DH handshake alongside OPKs and SPKs
```

### The Prekey Store Schema

```
Prekey Store
════════════

one_time_prekeys {
  uuid        UUID                           -- account that owns these keys
  device_id   SMALLINT
  key_id      UINT32                         -- client-assigned sequential ID
  public_key  BYTES(32)                      -- Curve25519 public key
  PRIMARY KEY (uuid, device_id, key_id)
  -- NO private key. The server only ever stores the public half.
}

signed_prekeys {
  uuid        UUID
  device_id   SMALLINT
  key_id      UINT32
  public_key  BYTES(32)
  signature   BYTES(64)                      -- Ed25519 signature by signing_key
  uploaded_at TIMESTAMP
  PRIMARY KEY (uuid, device_id)              -- only one active SPK per device
}
```

### Upload Flow

```
PUT /v2/keys
Authorization: Basic uuid:password
Content-Type:  application/json

{
  "preKeys": [
    {"keyId": 1,   "publicKey": "base64..."},
    {"keyId": 2,   "publicKey": "base64..."},
    ...
    {"keyId": 100, "publicKey": "base64..."}
  ],
  "signedPreKey": {
    "keyId":     5,
    "publicKey": "base64...",
    "signature": "base64..."     -- Ed25519-Sign(signing_key_priv, signed_prekey_pub)
  }
}

Server validation (MUST happen in this order):
  1. Authenticate the sender (HTTP Basic Auth)
  2. Verify signedPreKey.signature:
       Ed25519-Verify(account.signing_key, signedPreKey.publicKey, signedPreKey.signature)
       If verification fails → return HTTP 400 (reject the batch)
  3. Reject any request body containing a "privateKey" field → HTTP 400
     (Defense in depth: private keys must never be transmitted)
  4. Store each OPK in one_time_prekeys table
  5. Replace the current SPK in signed_prekeys table

Response (success):
  HTTP 200 OK
  {}
```

### Fetch Flow

When Alice wants to send a first message to Bob, she fetches his prekey bundle:

```
GET /v2/keys/{bob_uuid}/{device_id}
Authorization: Basic alice_uuid:alice_password

Response:
  HTTP 200 OK
  {
    "identityKey": "base64...",            -- Bob's Curve25519 identity public key
    "devices": [
      {
        "deviceId":       1,
        "registrationId": 12345,           -- Bob's device registration ID
        "preKey": {
          "keyId":     23,
          "publicKey": "base64..."         -- a one-time prekey (now deleted from server)
        },
        "signedPreKey": {
          "keyId":     5,
          "publicKey": "base64...",
          "signature": "base64..."
        }
      }
    ]
  }

Server actions (performed atomically):
  1. Look up Bob's account → fetch identity_key, signing_key
  2. Claim one OPK for this device: SELECT + DELETE (one operation, atomic)
     If no OPKs remain: omit the "preKey" field (use SPK only — still secure,
     but loses some forward secrecy at session establishment)
  3. Return the bundle

What Alice MUST verify after receiving this bundle:
  ┌────────────────────────────────────────────────────────────────────┐
  │ MANDATORY CLIENT-SIDE VERIFICATION                                 │
  │                                                                    │
  │ Ed25519-Verify(                                                    │
  │   public_key = bundle.identityKey,    ← from the bundle           │
  │                                        (this IS the signing key   │
  │                                         for the identity key)     │
  │   message    = bundle.signedPreKey.publicKey,                      │
  │   signature  = bundle.signedPreKey.signature                       │
  │ )                                                                  │
  │                                                                    │
  │ If this fails:                                                     │
  │   → The server (or a MITM) substituted a fake signed prekey       │
  │   → Alice must ABORT and surface an error to the user             │
  │   → Proceeding would encrypt to an attacker-controlled key        │
  │                                                                    │
  │ This check is what makes the server-as-MITM attack impossible:    │
  │ the server cannot forge a signature it does not hold the private  │
  │ key for, and it never learned the private key.                    │
  └────────────────────────────────────────────────────────────────────┘
```

### OPK Exhaustion and Replenishment

```
OPK Count Check
═══════════════

  GET /v2/keys
  Authorization: Basic uuid:password
  → {"count": 42}         -- how many OPKs remain for the primary device

  GET /v2/keys?device=2
  → {"count": 15}         -- OPK count for device 2

Replenishment policy:
  Client SHOULD upload more OPKs when count < 10.
  Client MUST upload more OPKs when count == 0.

  If count == 0 when a sender fetches keys:
    • The server returns only the SPK (no "preKey" field in response)
    • X3DH still works (SPK is used instead of OPK)
    • Forward secrecy guarantee is slightly weakened (SPK used for
      multiple sessions until rotation) but not broken
    • The server MUST NOT return 404 in this case — that would prevent
      establishing any session with Bob

Server-side alert (via push notification) when OPK count drops below 10:
  Server can include in the message fetch response:
    "preKeyCount": 8          -- hint that replenishment is needed
  The client checks this field and uploads more OPKs proactively.
```

### SPK Rotation Policy

```
Rotation Timeline
═════════════════

  Week 0  Week 1  Week 2  Week 3  Week 4
  ────────────────────────────────────────
  SPK-A   SPK-A   SPK-B   SPK-B   SPK-C
  (active)(active)(active)(grace) (active)
                          ↑
                     SPK-A grace period:
                     Sessions established with SPK-A may still be
                     active. Keep SPK-A until all those sessions
                     have exchanged at least one Double Ratchet
                     message (which provides new DH material,
                     making SPK-A no longer needed for decryption).

Why keep the old SPK briefly?
  Alice initiates a session using SPK-A at Week 1.
  Bob rotates to SPK-B at Week 2.
  Bob still needs SPK-A's private key to decrypt Alice's first
  message (the X3DH initial message uses SPK in its DH step).
  Once Bob receives Alice's first message and sends a reply,
  the Double Ratchet has advanced — SPK-A can be deleted.
```

## Component 3: Message Service

### The Message Pipeline

```
Message Lifecycle
═════════════════

  Phase 1: RECEIPT (server receives sealed message from Alice)
  ┌─────────────────────────────────────────────────────────┐
  │ Alice → PUT /v1/messages/bob_uuid                       │
  │   Server checks:                                        │
  │     ✓ Alice is authenticated (HTTP Basic Auth)          │
  │     ✓ Bob exists (UUID lookup)                          │
  │     ✓ Envelope ≤ 64KB                                   │
  │     ✓ All of Bob's devices included (or 409 returned)   │
  │     ✗ Server does NOT read the ciphertext               │
  └─────────────────────────────────────────────────────────┘

  Phase 2: QUEUE (stored until Bob comes online)
  ┌─────────────────────────────────────────────────────────┐
  │ message_queue row:                                      │
  │   recipient_uuid: bob_uuid                              │
  │   recipient_device: 1                                   │
  │   type: 52 (UNIDENTIFIED_SENDER)                        │
  │   content: <sealed ciphertext — unreadable>             │
  │   timestamp: server receive time                        │
  │   guid: randomly generated                              │
  │   NO sender field (sealed sender — server doesn't know) │
  └─────────────────────────────────────────────────────────┘

  Phase 3: NOTIFY (wake up Bob's device)
  ┌─────────────────────────────────────────────────────────┐
  │ Push notification:                                      │
  │   {"aps": {"content-available": 1}}                     │
  │   — Says only "wake up and fetch"                       │
  │   — Contains NO message content or sender info          │
  └─────────────────────────────────────────────────────────┘

  Phase 4: DELIVERY (Bob fetches and decrypts)
  ┌─────────────────────────────────────────────────────────┐
  │ Bob → GET /v1/messages                                  │
  │ Server returns sealed ciphertext blobs                  │
  │ Bob's app decrypts locally                              │
  │ Bob → DELETE /v1/messages/{uuid}/{timestamp}            │
  │ Server deletes the queue entry permanently              │
  └─────────────────────────────────────────────────────────┘
```

### Message Queue Schema

```
Message Queue
═════════════

message_queue {
  recipient_uuid    UUID NOT NULL                -- who this is for
  recipient_device  SMALLINT NOT NULL            -- which device
  guid              UUID NOT NULL PRIMARY KEY    -- randomly generated by sender
                                                 -- used for deduplication
  server_timestamp  BIGINT NOT NULL              -- Unix milliseconds, server receive time
  client_timestamp  BIGINT NOT NULL              -- from the sender's PUT request body
                                                 -- (inside ciphertext; here it's the
                                                 --  client-reported timestamp, not
                                                 --  decrypted — just for queue ordering)
  type              TINYINT NOT NULL             -- see type values below
  content           BYTEA NOT NULL               -- sealed ciphertext, up to 64KB
  expires_at        TIMESTAMP NOT NULL           -- 30 days from receipt
  INDEX (recipient_uuid, recipient_device, server_timestamp)
  -- No "sender_uuid" column (sealed sender — server does not know)
  -- No content index (obvious)
}
```

### Message Type Field

```
Type Field Values
═════════════════

  ┌──────┬──────────────────────────┬─────────────────────────────────────┐
  │ Type │ Name                     │ Meaning                             │
  ├──────┼──────────────────────────┼─────────────────────────────────────┤
  │  1   │ CIPHERTEXT               │ Regular Double Ratchet message.     │
  │      │                          │ Sender UUID is included in the      │
  │      │                          │ database row (NOT sealed sender).   │
  ├──────┼──────────────────────────┼─────────────────────────────────────┤
  │  3   │ PREKEY_BUNDLE            │ X3DH initial message. Establishes   │
  │      │                          │ a new session. Sender known to      │
  │      │                          │ server (not sealed sender).         │
  ├──────┼──────────────────────────┼─────────────────────────────────────┤
  │  52  │ UNIDENTIFIED_SENDER      │ Sealed sender. The server cannot    │
  │      │                          │ determine who sent this. No sender  │
  │      │                          │ UUID in the database row.           │
  └──────┴──────────────────────────┴─────────────────────────────────────┘

  Why is the gap between 3 and 52?
    Type 52 was chosen by Signal to be clearly distinct from all
    "normal" type codes, reducing the risk of accidental misclassification.
    There is no technical requirement — it is a design choice.

  Why does the server need a type field at all for sealed sender?
    The server needs to know HOW to structure the push notification.
    Type 52 means: "send a silent push (content-available: 1) only.
    Do not include any content in the push payload."
    Without this field, the server could not distinguish between
    a sealed message and a control message (e.g., receipt or typing indicator).
```

### The Fetch and Acknowledgment API

```
Fetch queued messages:
  GET /v1/messages
  Authorization: Basic uuid:password

  Response:
  {
    "messages": [
      {
        "guid":            "550e8400-e29b-41d4-a716-446655440000",
        "serverTimestamp": 1706789012350,
        "clientTimestamp": 1706789012100,
        "type":            52,
        "content":         "base64-encoded-sealed-ciphertext"
        -- NO "source" field (type=52, sealed sender)
        -- NO "sourceDevice" field
      },
      {
        "guid":            "6ba7b810-9dad-11d1-80b4-00c04fd430c8",
        "serverTimestamp": 1706789099000,
        "clientTimestamp": 1706789098700,
        "type":            1,
        "source":          "7f3e4b2a-...",   -- sender UUID (type=1 only)
        "sourceDevice":    2,                -- sender device ID (type=1 only)
        "content":         "base64-encrypted-double-ratchet-message"
      }
    ],
    "preKeyCount":   8         -- hint: upload more OPKs soon
  }

Acknowledge delivery (delete from queue):
  DELETE /v1/messages/{recipient_uuid}/{guid}
  Authorization: Basic uuid:password
  → HTTP 204 No Content
  (Message permanently deleted from queue)

Message ordering rules:
  • server_timestamp: the server's monotonically increasing receive time.
    Used for queue management only.
  • client_timestamp: the sender's self-reported send time.
    Used by the client app for display ordering (after decryption).
  • The app displays messages sorted by client_timestamp, NOT server_timestamp.
    server_timestamp can drift relative to true send order if Alice's clock
    is fast and Bob's is slow — display should use the decrypted content's
    own timestamp.
```

### Multi-Device Fanout

```
Multi-Device Message Send
═════════════════════════

  When Bob has 3 devices (device_id = 1, 2, 3), Alice must send
  a separate sealed message to each device. Each device has its own
  session (its own OPKs, its own Double Ratchet state).

  PUT /v1/messages/{bob_uuid}
  {
    "messages": [
      {"type": 52, "destinationDeviceId": 1, "destinationRegistrationId": 11111, "content": "..."},
      {"type": 52, "destinationDeviceId": 2, "destinationRegistrationId": 22222, "content": "..."},
      {"type": 52, "destinationDeviceId": 3, "destinationRegistrationId": 33333, "content": "..."}
    ],
    "timestamp": 1706789012100
  }

  Server validation:
    • Fetch all device_ids for bob_uuid from the user store
    • Check that every device in the request exists: reject unknown device_ids
    • Check that every existing device received a message
    • If any device is missing from the request:
        HTTP 409 Conflict
        {"missingDevices": [2, 3], "extraDevices": []}
    • If the request includes a device that no longer exists:
        HTTP 409 Conflict
        {"missingDevices": [], "extraDevices": [4]}

  Why this matters:
    Without strict fanout validation, a compromised server could silently
    drop messages to some of Bob's devices, causing silent data loss.
    Or Alice could "forget" one device, which would prevent synchronization.
    The 409 response gives Alice a complete picture so she can retry.
```

## Component 4: Push Gateway

### Why Pushes Must Be Minimal

Apple Push Notification Service (APNs) and Firebase Cloud Messaging (FCM)
are not end-to-end encrypted. Apple sees every APNs payload. Google sees
every FCM payload. Putting even "you have a message from Alice" in the push
payload hands metadata to these platforms.

```
Wrong approach (NEVER do this):
  {
    "aps": {
      "alert": "Alice: Hey, did you see the news about...",  ← content leak!
      "badge": 3
    }
  }

Worse approach (seen in production apps, still wrong):
  {
    "aps": {
      "alert": "New message from Alice"                      ← sender leak!
    }
  }

Correct approach (transit server style):
  {
    "aps": {
      "content-available": 1   -- silent push: wake the app in background
      -- no "alert"            -- no notification banner shown by iOS
      -- no "badge"            -- badge count is E2E-encrypted content; iOS
                               -- cannot know the right count
    }
  }
```

**How the app generates visible notifications:**
The app receives the silent push, wakes up in the background, fetches messages
from the server over TLS, decrypts them locally, and then uses the local
notification API to display a banner with the actual content. Apple and Google
never see the message content or the sender's identity — only the silent "wake
up" signal.

### Push Delivery Flow

```
APNs (iOS) Path
═══════════════

  Server               APNs                    Bob's iPhone
    │                    │                          │
    │ POST /3/device/    │                          │
    │  {apns_token}      │                          │
    │  {"aps":{"content- │                          │
    │   available":1}}   │                          │
    │ ──────────────────►│                          │
    │ ◄── HTTP 200 ──────│                          │
    │                    │ push notification         │
    │                    │ ──────────────────────── ►│
    │                    │                          │ app wakes in background
    │                    │                          │
    │                    │ Bob's app fetches:        │
    │◄──────────────────────── GET /v1/messages ─── │
    │ ────── sealed blobs ─────────────────────────►│
    │                    │                          │ app decrypts locally
    │                    │                          │ app shows: "Alice: Hey!"
    │                    │                          │ (notification generated
    │                    │                          │  locally, not from push)


FCM (Android) Path
══════════════════

  Server pushes to FCM:
  POST https://fcm.googleapis.com/fcm/send
  {
    "to": "{fcm_registration_token}",
    "data": {"action": "fetch"},   -- custom data key, NOT "notification"
    "content_available": true
    -- NOT: "notification": {"title": "...", "body": "..."} ← do not use
  }
```

### Push Token Rotation

Push tokens are not permanent. APNs rotates tokens on device restore, app
reinstall, and periodically. The server must handle stale tokens:

```
Token Lifecycle
═══════════════

Normal rotation:
  Bob reinstalls Signal → app gets new APNs token → app calls:
    PUT /v1/accounts/apns
    {"apnsRegistrationId": "new_token_here"}
  Server updates devices.push_token

Stale token detection (APNs error feedback):
  APNs returns HTTP 410 (Unregistered) when a push is sent to a stale token.
  Server action on 410:
    1. Clear push_token from devices table: UPDATE devices SET push_token=NULL
    2. Do NOT delete the account (the user may come back)
    3. The app will update the token when it next opens

FCM equivalent:
  FCM returns "NotRegistered" or "InvalidRegistration" error.
  Same server action: clear the token.

Delivery when push_token is NULL:
  The server queues the message normally.
  When Bob opens the app, he fetches messages (polling mode).
  Clients that cannot receive push notifications are marked:
    fetchesMessages = true
  The server keeps messages for these clients until they fetch.
```

## What a Server Compromise Reveals

### Full Database Dump Scenario

```
Database Dump by Attacker
═════════════════════════

  EXPOSED (attacker can read these):
  ─────────────────────────────────
  ✓ Phone number hashes
    (requires the pepper from the HSM to reverse — attacker probably
     does not have the pepper if the HSM was not compromised)
  ✓ Account UUIDs
    (random 128-bit identifiers — not useful alone)
  ✓ Device IDs and registration IDs
    (useful for targeting but not for reading messages)
  ✓ Public keys (identity keys, signed prekeys, queued OPKs)
    (these are PUBLIC by design — no harm to know them)
  ✓ Push tokens
    (can be used to send spam pushes — a nuisance, not a content breach)
  ✓ Queued sealed ciphertexts
    (unreadable without the clients' private keys, which are on devices)
  ✓ server_timestamp of queued messages
    ("Bob had an unread message at 10:47pm" — metadata, not content)
  ✓ "last_seen" timestamps
    (when Bob last used the app)

  NOT EXPOSED (attacker cannot learn these):
  ──────────────────────────────────────────
  ✗ Message content — encrypted, server never had the plaintext
  ✗ Who sent each queued message — no sender field for type=52
  ✗ Message history — delivered messages are permanently deleted
  ✗ Private keys — never transmitted to the server
  ✗ Contact relationships — no contact list stored server-side
  ✗ Past message traffic — the queue is not a history; it is a mailbox
```

### Traffic Analysis (The Residual Metadata Leak)

Even with perfect encryption, the server sees patterns. This is the hardest
problem in metadata-minimizing design:

```
Traffic Analysis Observations
══════════════════════════════

  What the server sees even in sealed-sender mode:
    • "Bob received 37 messages yesterday"
    • "Bob's last_seen was 11:03pm every night this week"
    • "Bob received a 4KB message every day at 7am"

  What an attacker can infer:
    • Bob is active and receiving communications
    • Bob may be in a relationship (nightly messages)
    • Bob may have a work context (7am daily message pattern)

  What the server does NOT see (sealed sender):
    • WHO sent those messages
    • WHAT the messages say

  Mitigations (and their trade-offs):
    Cover traffic: send fake messages to random recipients to mask real
      patterns. Expensive (wastes bandwidth), complex, impractical for
      most deployments.
    Tor / anonymous routing: app connects via Tor, server does not see
      client IP. Some Signal clients support this. Slows delivery.
    Padding: all messages padded to fixed sizes (e.g., always 1KB or
      4KB). Hides message length. Signal does this. Still reveals
      frequency patterns.
    Private set intersection / sealed sender: hides sender identity but
      not the fact that Bob received something. The best available approach
      for production systems.
```

## Forward Secrecy Under Server Compromise

### The Key Insight

Suppose an attacker records all encrypted traffic passing through your server
today. Six months later, they compromise the server's database. Can they
decrypt those recorded messages?

```
Forward Secrecy Analysis
════════════════════════

  With X3DH + Double Ratchet:
    Each session uses ephemeral keys (OPKs, ratchet keys).
    These ephemeral private keys live only in client memory.
    Once used, they are deleted from the device.
    They were NEVER sent to the server.

  What an attacker would need to decrypt recorded traffic:
    1. The clients' ephemeral private keys at the time of those sessions.
       → Impossible to obtain retroactively (deleted after use)
    2. OR: a MITM on the key exchange at the exact time the session
       was established.
       → Prevented by SPK signature verification (the server cannot
         forge a signature without the private signing key)

  Therefore: recording encrypted traffic today, then compromising the
  server database in 6 months, yields unreadable ciphertexts.
  This is the definition of Perfect Forward Secrecy (PFS).
```

### Compromise Scenario Matrix

```
┌──────────────────────────────────────────────────────────────────────────┐
│ Compromise Scenario           │ Message content?    │ Who talks to whom? │
├───────────────────────────────┼─────────────────────┼────────────────────┤
│ Server DB dump only           │ NO — E2E encrypted  │ PARTIAL: recipient │
│ (no MITM, no device seizure)  │                     │ known, sender not  │
│                               │                     │ known (sealed sndr)│
├───────────────────────────────┼─────────────────────┼────────────────────┤
│ DB dump + traffic recording   │ NO — forward        │ PARTIAL            │
│ (no MITM on key exchange)     │ secrecy prevents    │ (sealed sender     │
│                               │ decryption          │ hides sender)      │
├───────────────────────────────┼─────────────────────┼────────────────────┤
│ DB dump + traffic recording   │ YES — catastrophic  │ YES                │
│ + MITM on key exchange at     │                     │                    │
│ session setup time            │                     │                    │
├───────────────────────────────┼─────────────────────┼────────────────────┤
│ Client device seized,         │ YES — plaintext in  │ YES                │
│ phone unlocked                │ app's message store │                    │
├───────────────────────────────┼─────────────────────┼────────────────────┤
│ Sealed sender mode only       │ NO                  │ Recipient only     │
│ + DB dump                     │                     │ Sender unknown     │
├───────────────────────────────┼─────────────────────┼────────────────────┤
│ Server substitutes fake       │ NO — user is        │ YES, attack is     │
│ prekey (MITM attempt)         │ alerted via safety  │ detectable via     │
│                               │ number mismatch     │ safety numbers     │
└──────────────────────────────────────────────────────────────────────────┘
```

**Safety numbers** (Signal's term for key fingerprints): Alice and Bob can
compare a short numeric fingerprint derived from each other's identity keys.
If the server substituted a fake key, the fingerprints will not match and both
parties see a warning. This is why verifying safety numbers out-of-band is
important for high-security conversations.

## Full API Surface

The transit server exposes a REST API over HTTPS/2. All authenticated endpoints
use HTTP Basic Auth (UUID:password).

```
Account Endpoints
═════════════════

  PUT  /v1/accounts/sms/code/{phone}
    Request SMS OTP to the given E.164 phone number.
    Rate-limited: 3 attempts per phone per hour.

  PUT  /v1/accounts/code/{verification_code}
    Verify OTP and create account.
    Body: accountAttributes + identityKey + signingKey + password + preKeys + signedPreKey
    Response: {"uuid": "...", "storageCapable": true}

  GET  /v1/accounts/me                         [auth required]
    Fetch own account information.
    Response: {uuid, number (hashed), identityKey, devices: [...]}

  PUT  /v1/accounts/gcm                        [auth required]
    Update FCM push token.
    Body: {"gcmRegistrationId": "..."}

  PUT  /v1/accounts/apns                       [auth required]
    Update APNs push token.
    Body: {"apnsRegistrationId": "...", "voipRegistrationId": "..."}

  DELETE /v1/accounts/me                       [auth required]
    Delete account. Queues all pending messages for deletion.
    Schedules full data purge in 30 days (to handle re-registration
    timing edge cases). Immediately invalidates auth credentials.


Prekey Endpoints
════════════════

  PUT  /v2/keys                                [auth required]
    Upload one-time prekeys + signed prekey.
    Body: {"preKeys": [...], "signedPreKey": {...}}
    Server validates SPK signature before storing.

  GET  /v2/keys/{uuid}/{device_id}             [auth required]
    Fetch prekey bundle for a specific device.
    Atomically claims one OPK (deletes from server).
    If device_id is "*": returns bundles for all devices.

  GET  /v2/keys                                [auth required]
    Fetch own OPK count.
    Response: {"count": 42}


Message Endpoints
═════════════════

  PUT  /v1/messages/{recipient_uuid}           [auth required]
    Send message(s) to a recipient (all devices in one request).
    Body: see "Send message request body" below.
    Response 200: {"needsSync": false}
    Response 409: {"missingDevices": [...], "extraDevices": [...]}
    Response 410: recipient deleted their account
    Response 413: envelope too large (> 64KB)
    Response 429: rate limit exceeded

  GET  /v1/messages                            [auth required]
    Fetch queued messages for the authenticated account+device.
    Response: {"messages": [...], "preKeyCount": N}

  DELETE /v1/messages/{account_uuid}/{guid}    [auth required]
    Acknowledge delivery of a specific message. Permanently deleted.
    Response: 204 No Content


Profile Endpoints (optional extension)
═══════════════════════════════════════

  PUT  /v1/profile                             [auth required]
    Upload encrypted profile (name, avatar).
    The payload is encrypted with the profile key (a symmetric key
    the user shares only with their contacts — the server holds
    only the ciphertext blob, never the plaintext).

  GET  /v1/profile/{uuid}                      [auth required]
    Fetch encrypted profile blob.
    Response: {"name": "base64-encrypted", "avatar": "url-to-blob"}


Contact Discovery Endpoints
════════════════════════════

  PUT  /v1/discovery/registrations             [auth required]
    SGX-enclave-based contact discovery.
    Sends hashed phone numbers to an enclave that computes the
    intersection with registered users. The server operator cannot
    read the query (it executes inside the enclave).

  GET  /v1/discovery/attestation               [no auth]
    Fetch SGX remote attestation report.
    Allows clients to verify the enclave's code hash before sending
    any contact data to it.
```

### Send Message Request Body

```
PUT /v1/messages/{recipient_uuid}
Authorization: Basic alice_uuid:alice_password
Content-Type: application/json

{
  "messages": [
    {
      "type":                      52,         -- UNIDENTIFIED_SENDER
      "destinationDeviceId":       1,           -- Bob's device 1
      "destinationRegistrationId": 12345,       -- Bob's device 1 registration ID
                                               -- (prevents replaying to wrong device)
      "content":                   "base64-encoded-sealed-envelope"
    },
    {
      "type":                      52,
      "destinationDeviceId":       2,           -- Bob's device 2
      "destinationRegistrationId": 22222,
      "content":                   "base64-encoded-sealed-envelope-device2"
    }
  ],
  "timestamp": 1706789012100                   -- client-side timestamp (Unix ms)
                                               -- included in sealed content too;
                                               -- this outer field is for queue ordering
}
```

## Rate Limiting and Spam Prevention

### The Sealed-Sender Dilemma

Sealed sender hides the sender identity from the message queue, but the
server still authenticates the HTTP connection. Rate limiting operates at
the authentication layer:

```
Rate Limiting Architecture
══════════════════════════

  Layer 1: HTTP Authentication Rate Limit
    The server knows who made the HTTP request (Alice's UUID, from Basic Auth).
    Even in sealed-sender mode, the server can rate-limit per sender account
    at the network boundary — before any message content is examined.

    Limits (example values):
      250 messages per 10 minutes per account
      1,000 messages per 24 hours per account
      10 message burst per second (token bucket)

  Layer 2: Envelope Size Limit
    Maximum 64KB per envelope.
    Prevents flooding by volume even within rate limits.

  Layer 3: Device Count Limit
    Maximum 5 linked devices per account.
    Bounds the fanout cost per message send.

  Layer 4: Prekey Upload Limit
    Maximum 100 OPKs per upload batch.
    Maximum 1,000 OPKs stored per device.
    Prevents a malicious client from filling the database.
```

### Zero-Knowledge Rate Limiting (Advanced)

For deployments where even the server-level authentication link is
considered too much:

```
ZKSK (Zero-Knowledge Sender Key) Approach
══════════════════════════════════════════

  1. Each account receives a server-issued credential (a Ristretto group
     element + Schnorr proof) valid for 24 hours.

  2. Each message carry a zero-knowledge proof that the sender holds a
     valid credential, without revealing WHICH account they are.

  3. The credential is rate-limited (daily quota encoded in the credential).
     The server verifies the ZK proof without learning sender identity.

  4. Abusers: their credentials expire after 24 hours and they cannot
     obtain new ones (account is suspended at credential issuance time,
     not at message send time).

  Implementation: Signal's zkgroup library (Ristretto25519, Schnorr proofs)
  Status: research/advanced — most transit server deployments use Layer 1.
```

## Storage Minimization Patterns

### Message Retention

```
Message Retention Policy
════════════════════════

  Delivered messages:   DELETE immediately upon receipt of the
                        DELETE /v1/messages/{uuid}/{guid} acknowledgment.
                        No backup. No archive. Gone.

  Undelivered messages: Retained for up to 30 days.
                        After 30 days: expired messages deleted by a
                        background job (TTL sweep).
                        The recipient is notified that some messages
                        may have been dropped (via a special "missed
                        messages" control message) but the content
                        is not recoverable.

  Why 30 days?
    If Bob loses his phone, he has 30 days to restore from backup
    or set up a new device before messages expire. This is a balance
    between user experience and storage minimization.
```

### Log Minimization

```
What to log (operational health only):
  ✓ HTTP status codes (200, 409, 413, 429...)
  ✓ Request latency (p50, p99 milliseconds)
  ✓ Error rates by endpoint
  ✓ Queue depth (how many messages are pending)
  ✓ Push success/failure rates (APNs 200 vs 410 counts)

What to NEVER log:
  ✗ Message content (obviously)
  ✗ Recipient UUID at the application log level
    (can appear in structured access logs briefly — rotate daily)
  ✗ Sender UUID in sealed-sender mode
  ✗ Message size (leaks content characteristics)
  ✗ IP addresses beyond the minimum for abuse detection
    (log as /24 prefix, not full address; delete after 7 days)

Access log retention:
  Raw access logs: delete after 7 days
  Aggregated metrics (no per-request detail): retain indefinitely
```

### Data Retention Policy Summary

```
Data Retention Summary
══════════════════════

  Account data:        Retained until account deletion + 30 days grace
                       (grace period handles re-registration edge cases)
  Prekeys (OPKs):     Deleted the moment they are claimed by a sender
  Prekeys (SPKs):     Deleted when rotated + grace period for active sessions
  Message queue:       30 days TTL; deleted immediately on acknowledgment
  Push tokens:         Deleted on account deletion or APNs/FCM 410 response
  Phone number hash:   Deleted 30 days after account deletion
  Access logs:         7 days maximum
  Aggregated metrics:  Retained indefinitely (no PII)
```

## Algorithms

### Full Message Send (Client Pseudocode)

```
client_send_message(recipient_uuid, plaintext_bytes):
  # Step 1: Fetch recipient's prekey bundles (all devices)
  bundles = GET /v2/keys/{recipient_uuid}/*
  # bundles = [{"deviceId": 1, "identityKey": ..., "preKey": ..., "signedPreKey": ...}, ...]

  # Step 2: For each device, build a sealed message
  sealed_messages = []
  for bundle in bundles:

    # Step 3: Verify the signed prekey's signature
    ok = Ed25519_Verify(
           public_key = bundle.identityKey,
           message    = bundle.signedPreKey.publicKey,
           signature  = bundle.signedPreKey.signature
         )
    if not ok:
      ABORT("Prekey signature verification failed — possible MITM")

    # Step 4: Establish or load a Double Ratchet session
    session_key = (recipient_uuid, bundle.deviceId)
    if session_store.has(session_key):
      session = session_store.load(session_key)
    else:
      # Run X3DH to derive the initial shared secret SK
      if bundle.preKey exists:
        # Full X3DH with OPK
        SK = x3dh_send(
               my_identity_key      = self.identity_key,
               my_ephemeral_key     = generate_ephemeral_keypair(),
               recipient_identity   = bundle.identityKey,
               recipient_signed_pk  = bundle.signedPreKey.publicKey,
               recipient_one_time_pk = bundle.preKey.publicKey
             )
      else:
        # Reduced X3DH without OPK (OPKs exhausted)
        SK = x3dh_send_no_opk(
               my_identity_key      = self.identity_key,
               my_ephemeral_key     = generate_ephemeral_keypair(),
               recipient_identity   = bundle.identityKey,
               recipient_signed_pk  = bundle.signedPreKey.publicKey
             )
      session = double_ratchet_initialize_sender(SK, bundle.signedPreKey.publicKey)
      session_store.save(session_key, session)

    # Step 5: Encrypt with Double Ratchet
    header, ciphertext = double_ratchet_encrypt(session, plaintext_bytes)
    session_store.save(session_key, session)  # save updated ratchet state

    # Step 6: Build sealed sender envelope
    # The envelope hides Alice's identity from the server's message queue
    sender_certificate = self.fetch_or_cache_sender_certificate()
    # sender_certificate is an ephemeral credential signed by the server,
    # proving Alice is a valid registered user (without revealing her UUID
    # to Bob's message queue row — Bob's app decrypts and sees Alice's
    # identity key inside the sealed layer)
    sealed_envelope = seal_sender_encrypt(
      inner_message    = (header, ciphertext),
      sender_cert      = sender_certificate,
      recipient_pub_key = bundle.identityKey
    )

    sealed_messages.append({
      "type":                      52,
      "destinationDeviceId":       bundle.deviceId,
      "destinationRegistrationId": bundle.registrationId,
      "content":                   base64(sealed_envelope)
    })

  # Step 7: Send all device messages in one HTTP request
  response = PUT /v1/messages/{recipient_uuid}, {
    "messages":  sealed_messages,
    "timestamp": current_time_ms()
  }

  if response.status == 409:
    # Device list changed — update local device list and retry
    update_device_list(recipient_uuid, response.missingDevices, response.extraDevices)
    retry client_send_message(recipient_uuid, plaintext_bytes)
```

### Server Receive (Server Pseudocode)

```
server_receive_message(http_request):
  # Step 1: Authenticate the sender
  sender = basic_auth_verify(http_request.authorization)
  if sender is None:
    return HTTP 401 Unauthorized

  # Step 2: Rate limit check (per sender UUID)
  if rate_limiter.is_exceeded(sender.uuid):
    return HTTP 429 Too Many Requests
  rate_limiter.record(sender.uuid)

  # Step 3: Validate recipient exists
  recipient_uuid = http_request.path_param("uuid")
  recipient = account_store.lookup(recipient_uuid)
  if recipient is None:
    return HTTP 404 Not Found

  # Step 4: Validate the device list matches exactly
  request_device_ids = {msg.destinationDeviceId for msg in request.messages}
  known_device_ids   = {d.device_id for d in recipient.devices}

  missing = known_device_ids - request_device_ids    # devices we needed but sender omitted
  extra   = request_device_ids - known_device_ids    # devices sender included that don't exist

  if missing or extra:
    return HTTP 409 Conflict, {"missingDevices": list(missing), "extraDevices": list(extra)}

  # Step 5: Validate each envelope
  for msg in request.messages:
    if len(base64_decode(msg.content)) > 65536:
      return HTTP 413 Request Entity Too Large

    # Verify destinationRegistrationId matches (prevents replay to wrong device)
    device = recipient.device_by_id(msg.destinationDeviceId)
    if device.registration_id != msg.destinationRegistrationId:
      return HTTP 410 Gone  # device was re-registered

  # Step 6: Store in message queue
  # NOTE: the server stores content WITHOUT reading it.
  for msg in request.messages:
    message_queue.insert({
      "recipient_uuid":   recipient_uuid,
      "recipient_device": msg.destinationDeviceId,
      "guid":             generate_uuid(),
      "server_timestamp": current_time_ms(),
      "client_timestamp": request.body.timestamp,
      "type":             msg.type,
      "content":          msg.content,     # sealed ciphertext — server blind
      "expires_at":       now() + 30_days
      # NO sender_uuid (type=52 sealed sender)
    })

  # Step 7: Send silent push notification to all of recipient's devices
  for device in recipient.devices:
    if device.push_token is not None:
      push_gateway.send_silent(device.push_token, device.push_type)

  needs_sync = check_if_sender_needs_sync(sender, recipient_uuid)
  return HTTP 200 OK, {"needsSync": needs_sync}
```

### Server Deliver (Fetch Pseudocode)

```
server_fetch_messages(http_request):
  # Step 1: Authenticate
  account = basic_auth_verify(http_request.authorization)
  if account is None:
    return HTTP 401

  device_id = http_request.device_id or 1

  # Step 2: Fetch queued messages for this (account, device) pair
  messages = message_queue.fetch(
    recipient_uuid   = account.uuid,
    recipient_device = device_id,
    limit            = 100           # max messages per fetch
  )

  # Step 3: Format for response
  # NOTE: content is returned as-is (sealed ciphertext).
  # The server never sees plaintext.
  formatted = []
  for msg in messages:
    entry = {
      "guid":            msg.guid,
      "serverTimestamp": msg.server_timestamp,
      "clientTimestamp": msg.client_timestamp,
      "type":            msg.type,
      "content":         msg.content
    }
    if msg.type != 52:  # not sealed sender — include sender info
      entry["source"]       = msg.sender_uuid
      entry["sourceDevice"] = msg.sender_device_id
    formatted.append(entry)

  # Step 4: Count OPKs remaining (hint for replenishment)
  opk_count = prekey_store.count(account.uuid, device_id)

  return HTTP 200 OK, {"messages": formatted, "preKeyCount": opk_count}
```

## Test Strategy

### Unit Tests — Account Service

```
Test 1: Valid registration
  Input:  phone="+15551234567", valid OTP, identity_key (32 bytes),
          signing_key (32 bytes), password (64 chars), initial prekeys
  Expect: HTTP 200, response contains "uuid" (valid UUID format),
          accounts table has one row with identity_key stored,
          phone_hash stored (not plaintext phone),
          password NOT stored plaintext (SHA-256 hash stored instead)

Test 2: Duplicate registration (same phone)
  Input:  register phone A, then register phone A again with new keys
  Expect: HTTP 200 both times. Second registration updates the account
          (new identity key, new devices). Old session data invalidated.

Test 3: Authentication — correct credentials
  Input:  GET /v1/accounts/me with valid UUID:password Basic Auth
  Expect: HTTP 200, returns account info

Test 4: Authentication — wrong password
  Input:  GET /v1/accounts/me with valid UUID but wrong password
  Expect: HTTP 401 Unauthorized

Test 5: Authentication — nonexistent UUID
  Input:  GET /v1/accounts/me with random UUID:password
  Expect: HTTP 401 (same response as wrong password — prevents UUID enumeration)

Test 6: Device registration limit
  Input:  Link 5 devices, then attempt to link a 6th
  Expect: First 5 succeed. 6th returns HTTP 411 (or 400) indicating
          device limit reached.

Test 7: Push token update — APNs
  Input:  PUT /v1/accounts/apns with new APNs token
  Expect: HTTP 200, devices.push_token updated in database

Test 8: Push token cleared on APNs 410
  Input:  Server sends push, APNs returns 410 Unregistered
  Expect: devices.push_token set to NULL for that device

Test 9: Account deletion
  Input:  DELETE /v1/accounts/me
  Expect: HTTP 200, subsequent auth attempts fail (401),
          all queued messages for this account are marked for deletion
```

### Unit Tests — Prekey Service

```
Test 10: Upload 100 OPKs
  Input:  PUT /v2/keys with 100 OPKs + valid SPK (with correct signature)
  Expect: HTTP 200, one_time_prekeys has 100 rows for this device,
          signed_prekeys has 1 row with the SPK

Test 11: OPK claim-and-delete (atomic)
  Setup:  Upload 100 OPKs
  Input:  GET /v2/keys/{uuid}/1 (fetch prekey bundle for device 1)
  Expect: Response includes "preKey" field with one OPK.
          After fetch: one_time_prekeys has 99 rows (the claimed OPK is gone).
          Second fetch: a different OPK is returned (not the same one).

Test 12: OPK count endpoint
  Setup:  Upload 100 OPKs, then claim 80 (via 80 fetches)
  Input:  GET /v2/keys
  Expect: {"count": 20}

Test 13: SPK signature validation — tampered key rejected
  Input:  PUT /v2/keys with a signedPreKey where the signature is wrong
          (e.g., flip one bit in the signature)
  Expect: HTTP 400 Bad Request. No OPKs stored. No SPK stored.
          The entire batch is rejected.

Test 14: OPK exhaustion — SPK-only response
  Setup:  Claim all 100 OPKs (100 fetches)
  Input:  GET /v2/keys/{uuid}/1
  Expect: HTTP 200, response has "signedPreKey" but NO "preKey" field.
          (Server does not 404 — just omits the OPK field.)

Test 15: Private key field rejected
  Input:  PUT /v2/keys with a request body containing "privateKey": "..."
  Expect: HTTP 400 Bad Request. Nothing stored.

Test 16: SPK rotation
  Input:  Upload SPK with keyId=5. Then upload SPK with keyId=6.
  Expect: signed_prekeys table has only keyId=6 (old one replaced).
          OPKs are unchanged.
```

### Unit Tests — Message Service

```
Test 17: Send to valid recipient, single device
  Input:  PUT /v1/messages/{bob_uuid} with one message (type=52, device=1)
  Expect: HTTP 200 {"needsSync": false}
          message_queue has one row for (bob_uuid, device=1)
          Push notification sent to Bob's push token

Test 18: Send with missing device
  Setup:  Bob has 3 devices (1, 2, 3)
  Input:  PUT /v1/messages/{bob_uuid} with messages for devices 1 and 2 only
  Expect: HTTP 409 {"missingDevices": [3], "extraDevices": []}

Test 19: Send with extra device
  Setup:  Bob has 2 devices (1, 2). Alice sends to device 1, 2, and 4.
  Expect: HTTP 409 {"missingDevices": [], "extraDevices": [4]}

Test 20: Sealed sender — no sender in queue
  Input:  PUT /v1/messages/{bob_uuid} with type=52 message
  Expect: message_queue row has NO sender_uuid field populated.
          Only recipient_uuid, type, content, timestamps are set.

Test 21: Message too large
  Input:  PUT /v1/messages/{bob_uuid} with content of 65537 bytes
  Expect: HTTP 413 Request Entity Too Large. Nothing stored.

Test 22: Fetch messages
  Setup:  3 messages queued for Bob, device 1
  Input:  GET /v1/messages (as Bob, device 1)
  Expect: HTTP 200, response has 3 messages. Type=52 messages have
          no "source" or "sourceDevice" fields.

Test 23: Delete (acknowledge) message
  Setup:  1 message queued with known guid
  Input:  DELETE /v1/messages/{bob_uuid}/{guid}
  Expect: HTTP 204. message_queue has 0 rows for that guid.
          A second DELETE for the same guid returns HTTP 404.

Test 24: TTL expiry
  Setup:  Insert a message with expires_at = now() - 1 second
          (simulate an expired message via direct DB insert in test)
  Input:  GET /v1/messages (as Bob)
  Expect: Expired message NOT returned in response.
          Background sweep has deleted it (or it is filtered at query time).

Test 25: Registration ID mismatch (device re-registered)
  Setup:  Bob's device 1 has registration_id = 12345.
          Alice sends with destinationRegistrationId = 99999.
  Expect: HTTP 410 Gone (the device was re-registered; Alice must refresh).
```

### Integration Tests

```
Test 26: Full round-trip (happy path)
  Steps:
    1. Alice registers → uploads 100 OPKs + SPK
    2. Bob registers → uploads 100 OPKs + SPK
    3. Alice fetches Bob's prekey bundle
    4. Alice verifies SPK signature (succeeds)
    5. Alice runs X3DH, initializes Double Ratchet, encrypts "Hello Bob"
    6. Alice seals sender envelope
    7. Alice sends to Bob → server returns 200
    8. Bob receives silent push notification
    9. Bob fetches messages → receives 1 sealed ciphertext
    10. Bob unseals the envelope (learns Alice's identity from sender cert)
    11. Bob decrypts with Double Ratchet → plaintext = "Hello Bob"
    12. Bob sends DELETE → server removes from queue
    13. Bob fetches again → 0 messages
  Assertions at each step: HTTP status, database state, OPK count decrements

Test 27: Multi-device round-trip
  Setup:  Bob has 3 linked devices (device_id = 1, 2, 3)
  Steps:
    1. Alice fetches all 3 device bundles for Bob (GET /v2/keys/bob/*)
    2. Alice sends 3 separate sealed messages (one per device)
    3. 3 push notifications sent (one per device)
    4. Each of Bob's 3 devices fetches and gets its own message
    5. All 3 acknowledge delivery → queue empty
  Assert: Each device_id received a different ciphertext
          (different sessions, different ephemeral keys)

Test 28: OPK replenishment cycle
  Setup:  Bob uploads exactly 10 OPKs
  Steps:
    1. Send 10 messages to Bob (10 senders, each claims one OPK)
    2. Fetch Bob's OPK count → 0
    3. 11th fetch: response has no "preKey" field (OPK exhausted)
    4. GET /v1/messages as Bob → response includes "preKeyCount": 0
    5. Bob uploads 100 more OPKs
    6. 12th sender fetches Bob's bundle → has "preKey" again
  Assert: System degrades gracefully at OPK=0, recovers on upload

Test 29: Message expiry under 30-day TTL
  Setup:  Insert message with expires_at = now() (already expired)
  Steps:
    1. Run TTL sweep job
    2. GET /v1/messages as Bob
  Assert: Expired message not returned

Test 30: Server compromise simulation
  Steps:
    1. Alice and Bob exchange 10 messages (full E2E flow)
    2. Dump the message_queue table after all messages are queued
       (before Bob fetches)
    3. Attempt to decrypt each content field using only server-side data
       (server's private keys if any, the database, the code)
  Assert: Every content field is an opaque byte blob. No server-side
          key material exists that would allow decryption.
          The server has NO private keys that participate in X3DH or
          the Double Ratchet.
```

### Security Tests

```
Test 31: Prekey MITM — fake key injection
  Setup:  Replace Bob's signedPreKey.publicKey with an attacker's key
          BUT keep Bob's real signature (signature is over the real key,
          so it will fail to verify over the fake key)
  Steps:
    1. Alice fetches the tampered prekey bundle
    2. Alice runs Ed25519-Verify on the bundle
  Assert: Verification fails. Alice aborts with an error.
          No session is established. No message is sent.
          The attack is detected before any plaintext is exposed.

Test 32: Replay attack (resend a delivered message)
  Steps:
    1. Alice sends message M1 to Bob → guid=G1
    2. Bob fetches and acknowledges M1 (DELETE /v1/messages/bob/G1)
    3. Attacker resends the exact same PUT /v1/messages/bob request
  Assert: Server stores a new queue entry (guid is generated server-side,
          different from G1). Bob's app sees the duplicate and deduplicates
          by the message content's own sequence number (from the Double
          Ratchet). The Double Ratchet's anti-replay mechanism rejects
          replayed ciphertexts.

Test 33: Rate limiting
  Steps:
    1. Send 250 messages from Alice within 10 minutes
    2. Send 251st message from Alice
  Assert: First 250 return HTTP 200. 251st returns HTTP 429.
          Rate limit resets after the window expires.

Test 34: Oversized envelope
  Input:  Send a content field of exactly 65536 bytes (within limit)
  Assert: HTTP 200
  Input:  Send a content field of exactly 65537 bytes
  Assert: HTTP 413

Test 35: Account enumeration prevention
  Steps:
    1. GET /v2/keys/{nonexistent_uuid}/1
  Assert: HTTP 404 response is identical in timing and body to the
          response when UUID exists but has no OPKs. Attacker cannot
          distinguish "account does not exist" from "account exists
          but has exhausted OPKs."

Test 36: Authentication timing attack prevention
  Steps:
    1. POST authentication request with wrong password (timing measured)
    2. POST authentication request with wrong UUID (timing measured)
  Assert: Response times are statistically indistinguishable.
          The comparison is constant-time (hmac_compare rather than ==).

Test 37: Sealed sender — server cannot determine sender
  Steps:
    1. Alice sends type=52 message to Bob
    2. Inspect message_queue database row
  Assert: Row has no sender_uuid field. The "source" of the message
          is unknowable from the database alone without decrypting
          Alice's sealed envelope (which requires Bob's private key).

Test 38: SPK with private key field rejected
  Input:  PUT /v2/keys request body that includes a "privateKey" field
          anywhere in the JSON (even if it is null or empty)
  Assert: HTTP 400. Server logs a security alert. Nothing is stored.
          The key material is not echoed back in the response.
```

### Coverage Target

Target 90%+ line coverage across all components. Every branch of the device
validation logic (missing devices, extra devices, registration ID mismatch),
every push notification path (APNs, FCM, null token), every prekey state
(normal OPK claim, OPK exhausted, SPK rotation), and every error response
code (400, 401, 404, 409, 410, 413, 429) must be exercised by tests.

Security tests (31–38) are not optional — they verify the threat model
properties that motivate the entire architecture.

## Future Extensions

**Sealed Sender Sender Certificates:** The sender certificate (embedded inside
the sealed envelope) has an expiry. Implementing certificate rotation and
revocation prevents a stolen certificate from being replayed indefinitely.

**Multi-Party Sessions (Groups):** Group messages use SenderKey, where Alice
distributes a symmetric "sender key" to all group members via individual
sealed-sender messages. The transit server never learns group membership — it
just sees individual sealed messages to individual recipients.

**Message Storage Keys:** For users who want server-side backup of their
message history, the backup can be encrypted with a key derived from the
user's 30-digit numeric "backup passphrase" — the server stores an opaque
encrypted blob, never the decryption key.

**SGX Contact Discovery:** Contact discovery via SGX remote attestation
prevents the server from learning a user's full contact list during the
"who is on this service?" query. The attestation report proves the enclave's
code hash before any data is sent into it.

**Server-Side Key Transparency:** A transparency log (similar to Certificate
Transparency) can publish a cryptographically verifiable record of all
identity key changes. Users can query the log to detect if their contact's
key was silently replaced — even without comparing safety numbers in person.
