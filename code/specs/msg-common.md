# MSG-COMMON — Messaging Protocol Patterns and Tradeoffs

## Overview

Messaging is one of the most-studied protocol domains in computing. Eight major
systems — SMS, MMS, RCS, Signal, MTProto (Telegram), XMPP/WhatsApp, iMessage,
and Matrix — have each solved the same fundamental problems of moving text
between humans, and each has made different tradeoffs. By studying these
systems side-by-side, you learn not just how they work, but WHY they were
designed the way they were and what each system had to give up to get what it
got.

**Analogy:** The history of messaging protocols is like the history of
transportation. Walking (SMS) is universal and works everywhere. The car
(iMessage) is fast and comfortable but requires infrastructure. The train
(XMPP/federated systems) is efficient for large groups but only runs on fixed
tracks. The airplane (Signal) is the fastest but requires airports. Every
option serves a real need; none is strictly better. The interesting engineering
is in the tradeoffs.

This document surveys the ten fundamental problems every messaging system must
solve, analyzes how each protocol addresses each problem, and extracts patterns
and design lessons. It is intended as a companion to the individual protocol
specs (msg-sms, msg-mms, msg-rcs, msg-signal, msg-mtproto, msg-xmpp,
msg-imessage, msg-matrix).

## The Eight Protocols

```
Protocol Overview
═════════════════

  SMS (Short Message Service)
    Year: 1992    Carrier: Cellular networks (GSM/CDMA/LTE)
    Identity: E.164 phone number
    Transport: SS7 / SMSC store-and-forward
    E2E Encryption: None
    Max message: 160 chars (7-bit GSM) or 70 chars (UCS-2)
    Standard: 3GPP TS 23.040

  MMS (Multimedia Messaging Service)
    Year: 2002    Carrier: Cellular networks
    Identity: E.164 phone number
    Transport: WAP push + HTTP pull from MMSC
    E2E Encryption: None
    Content: SMIL presentation + media parts
    Standard: 3GPP TS 23.140

  RCS (Rich Communication Services)
    Year: 2007 (GSMA Universal Profile: 2016)
    Carrier: Cellular carriers (IMS infrastructure)
    Identity: SIP URI (tel:+1234567890 or sip:user@domain)
    Transport: SIP/MSRP over LTE/IMS
    E2E Encryption: Optional (GSMA spec; not universally deployed)
    Standard: GSMA RCC.07 (Universal Profile)

  Signal
    Year: 2014 (as Signal; 2010 as TextSecure)
    Operator: Signal Foundation (nonprofit)
    Identity: Phone number → Signal ID (migrating to usernames)
    Transport: HTTPS to Signal servers; APNs/FCM for wakeup
    E2E Encryption: Yes (Signal Protocol, mandatory)
    Standard: Open Signal Protocol spec; app is open source

  Telegram (MTProto)
    Year: 2013
    Operator: Telegram FZ-LLC
    Identity: Phone number → Telegram user ID
    Transport: MTProto custom binary protocol over TCP/HTTPS
    E2E Encryption: Cloud chats: no E2E. Secret chats: E2E (MTProto 2.0)
    Standard: Proprietary (MTProto), partially documented

  WhatsApp
    Year: 2009 (E2E added 2016)
    Operator: Meta Platforms
    Identity: Phone number
    Transport: HTTPS + XMPP-based proprietary protocol
    E2E Encryption: Yes (Signal Protocol, mandatory)
    Standard: Proprietary; uses Signal Protocol underneath

  iMessage
    Year: 2011
    Operator: Apple
    Identity: Apple ID (email) or phone number
    Transport: APNs for delivery; HTTPS for IDS and media
    E2E Encryption: Yes (IDS-based; Apple controls key directory)
    Standard: Proprietary

  Matrix
    Year: 2014 (v1.0 stable: 2022)
    Operator: Matrix.org Foundation (and self-hosters)
    Identity: @username:homeserver (e.g., @alice:matrix.org)
    Transport: HTTPS REST; federation API; long-poll /sync
    E2E Encryption: Optional (Olm/Megolm, based on Signal Protocol)
    Standard: Open Matrix Specification (spec.matrix.org); open source
```

## Problem 1: Identity

Every messaging system must answer: what is a "user"? How do you address
someone? How do you verify they are who they say they are?

```
Identity Models Across Systems
══════════════════════════════

  ┌──────────────┬───────────────────────────────┬──────────────────────────┐
  │ System       │ Identity Type                 │ Verification             │
  ├──────────────┼───────────────────────────────┼──────────────────────────┤
  │ SMS          │ E.164 phone number            │ Carrier SIM assignment   │
  │              │ Example: +14155552671         │ (government-linked in    │
  │              │                               │  most countries)          │
  ├──────────────┼───────────────────────────────┼──────────────────────────┤
  │ MMS          │ E.164 phone number            │ Same as SMS              │
  ├──────────────┼───────────────────────────────┼──────────────────────────┤
  │ RCS          │ SIP URI                       │ Carrier IMS registration  │
  │              │ tel:+14155552671              │ Links to phone number    │
  │              │ (or email-style SIP)          │ in practice              │
  ├──────────────┼───────────────────────────────┼──────────────────────────┤
  │ Signal       │ Phone number (primary)        │ SMS/voice OTP at signup  │
  │              │ + Signal username (new)       │ Phone verified by Signal │
  │              │ Example: @alice.01            │                          │
  ├──────────────┼───────────────────────────────┼──────────────────────────┤
  │ Telegram     │ Phone number at signup        │ SMS/voice OTP at signup  │
  │              │ then: Telegram user ID        │ Phone number can be      │
  │              │ + optional username           │ hidden from contacts     │
  ├──────────────┼───────────────────────────────┼──────────────────────────┤
  │ WhatsApp     │ Phone number (immutable)      │ SMS/voice OTP at signup  │
  │              │ Example: +14155552671         │ Cannot separate from     │
  │              │                               │ phone number             │
  ├──────────────┼───────────────────────────────┼──────────────────────────┤
  │ iMessage     │ Apple ID (email)              │ Apple account creation   │
  │              │ or phone number               │ Email/SMS verification   │
  │              │ or both (linked)              │ by Apple                 │
  ├──────────────┼───────────────────────────────┼──────────────────────────┤
  │ Matrix       │ @username:homeserver          │ Homeserver-dependent     │
  │              │ Example: @alice:matrix.org    │ (email, SSO, no verify)  │
  │              │ No phone number required      │ User controls identity   │
  └──────────────┴───────────────────────────────┴──────────────────────────┘
```

### The Phone Number Identity Problem

Using phone numbers as identities creates an immediate tension: they are easy
to verify (via SMS OTP) but they are not private. When you message someone on
WhatsApp, you must give them your phone number. Your number is tied to your
real identity via your carrier, your government, and your location history.

**Analogy:** Using a phone number as your messaging identity is like using your
home address as your username. Easy to look up, hard to change, impossible to
separate from your physical identity.

**Design fork:** Signal recognizes this and is migrating to self-chosen
usernames (@alice.01) as the primary contact method, so you can message
someone without sharing your phone number. Matrix takes this further: there is
no phone number requirement at all. The tradeoff is verification difficulty:
anyone can claim to be @alice:some-sketchy-server.

```
Identity Tradeoff Space
═══════════════════════

  High Verification                              Low Verification
  ◄──────────────────────────────────────────────────────────────►
  SMS/WhatsApp    Signal (current)    Telegram    Matrix
  (phone = you)   (phone-verified)    (username)  (self-asserted)

  Low Privacy                                    High Privacy
  ◄──────────────────────────────────────────────────────────────►
  SMS/WhatsApp    iMessage            Telegram    Signal/Matrix
  (phone exposed) (Apple ID)         (hideable)  (username possible)

  High Portability                               Low Portability
  ◄──────────────────────────────────────────────────────────────►
  Matrix          Telegram            Signal      SMS/WhatsApp
  (migrate server)(username survives) (migrating) (tied to SIM)
```

