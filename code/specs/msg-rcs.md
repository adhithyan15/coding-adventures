# MSG-RCS — RCS (Rich Communication Services)

## Overview

SMS (Short Message Service) was designed in 1985 to piggyback tiny 160-character
text messages on the signaling channel of 2G cellular networks — the narrow
bandwidth left over after voice calls. For its era, SMS was miraculous. By 2010
it was showing its age: no read receipts, no typing indicators, no images, no
group chats with more than rudimentary support, and a per-message cost that made
no engineering sense in a world of always-on data connections.

The carriers watched WhatsApp, iMessage, and Telegram eat their lunch. Users sent
billions of messages per day through these apps — all tunneled over the carriers'
own data networks, generating no SMS revenue and building brand loyalty for
Apple and Facebook, not AT&T or Vodafone.

RCS (Rich Communication Services) is the carriers' answer: a modern messaging
protocol built directly into the phone's SIM and dialer, requiring no separate
app, delivering iMessage-like features universally between Android phones.

**What makes RCS different from OTT (Over-The-Top) apps like WhatsApp:**
- RCS is a carrier service, not an app. It works from your phone number, not
  a separate account.
- RCS is built on open standards: SIP (Session Initiation Protocol), MSRP
  (Message Session Relay Protocol), and the GSMA's Universal Profile.
- RCS interoperates between carriers. A T-Mobile subscriber can exchange RCS
  messages with a Vodafone subscriber without either installing an app.
- RCS falls back to SMS when the other party does not support RCS, giving
  universal coverage.

**Analogy:** Think of SMS as a telegraph — it works, it's universal, but it only
sends Morse code. iMessage is like a private telephone company that only works if
both parties subscribe to the same service. RCS is like upgrading the entire
public telephone network to carry not just voice but video, files, and
interactive content — still using your existing phone number, still working with
any carrier.

**GSMA Universal Profile** is the GSMA's (Global System for Mobile
Communications Association) standardized subset of RCS features that all
implementations must support. Universal Profile 1.0 (2016) defined basic
messaging. UP 2.x (2020+) adds chatbots, suggested replies, and richer media.

```
Evolution of mobile messaging
══════════════════════════════════════════════════════════════════════════

  1985          1992          2009              2012          2016
   │             │             │                 │             │
   ▼             ▼             ▼                 ▼             ▼
  ISDN         SMS           iMessage          WhatsApp      RCS UP 1.0
  (voice)    (160 chars,     (Apple-only,      (app-only,    (carrier-
             carrier billed)  E2E, rich media)  E2E, rich)   native, rich)

SMS problems:                     RCS solutions:
  ✗ 160-char limit           →      ✓ No practical message size limit
  ✗ No delivery receipts     →      ✓ Delivery + read receipts
  ✗ No typing indicators     →      ✓ Is-composing notifications
  ✗ No group chat            →      ✓ Full group chat with conference server
  ✗ MMS limited to ~300 KB   →      ✓ File transfer up to 100 MB+
  ✗ No rich formatting       →      ✓ Cards, carousels, suggested actions
```

## Architecture

RCS is not a monolithic system. It is built by layering several existing
telecom and internet protocols:

```
RCS Architecture Stack
══════════════════════════════════════════════════════════════════════════

  ┌────────────────────────────────────────────────────────────────────┐
  │  RCS Application Layer                                             │
  │  ┌──────────────────┐ ┌─────────────────┐ ┌───────────────────┐  │
  │  │  1:1 Messaging   │ │  Group Chat     │ │  File Transfer    │  │
  │  │  (MSRP SEND)     │ │  (MCF + MSRP)  │ │  (MSRP chunking) │  │
  │  └──────────────────┘ └─────────────────┘ └───────────────────┘  │
  │  ┌──────────────────┐ ┌─────────────────┐ ┌───────────────────┐  │
  │  │  Capabilities    │ │  Pager Mode     │ │  Chatbots         │  │
  │  │  (SIP OPTIONS)   │ │  (SIP MESSAGE)  │ │  (UP 2.4+ APIs)  │  │
  │  └──────────────────┘ └─────────────────┘ └───────────────────┘  │
  ├────────────────────────────────────────────────────────────────────┤
  │  MSRP (Message Session Relay Protocol, RFC 4975)                   │
  │  Session-based message transport. Runs over TCP/TLS.               │
  │  Carries the actual message bytes after SIP sets up the session.   │
  ├────────────────────────────────────────────────────────────────────┤
  │  SIP (Session Initiation Protocol, RFC 3261)                       │
  │  Signaling. Registers users, negotiates sessions, discovers caps.  │
  │  Runs over UDP or TCP (TLS for security).                          │
  ├────────────────────────────────────────────────────────────────────┤
  │  IMS (IP Multimedia Subsystem)                                     │
  │  Carrier infrastructure. Authentication, routing, user registry.   │
  │  ┌──────────────┐ ┌──────────────┐ ┌──────────────┐ ┌─────────┐ │
  │  │  P-CSCF      │ │  I-CSCF      │ │  S-CSCF      │ │   HSS   │ │
  │  │  (Proxy)     │ │  (Interrogat)│ │  (Serving)   │ │(User DB)│ │
  │  └──────────────┘ └──────────────┘ └──────────────┘ └─────────┘ │
  ├────────────────────────────────────────────────────────────────────┤
  │  IP Network (LTE / 5G / WiFi data plane)                           │
  └────────────────────────────────────────────────────────────────────┘

Data plane separation:

  SIP Signaling path:    UE ←→ P-CSCF ←→ I-CSCF ←→ S-CSCF
  MSRP Media path:       UE ←→ (direct peer-to-peer, or via relay)
  Authentication:        S-CSCF ←→ HSS (via Cx/Dx interface, Diameter)
```

### IMS Components Explained

The IMS core is the carrier's signaling brain. Understanding its components
is essential to understanding how RCS registration and routing work.