### Federated vs Centralized Identity

Phone-number systems and centralized apps (Signal, WhatsApp, iMessage, Telegram)
all rely on one authority to say "this phone number belongs to this account."
XMPP and Matrix allow self-hosted identities.

**The tradeoff in practice:**
- Centralized: easy to verify, easy to look up, provider can block you
- Federated: you control your identity, but harder to discover new contacts

## Problem 2: Key Distribution

End-to-end encryption requires that Alice have Bob's public key before she
can encrypt anything to him. Getting that key securely — without anyone in the
middle being able to substitute a different key — is one of the hardest
problems in applied cryptography.

**Analogy:** Imagine you want to send Bob a locked box. You need his padlock
(public key) to lock it. But the only way to get his padlock is to ask a store
(the key server). What stops the store from giving you a fake padlock that
the store can also open? This is the Man-in-the-Middle (MITM) problem.

```
Key Distribution Strategies
════════════════════════════

  Signal:
  ───────
  Device registers public keys + one-time keys at Signal's key server.
  Alice fetches Bob's keys from Signal's server.

  MITM protection: Safety Numbers — a fingerprint that both parties can
  compare out-of-band (read aloud, scan QR code). If the server MITM'd
  the exchange, the safety numbers would not match. Users are prompted
  to verify when keys change. Signal detects key changes and warns the user.

  Strength: server cannot MITM without client detecting it (if user verifies).
  Weakness: most users never verify safety numbers.

  WhatsApp:
  ─────────
  Same Signal Protocol implementation. Also uses safety numbers (called
  "Security Codes"). Key change notifications shown in chat ("Your security
  code with X changed").

  Strength: identical to Signal.
  Weakness: Meta/WhatsApp server could substitute keys and most users
            would not notice.

  Telegram (Secret Chats):
  ────────────────────────
  Client-to-client DH key exchange mediated by Telegram's server.
  Telegram server carries the DH messages and could MITM if it chose to.

  MITM protection: "Key Visualization" — both parties see the same
  emoji/image derived from the shared key. Compare out-of-band.

  Strength: verifiable if users compare key visualization.
  Weakness: server can inject itself into the DH exchange. No persistent
            one-time prekeys (each new secret chat requires both parties
            to be online or the DH exchange to buffer on the server).

  Cloud Chats (Telegram's default): No E2E. Keys held by Telegram.

  iMessage:
  ─────────
  Apple's Identity Directory Server (IDS) maps Apple IDs and phone
  numbers to device public keys. All apps blindly trust whatever key
  IDS returns.

  MITM protection: None exposed to users. Apple can substitute keys.
  Apple's claim: they would not do this without legal compulsion.

  Strength: seamless UX, zero setup for users.
  Weakness: Apple is a trusted third party; no cryptographic detection
            of substitution.

  Matrix (Olm/Megolm):
  ─────────────────────
  Each device uploads its public keys to the homeserver. The user's cross-
  signing keys sign device keys, forming a trust chain rooted in the master
  key. Verification via QR code scan or emoji comparison.

  MITM protection: TOFU (Trust on First Use) by default. Users can verify
  with cross-signing. Verified status visible in app.

  Strength: Self-hosted users can run their own homeserver (no third party
            at all). Cross-signing makes multi-device verification tractable.
  Weakness: Default TOFU means first-contact trust is unverified.
```

### The TOFU Problem

**Trust on First Use (TOFU):** most E2E systems default to trusting the first
key seen for a new contact. This means the window of vulnerability is the
moment of first contact — exactly when neither party has any prior relationship
to detect a MITM.

```
TOFU Timeline:

  Day 0: Alice messages Bob for the first time.
         Alice's app fetches Bob's key from the server.
         If the server is malicious, it gives Alice a different key.
         Alice's app has no way to detect this.
         Alice encrypts to the wrong key. MITM succeeds silently.

  Day 1: Alice and Bob meet in person. They compare safety numbers.
         Mismatch detected. Too late: all prior messages compromised.

Key Change (Later):

  Day 0: Alice has been talking to Bob securely for months.
         Bob gets a new phone. His key changes.
         Alice's app gets the new key from the server.

         Signal: shows "Bob's safety number changed. Verify to continue."
         WhatsApp: shows "Your security code with Bob changed."
         iMessage: silently uses new key.
         Telegram: silently uses new key (for cloud chats).
         Matrix: shows "Bob's device list changed" with a warning badge.
```

## Problem 3: Transport and Delivery

How does a message actually reach the recipient's device?

### Store-and-Forward vs Push

Every messaging system must handle a fundamental constraint: the recipient's
device is often offline (asleep, out of coverage). Messages must be held
somewhere until the device comes online.

```
Transport Architectures
════════════════════════

  SMS — Circuit-switched SS7 Store-and-Forward:
  ──────────────────────────────────────────────
  Sender Device → Cell Tower → MSC → SMSC → Cell Tower → Recipient Device

  The SMSC (Short Message Service Center) stores the message until the
  recipient's device registers on the network. Default retention: 3-7 days.
  No application layer; delivery is handled by the cellular network itself.

  Delivery confirmation: SMSC returns a "delivery report" to the sender
  when the SMSC has successfully delivered to the device. This confirms
  SMSC delivery, NOT that the user read it.

  MMS — WAP Push + Pull:
  ──────────────────────
  Sender → MMSC ──wap push──► Recipient Device (notification: "You have MMS")
                ◄──http pull── Recipient Device (fetches from MMSC URL)

  The message is not sent directly. The MMSC stores it; the recipient
  device fetches it over HTTP when notified. This keeps large media off
  the SMS channel.

  RCS — SIP/MSRP over IMS:
  ─────────────────────────
  Client ──SIP INVITE──► IMS Core ──SIP──► Client
         ──MSRP chunks─► MSRP relay ──► Client

  SIP establishes the session; MSRP carries the message data in chunks.
  Presence (SIP PUBLISH) tells the system whether the user is online.
  If offline, messages queue at an Application Server.

  Signal/WhatsApp — HTTPS + APNs/FCM Wake-Up:
  ─────────────────────────────────────────────
  Sending device:
    1. Encrypt message with Signal Protocol (Megolm/Double Ratchet).
    2. POST encrypted payload to Signal/WhatsApp servers (HTTPS).

  Server:
    1. Store encrypted message (Signal server cannot read it).
    2. Send a "push notification" to recipient's APNs (iOS) or FCM (Android)
       device token.

  APNs/FCM:
    1. Wake up the app on the recipient's device.

  Recipient device:
    1. App wakes, connects to Signal/WhatsApp server.
    2. Downloads and decrypts pending messages.

  Telegram — MTProto Long-Poll or Direct:
  ────────────────────────────────────────
  Telegram maintains persistent TCP connections using MTProto protocol.
  For mobile: uses APNs/FCM as wake-up.
  For desktop: long-lived TCP connection using MTProto's custom transport.

  Matrix — Long-Poll /sync:
  ─────────────────────────
  Client: GET /sync?since=<token>&timeout=30000
  Server: holds the HTTP connection open for up to 30 seconds.
          Returns immediately when new events arrive.
          Returns empty response after timeout.
  Client: immediately issues the next /sync with the new token.

  For mobile: push gateway converts APNs/FCM push to trigger /sync.
  The push payload is minimal (just a wakeup, not the message content).
```