```
IMS Component Map
══════════════════════════════════════════════════════════════════════════

  Phone (UE)                     Carrier Network
  ─────────────────────────────────────────────────────
                                 ┌─────────────────────┐
                                 │       P-CSCF        │
  ┌───────────┐  SIP over TLS   │  Proxy-CSCF         │
  │           │ ──────────────→ │                     │
  │    RCS    │                 │  First point of      │
  │   Client  │                 │  contact. The        │
  │           │                 │  "front door" of     │
  │  (on the  │                 │  IMS. Forwards SIP   │
  │   phone)  │                 │  messages inward.    │
  └───────────┘                 │  May compress SIP    │
                                │  (SigComp, RFC 3486) │
                                └──────────┬──────────┘
                                           │
                                           ▼
                                 ┌─────────────────────┐
                                 │       I-CSCF        │
                                 │  Interrogating-CSCF  │
                                 │                     │
                                 │  Receives REGISTER  │
                                 │  from P-CSCF. Asks  │
                                 │  HSS "which S-CSCF  │
                                 │  should serve this  │
                                 │  user?" Then routes │
                                 │  to that S-CSCF.    │
                                 └──────────┬──────────┘
                                            │
                          ┌─────────────────┴──────────────────┐
                          ▼                                     ▼
                ┌─────────────────────┐             ┌─────────────────────┐
                │       S-CSCF        │             │         HSS         │
                │  Serving-CSCF       │◄──Cx/Dx────►│  Home Subscriber   │
                │                     │  (Diameter) │  Server            │
                │  The registrar.     │             │                     │
                │  Authenticates the  │             │  The carrier's user │
                │  user via AKA.      │             │  database. Stores:  │
                │  Stores the binding │             │  - IMSI, IMPU, IMPI │
                │  contact→URI.       │             │  - Auth vectors     │
                │  Routes in-dialog   │             │  - Service profiles │
                │  SIP messages.      │             │  - Subscribed caps  │
                └─────────────────────┘             └─────────────────────┘
```

**Why split P-CSCF, I-CSCF, and S-CSCF?** Each serves a different role:
- P-CSCF is close to the radio network, optimized for mobile clients (handles
  NAT traversal, SIP compression, IPSec for IMS-AKA).
- I-CSCF hides the internal topology. External carriers only see the I-CSCF
  IP address, not the internal S-CSCF farm.
- S-CSCF holds session state. Multiple S-CSCFs can serve different users,
  load-balanced across the HSS.

### RCS Basic vs RCS Advanced (Universal Profile)

```
RCS Basic (deprecated, pre-UP)    RCS Advanced / Universal Profile
────────────────────────────────────────────────────────────────────
  File Transfer (MSRP)            Everything in Basic, plus:
  1:1 Chat (MSRP)                 Group Chat (MCF-based)
  Capability Discovery            Geolocation sharing
  (SIP OPTIONS)                   Audio/video calls (SIP/RTP)
                                  Read receipts (CPIM)
                                  Typing indicators (iscomposing)
                                  Chatbots (UP 2.4+)
                                  Suggested replies / actions
                                  RCS Business Messaging
                                  Rich cards and carousels
```

## Key Concepts

### SIP: The Signaling Language of RCS

SIP is a text-based protocol modeled after HTTP. Every SIP message is either
a **request** (sent by either side) or a **response** (a 3-digit status code
like HTTP).

**SIP request methods used in RCS:**

```
Method     Purpose
────────────────────────────────────────────────────────────────────────
REGISTER   Client announces its current IP address to the IMS registrar.
           Without this, the network does not know where to route calls.
OPTIONS    Capability query. "What features do you support?"
INVITE     Session setup. "Let's start an MSRP messaging session."
ACK        Confirms receipt of the 200 OK for INVITE. Completes 3-way
           handshake.
BYE        Terminates an established session.
MESSAGE    Pager-mode: send a single short message without a session.
INFO       Mid-session signaling (e.g., typing indicators).
SUBSCRIBE  Subscribe to an event (e.g., presence/online status).
NOTIFY     Delivers an event to a subscriber.
```

SIP messages carry SDP (Session Description Protocol) in their body when
negotiating media sessions (like MSRP or voice/video).

### MSRP: The Message Carrier

**Analogy:** If SIP is the phone call setup ("can we talk?"), MSRP is the
actual conversation ("here is my message"). SIP negotiates who talks to whom
and what format; MSRP delivers the bytes.

MSRP (RFC 4975) is a session-based protocol designed for exchanging messages
of arbitrary size over a reliable transport (TCP or TLS). Unlike SIP MESSAGE,
which is fire-and-forget (pager mode), MSRP maintains a persistent connection
for the duration of a chat session, enabling efficient delivery of many
messages and large file transfers.

```
SIP vs MSRP responsibilities
══════════════════════════════════════════════════════════════════════════

  SIP:
  ┌─────────────────────────────────────────────────────────┐
  │  "Alice wants to chat with Bob."                        │
  │  "Alice's MSRP endpoint is at msrp://10.1.2.3:9000/    │
  │   abc123;tcp"                                           │
  │  "Bob accepts. His MSRP endpoint is at msrp://10.4.5.6: │
  │   9001/xyz789;tcp"                                      │
  │  "The session is established."                          │
  └─────────────────────────────────────────────────────────┘

  MSRP (takes over after SIP INVITE/200 OK/ACK completes):
  ┌─────────────────────────────────────────────────────────┐
  │  Alice → Bob: MSRP SEND "Hello!"                        │
  │  Bob → Alice: MSRP 200 OK (delivery acknowledgment)     │
  │  Bob → Alice: MSRP SEND "Hi there!"                     │
  │  Alice → Bob: MSRP 200 OK                               │
  │  ...                                                    │
  │  (session continues until BYE)                          │
  └─────────────────────────────────────────────────────────┘
```

### CPIM: Wrapping Messages in Metadata

CPIM (Common Profile for Instant Messaging, RFC 3860) is a thin metadata
wrapper placed around each message body inside MSRP. It carries:
- The sender's display name and URI
- The recipient's URI
- The message timestamp
- The message content type

CPIM enables rich features like read receipts, message threading, and sender
attribution in group chats without modifying the MSRP layer.

## SIP Registration

Before an RCS client can send or receive any messages, it must **register**
with the IMS core. Registration tells the S-CSCF "Alice is currently reachable
at this IP:port." Without registration, the network cannot route incoming
calls or messages to Alice's device.

### The AKA Authentication Flow