### The Battery-Latency Tradeoff

```
Battery vs Latency Tradeoff
════════════════════════════

  Persistent TCP Connection (Telegram desktop):
    + Near-zero latency for new messages
    - Battery drain (keepalives every 30s prevent NAT timeout)
    - NAT traversal complexity

  Long-Poll HTTP (Matrix /sync, timeout=30s):
    + Low latency (sub-second when messages arrive)
    + Stateless server (any server handles any /sync request)
    - One HTTP connection held per client (resource usage)
    - Moderate battery drain

  Push Wake-Up + Fetch (Signal, WhatsApp):
    + Excellent battery life (device sleeps until push arrives)
    + APNs/FCM are optimized for this workload
    - Latency: push + wakeup + connect + fetch = 2-5 seconds
    - Requires Apple/Google infrastructure for iOS/Android

  Poll (old email clients, IMAP):
    + Simple implementation
    - High battery drain (poll every minute = 60 connections/hour)
    - High latency (up to poll_interval seconds)
```

## Problem 4: Message IDs and Deduplication

Networks are unreliable. Messages can be lost, duplicated, or reordered.
Every robust messaging system needs a way to identify messages uniquely and
avoid delivering duplicates.

```
Message ID Strategies
═════════════════════

  SMS:
  ────
  No application-layer message ID. The network provides a "message reference"
  for concatenated SMS (to reassemble multipart messages), but this is
  per-transport-session and not globally unique. Deduplication at the
  application layer is not specified; duplicates can appear if the SMSC
  retransmits.

  RCS / MSRP:
  ───────────
  MSRP uses per-transaction IDs (Byte-Range + Message-ID headers) to
  identify message parts. The MSRP Transaction-ID is unique per session.
  Deduplication at the application level uses the Message-ID.

  Signal:
  ───────
  Every Signal message has a UUID generated by the sender. The recipient
  deduplicates based on UUID. The server stores messages and delivers
  them in order; the client deduplicates on UUID for safety.

  Telegram:
  ─────────
  Every message has a server-assigned message_id (monotonically increasing
  integer per dialog/channel). The client tracks a pts (persistent timestamp)
  counter and uses it to detect gaps. If pts jumps, the client fetches
  the missing range via getDifference().

  The pts system:
    - Every user event increments pts by 1.
    - Client stores last_pts.
    - On new update: if server_pts == last_pts + 1, apply normally.
    - If server_pts > last_pts + 1: gap detected, fetch missing.
    - If server_pts <= last_pts: duplicate, ignore.

  WhatsApp:
  ─────────
  Every message has a 64-bit random ID in the WhatsApp protobuf MessageKey.
  The MessageKey = { remoteJid, fromMe, id }. The id is a random hex string.
  Deduplication: server and client both track seen IDs.

  iMessage:
  ─────────
  Every iMessage has a UUID. APNs delivers at-most-once in practice;
  iMessage uses the UUID for deduplication.

  Matrix:
  ───────
  The event_id (SHA-256 hash of event content) is globally unique and
  self-certifying. Deduplication is trivially exact: two events with the
  same event_id are the same event. The client's txnId provides idempotent
  sends: same txnId → same event_id, no duplicate created.

  The txnId Pattern (Matrix):
    client_txn_id = random string per send attempt
    PUT /rooms/{id}/send/{type}/{client_txn_id}
    server: if this txnId was seen before → return previous event_id
            if new → create event, return new event_id
    client: retransmit safely with same txnId on network failure
```

### The txnId Pattern as a General Design Principle

The txnId (or idempotency key) pattern appears in many distributed systems
beyond messaging: Stripe payments, AWS API calls, database upserts. The
pattern is:

```
Idempotency Key Pattern
═══════════════════════

  Problem: "Did my request succeed? The network dropped before I got the reply."

  Solution:
    1. Client generates a unique key per logical operation (txnId).
    2. Client sends the request with the key.
    3. Server stores (key → result) before returning.
    4. If server sees the same key again: return stored result.
    5. Client can retransmit safely — it gets the same result.

  Properties:
    - Exactly-once semantics even over unreliable networks.
    - Key can be reused if the previous result was an error (depends on spec).
    - Key space must be large enough to avoid collisions.
    - Keys can be expired after some time (e.g., Matrix: 1 hour).
```

## Problem 5: Group Membership

How is a group of participants represented, and who controls membership?

```
Group Membership Models
═══════════════════════

  SMS — No native groups:
  ───────────────────────
  SMS has no concept of a group at the protocol level. "Group SMS" is a
  convention: one device sends individual SMS messages to each recipient.
  There is no shared thread; replies go to the original sender only
  (not to the "group"). MMS is required for true group messaging.

  MMS — CC/BCC Header Groups:
  ────────────────────────────
  MMS uses email-style To/CC/BCC headers. The sending MMSC duplicates
  the MMS to each recipient. There is no server-maintained group state;
  the "group" is just the set of recipients in the original MMS.

  Limitation: if one member leaves or is added, that change is local
  to the sender's MMS. Recipients have different views of the group.

  RCS — Conference Focus (MCF):
  ──────────────────────────────
  RCS group chats use a SIP Conference Focus server (MCF) that maintains
  the conference roster. Participants are represented as a SIP URI list.
  The MCF delivers messages to all participants and handles join/leave.

  Signal — Sealed Groups:
  ────────────────────────
  Signal groups (v2) have server-side group state, but the server sees
  only the group ID — not member identities or group content.

  Group structure:
    - Group ID: random 32 bytes
    - Group title, avatar: encrypted
    - Group membership: stored as a Merkle tree on Signal's server,
      but encrypted. The server knows the group exists but not who is in it.
    - Group admins: tracked in encrypted group state
    - Join via link or invite from admin

  The server enforces membership changes (add/remove) but cannot read
  group metadata. Members receive encrypted group update messages.

  Telegram — Server-Enforced Channels and Supergroups:
  ──────────────────────────────────────────────────────
  Telegram has several group types:
    Basic Group: up to 200 members, full history for all.
    Supergroup: up to 200,000+ members, admin roles, slow mode, etc.
    Channel: broadcast-only, unlimited subscribers.

  Membership is fully server-managed. Telegram sees all group metadata
  and (for cloud chats) all content. Admins have granular permissions
  (pin messages, ban users, restrict posting, etc.).

  WhatsApp — Server-Side + Sender Keys:
  ──────────────────────────────────────
  WhatsApp group state (members, admin) is server-managed. WhatsApp sees
  the group participant list. Message content is E2E encrypted using the
  Signal Protocol's Sender Keys (equivalent to Matrix's Megolm).

  One Sender Key per member per group: each sender distributes their
  Sender Key to each current group member. When someone joins or leaves,
  keys are rotated.

  iMessage — Server-Side Groups:
  ────────────────────────────────
  iMessage group threads have a server-assigned thread ID. Apple manages
  the participant list. Adding/removing participants is a server operation.
  New participants see the full group name but not prior history by default.

  Matrix — Membership Events in the DAG:
  ────────────────────────────────────────
  Matrix room membership is stored as state events in the room DAG. Every
  join, leave, invite, kick, and ban is a permanent, auditable event.

  m.room.member state events:
    membership: invite | join | leave | ban | knock

  The entire membership history is visible to anyone with access to the
  room. This is intentional: it provides auditability (you can see that
  @alice was kicked by @admin at timestamp T for reason R).

  Drawback: membership history never disappears (unless redacted, and even
  then, the redaction is visible). This is a privacy cost.
```

### The Admin Rights Problem

Every group system must answer: who can add members, remove members, change
the group name, or delete the group?

```
Admin Rights Models
════════════════════

  Telegram: fine-grained per-admin permissions
    - Delete others' messages
    - Ban users
    - Invite users
    - Pin messages
    - Manage video chats
    - Change group info
    (Each can be independently granted/revoked)

  Signal: group admin vs regular member
    (binary distinction: admin or not)

  WhatsApp: group admin vs participant
    (admins can add/remove admins, remove participants)

  Matrix: numeric power levels (0-100)
    - Any user can be given any level
    - Level requirements per action are configurable
    - This is the most flexible but most complex model
```

## Problem 6: Presence and Typing Indicators

Presence tells you whether someone is online and available. Typing indicators
tell you they are composing a reply right now. Both are high-frequency, low-
value signals — they must not be stored permanently.

```
Presence Implementation Approaches
════════════════════════════════════

  XMPP — First-class <presence> stanza:
  ──────────────────────────────────────
  XMPP was designed around presence. The <presence> stanza is a core
  protocol element, not an afterthought. Users "subscribe" to each other's
  presence. Status: available, away, dnd (do not disturb), xa (extended away).

  Presence is part of the core XMPP protocol, not an extension.
  Servers are required to deliver and cache presence information.

  Matrix — Ephemeral Data Units (EDUs):
  ──────────────────────────────────────
  Presence in Matrix is delivered as EDUs in the federation transaction
  and as ephemeral events in /sync. EDUs are:
    - Not stored in the room DAG (they vanish)
    - Not persisted across server restarts
    - Best-effort delivery

  m.presence EDU: { user_id, presence: "online"|"offline"|"unavailable", last_active_ago }
  m.typing EDU: { room_id, user_ids: [...] }

  The deliberate non-persistence means privacy: there is no permanent
  record of "Alice was typing at 3:17pm."

  Telegram — "Last Seen" with Privacy Controls:
  ───────────────────────────────────────────────
  Telegram shows "last seen" time (e.g., "last seen 2 hours ago").
  Users can configure privacy: show to everyone | contacts only | nobody.
  Online/offline status updates in real time when chatting.

  WhatsApp — "Last Seen" and Online Status:
  ──────────────────────────────────────────
  Similar to Telegram. "Last seen" with privacy controls. "Online" shown
  when the user has WhatsApp open. Optional: hide last seen from everyone.

  Signal — No Presence (Privacy by Design):
  ──────────────────────────────────────────
  Signal shows no online status, no last seen, no typing indicators by
  default (typing indicators are optional and hidden behind a setting).
  This is a deliberate privacy choice: Signal cannot provide presence
  information because it does not track when users are online.

  iMessage — Typing Indicators per Device:
  ─────────────────────────────────────────
  iMessage shows typing bubbles (the animated "...") when the recipient is
  composing. This is a push event delivered via APNs. It is not cross-
  device: if you are chatting on your iPhone and your Mac also has iMessage
  open, the sender sees typing from whichever device is active.
```

### Typing Indicator Wire Formats

```
RCS — SIP INFO with iscomposing XML:
────────────────────────────────────
  SIP INFO sip:bob@example.com SIP/2.0
  Content-Type: application/im-iscomposing+xml

  <?xml version="1.0" encoding="UTF-8"?>
  <isComposing xmlns="urn:ietf:params:xml:ns:im-iscomposing">
    <state>active</state>
    <contenttype>text/plain</contenttype>
    <lastactive>2024-01-01T12:00:00Z</lastactive>
    <refresh>60</refresh>
  </isComposing>

  States: active (typing), idle (stopped)
  refresh: how often the sender will re-send "active" (liveness check)

Matrix — m.typing EDU:
─────────────────────
  { "edu_type": "m.typing",
    "content": { "room_id": "!abc:matrix.org",
                 "user_id": "@alice:matrix.org",
                 "typing": true } }

  Sent as an EDU in federation transactions, and as an ephemeral event
  in /sync responses.

Telegram — sendChatAction API:
──────────────────────────────
  Method: messages.setTyping
  Params: { peer: <chat>, action: SendMessageTypingAction }

  Actions: TypingAction, CancelAction, RecordVideoAction,
           UploadPhotoAction, RecordAudioAction, ...

  Telegram-specific: can indicate "recording a voice message" or
  "uploading a file" — richer than just text typing.
```

## Problem 7: Read Receipts

Does the sender know the recipient has read the message?

```
Read Receipt Implementations
═════════════════════════════

  SMS — Delivery Reports Only:
  ─────────────────────────────
  SMS delivery reports (requested via TP-SRR flag in the TPDU header)
  confirm that the SMSC has delivered the message to the device. They
  do NOT confirm the user has read the message. There is no "read"
  receipt in SMS.

  RCS — MSRP REPORT:
  ───────────────────
  RCS uses MSRP's REPORT request mechanism:
    Disposition: notification   → delivery confirmation (message arrived)
    Disposition: display        → read receipt (message displayed to user)

  The sender requests receipts via:
    Content-Disposition: inline; handling=required;
      disposition-notification="positive-delivery,display-delivery"

  iMessage — Blue Ticks:
  ───────────────────────
  iMessage shows "Delivered" (gray) when delivered to the device,
  and "Read" (blue) when the recipient opens the conversation.
  Read receipts are per-conversation optional: the user can disable
  sending read receipts in Settings.

  WhatsApp — Three-State Ticks:
  ──────────────────────────────
  ✓  (single gray)  — message sent to WhatsApp's server
  ✓✓ (double gray)  — delivered to recipient's device
  ✓✓ (double blue)  — read by recipient

  Privacy option: disable sending read receipts (disables receiving too).
  In groups: blue ticks only when all members have read the message.

  Telegram — View Count for Channels; "Read" for Private:
  ─────────────────────────────────────────────────────────
  In private chats: "read" marker shown when the message is seen.
  In channels: view count (how many subscribers have opened the message).
  In groups: Telegram shows when individual members have read (in detail
  view), but not in the main chat UI for large groups.

  Signal — Optional Read Receipts:
  ──────────────────────────────────
  Read receipts in Signal are optional. When enabled: the app sends a
  ReadMessage signal to the sender when the conversation is opened.
  Privacy mode: disable read receipts (no indication to sender).
  Read receipts are E2E encrypted like all Signal messages.

  Matrix — m.read Ephemeral Event:
  ──────────────────────────────────
  Read receipts in Matrix are sent as m.receipt ephemeral events:

  POST /_matrix/client/v3/rooms/{roomId}/receipt/m.read/{eventId}

  This tells the server (and through /sync, all other devices in the room)
  that the user has read up to this event. The receipt is propagated to
  other servers in the room via the federation EDU mechanism.

  Receipts are NOT stored in the DAG — they are ephemeral. This is a
  privacy tradeoff: the homeserver does not permanently record when
  each user read each message.
```

## Problem 8: Media Handling

Sending a photo, video, or file requires solving problems that text messages
do not have: large payloads, content-type negotiation, storage and retrieval,
and end-to-end encryption of binary data.