IMS uses AKA (Authentication and Key Agreement), a challenge-response
protocol defined in 3GPP TS 33.203. The HSS stores a shared secret (derived
from the SIM's Ki key) and generates authentication vectors (AV). When the
UE registers, the S-CSCF challenges it with a RAND+AUTN pair. The UE uses
its SIM to compute the response (RES) and session keys.

This is a significant improvement over plain HTTP Digest: the challenge is
generated by the HSS (derived from the SIM), so the authentication is
**mutual** — both the network authenticates the UE and the UE authenticates
the network. A rogue base station cannot pretend to be the carrier's IMS core.

```
SIP REGISTER Flow (complete)
══════════════════════════════════════════════════════════════════════════

  UE (Alice's phone)           P-CSCF          S-CSCF           HSS
       │                          │                │               │
       │                          │                │               │
  (1) │──── SIP REGISTER ────────►│                │               │
       │  (no Authorization)      │                │               │
       │                          │                │               │
       │                          │──REGISTER─────►│               │
       │                          │                │               │
       │                          │                │──Cx: MAR─────►│
       │                          │                │ (fetch auth   │
       │                          │                │  vectors)     │
       │                          │                │◄─Cx: MAA─────│
       │                          │                │ (AKA vector:  │
       │                          │                │  RAND, AUTN,  │
       │                          │                │  XRES, CK, IK)│
       │                          │                │               │
  (2) │◄─ 401 Unauthorized ──────│◄───────────────│               │
       │  WWW-Authenticate:        │                │               │
       │  Digest realm=...,        │                │               │
       │  algorithm=AKAv1-MD5,     │                │               │
       │  nonce=<RAND+AUTN>        │                │               │
       │                          │                │               │
  (3) │ UE SIM computes RES,      │                │               │
       │ derives CK and IK for    │                │               │
       │ IPSec SA establishment   │                │               │
       │                          │                │               │
  (4) │──── SIP REGISTER ────────►│                │               │
       │  Authorization:           │                │               │
       │  Digest username=...,     │                │               │
       │  nonce=..., response=RES  │                │               │
       │                          │                │               │
       │                          │──REGISTER─────►│               │
       │                          │                │ verify RES==  │
       │                          │                │ XRES → OK     │
       │                          │                │               │
       │                          │                │──Cx: SAR─────►│
       │                          │                │ (save binding)│
       │                          │                │◄─Cx: SAA─────│
       │                          │                │               │
  (5) │◄─ 200 OK ────────────────│◄───────────────│               │
       │  P-Associated-URI:        │                │               │
       │  Service-Route: ...       │                │               │
       │  Expires: 3600            │                │               │
       │                          │                │               │
  (6) Alice is now registered.    │                │               │
      Network can route to her.   │                │               │
```

### SIP REGISTER Wire Example

**Step 1: Initial REGISTER (no credentials)**

```
REGISTER sip:ims.example-carrier.com SIP/2.0
Via: SIP/2.0/TLS 10.1.2.3:5061;branch=z9hG4bK7734a
Max-Forwards: 70
From: <sip:alice@ims.example-carrier.com>;tag=1928301774
To: <sip:alice@ims.example-carrier.com>
Call-ID: a84b4c76e66710@10.1.2.3
CSeq: 1 REGISTER
Contact: <sip:alice@10.1.2.3:5061;transport=tls>
P-Preferred-Identity: <sip:alice@ims.example-carrier.com>
P-Access-Network-Info: 3GPP-E-UTRAN-FDD;utran-cell-id-3gpp=0123456789ABCDEF
User-Agent: ExamplePhone/1.0 RCS-UP/2.3
Supported: path, gruu, outbound, urn:ietf:params:sip:option-tag:sec-agree
Expires: 3600
Content-Length: 0

```

**Step 2: 401 Unauthorized (IMS AKA challenge)**

```
SIP/2.0 401 Unauthorized
Via: SIP/2.0/TLS 10.1.2.3:5061;branch=z9hG4bK7734a
From: <sip:alice@ims.example-carrier.com>;tag=1928301774
To: <sip:alice@ims.example-carrier.com>;tag=9882731
Call-ID: a84b4c76e66710@10.1.2.3
CSeq: 1 REGISTER
WWW-Authenticate: Digest
  realm="ims.example-carrier.com",
  algorithm=AKAv1-MD5,
  nonce="6c9f47cb5f7f1914932571e8c7a5a6af",
  qop="auth"
Content-Length: 0

```

The `nonce` encodes the AKA RAND and AUTN values. The UE passes this to
the SIM which computes the response.

**Step 3: REGISTER with credentials**

```
REGISTER sip:ims.example-carrier.com SIP/2.0
Via: SIP/2.0/TLS 10.1.2.3:5061;branch=z9hG4bK7734b
Max-Forwards: 70
From: <sip:alice@ims.example-carrier.com>;tag=1928301774
To: <sip:alice@ims.example-carrier.com>
Call-ID: a84b4c76e66710@10.1.2.3
CSeq: 2 REGISTER
Contact: <sip:alice@10.1.2.3:5061;transport=tls>;expires=3600
Authorization: Digest
  username="alice@ims.example-carrier.com",
  realm="ims.example-carrier.com",
  nonce="6c9f47cb5f7f1914932571e8c7a5a6af",
  uri="sip:ims.example-carrier.com",
  response="a3d5cbf9e8f34a12bec6d7801e4f29bb",
  algorithm=AKAv1-MD5,
  qop=auth, nc=00000001,
  cnonce="0a4f113b"
Security-Client: ipsec-3gpp;
  alg=hmac-sha-1-96;
  ealg=aes-cbc;
  spi-c=12345678;spi-s=87654321;
  port-c=5100;port-s=5101
Expires: 3600
Content-Length: 0

```

**Step 4: 200 OK (registered)**

```
SIP/2.0 200 OK
Via: SIP/2.0/TLS 10.1.2.3:5061;branch=z9hG4bK7734b
From: <sip:alice@ims.example-carrier.com>;tag=1928301774
To: <sip:alice@ims.example-carrier.com>;tag=9882731
Call-ID: a84b4c76e66710@10.1.2.3
CSeq: 2 REGISTER
Contact: <sip:alice@10.1.2.3:5061;transport=tls>;expires=3600
P-Associated-URI: <sip:alice@ims.example-carrier.com>,
  <tel:+14085551234>
Service-Route: <sip:orig@scscf.ims.example-carrier.com;lr>
Security-Server: ipsec-3gpp;
  alg=hmac-sha-1-96;
  ealg=aes-cbc;
  spi-c=87654321;spi-s=12345678;
  port-c=5101;port-s=5100
P-Charging-Function-Addresses: ccf=ccf.example-carrier.com
Expires: 3600
Content-Length: 0

```

### Key SIP Headers for RCS/IMS

```
Header                    Description
────────────────────────────────────────────────────────────────────────
P-Associated-URI          Lists all public user identities (SIP URIs and
                          tel: URIs) associated with this registration.
                          Alice might have sip:alice@carrier.com AND
                          tel:+14085551234. Both are in PAU.

P-Preferred-Identity      The identity the UE prefers to use when
                          originating calls. The P-CSCF validates it
                          against the PAU list and inserts P-Asserted-
                          Identity in the forwarded message.

P-Asserted-Identity       Added by the P-CSCF after validating the
                          user's identity. Trusted by elements inside
                          the IMS core. Removed at the network border.

P-Access-Network-Info     Information about the access network (LTE cell
                          ID, WiFi SSID). Used for lawful intercept and
                          charging.

Service-Route             A set of SIP proxies the UE must route through
                          for outbound requests within the same
                          registration. Stored by the UE.

Security-Client           IKE parameters for establishing an IPSec SA
(Security-Server)         between the UE and P-CSCF. Protects SIP
                          signaling at Layer 3.
```

### Re-registration

The `Expires` value (typically 3600 seconds) is a lease. The UE must
re-register before expiry or the binding is deleted. Recommended practice
is to re-register at roughly 600 seconds before expiry (T_re = Expires - 600).

```
Re-registration timeline:
  t=0      REGISTER → 200 OK, Expires: 3600
  t=3000   REGISTER again (600 seconds before expiry)
  t=3000   200 OK, Expires: 3600 (binding renewed)
  ...      (repeats indefinitely while UE is powered on)
```

If the UE loses network connectivity and misses re-registration, the binding
expires. When connectivity is restored, the UE performs a fresh REGISTER.

## Capabilities Exchange

RCS clients must discover whether the other party supports RCS before
attempting an MSRP session. Nobody wants to see a raw SIP INVITE rejected
because the other phone doesn't know what MSRP is.

### SIP OPTIONS for Capability Discovery

```
Capability Discovery Flow
══════════════════════════════════════════════════════════════════════════

  Alice's Phone                          Bob's Phone
        │                                     │
        │──── SIP OPTIONS ──────────────────►│
        │  Accept-Contact:                    │
        │    *;+g.3gpp.iari-ref="             │
        │    urn%3Aurn-7%3A3gpp-application. │
        │    ims.iari.rcse-ip-msg"            │
        │                                     │
        │◄─── 200 OK ────────────────────────│
        │  Contact: <sip:bob@10.4.5.6>        │
        │  Feature-Caps:                      │
        │    +g.3gpp.iari-ref="..."           │
        │  Accept: message/cpim,              │
        │    text/plain, image/jpeg,          │
        │    video/mp4                        │
        │                                     │
  Alice now knows Bob supports RCS.
  She can send an MSRP INVITE.
```

### Feature Tags (IARI References)

Feature tags are encoded in the `Contact` and `Feature-Caps` headers using
URNs registered with the GSMA's IARI (IMS Application Reference Identifier)
registry:

```
Feature Tag URN                                      Meaning
─────────────────────────────────────────────────────────────────────────
urn:urn-7:3gpp-application.ims.iari.rcse-ip-msg     1:1 Chat (MSRP)
urn:urn-7:3gpp-application.ims.iari.rcse-ft         File Transfer (MSRP)
urn:urn-7:3gpp-application.ims.iari.gsma.rcs.fthttp File Transfer (HTTP)
urn:urn-7:3gpp-application.ims.iari.rcs.chatbot     Chatbot messaging
urn:urn-7:3gpp-application.ims.iari.rcs.geopush     Geolocation push
urn:urn-7:3gpp-application.ims.iari.rcs.vemoticon   Visual voicemail
```

The encoded form URL-encodes the colons: `urn%3Aurn-7%3A3gpp-application...`
This is because the feature tag appears as a quoted string inside a SIP
header value, and colons could confuse parsers.

## MSRP: Message Session Relay Protocol

MSRP (RFC 4975) is the workhorse of RCS messaging. It runs over TCP (or TLS)
and delivers messages after SIP has set up the session.

**Analogy:** SIP is like the phone system's dial-up procedure — it rings the
other party, negotiates codec preferences, and establishes the call. MSRP is
like the actual voice channel once the call is connected, but for text/file
data instead of audio.

### MSRP URI

Every MSRP endpoint is identified by an MSRP URI:

```
msrp://10.1.2.3:9000/abc123xyz;tcp
  │      │       │    │          │
  │      │       │    │          └── transport (tcp or tls)
  │      │       │    └─────────── session token (random, unique per session)
  │      │       └──────────────── port number
  │      └──────────────────────── host (IP or FQDN)
  └─────────────────────────────── scheme (msrp or msrps for TLS)

msrps://relay.carrier.com:2855/def456uvw;tcp
  │── msrps = MSRP over TLS (secure)
```

The session token is a random string chosen when the session is created. It
is included in every MSRP frame to tie the frame to a specific session.

### MSRP Frame Format

Every MSRP message is called a **frame** (or **chunk**). The format is:

```
MSRP Frame Wire Format
══════════════════════════════════════════════════════════════════════════

  ┌────────────────────────────────────────────────────────────────────┐
  │ MSRP <transaction-id> <method|status-code>\r\n                     │
  │ <Header-Name>: <header-value>\r\n                                  │
  │ [more headers]\r\n                                                 │
  │ \r\n                                                               │
  │ [message body]\r\n                                                 │
  │ -------<transaction-id><continuation-flag>\r\n                     │
  └────────────────────────────────────────────────────────────────────┘

Where:
  <transaction-id>     Random alphanumeric string identifying this
                       transaction. Same string appears in the start line
                       and the end boundary.

  <method>             SEND (client→server or client→client message)
                       REPORT (delivery/read receipt notification)
                       AUTH (MSRP relay authentication)

  <status-code>        3-digit code in responses (200 OK, 400 Bad Request,
                       413 Request Entity Too Large, etc.)

  <continuation-flag>  One of three characters:
                       $  — final chunk (complete message)
                       +  — more chunks follow (large file split)
                       #  — abort (stop sending this message)

End boundary format:
  "-------" (7 dashes) + transaction-id + continuation-flag + \r\n
```

### MSRP Headers

```
Header           Direction   Description
────────────────────────────────────────────────────────────────────────
To-Path          SEND/AUTH   MSRP URI(s) of the destination. May list
                             multiple URIs if relays are in the path
                             (each relay peels off one URI).
From-Path        SEND        MSRP URI of the sender. The receiver uses
                             this to send REPORT frames back.
Message-ID       SEND        Unique identifier for this message (used
                             to correlate with REPORT receipts).
Byte-Range        SEND       Start-End/Total bytes. Essential for
                             chunked file transfer.
                             Format: "1-*/*" for unknown total size,
                             "1-8192/102400" for first 8 KB of 100 KB.
Content-Type     SEND        MIME type of the body:
                             message/cpim — CPIM-wrapped message
                             text/plain   — plain text (rarely used bare)
                             image/jpeg   — inline image
                             application/octet-stream — file transfer
Status           REPORT      The result being reported.
Failure-Report   SEND        "yes", "no", or "partial" — whether the
                             sender wants failure REPORT frames.
Success-Report   SEND        "yes" or "no" — whether success (delivery)
                             REPORT frames are desired.
```

### SDP Negotiation for MSRP

Before MSRP can start, SIP negotiates the session parameters using SDP in
the INVITE body. SDP is a simple key=value format:

```
Alice's SDP in the SIP INVITE body (offer)
══════════════════════════════════════════

v=0
o=alice 2890844526 2890844526 IN IP4 10.1.2.3
s=Chat Session
c=IN IP4 10.1.2.3
t=0 0
m=message 9000 TCP/TLS/MSRP *
a=accept-types:message/cpim text/plain image/jpeg image/png
a=accept-wrapped-types:text/plain
a=setup:active
a=path:msrps://10.1.2.3:9000/abc123xyz;tcp
a=sendrecv

Bob's SDP in the 200 OK body (answer)
══════════════════════════════════════

v=0
o=bob 2890844730 2890844730 IN IP4 10.4.5.6
s=Chat Session
c=IN IP4 10.4.5.6
t=0 0
m=message 9001 TCP/TLS/MSRP *
a=accept-types:message/cpim text/plain image/jpeg image/png
a=accept-wrapped-types:text/plain
a=setup:passive
a=path:msrps://10.4.5.6:9001/xyz789abc;tcp
a=sendrecv
```

```
SDP fields for MSRP
────────────────────────────────────────────────────────────────────────
Field                       Meaning
────────────────────────────────────────────────────────────────────────
m=message <port> TCP/TLS/MSRP *
                            Media line: "message" type, MSRP over TLS.
                            Port is the MSRP listening port.
a=accept-types             MIME types this endpoint can receive.
a=setup:active             This endpoint opens the TCP connection.
a=setup:passive            This endpoint listens for the connection.
                           (The active side connects to the passive side.)
a=path:msrps://...         The MSRP URI for this endpoint.
a=sendrecv                 Bidirectional (both can send and receive).
```

**The setup:active/passive negotiation:** Exactly one side must be TCP client
(active) and one must be TCP server (passive). The SDP offer includes
`setup:active` or `setup:actpass`. If the offer is `actpass`, the answerer
may choose `active` or `passive`. This avoids both sides trying to connect
simultaneously (which would create a stalemate).

## Complete Message Flows

### Flow 1: 1:1 Messaging (Session Mode)

```
Complete 1:1 RCS Message Flow
══════════════════════════════════════════════════════════════════════════

  Alice                    IMS Core                    Bob
    │                         │                          │
    │                         │                          │
    │  SIP INVITE             │                          │
    │  (SDP: MSRP offer)      │                          │
    │ ────────────────────── ►│────────────────────────►│
    │                         │                          │
    │                         │         SIP 180 Ringing │
    │                         │◄────────────────────────│
    │◄──────────────────────  │                          │
    │                         │                          │
    │                         │         SIP 200 OK       │
    │                         │         (SDP: MSRP ans.) │
    │                         │◄────────────────────────│
    │◄──────────────────────  │                          │
    │                         │                          │
    │  SIP ACK                │                          │
    │ ────────────────────── ►│────────────────────────►│
    │                         │                          │
    │                         │   (SIP signaling done)   │
    │                         │                          │
    │ ═══════════ MSRP TCP connection established ══════ │
    │ (Alice connects to Bob's MSRP endpoint directly    │
    │  or via a relay if NAT prevents direct connection) │
    │                         │                          │
    │  MSRP SEND "Hello Bob!" │                          │
    │ ═════════════════════════════════════════════════►│
    │                         │                          │
    │  MSRP 200 OK            │                          │
    │◄═════════════════════════════════════════════════  │
    │  (delivery acknowledgment from MSRP layer)         │
    │                         │                          │
    │  MSRP REPORT            │                          │
    │◄═════════════════════════════════════════════════  │
    │  (read receipt: Bob has displayed the message)     │
    │                         │                          │
    │                         │                          │
    │  SIP BYE                │                          │
    │ ────────────────────── ►│────────────────────────►│
    │                         │                          │
    │  SIP 200 OK             │                          │
    │◄──────────────────────  │                          │
    │ (MSRP session closed)   │                          │
```

### MSRP SEND Wire Example

Alice sends "Hello Bob!" to Bob. This is a complete MSRP SEND frame:

```
MSRP Transaction Wire Format
══════════════════════════════════════════════════════════════════════════

Bytes on the wire (with annotations):

  4D 53 52 50 20          "MSRP "
  61 32 62 35 66 33 20    "a2b5f3 "      ← transaction ID
  53 45 4E 44 0D 0A       "SEND\r\n"     ← method

  54 6F 2D 50 61 74 68 3A 20      "To-Path: "
  6D 73 72 70 73 3A 2F 2F        "msrps://"
  31 30 2E 34 2E 35 2E 36 3A 39 30 30 31 2F
                                  "10.4.5.6:9001/"
  78 79 7A 37 38 39 61 62 63     "xyz789abc"
  3B 74 63 70 0D 0A              ";tcp\r\n"

  46 72 6F 6D 2D 50 61 74 68 3A 20    "From-Path: "
  6D 73 72 70 73 3A 2F 2F             "msrps://"
  31 30 2E 31 2E 32 2E 33 3A 39 30 30 30 2F
                                       "10.1.2.3:9000/"
  61 62 63 31 32 33 78 79 7A          "abc123xyz"
  3B 74 63 70 0D 0A                   ";tcp\r\n"

  4D 65 73 73 61 67 65 2D 49 44 3A 20 "Message-ID: "
  6D 73 67 2D 31 30 30 31 0D 0A      "msg-1001\r\n"

  42 79 74 65 2D 52 61 6E 67 65 3A 20 "Byte-Range: "
  31 2D 2A 2F 2A 0D 0A               "1-*/*\r\n"

  46 61 69 6C 75 72 65 2D 52 65 70 6F 72 74 3A 20
                                       "Failure-Report: "
  79 65 73 0D 0A                      "yes\r\n"

  53 75 63 63 65 73 73 2D 52 65 70 6F 72 74 3A 20
                                       "Success-Report: "
  79 65 73 0D 0A                      "yes\r\n"

  43 6F 6E 74 65 6E 74 2D 54 79 70 65 3A 20
                                       "Content-Type: "
  6D 65 73 73 61 67 65 2F 63 70 69 6D "message/cpim"
  0D 0A                               "\r\n"

  0D 0A                               "\r\n"  ← blank line = header/body boundary

  [CPIM body — see below]

  2D 2D 2D 2D 2D 2D 2D               "-------"  ← 7 dashes
  61 32 62 35 66 33                   "a2b5f3"   ← transaction ID
  24 0D 0A                           "$\r\n"     ← final chunk ($)
```

In human-readable form:

```
MSRP a2b5f3 SEND
To-Path: msrps://10.4.5.6:9001/xyz789abc;tcp
From-Path: msrps://10.1.2.3:9000/abc123xyz;tcp
Message-ID: msg-1001
Byte-Range: 1-*/*
Failure-Report: yes
Success-Report: yes
Content-Type: message/cpim

From: <sip:alice@ims.example-carrier.com>
To: <sip:bob@ims.example-carrier.com>
DateTime: 2026-04-22T14:30:00.000Z
Content-Type: text/plain; charset=UTF-8

Hello Bob!
-------a2b5f3$
```

**MSRP 200 OK response (from Bob's MSRP stack):**

```
MSRP a2b5f3 200 OK
To-Path: msrps://10.1.2.3:9000/abc123xyz;tcp
From-Path: msrps://10.4.5.6:9001/xyz789abc;tcp
-------a2b5f3$
```

### Flow 2: Pager Mode (SIP MESSAGE)

For very short messages (under ~1300 bytes) when establishing a full MSRP
session would be wasteful, RCS can use **pager mode** — a single SIP MESSAGE
request carrying the message body inline.

```
Pager Mode: SIP MESSAGE
══════════════════════════════════════════════════════════════════════════

  Alice                    IMS Core                    Bob
    │                         │                          │
    │  SIP MESSAGE            │                          │
    │  Content-Type: msg/cpim │                          │
    │  Body: "Hey!"           │                          │
    │ ────────────────────── ►│────────────────────────►│
    │                         │                          │
    │                         │          SIP 200 OK      │
    │                         │◄────────────────────────│
    │◄──────────────────────  │                          │

No TCP connection for MSRP needed. One round trip.
Tradeoff: no typing indicators, no delivery reports,
          message size limited by SIP body limits (~1 MB in practice
          but carriers often cap at 1300 bytes to fit in a single
          IP packet without fragmentation).
```

Wire example:

```
MESSAGE sip:bob@ims.example-carrier.com SIP/2.0
Via: SIP/2.0/TLS 10.1.2.3:5061;branch=z9hG4bKpager01
Max-Forwards: 70
From: <sip:alice@ims.example-carrier.com>;tag=8765
To: <sip:bob@ims.example-carrier.com>
Call-ID: pager-9900@10.1.2.3
CSeq: 1 MESSAGE
P-Preferred-Identity: <sip:alice@ims.example-carrier.com>
Content-Type: message/cpim
Content-Length: 218

From: <sip:alice@ims.example-carrier.com>
To: <sip:bob@ims.example-carrier.com>
DateTime: 2026-04-22T14:35:00.000Z
Content-Type: text/plain; charset=UTF-8

Hey!
```

### Flow 3: Group Chat (Conference)

RCS group chat uses a **conference focus server** (MCF — Multi-party
Conference Function) operated by the carrier. The MCF fans out messages
from one sender to all group participants.

```
Group Chat Architecture
══════════════════════════════════════════════════════════════════════════

  Alice ──── MSRP SEND ────►  MCF  ──── MSRP SEND ────► Bob
                              (Multi-party           │
                               Conference            └─── MSRP SEND ────► Carol
                               Function)
                              sip:conf-xyz@mcf.carrier.com

Each participant has an individual MSRP session with the MCF.
The MCF distributes each message to all other members.
```

**Joining a group chat:**

Alice creates a group chat by sending a SIP INVITE with a `resource-list`
body listing the invitees:

```
SIP INVITE to MCF (group creation)
══════════════════════════════════════════════════════════════════════════

INVITE sip:conf-factory@mcf.ims.example-carrier.com SIP/2.0
...standard headers...
Require: recipient-list-invite
Content-Type: multipart/mixed;boundary="boundary1"

--boundary1
Content-Type: application/sdp

v=0
o=alice 2890844526 1 IN IP4 10.1.2.3
...MSRP SDP offer...

--boundary1
Content-Type: application/resource-lists+xml
Content-Disposition: recipient-list

<?xml version="1.0" encoding="UTF-8"?>
<resource-lists xmlns="urn:ietf:params:xml:ns:resource-lists">
  <list>
    <entry uri="sip:bob@ims.example-carrier.com"/>
    <entry uri="sip:carol@ims.example-carrier.com"/>
  </list>
</resource-lists>
--boundary1--
```

The MCF:
1. Creates a conference room with a unique SIP URI (e.g.,
   `sip:conf-abc123@mcf.carrier.com`).
2. Sends the 200 OK to Alice with the MCF's MSRP path.
3. Sends SIP INVITE to Bob and Carol on Alice's behalf.
4. Once all parties join, the MCF bridges all MSRP sessions.

## Rich Messaging Features

### Read Receipts

CPIM (RFC 3860) wraps message bodies and carries disposition notifications.
The sender requests a read receipt by setting `Success-Report: yes` in the
MSRP SEND. When Bob's client displays the message, it sends an MSRP REPORT:

```
MSRP b3c6d9 REPORT
To-Path: msrps://10.1.2.3:9000/abc123xyz;tcp
From-Path: msrps://10.4.5.6:9001/xyz789abc;tcp
Message-ID: msg-1001
Byte-Range: 1-10/10
Status: 000 200 OK
-------b3c6d9$
```

The `Status: 000 200 OK` line is MSRP's status format (namespace 000 = MSRP
namespace, 200 OK = success). The absence of a body in REPORT frames is
intentional — they are pure control messages.

### Typing Indicators

Typing indicators use **SIP INFO** sent in-dialog (after the INVITE/200/ACK):

```
INFO sip:bob@10.4.5.6;transport=tls SIP/2.0
...in-dialog headers (same Call-ID, CSeq incremented)...
Content-Type: application/im-iscomposing+xml
Content-Length: 159

<?xml version="1.0" encoding="UTF-8"?>
<isComposing xmlns="urn:ietf:params:xml:ns:im-iscomposing">
  <state>active</state>
  <contenttype>text/plain</contenttype>
  <refresh>60</refresh>
</isComposing>
```

`<state>active</state>` means "currently typing." `<state>idle</state>`
means "stopped typing." The `<refresh>` element tells the receiver how
often to expect updates — if no update arrives within `refresh` seconds,
assume the user is no longer typing.

### File Transfer

File transfer uses MSRP with chunking. For a 100 KB file:

```
MSRP t1 SEND
To-Path: msrps://10.4.5.6:9001/xyz789abc;tcp
From-Path: msrps://10.1.2.3:9000/abc123xyz;tcp
Message-ID: file-transfer-001
Content-Type: application/octet-stream
Content-Disposition: attachment; filename="photo.jpg"
Byte-Range: 1-8192/102400

[first 8192 bytes of photo.jpg]
-------t1+
```

Note `+` (not `$`) — more chunks follow. The next chunk:

```
MSRP t2 SEND
To-Path: msrps://10.4.5.6:9001/xyz789abc;tcp
From-Path: msrps://10.1.2.3:9000/abc123xyz;tcp
Message-ID: file-transfer-001
Byte-Range: 8193-16384/102400

[next 8192 bytes]
-------t2+
```

...continuing until the final chunk ends with `$`. Each chunk uses a new
transaction ID but the same `Message-ID` to link all chunks to the same
transfer. The `Byte-Range` header tells the receiver the exact position of
each chunk within the total file.

**File info is communicated in the SIP INVITE SDP:**

```
a=file-transfer-id:file-transfer-001
a=file-disposition:attachment
a=file-size:102400
a=file-name:photo.jpg
a=file-type:image/jpeg
a=file-hash:sha256:abc123def456...
```

The receiver can show progress: it knows the total size (102400 bytes) and
counts received bytes from `Byte-Range` values.

### Geolocation Sharing

Geolocation is shared as a CPIM-wrapped message with Content-Type
`application/vnd.gsma.rcspushlocation+xml`:

```
Content-Type: application/vnd.gsma.rcspushlocation+xml

<?xml version="1.0" encoding="UTF-8"?>
<rcspushlocation xmlns="urn:gsma:params:xml:ns:rcspushlocation"
                 label="I'm here!">
  <ll>
    <latitude>37.422408</latitude>
    <longitude>-122.085073</longitude>
    <accuracy>50</accuracy>
  </ll>
</rcspushlocation>
```

### RCS Chatbots (Universal Profile 2.4+)

Chatbots are RCS endpoints operated by businesses. The GSMA's RCS Business
Messaging (RBM) specification defines a REST API that businesses use to send
interactive messages: cards, carousels, and suggested actions.

```
Bot Card Message Structure
══════════════════════════════════════════════════════════════════════════

MSRP SEND carrying:
Content-Type: application/vnd.gsma.botmessage.v1.0+json

{
  "message": {
    "generalPurposeCard": {
      "layout": {
        "cardOrientation": "VERTICAL",
        "imageAlignment": "LEFT"
      },
      "content": {
        "title": "Flight BA245",
        "description": "Your flight departs at 14:30 from Gate B12",
        "media": {
          "height": "MEDIUM",
          "file": {
            "url": "https://cdn.example.com/flight-card-image.jpg"
          }
        },
        "suggestions": [
          {
            "reply": {
              "text": "Check In",
              "postbackData": "checkin:BA245"
            }
          },
          {
            "action": {
              "text": "Get Directions",
              "postbackData": "directions:LHR-B12",
              "mapAction": {
                "query": "Heathrow Terminal 5 Gate B12"
              }
            }
          }
        ]
      }
    }
  }
}
```

## Security

### TLS for MSRP Transport

MSRP sessions use `msrps://` URIs (MSRP Secure) which run MSRP over TLS.
The TLS handshake occurs when the active side opens the TCP connection to
the passive side. The certificate is that of the MSRP relay (if a relay
is used) or the peer (if direct P2P connection, using self-signed certs
verified by the SIP signaling layer).

### SIP over TLS

All SIP signaling between the UE and P-CSCF uses TLS. The `Via` header
uses `SIP/2.0/TLS` and contact URIs use `sips:` scheme. Port 5061 is the
standard TLS port for SIP.

### IMS-AKA Authentication

AKA (3GPP TS 33.203) provides mutual authentication between the UE and the
IMS network. The shared secret is derived from the SIM card's Ki value, which
never leaves the SIM or the HSS. This means:

- The password is never transmitted over the air in any form.
- Each authentication challenge is unique (fresh RAND each time).
- The UE authenticates the network (verifies AUTN), preventing rogue base
  station attacks.
- Session keys (CK, IK) are derived and used to establish IPSec Security
  Associations for subsequent SIP messages.

```
AKA Security Properties
══════════════════════════════════════════════════════════════════════════

  Property                How it's achieved
  ─────────────────────────────────────────────────────────────────────
  UE authentication       S-CSCF verifies RES == XRES (from HSS)
  Network authentication  UE verifies AUTN using SIM Ki
  Replay protection       SQN sequence number inside AUTN prevents
                          replay of captured challenges
  Session key derivation  CK and IK derived from Ki and RAND;
                          used for IPSec SA between UE and P-CSCF
  Confidentiality         IPSec ESP encrypts SIP signaling
  Integrity               IPSec AH/ESP protects against tampering
```

## Algorithms

### SIP Registration State Machine

```
state = UNREGISTERED

procedure register():
    send REGISTER(expires=3600, no_auth=true)
    response = await_response()

    if response.status == 401:
        # IMS AKA challenge
        nonce = response.www_authenticate.nonce
        (rand, autn) = decode_aka_nonce(nonce)
        (res, ck, ik) = sim_aka_response(rand, autn)
        # If AUTN invalid → network authentication failure → abort
        if res is ERROR:
            state = AUTH_FAILED
            return

        establish_ipsec_sa(ck, ik)
        auth_header = build_digest_auth(res, nonce)
        send REGISTER(expires=3600, authorization=auth_header)
        response = await_response()

    if response.status == 200:
        state = REGISTERED
        expires = response.contact.expires
        p_associated_uri = response.p_associated_uri
        service_route = response.service_route
        schedule_reregister(at = now() + expires - 600)

    else:
        state = FAILED
        schedule_retry(after = 30s)
```

### MSRP Session Setup (Offer/Answer)

```
procedure setup_msrp_session(remote_sdp):
    local_port = allocate_tcp_port()
    local_token = random_base64(12)
    local_uri = "msrps://" + local_ip + ":" + local_port +
                "/" + local_token + ";tcp"

    if remote_sdp.setup == "active":
        local_setup = "passive"
        # Wait for incoming TCP connection
        conn = accept_tcp(local_port)
    elif remote_sdp.setup == "passive":
        local_setup = "active"
        # Connect to remote
        conn = connect_tcp(remote_sdp.path.host, remote_sdp.path.port)
    else:  # actpass
        local_setup = "active"
        conn = connect_tcp(remote_sdp.path.host, remote_sdp.path.port)

    tls_conn = wrap_tls(conn)
    session = MsrpSession(
        conn = tls_conn,
        to_path = remote_sdp.path,
        from_path = local_uri,
    )
    return session
```

### MSRP SEND

```
procedure msrp_send(session, content_type, body):
    transaction_id = random_alphanum(10)
    message_id = "msg-" + monotonic_id()
    body_bytes = encode_utf8(body)
    byte_range = "1-" + len(body_bytes) + "/" + len(body_bytes)

    frame = join_lines([
        "MSRP " + transaction_id + " SEND",
        "To-Path: " + session.to_path,
        "From-Path: " + session.from_path,
        "Message-ID: " + message_id,
        "Byte-Range: " + byte_range,
        "Failure-Report: yes",
        "Success-Report: yes",
        "Content-Type: " + content_type,
        "",
        body,
        "-------" + transaction_id + "$",
    ])

    session.conn.write(frame)
    ack = await_msrp_response(transaction_id)
    if ack.status != 200:
        raise MsrpError(ack.status)
    return message_id

procedure msrp_send_large_file(session, filename, data):
    message_id = "file-" + monotonic_id()
    total = len(data)
    chunk_size = 8192
    offset = 1

    for chunk in chunks(data, chunk_size):
        transaction_id = random_alphanum(10)
        end = offset + len(chunk) - 1
        is_last = (end == total)
        cont = "$" if is_last else "+"

        frame = join_lines([
            "MSRP " + transaction_id + " SEND",
            "To-Path: " + session.to_path,
            "From-Path: " + session.from_path,
            "Message-ID: " + message_id,
            "Content-Type: application/octet-stream",
            "Content-Disposition: attachment; filename=\"" +
                filename + "\"",
            "Byte-Range: " + offset + "-" + end + "/" + total,
            "",
            chunk,
            "-------" + transaction_id + cont,
        ])

        session.conn.write(frame)
        offset += len(chunk)
```

### CPIM Envelope Construction

```
procedure build_cpim(from_uri, to_uri, content_type, body):
    timestamp = utc_now_iso8601()
    cpim_body = join_lines([
        "From: <" + from_uri + ">",
        "To: <" + to_uri + ">",
        "DateTime: " + timestamp,
        "Content-Type: " + content_type + "; charset=UTF-8",
        "",
        body,
    ])
    return cpim_body

# Usage:
cpim = build_cpim(
    "sip:alice@ims.example-carrier.com",
    "sip:bob@ims.example-carrier.com",
    "text/plain",
    "Hello Bob!"
)
msrp_send(session, "message/cpim", cpim)
```

### SIP OPTIONS Capability Query

```
procedure query_capabilities(remote_uri):
    request = SipRequest(
        method = "OPTIONS",
        request_uri = remote_uri,
        headers = {
            "From": local_identity + ";tag=" + random_tag(),
            "To": remote_uri,
            "Call-ID": random_call_id(),
            "CSeq": "1 OPTIONS",
            "Accept-Contact": build_accept_contact(RCS_FEATURE_TAGS),
            "Accept": "application/sdp",
        }
    )
    response = send_via_ims_core(request)

    if response.status == 200:
        feature_caps = parse_feature_caps(response.headers["Feature-Caps"])
        return CapabilitySet(
            rcs_messaging = RCS_MESSAGING_TAG in feature_caps,
            file_transfer = RCS_FT_TAG in feature_caps,
            video_calling  = RCS_VIDEO_TAG in feature_caps,
        )
    elif response.status == 404:
        return CapabilitySet(rcs_supported = False)
```

## Test Strategy

### Unit Tests

**SDP Parsing**
- Parse a minimal MSRP SDP offer and verify `a=path` is extracted correctly.
- Parse SDP with multiple `a=accept-types` values.
- Verify `setup:active` / `setup:passive` / `setup:actpass` round-trips.

**MSRP Frame Encoding**
- Encode a short SEND frame; verify the byte-for-byte output matches the
  expected wire format.
- Encode a SEND with an empty body (0 bytes); verify `Byte-Range: 1-0/0`.
- Encode a chunked file in 3 chunks; verify `+` on first two and `$` on last.
- Verify the end boundary is always exactly 7 dashes + transaction ID.

**MSRP Frame Parsing**
- Parse a SEND with a CPIM body; verify all headers and body extracted.
- Parse a REPORT frame (no body); verify no body field set.
- Parse an incomplete frame (truncated before end boundary) → should return
  `NeedMoreData` error.
- Parse a frame with binary data in the body (contains `\r\n` inside body).
- Parse two concatenated MSRP frames in a single read buffer.

**CPIM Envelope**
- Build a CPIM envelope and parse it back; verify round-trip fidelity.
- Verify `DateTime` field parses correctly to a UTC timestamp.
- Verify UTF-8 bodies with multi-byte characters (emoji) are preserved.

### Integration Tests

**SIP Registration Flow**
- Mock the IMS core responding with 401 + AKA challenge, then 200 OK.
- Verify the UE correctly extracts `P-Associated-URI` and `Service-Route`.
- Verify re-registration is scheduled at `expires - 600` seconds.
- Test registration failure (wrong AKA response → 403 Forbidden).

**1:1 Chat Session**
- Set up two MSRP clients connected via an in-memory TCP pair.
- Send "Hello" from Alice; verify Bob receives the CPIM-wrapped message.
- Send a read receipt from Bob; verify Alice receives the REPORT.
- Close the session with SIP BYE; verify both sides clean up.

**Chunked File Transfer**
- Transfer a 50 KB byte sequence in 8 KB chunks.
- Verify all chunks arrive in order with correct `Byte-Range` values.
- Verify the recipient can reassemble the original 50 KB.
- Inject a chunk out of order; verify the receiver detects the gap.

**Capability Discovery**
- Mock a remote party supporting only `rcse-ip-msg` (no file transfer).
- Verify `CapabilitySet.file_transfer == False`.
- Mock a 404 response; verify `rcs_supported == False`.

**Typing Indicators**
- Send `isComposing` with `state=active`; verify the receiver triggers
  a "typing" UI state.
- After `refresh` seconds with no update, verify the receiver clears
  the "typing" state automatically.

### Protocol Conformance Tests

- Verify `CSeq` increments on each REGISTER within the same Call-ID.
- Verify `branch` parameter in `Via` header is unique per request
  (must start with magic cookie `z9hG4bK`).
- Verify MSRP end boundary uses exactly 7 dashes (not 6, not 8).
- Verify `session token` in MSRP URI is cryptographically random
  (at least 80 bits of entropy per RFC 4975 §14.2).
- Verify that a SEND with `Success-Report: no` does not trigger a REPORT.
- Verify `Failure-Report: yes` causes a REPORT on 4xx/5xx responses.

### Error Handling Tests

- IMS core returns 503 Service Unavailable → verify exponential backoff
  retry (1s, 2s, 4s, 8s, cap at 64s).
- MSRP TCP connection drops mid-transfer → verify graceful cleanup and
  session re-establishment.
- AKA AUTN verification fails (rogue network) → verify the UE logs the
  event and does NOT proceed with registration.
- File transfer abort (sender sends `#` continuation flag) → verify
  receiver discards partial file and notifies the application layer.