```
Media Architecture Comparison
══════════════════════════════

  SMS — In-Band Text Only:
  ─────────────────────────
  SMS carries text only (160 chars 7-bit, or 70 chars UCS-2). Binary data
  must be encoded (UCP/EMI or similar SMSC protocols handle some binary
  SMS, but this is rarely used at the application layer). Images require
  MMS.

  MMS — WAP Fetch Model:
  ───────────────────────
  Media stored on MMSC. Recipient notified via WAP Push (SMS-delivered
  notification with URL). Recipient device fetches media over HTTP from
  the MMSC URL. Media parts are MIME-typed within the MMS PDU (SMIL).

  Maximum MMS size: typically 300KB-1MB depending on carrier.

  RCS — MSRP In-Session Transfer:
  ────────────────────────────────
  RCS sends files in-session via MSRP chunks. The file content flows
  directly through the MSRP relay:

    MSRP a786hjs2 SEND
    Message-ID: 87652491@example.com
    Byte-Range: 1-1024/5120
    Content-Type: image/jpeg

    [1024 bytes of JPEG data]
    -------a786hjs2$

  For large files (> threshold), RCS uses HTTP File Transfer via upload
  to an HTTP server, then sends the URL in-chat.

  Signal — CDN Upload + Key in Message:
  ──────────────────────────────────────
  Signal encrypts media before uploading:

  1. Client generates random AES-256 key + IV + HMAC key.
  2. Client encrypts media: ciphertext = AES-256-CBC(key, IV, plaintext)
     mac = HMAC-SHA256(mac_key, ciphertext)
  3. Client uploads ciphertext + mac to Signal's CDN over HTTPS.
     Signal CDN receives only encrypted bytes.
  4. Client sends Signal message with:
     { attachmentUrl: "https://cdn.signal.org/...",
       key: <aes_key || mac_key>,   ← also encrypted by Signal Protocol
       digest: <sha256 of encrypted attachment>,
       contentType: "image/jpeg",
       size: 102400 }
  5. Recipient decrypts with the embedded key.

  Signal CDN cannot read the image content. The key is E2E encrypted
  inside the Signal Protocol message.

  Telegram — Telegram CDN, Cloud-Encrypted:
  ──────────────────────────────────────────
  For cloud chats: Telegram stores media on its servers. Media is
  encrypted at rest with Telegram's keys (not E2E). Telegram can access it.

  For secret chats: media is also E2E encrypted (AES-256) with keys
  that Telegram does not hold.

  Large files (up to 4GB) served from Telegram's CDN. Fast delivery
  due to CDN infrastructure. Telegram does not delete files if they are
  referenced by multiple messages (deduplication by hash).

  WhatsApp — WhatsApp CDN + Signal-Style Encryption:
  ────────────────────────────────────────────────────
  Same pattern as Signal: media encrypted client-side, uploaded to
  WhatsApp CDN as ciphertext, key sent in the Signal Protocol message.

  WhatsApp media URL format: https://mmg.whatsapp.net/...
  Media key: 32 bytes, sent in the protobuf message, E2E encrypted.

  WhatsApp-specific: 4 derived keys from media key:
    media_key_expanded = HKDF(media_key, "WhatsApp " + media_type + " Keys")
    → IV (16 bytes), aes_key (32 bytes), mac_key (32 bytes), ref_key (32 bytes)

  iMessage — iCloud for Large Attachments:
  ─────────────────────────────────────────
  Small attachments: delivered inline via APNs (limited size).
  Large attachments: uploaded to iCloud, URL + decryption key sent in iMessage.

  iCloud stores content encrypted. Key management is by Apple.
  For iMessages (not SMS), content is E2E encrypted. For iCloud Backup,
  encryption may be broken (if iCloud Backup is enabled and the user
  is not using Advanced Data Protection, Apple holds a backup key).

  Matrix — Homeserver Media Repository:
  ──────────────────────────────────────
  Unencrypted rooms: media uploaded to homeserver, served via:
    mxc://matrix.org/<mediaId>  →  /_matrix/media/v3/download/matrix.org/<mediaId>

  Homeserver stores and serves the file. Federated: any homeserver can
  serve its own media.

  E2E encrypted rooms: media encrypted before upload, similar to Signal:
  1. Client generates AES-256 key + IV + SHA-256 hash.
  2. Uploads encrypted ciphertext to homeserver media repo.
  3. Sends event with:
     { url: "mxc://matrix.org/<id>",
       file: { key: { k: <aes key base64url>, alg: "A256CTR" },
               iv: <iv base64url>,
               hashes: { sha256: <hash base64url> } } }
  4. Recipient decrypts with embedded key.

  The homeserver stores only the ciphertext; it cannot read the content.
```

## Problem 9: Federation vs Centralization

Who controls the servers? Can anyone run one?

```
The Federation Spectrum
═══════════════════════

  Fully Centralized                           Fully Federated
  ◄───────────────────────────────────────────────────────────►

  Signal    WhatsApp    iMessage    Telegram    RCS      XMPP    Matrix   SMS
  ─────────────────────────────────────────────────────────────────────────────
  One set   One set     One set     One set     GSMA     Open    Open     SS7
  of Signal of Meta     of Apple    of Tele-    carriers admin   (anyone) (any
  servers   servers     servers     gram        inter-   (any-   can run) carrier)
                                    servers     connect  one can
                                                         run)

  Control   Control     Control     Control     Carriers  Servers Anyone   Any
  one set   one set     one set     one set     control   control their    carrier
  of TLS    of keys     of IDS      of servers  routing   their   own      can
  keys.     Apple                   MTProto     and some  data    home-    connect
            can MITM    can MITM    can MITM    keys.              server.  to SS7.
```

### Federation Makes E2E Harder

In a centralized system, establishing E2E encryption is relatively simple:
one organization operates the key server, and if you trust them, everything
works. In a federated system, there is no single trusted party.

**The Matrix problem:** When @alice:matrix.org messages @bob:mozilla.org, she
fetches Bob's keys from his homeserver (mozilla.org). She must trust that
mozilla.org is honest about Bob's keys. If mozilla.org is compromised, it
could serve Alice a different key — and Alice has no way to detect this without
out-of-band verification.

**Comparison:** In Signal, there is exactly one server to trust. In Matrix,
you must trust your homeserver AND your contact's homeserver AND every
homeserver in every room you share. Cross-signing helps (a verified user's
signing key vouches for their devices), but the root of trust is still the
homeserver.

### Federation and Moderation

Another tradeoff: centralized systems can moderate content globally. Signal
can ban a user and they are banned everywhere. In Matrix, a ban in one room
does not ban someone from other rooms or their home server. The Matrix spec
has proposals for federated moderation (Mjolnir bots, ban lists), but this
is an unsolved problem in federated systems generally.

```
Moderation Tradeoffs
════════════════════

  Centralized (Signal, Telegram, WhatsApp):
    + Can remove harmful content globally
    + Can ban users from the platform
    + Easier to comply with legal orders
    - Single point of censorship
    - Can deplatform users arbitrarily

  Federated (Matrix, XMPP):
    + Cannot be deplatformed globally (join another server)
    + No single authority to censor content
    - Harder to remove harmful content (each server decides)
    - Harder to coordinate cross-server bans
    - Abusers can self-host and be unreachable
```

## Problem 10: Push Notifications

When a user's device is sleeping, how does a message wake it up?

### APNs and FCM: Universal Push Infrastructure

Almost all modern messaging systems on mobile use Apple Push Notification
Service (APNs) for iOS and Firebase Cloud Messaging (FCM, formerly GCM) for
Android. These are the two canonical push delivery systems.

```
APNs Architecture
══════════════════

  Provider Server (e.g., Signal) ─────────────────────────────────►  APNs
                                   HTTP/2 + TLS + auth token or cert   │
                                   Push request:                        │
                                   { device_token: "<64-byte token>",  │
                                     aps: { alert: "New message",       │
                                            badge: 1,                   │
                                            sound: "default" },         │
                                     payload: { <app-specific data> }}  │
                                                                        │
  APNs Infrastructure ────────────────────────────────────────────────►  Device
                         Persistent TLS connection                        (iPhone)
                         kept alive by iOS/iPadOS                         Wakes app
                                                                          App fetches
                                                                          message

  Key design decisions:
    1. APNs maintains persistent connections from servers and devices.
       Provider servers use short-lived HTTP/2 connections or long-lived
       persistent connections (newer: HTTP/2 for providers).
    2. Device token = ephemeral identifier for the device+app combination.
       Rotates periodically. Provider must handle token refresh.
    3. APNs guarantees at-most-once delivery. Coalescing: if multiple
       pushes are queued while the device is offline, APNs delivers
       only the most recent one (with the "apns-collapse-id" header).
    4. Payload size limit: 4KB for most notifications.

FCM Architecture (Android equivalent):
  Functionally identical to APNs. Key differences:
    - FCM registration token instead of APNs device token.
    - Data messages vs notification messages (data: fully app-handled;
      notification: system-handled, shown as system notification).
    - FCM supports topic messaging (send to subscribers of a topic)
      and device group messaging.
```

### Push and Privacy

A critical design tension: the push notification itself reveals information.
If the push says "New message from Alice," the provider (Apple/Google) learns
that Alice sent you a message.

```
Push Privacy Strategies
════════════════════════

  Minimal payload approach (Signal, Matrix push gateways):
    Push payload: { "new_message": true }  (no sender, no content)
    App wakes up and fetches the message from its own server.
    APNs/FCM sees only: "Signal has a notification for this device."
    No message content or sender identity in the push.

  Rich notification approach (Telegram, WhatsApp):
    Push payload includes sender name and message preview.
    APNs/FCM sees: "Alice: Hey, are you free tonight?"
    Better UX (lock screen preview), worse privacy.

  Silent push approach:
    Push has no visible notification; just wakes the app.
    App fetches and shows its own notification.
    Used by Matrix Unified Push integration.
```

## Comparison Table

```
Full Protocol Comparison Matrix
════════════════════════════════

  Feature           │ SMS      │ MMS      │ RCS      │ Signal   │ Telegram │ WhatsApp │ iMessage │ Matrix
  ──────────────────┼──────────┼──────────┼──────────┼──────────┼──────────┼──────────┼──────────┼────────
  Identity          │ Phone#   │ Phone#   │ SIP URI  │ Phone#   │ Phone#   │ Phone#   │ Apple ID │ @user:
  type              │ (E.164)  │ (E.164)  │ (tel:)   │ →username│ →user ID │ (E.164)  │ or phone │ server
  ──────────────────┼──────────┼──────────┼──────────┼──────────┼──────────┼──────────┼──────────┼────────
  E2E encryption    │ No       │ No       │ Optional │ Yes      │ Secret   │ Yes      │ Yes      │ Optional
                    │          │          │ (GSMA)   │ always   │ chats    │ always   │ always   │ (Olm)
  ──────────────────┼──────────┼──────────┼──────────┼──────────┼──────────┼──────────┼──────────┼────────
  Forward secrecy   │ No       │ No       │ No       │ Yes      │ Secret   │ Yes      │ Yes      │ Yes
                    │          │          │          │ (DR)     │ chats    │ (DR)     │ (limited)│ (Megolm
                    │          │          │          │          │ only     │          │          │ rotation)
  ──────────────────┼──────────┼──────────┼──────────┼──────────┼──────────┼──────────┼──────────┼────────
  Federation        │ Yes      │ Yes      │ Semi     │ No       │ No       │ No       │ No       │ Yes
                    │ (SS7)    │ (SS7)    │ (GSMA)   │          │          │          │          │ (full)
  ──────────────────┼──────────┼──────────┼──────────┼──────────┼──────────┼──────────┼──────────┼────────
  Media handling    │ Text     │ WAP+     │ MSRP     │ CDN+     │ Telegram │ CDN+     │ iCloud   │ Homesvr
                    │ only     │ MMSC     │ chunks   │ E2E key  │ CDN      │ E2E key  │ + HTTPS  │ media
  ──────────────────┼──────────┼──────────┼──────────┼──────────┼──────────┼──────────┼──────────┼────────
  Group messaging   │ None     │ CC/BCC   │ MCF      │ Sealed   │ Server-  │ Signal   │ Server-  │ DAG
  model             │ native   │ headers  │ (SIP)    │ groups   │ managed  │ Sender   │ managed  │ member-
                    │          │          │          │ (E2E)    │          │ Keys     │          │ ship
  ──────────────────┼──────────┼──────────┼──────────┼──────────┼──────────┼──────────┼──────────┼────────
  Typing indicators │ No       │ No       │ Yes      │ Optional │ Yes      │ Yes      │ Yes      │ Yes
                    │          │          │ (iscomp) │          │          │          │ (bubble) │ (EDU)
  ──────────────────┼──────────┼──────────┼──────────┼──────────┼──────────┼──────────┼──────────┼────────
  Read receipts     │ Delivery │ Delivery │ Yes      │ Optional │ Yes      │ Yes (3   │ Yes      │ Yes
                    │ only     │ only     │ (REPORT) │          │ (limited)│ ticks)   │ (blue)   │ (m.read)
  ──────────────────┼──────────┼──────────┼──────────┼──────────┼──────────┼──────────┼──────────┼────────
  Push architecture │ SS7      │ WAP Push │ SIP/IMS  │ APNs+FCM │ APNs+FCM │ APNs+FCM │ APNs     │ APNs+
                    │ (always) │          │          │ + Signal │ + MTProto│ + Signal │ (native) │ FCM+
                    │          │          │          │ servers  │ servers  │ servers  │          │ push gw
  ──────────────────┼──────────┼──────────┼──────────┼──────────┼──────────┼──────────┼──────────┼────────
  Open standard /   │ Yes      │ Yes      │ Yes      │ Protocol │ Partial  │ No       │ No       │ Yes
  open source       │ (3GPP)   │ (3GPP)   │ (GSMA)   │ open,    │ docs,    │          │          │ (Matrix
                    │          │          │          │ app open │ app open │          │          │ spec +
                    │          │          │          │ source   │ source   │          │          │ Synapse)
  ──────────────────┼──────────┼──────────┼──────────┼──────────┼──────────┼──────────┼──────────┼────────
  Self-hostable     │ Carrier  │ Carrier  │ Carrier  │ No       │ No       │ No       │ No       │ Yes
                    │ only     │ only     │ only     │          │          │          │          │ (fully)
  ──────────────────┼──────────┼──────────┼──────────┼──────────┼──────────┼──────────┼──────────┼────────
  Message retention │ SMSC:    │ MMSC:    │ Server   │ Server:  │ Server:  │ Server:  │ Server:  │ Server:
  policy            │ 3-7 days │ varies   │ depends  │ until    │ until    │ until    │ until    │ per room
                    │          │          │          │ delivered│ delete   │ delivered│ delivered│ config
```

## Design Patterns

### Pattern 1: The Signal Protocol as a Reusable Primitive

The Signal Protocol (X3DH key agreement + Double Ratchet session encryption)
has become the de facto standard for E2E messaging encryption. It is used by:

- Signal (original implementation)
- WhatsApp (licensed/ported from Signal Protocol by Open Whisper Systems)
- Facebook Messenger (Secret Conversations mode)
- Matrix/Olm (Olm is an independent implementation of the same cryptographic
  design; Megolm is an adaptation of Signal's Sender Keys)
- Google Messages (RCS E2E encryption, in development)
- Skype (private conversations feature)

The Signal Protocol's key properties that make it reusable:
- **X3DH (Extended Triple Diffie-Hellman):** offline key agreement using prekeys.
  No need for both parties to be online at the same time.
- **Double Ratchet:** forward secrecy per message. Compromise of one message
  key does not reveal past or future messages.
- **Prekey rotation:** one-time prekeys prevent replay attacks and provide
  break-in recovery.

```
Signal Protocol Stack
══════════════════════

  Application
      │
  ┌───▼───────────────────────────────────────────────────────────┐
  │                     Megolm / Sender Keys                      │
  │  (group encryption: one OutboundSession per sender per room)  │
  └───▲───────────────────────────────────────────────────────────┘
      │  Key distribution via:
  ┌───▼───────────────────────────────────────────────────────────┐
  │                     Olm / Double Ratchet                      │
  │       (1:1 sessions: established via X3DH)                    │
  └───▲───────────────────────────────────────────────────────────┘
      │  Initial key material from:
  ┌───▼───────────────────────────────────────────────────────────┐
  │                    X3DH Key Agreement                         │
  │   (offline session establishment using prekeys)               │
  └───────────────────────────────────────────────────────────────┘
```

### Pattern 2: Store-and-Forward vs Real-Time Delivery

Every messaging system is fundamentally a queue. The design question is where
the queue lives and how long messages stay in it.

```
Store-and-Forward Spectrum
══════════════════════════

  SMSC (SMS):
    Queue: at the carrier's SMSC
    Duration: 3-7 days (configurable)
    Retrieval: automatic when device registers on network
    Encryption: none; carrier reads all messages

  Signal Server:
    Queue: encrypted blobs at Signal's server
    Duration: until delivered (max 30 days if device never connects)
    Retrieval: client pulls on connect
    Encryption: E2E; Signal cannot read content

  Telegram Server:
    Queue: cloud storage at Telegram
    Duration: forever (unless user deletes)
    Retrieval: client pulls via MTProto
    Encryption: server-side; Telegram can read cloud chats

  Matrix Homeserver:
    Queue: permanent event store in the room DAG
    Duration: forever (per server retention policy)
    Retrieval: client polls via /sync
    Encryption: optional E2E; homeserver sees plaintext unless E2E enabled

  Key insight: "store-and-forward" is not just a transport concern.
  It determines: privacy (who can read queued messages), history
  (can new users see old messages), and reliability (how long can a
  device be offline).
```

### Pattern 3: The Envelope Pattern

Every E2E encrypted messaging system uses an "envelope" structure: outer
routing information that the server can read, plus an inner encrypted payload
that only the recipient can read.

```
The Envelope Pattern
════════════════════

  OUTER (visible to server/carrier):
  ┌─────────────────────────────────────────────────────────────────┐
  │ from:    @alice:matrix.org                                      │
  │ to:      @bob:mozilla.org                                       │
  │ room:    !jNBbFgSWJBjEtmVquB:matrix.org                         │
  │ type:    m.room.encrypted                                        │
  │ time:    2024-01-01T12:00:00Z                                   │
  │ size:    312 bytes                                              │
  └─────────────────────────────────────────────────────────────────┘
        │
        ▼ INNER (visible only to recipient who holds the key)
  ┌─────────────────────────────────────────────────────────────────┐
  │ ████████████████████████████████████████████████████████████   │
  │ ██ (AES-256-CBC encrypted, decrypts to:)                   ██   │
  │ ██  { "type": "m.room.message",                            ██   │
  │ ██    "content": { "msgtype": "m.text",                    ██   │
  │ ██                 "body": "Meet at 3pm?" } }              ██   │
  │ ████████████████████████████████████████████████████████████   │
  └─────────────────────────────────────────────────────────────────┘

  The server sees: Alice sent Bob a 312-byte encrypted message at noon.
  The server does NOT see: "Meet at 3pm?"

  This is the fundamental limit of E2E encryption in messaging:
  METADATA is almost always exposed. The server knows:
    - Who is messaging whom
    - When
    - How often
    - How large the messages are
    - (Sometimes) which room or group
```

### Pattern 4: Why Metadata is Hard to Hide

Even with perfect E2E encryption, metadata leakage is severe.

```
Metadata Leakage Examples
══════════════════════════

  "We can tell from metadata that two people are in a relationship."
    How: they message each other 50 times per day, every day,
         starting at 8am and ending at 11pm.

  "We can tell when someone is having a mental health crisis."
    How: they message a crisis hotline at 2am three nights in a row.

  "We can tell someone is a journalist with a government source."
    How: an unknown number called them once; they sent an encrypted
         message the next day; 3 days later a news story broke.

  None of these require reading a single message.

  Metadata-resistant designs:
    Signal: Sealed Sender hides who is messaging whom from the server.
      (The server only sees: "some Signal user sent something to @bob")
    Tor + Matrix: hide IP addresses from the homeserver.
    No system currently hides group membership from the server while
    also being federated. This is an open research problem.

  Sealed Sender (Signal):
    Normal message:   server sees sender A → recipient B
    Sealed sender:    message is encrypted to B, then wrapped in
                      a "delivery token" that is encrypted to the
                      server's key. Server decrypts only the routing
                      token (enough to deliver), not the sender identity.

    Implementation:
      1. Alice encrypts her Signal message to Bob (normal Double Ratchet).
      2. Alice wraps the ciphertext in a SenderCertificate, which is
         also encrypted. The certificate proves Alice is a valid Signal
         user, but only Bob can read it.
      3. Signal server decrypts only the outer wrapper to find Bob's
         address. It cannot read the SenderCertificate.
      4. Bob decrypts and learns the message is from Alice.
```

### Pattern 5: Protocol Layering and Separation of Concerns

Every well-designed messaging system separates:

```
Protocol Layering in Messaging Systems
═══════════════════════════════════════

  Layer             │ Concern                  │ Example
  ──────────────────┼──────────────────────────┼────────────────────────────
  Crypto layer      │ Encrypt/decrypt payload  │ Signal Protocol, Megolm
  ──────────────────┼──────────────────────────┼────────────────────────────
  Serialization     │ Wire format for messages │ Protobuf (WhatsApp/Signal),
  layer             │                          │ JSON (Matrix), SMIL (MMS),
                    │                          │ RESP-inspired (XMPP XML)
  ──────────────────┼──────────────────────────┼────────────────────────────
  Transport layer   │ Get bytes from A to B    │ HTTPS, MTProto, SS7, SIP
  ──────────────────┼──────────────────────────┼────────────────────────────
  Delivery layer    │ Store-and-forward queuing│ SMSC, Signal server, MMSC,
                    │                          │ homeserver DAG
  ──────────────────┼──────────────────────────┼────────────────────────────
  Identity layer    │ Who are the parties?     │ Phone book, IDS, key server
  ──────────────────┼──────────────────────────┼────────────────────────────
  Application layer │ Display UI; user actions │ Element, Signal app, SMS app
```

Systems that mix these layers are hard to maintain. XMPP's success as a
protocol comes from its clean separation: the core is just extensible XML
stanzas (serialization + transport + delivery). Everything else — E2E, presence,
groupchat — is an extension (XEP). Matrix similarly separates: the DAG is the
delivery layer, JSON is the serialization layer, Olm/Megolm is the crypto layer.

SMS/MMS is an example of poor layering: the 160-character limit is baked into
the circuit-switched transport (SS7 TPDU), the content type (GSM 7-bit charset)
is baked into the payload encoding, and there is no separation between delivery
and application concerns.

## Algorithms: Key Comparison

### Double Ratchet (Signal Protocol) vs Megolm vs MTProto

```
Ratchet Algorithm Comparison
═════════════════════════════

  Double Ratchet (Signal, WhatsApp, Olm for 1:1):
  ─────────────────────────────────────────────────
  Two interleaved ratchets:
    KDF Chain (Symmetric Ratchet): derives message keys from chain keys
    DH Ratchet: ratchets on every reply using new DH key pair

  On each reply, the DH ratchet turns:
    (Alice sends A1, A2, A3)  Bob replies → DH ratchet advances
    (Bob sends B1, B2)        Alice replies → DH ratchet advances
    ...

  Forward secrecy: compromise of message key for message N does not
    reveal keys for messages N+1, N+2, ... (KDF is one-way)
  Break-in recovery: after a DH ratchet step, old compromise is healed
    (attacker must break new DH to continue reading)

  Cost: one DH operation per "conversation turn" (reply)

  Megolm (Matrix, WhatsApp groups, Signal groups):
  ─────────────────────────────────────────────────
  Single ratchet (no DH ratchet):
    R[i+1] = HMAC-SHA256(R[i], 0x00..0x03)

  Forward secrecy: knowing key for message N does not reveal keys < N
  Break-in recovery: NONE. If someone learns session_key at index N,
    they can decrypt all messages N, N+1, N+2, ... forever.
    Mitigation: rotate session (create new OutboundGroupSession) periodically,
    when members join/leave, and after some number of messages.

  Cost: no DH operations after session creation (very fast)

  MTProto 2.0 (Telegram secret chats):
  ──────────────────────────────────────
  Based on AES-256-IGE (Infinite Garble Extension) + SHA-256.
  Initial key exchange: DH between client and server (not client-to-client).
  Ratchet: uses a KDF chain similar to Signal but without the DH ratchet.
  Session keys: derived per-message from a root key chain.

  Forward secrecy: partial (chain advances, old keys deleted)
  Break-in recovery: limited (no periodic DH exchange)
  Cloud chats: no E2E; MTProto is only transport encryption (server reads all)
```

## Test Strategy

### Cross-Protocol Compatibility Tests

```
test_identity_format_parsing:
  For each protocol's identity format:
    SMS:     "+14155552671" → valid E.164
             "1415555267"  → invalid (no +)
    Matrix:  "@alice:matrix.org" → valid
             "alice:matrix.org"  → invalid (no @)
             "@alice"            → invalid (no homeserver)
    Signal:  "@alice.01"        → valid username (new format)
    RCS:     "tel:+14155552671" → valid SIP URI
             "sip:alice@example.com" → valid SIP URI

test_e2e_encryption_round_trip (for each E2E system):
  given:  plaintext = "hello world"
  when:   alice encrypts to bob
  expect: bob decrypts correctly
  when:   server sees the ciphertext
  expect: cannot recover plaintext without bob's key

test_forward_secrecy (Double Ratchet):
  given:  alice and bob have an established Double Ratchet session
  given:  messages M1, M2, M3 sent
  when:   attacker captures M1's message key
  expect: M2 and M3 cannot be decrypted with M1's key
  when:   attacker captures the entire ratchet state after M3
  expect: M1 and M2 cannot be decrypted (forward secrecy)

test_break_in_recovery (Double Ratchet):
  given:  attacker captures ratchet state at M3
  when:   bob replies (triggers DH ratchet advance)
  expect: attacker cannot decrypt any messages after the reply
          (new DH key pair not known to attacker)
```

### Protocol-Specific Tests

```
SMS/MMS:
  test_concat_sms_reassembly:
    given:  a 500-char message split across 4 SMS PDUs with matching UDH ref
    expect: receiver reassembles in correct order regardless of arrival order

  test_mms_smil_parse:
    given:  a MMS PDU with SMIL presentation + JPEG + text parts
    expect: parser extracts each part with correct Content-Type and data

RCS:
  test_msrp_chunk_reassembly:
    given:  a 100KB file sent in 10 MSRP chunks (Byte-Range: X-Y/102400)
    expect: all chunks arrive and reassemble to original file

  test_iscomposing_timeout:
    given:  bob sends isComposing "active"
    when:   no refresh arrives within 2x the refresh interval
    expect: alice's app shows bob as no longer typing

Signal Protocol:
  test_prekey_bundle_validation:
    given:  a PreKeyBundle from Bob
    expect: all signatures verify (identity key, signed prekey, one-time prekey)
    given:  a PreKeyBundle with bad signature
    expect: session initiation fails with SignatureVerificationError

  test_out_of_order_message_decryption:
    given:  alice sends M1, M2, M3 but M2 is delayed
    expect: bob decrypts M1 and M3 first, then M2 (out of order)
    note:   Double Ratchet must buffer future message keys for M3

Telegram MTProto:
  test_pts_gap_detection:
    given:  client has pts=100, server sends update with pts=105
    expect: client detects gap (pts_diff=5), calls getDifference
            to fetch missing 4 updates

  test_secret_chat_dh_nist_vector:
    given:  Telegram DH parameters g=3, p=known 2048-bit prime
    given:  Alice's random a, Bob's random b
    expect: g^ab mod p = g^ba mod p (DH property)
    expect: key_fingerprint = last 8 bytes of SHA1(key) matches

Matrix:
  test_state_resolution_idempotent:
    given:  two conflicting state sets S1, S2
    when:   resolve_state(S1, S2) called twice
    expect: same result both times (deterministic)

  test_event_id_self_certifying:
    given:  any room version 4 event
    expect: "$" + base64url(sha256(canonical_json(redact(event))))
            == event["event_id"]

  test_federation_pdu_signature_invalid_rejected:
    given:  a PDU with a forged ed25519 signature
    expect: receiving server rejects it without adding to DAG
```

### Metadata Privacy Tests

```
test_sealed_sender_hides_sender_identity:
  given:  Signal SealedSenderMessage from alice to bob
  when:   signal server processes it
  expect: server extracts only delivery token (bob's address)
          server cannot determine alice's user ID from the outer envelope

test_megolm_session_rotation_on_member_leave:
  given:  a Matrix room with alice, bob, carol
  given:  alice has an active Megolm OutboundGroupSession
  when:   carol leaves the room
  expect: alice's client creates a new OutboundGroupSession
          distributes new session key to alice and bob only (NOT carol)
          carol cannot decrypt messages sent after her departure

test_push_notification_minimal_payload:
  given:  a new Signal message arrives for alice's device
  when:   Signal server sends APNs push notification
  expect: APNs payload contains no sender ID, no message preview
          only a wakeup signal that causes the app to fetch from Signal
```

## Summary: What Each System Optimized For

```
Design Priority Summary
═══════════════════════

  SMS:      Universal reach (every phone, every carrier, no app needed)
            Sacrificed: all security, rich features, group functionality

  MMS:      Media delivery on existing SMS infrastructure
            Sacrificed: security, reliability (size limits, MMSC delays)

  RCS:      Modern features on carrier infrastructure (no app needed)
            Sacrificed: E2E encryption (rarely deployed), fragmented rollout

  Signal:   Maximum privacy and security (open source, nonprofit)
            Sacrificed: federation, group scalability, UX (phone # required)

  Telegram: Maximum features and speed (bots, channels, file sharing)
            Sacrificed: E2E encryption by default, privacy of cloud content

  WhatsApp: Privacy of content with network effects (2B+ users)
            Sacrificed: metadata privacy (Meta), no federation, phone # required

  iMessage: Seamless Apple ecosystem integration, beautiful UX
            Sacrificed: cross-platform (falls back to SMS), Apple trust

  Matrix:   Openness, federation, self-hosting, no single point of failure
            Sacrificed: UX simplicity, performance (federation overhead),
                        requires understanding of homeservers by users
```

Every system in this list is successful in its chosen optimization. The lesson
is not "which system is best" but "what do you care most about?" Once you know
your priorities — privacy, reach, features, openness, or simplicity — the right
protocol follows from first principles.
