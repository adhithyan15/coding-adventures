# MSG-IMESSAGE — iMessage, APNs, and IDS

## Overview

Before Apple built their push notification infrastructure in 2009, every iPhone
app that wanted real-time updates had to **poll**: wake up every 60 seconds, open
a TCP connection to the server, ask "Any new data?", get a reply, close the
connection. Multiply this by thousands of apps on millions of devices and you
have a radio that never sleeps, a battery that drains overnight, and a cellular
network that collapses under the constant connection storm.

**Analogy: APNs is a postal locker room.**
Instead of every app keeping its own mailbox open, Apple opened one centralized
locker room at the post office (APNs). Your phone maintains one single TLS
connection to Apple's locker room. When anyone wants to deliver something to any
app on your device, they tell Apple's locker room ("deliver to locker 4DF3 for
the Slack app"). Apple's locker room knocks on the existing connection and
delivers the parcel. One connection for all apps.

**Analogy: iMessage is a private postal network inside that locker room.**
When Alice sends Bob an iMessage, it travels encrypted to Apple's relay, Apple
delivers it to Bob's device via APNs, and Bob's device decrypts it. Apple's
servers see the encrypted blob but cannot read the content — provided the
encryption is working correctly.

This document covers:

1. **APNs** — the push notification delivery engine (legacy binary and HTTP/2)
2. **IDS** — the Identity Directory Service, Apple's key distribution system
3. **iMessage E2E encryption** — RSA + AES per-device encryption
4. **SMS fallback** — how iMessage detects and falls back to SMS
5. **iMessage wire format** — APNs payloads, binary plists, the Madrid topic
6. **FaceTime relay** — STUN/TURN/ICE for video calls

## Architecture

```
  ┌─────────────────────────────────────────────────────────────────────┐
  │                   iMessage / APNs System Architecture               │
  │                                                                     │
  │  Alice (iPhone + MacBook)           Bob (iPhone + iPad + MacBook)   │
  │  ┌─────────────────────┐            ┌─────────────────────────────┐ │
  │  │  Messages.app       │            │  Messages.app (3 devices)   │ │
  │  │  ┌───────────────┐  │            │  ┌─────────────────────┐    │ │
  │  │  │ iMessage      │  │            │  │ iMessage E2E layer  │    │ │
  │  │  │ E2E layer     │  │            │  │ (RSA-1280 + AES-128)│    │ │
  │  │  │(RSA-1280+AES) │  │            │  └──────────┬──────────┘    │ │
  │  │  └──────┬────────┘  │            │             │               │ │
  │  └─────────┼───────────┘            └─────────────┼───────────────┘ │
  │            │                                      │                 │
  │            ▼ TLS to APNs                          ▼ TLS to APNs    │
  │  ┌─────────────────────────────────────────────────────────────┐    │
  │  │              Apple Infrastructure                            │    │
  │  │                                                              │    │
  │  │  ┌──────────────────┐   ┌─────────────────┐                 │    │
  │  │  │  APNs            │   │  IDS             │                 │    │
  │  │  │  (Push delivery) │   │  (Key directory) │                 │    │
  │  │  │                  │   │                  │                 │    │
  │  │  │ api.push.apple.  │   │ Maps phone# and  │                 │    │
  │  │  │ com:443          │   │ Apple ID to:     │                 │    │
  │  │  │                  │   │ - encryption key │                 │    │
  │  │  │ Maintains one    │   │ - signing key    │                 │    │
  │  │  │ TCP conn per     │   │ - APNs token     │                 │    │
  │  │  │ device           │   │ per device       │                 │    │
  │  │  └──────────────────┘   └─────────────────┘                 │    │
  │  │                                                              │    │
  │  │  ┌──────────────────────────────────────────────────────┐    │    │
  │  │  │  iMessage Relay (escrow, message routing)            │    │    │
  │  │  │  Internal codename: "Madrid"                        │    │    │
  │  │  │  APNs topic: com.apple.madrid                       │    │    │
  │  │  └──────────────────────────────────────────────────────┘    │    │
  │  └─────────────────────────────────────────────────────────────┘    │
  └─────────────────────────────────────────────────────────────────────┘
```

## Key Concepts: APNs

### The Persistent Device Connection

Every iOS/macOS device maintains exactly one long-lived TLS connection to APNs.
This is the beating heart of the entire Apple push notification ecosystem.

```
  Device ↔ APNs Connection Lifecycle
  ════════════════════════════════════

  Boot:
    Device powers on.
    iOS makes TCP connection to courier.push.apple.com (legacy)
    or 17.0.0.0/8 IP range (modern, anycast APNs).
    TLS handshake: device presents device certificate.
    Apple authenticates device (is this a real, non-jailbroken Apple device?).
    Connection stays open indefinitely.

  Keep-alive:
    iOS sends a TCP keep-alive every ~10 minutes.
    If the connection drops (network change, sleep/wake cycle):
      iOS immediately reconnects and re-authenticates.

  Incoming notification:
    Apple writes to the open TLS socket.
    iOS wakes the relevant app's process (background mode).
    App calls completionHandler to update badge/content.

  Why one connection for all apps?
  ──────────────────────────────────
  Each TCP connection requires a SYN-ACK round trip (~100ms on cellular),
  TLS handshake (~200ms), and keeps a radio "context" alive.
  Modern iPhones have 50-100 apps that want push notifications.
  100 connections × 300ms setup × constant radio drain = dead battery.
  One APNs connection amortizes all of this across every app.
```

### Push Token: What It Is and How It Works

```
  Push Token Lifecycle
  ═════════════════════

  ┌────────────────────────────────────────────────────────────────────┐
  │  Step 1: App calls UIApplication.registerForRemoteNotifications()  │
  │          (on iOS) or NSApplication.registerForRemoteNotifications()│
  │          (on macOS)                                                │
  │                                                                    │
  │  Step 2: iOS makes a request to APNs:                              │
  │          "Generate a push token for this device + this bundle ID"  │
  │                                                                    │
  │  Step 3: APNs generates a 32-byte token:                           │
  │            token = HMAC(device_secret, app_bundle_id || timestamp) │
  │          This is not a random value — Apple can route notifications │
  │          to the correct device because the token encodes           │
  │          (device, app) information that only Apple can decode.     │
  │                                                                    │
  │  Step 4: APNs delivers the token to the iOS device.               │
  │                                                                    │
  │  Step 5: iOS calls the app's delegate:                             │
  │          func application(_ application: UIApplication,           │
  │            didRegisterForRemoteNotificationsWithDeviceToken        │
  │            deviceToken: Data)                                      │
  │                                                                    │
  │  Step 6: App sends token to provider (your backend server).        │
  │          The provider stores token → user mapping.                 │
  │                                                                    │
  │  Token invalidation happens when:                                  │
  │    - App is uninstalled and reinstalled                            │
  │    - User restores device from backup to a new device              │
  │    - APNs rotates the device secret (periodic, Apple-controlled)   │
  │                                                                    │
  │  Token format (32 bytes hex-encoded = 64 hex chars):               │
  │  A1B2C3D4E5F6789012345678901234567890ABCDEF1234567890ABCDEF123456 │
  └────────────────────────────────────────────────────────────────────┘
```

### APNs Legacy Binary Protocol (Historical)

The original APNs protocol (2009–2015) used a custom binary format over TCP.
Understanding it reveals the design tradeoffs that led to the HTTP/2 protocol.

```
  Legacy Binary Format: Enhanced Notification (command=0x01)
  ═══════════════════════════════════════════════════════════

  Frame layout:
  ┌──────┬──────────────┬─────────────────────────────────────────────┐
  │ 0x01 │ Frame length │ Items (variable)                             │
  │ cmd  │ 4 bytes BE   │                                             │
  │ byte │ uint32       │                                             │
  └──────┴──────────────┴─────────────────────────────────────────────┘

  Each item:
  ┌─────────┬─────────────────┬────────────────────────────────────────┐
  │ Item ID │ Item data length│ Item data                               │
  │ 1 byte  │ 2 bytes BE      │ variable                               │
  └─────────┴─────────────────┴────────────────────────────────────────┘

  Item IDs:
  ┌─────────┬──────────────────────────────────────────────────────────┐
  │ Item ID │ Description                                              │
  ├─────────┼──────────────────────────────────────────────────────────┤
  │ 1       │ Device token. 32 bytes.                                  │
  ├─────────┼──────────────────────────────────────────────────────────┤
  │ 2       │ Payload. JSON-encoded string. Max 256 bytes (later 2KB). │
  ├─────────┼──────────────────────────────────────────────────────────┤
  │ 3       │ Notification identifier. 4-byte uint32. Provider's       │
  │         │ unique ID — echoed back in error responses.              │
  ├─────────┼──────────────────────────────────────────────────────────┤
  │ 4       │ Expiration date. 4-byte uint32 UNIX timestamp.           │
  │         │ Notification is discarded if not delivered before this.  │
  ├─────────┼──────────────────────────────────────────────────────────┤
  │ 5       │ Priority. 1 byte: 10 = immediate, 5 = power-saving.      │
  └─────────┴──────────────────────────────────────────────────────────┘

  Complete wire example — send "New message from Alice" to a device:
  ───────────────────────────────────────────────────────────────────

  Hex (annotated):
  01                   command = enhanced notification
  00 00 00 87          frame length = 135 bytes (total of all items below)

  01                   item ID = device token
  00 20                item length = 32 bytes
  A1B2C3D4 E5F6A1B2    device token (32 bytes)
  C3D4E5F6 A1B2C3D4
  E5F6A1B2 C3D4E5F6
  A1B2C3D4 E5F6A1B2

  02                   item ID = payload
  00 5B                item length = 91 bytes
  7B226170 73223A7B    {"aps":{"alert":"New message from Alice","badge":1,"sound":"default"}}
  22616C65 72...       (JSON payload, 91 bytes)

  03                   item ID = notification ID
  00 04                item length = 4 bytes
  00 00 01 23          notification ID = 291

  04                   item ID = expiration
  00 04                item length = 4 bytes
  67 8A BC DE          UNIX timestamp (1 hour from now)

  05                   item ID = priority
  00 01                item length = 1 byte
  0A                   priority = 10 (immediate delivery)

  Error response (command 0x08):
  ──────────────────────────────
  08                   command = error response
  06                   status code
  00 00 01 23          notification ID that failed (echoed from item 3)

  Status codes:
  ┌──────┬──────────────────────────────────────────────────────────────┐
  │ Code │ Meaning                                                      │
  ├──────┼──────────────────────────────────────────────────────────────┤
  │ 0    │ No errors                                                    │
  │ 1    │ Processing error                                             │
  │ 2    │ Missing device token                                         │
  │ 3    │ Missing topic (bundle ID)                                    │
  │ 4    │ Missing payload                                              │
  │ 5    │ Invalid token size                                           │
  │ 6    │ Invalid topic size                                           │
  │ 7    │ Invalid payload size (over limit)                            │
  │ 8    │ Invalid token (bad token — app uninstalled)                  │
  │ 255  │ Unknown error                                                │
  └──────┴──────────────────────────────────────────────────────────────┘

  Why was the payload limit only 256 bytes?
  ──────────────────────────────────────────
  The original APNs was designed in 2009 when:
  - 3G networks had ~1 Mbps throughput
  - iPhone 3G had 128 MB RAM
  - Most push notifications were alerts: "You have 3 new emails"
  256 bytes was generous for an alert text + badge count + sound name.
  As richer notifications (actionable buttons, media) were added, the
  limit grew to 2KB (2012), then 4KB with HTTP/2.

  The Feedback Service (legacy):
  ───────────────────────────────
  When a device unregisters from APNs (app deleted), Apple doesn't
  immediately tell every provider. Instead, providers periodically
  poll the Feedback Service (feedback.push.apple.com:2196).

  Response format:
  ┌────────────────────┬──────────────┬────────────────────────────────┐
  │ Timestamp (4 byte) │ Token length │ Device token                   │
  │ UNIX uint32        │ (2 byte)     │ (variable, usually 32 bytes)   │
  └────────────────────┴──────────────┴────────────────────────────────┘

  Provider should: remove the token from their database and stop sending.
```

### APNs HTTP/2 Protocol (Modern)

Apple replaced the binary protocol with HTTP/2 in 2015. HTTP/2 has built-in
multiplexing (many requests on one connection), flow control, header compression
(HPACK), and is well-understood by every engineer.

```
  HTTP/2 Push Request
  ════════════════════

  Connection: TLS to api.push.apple.com:443 (HTTP/2 ALPN negotiation)

  REQUEST:
  POST /3/device/{device-token} HTTP/2

  Required headers:
  ┌──────────────────────┬──────────────────────────────────────────────┐
  │ Header               │ Description                                  │
  ├──────────────────────┼──────────────────────────────────────────────┤
  │ apns-topic           │ The app's bundle ID. Required. APNs uses this│
  │                      │ to validate the certificate or JWT and route. │
  │                      │ Example: com.apple.madrid (for iMessage)      │
  ├──────────────────────┼──────────────────────────────────────────────┤
  │ apns-push-type       │ Type of push. Required since iOS 13:          │
  │                      │   alert       — shows a visible notification  │
  │                      │   background  — wakes app silently            │
  │                      │   voip        — incoming call (PushKit)       │
  │                      │   complication— Apple Watch update            │
  │                      │   fileprovider— File Provider extension       │
  │                      │   mdm         — Mobile Device Management      │
  │                      │   location    — Location update               │
  ├──────────────────────┼──────────────────────────────────────────────┤
  │ apns-priority        │ 10 = deliver immediately (may wake device)   │
  │                      │ 5  = opportunistic (respect battery/radio)    │
  ├──────────────────────┼──────────────────────────────────────────────┤
  │ apns-id              │ UUID for this notification. If omitted,       │
  │                      │ Apple generates one. Echoed in response.      │
  ├──────────────────────┼──────────────────────────────────────────────┤
  │ apns-expiration      │ UNIX timestamp: discard if not delivered by  │
  │                      │ this time. 0 = discard if device offline.    │
  ├──────────────────────┼──────────────────────────────────────────────┤
  │ authorization        │ JWT bearer token OR TLS client certificate.   │
  │                      │ See authentication section below.            │
  └──────────────────────┴──────────────────────────────────────────────┘

  Complete HTTP/2 push request example:
  ──────────────────────────────────────
  POST /3/device/A1B2C3D4E5F6789012345678901234567890ABCDEF1234567890ABCDEF123456

  Headers:
    :method: POST
    :path: /3/device/A1B2C3...
    :scheme: https
    :authority: api.push.apple.com
    content-type: application/json
    content-length: 89
    apns-id: 8F9D4E7C-1A2B-3C4D-5E6F-7A8B9C0D1E2F
    apns-topic: com.example.myapp
    apns-push-type: alert
    apns-priority: 10
    apns-expiration: 1735689600
    authorization: bearer eyJhbGciOiJFUzI1NiIsImtpZCI6IktFWUlE...

  Body (JSON):
  {
    "aps": {
      "alert": {
        "title": "New message",
        "body": "Alice: Hello Bob!"
      },
      "badge": 3,
      "sound": "default",
      "content-available": 1,
      "mutable-content": 1,
      "thread-id": "alice-conversation-thread"
    }
  }

  RESPONSE (success):
  HTTP/2 200

  Headers:
    apns-id: 8F9D4E7C-1A2B-3C4D-5E6F-7A8B9C0D1E2F

  Body: (empty)

  RESPONSE (failure):
  HTTP/2 400

  Body:
  {
    "reason": "BadDeviceToken"
  }

  aps dictionary fields:
  ┌─────────────────────┬─────────────────────────────────────────────────┐
  │ Field               │ Description                                     │
  ├─────────────────────┼─────────────────────────────────────────────────┤
  │ alert               │ String or object. If string: shown as body.      │
  │                     │ If object: title + subtitle + body + loc keys.  │
  ├─────────────────────┼─────────────────────────────────────────────────┤
  │ badge               │ Integer. App icon badge number. 0 = clear badge. │
  ├─────────────────────┼─────────────────────────────────────────────────┤
  │ sound               │ String filename of sound in app bundle,          │
  │                     │ or "default" for system notification sound.     │
  ├─────────────────────┼─────────────────────────────────────────────────┤
  │ content-available   │ 1 = background push. Wakes app without showing  │
  │                     │ a visible notification. Used by iMessage to      │
  │                     │ deliver encrypted content silently.             │
  ├─────────────────────┼─────────────────────────────────────────────────┤
  │ mutable-content     │ 1 = Notification Service Extension may modify   │
  │                     │ the notification before display. Used by        │
  │                     │ iMessage to decrypt the content locally.        │
  ├─────────────────────┼─────────────────────────────────────────────────┤
  │ category            │ Identifier for actionable notification. Maps    │
  │                     │ to registered UNNotificationCategory with       │
  │                     │ UNNotificationActions (Reply, Like, etc.).      │
  ├─────────────────────┼─────────────────────────────────────────────────┤
  │ thread-id           │ Groups notifications from the same conversation. │
  │                     │ iOS shows grouped notifications in the lockscreen│
  └─────────────────────┴─────────────────────────────────────────────────┘

  HTTP/2 response codes:
  ┌──────┬────────────────────────────────────────────────────────────────┐
  │ Code │ Meaning                                                        │
  ├──────┼────────────────────────────────────────────────────────────────┤
  │ 200  │ OK. Notification accepted for delivery.                        │
  │ 400  │ Bad request. reason field contains specifics.                  │
  │      │   BadDeviceToken: token malformed                              │
  │      │   BadExpirationDate: apns-expiration not a number              │
  │      │   BadMessageId: apns-id not a UUID                             │
  │      │   BadPriority: priority not 5 or 10                           │
  │      │   BadTopic: apns-topic missing or bad cert match               │
  │      │   DeviceTokenNotForTopic: token valid but wrong app            │
  │      │   DuplicateHeaders: repeated header                            │
  │      │   IdleTimeout: connection idle too long                        │
  │      │   InvalidPushType: apns-push-type not recognized               │
  │      │   MissingDeviceToken: path missing token                       │
  │      │   MissingTopic: header missing                                 │
  │      │   PayloadEmpty: empty body                                     │
  │      │   TopicDisallowed: push type not allowed for topic             │
  │ 403  │ Forbidden. cert/JWT invalid or topic mismatch.                 │
  │ 404  │ Not found. Path does not match expected format.                │
  │ 405  │ Method not allowed. Only POST is supported.                    │
  │ 410  │ Gone. Device token is no longer active. Remove from database.  │
  │ 413  │ Payload too large. Exceeds 4096 bytes.                         │
  │ 429  │ Too many requests. Provider is sending too fast.               │
  │ 500  │ Internal server error.                                         │
  │ 503  │ Service unavailable. APNs server is down.                      │
  └──────┴────────────────────────────────────────────────────────────────┘

  Provider Authentication — JWT vs Certificate:
  ───────────────────────────────────────────────
  Option A: JWT bearer tokens (simpler, team-wide)
    - Provider generates a private key (P-256 EC) in App Store Connect
    - Downloads .p8 file (PEM-encoded private key)
    - Creates JWT: {alg: "ES256", kid: "KEY_ID"}.{iss: "TEAM_ID", iat: now}
    - Signs with the .p8 key using ES256 (ECDSA with SHA-256)
    - JWT is valid for 45 minutes; regenerate when it expires
    - One key can be used for ALL apps in the team

  Option B: TLS certificate (per-app, legacy)
    - Per-app APNS certificate from App Store Connect
    - Installed in provider's keychain
    - Presented during TLS handshake (client certificate)
    - Certificates expire annually; must be renewed manually
```

## Key Concepts: IDS (Identity Directory Service)

### Why iMessage Needs a Key Directory

End-to-end encryption requires knowing the recipient's public key before
you can encrypt a message for them. The key directory solves the bootstrap
problem: how does Alice's device know Bob's encryption key?

```
  The Key Directory Problem
  ══════════════════════════

  Without IDS:
    Alice types Bob's phone number.
    Alice's device wants to encrypt a message.
    Alice's device has NO IDEA what Bob's public key is.
    Alice's device cannot encrypt anything.
    → iMessage cannot work.

  With IDS:
    Alice's device asks IDS: "What are the public keys for +1-555-9876?"
    IDS responds: "Bob has 3 devices. Here are their encryption keys:
      iPhone: RSA-1280 key pub_key_iphone + push_token_iphone
      iPad:   RSA-1280 key pub_key_ipad   + push_token_ipad
      MacBook: RSA-1280 key pub_key_mac   + push_token_mac"
    Alice's device encrypts separately for each of Bob's 3 devices.
    → All Bob's devices can decrypt the message.

  IDS stores per device per identity:
  ┌─────────────────────┬─────────────────────────────────────────────┐
  │ Field               │ Description                                 │
  ├─────────────────────┼─────────────────────────────────────────────┤
  │ encryption_key      │ RSA-1280 (OAEP) public key. Used by         │
  │                     │ senders to encrypt a per-message key.       │
  ├─────────────────────┼─────────────────────────────────────────────┤
  │ signing_key         │ EC P-256 public key. Used to verify that    │
  │                     │ messages claiming to be from this device     │
  │                     │ were actually signed by this device.        │
  ├─────────────────────┼─────────────────────────────────────────────┤
  │ push_token          │ The APNs token for this device × this app.  │
  │                     │ IDS tells the sender which APNs token to    │
  │                     │ deliver the encrypted payload to.           │
  ├─────────────────────┼─────────────────────────────────────────────┤
  │ identities          │ The phone numbers and Apple IDs registered  │
  │                     │ to this device. One device can be associated │
  │                     │ with multiple identities.                   │
  └─────────────────────┴─────────────────────────────────────────────┘
```

### IDS Registration

```
  Device Registration Flow
  ══════════════════════════

  When a user sets up iMessage on a new device:

  Step 1: Generate key material (all on-device, private keys never leave)
    identity_key_pair = RSA(1280 bits)    ← encryption
    signing_key_pair  = EC(P-256)         ← signing (ECDSA)

  Step 2: Authenticate Apple ID
    Apple uses SRP (Secure Remote Password) to authenticate without
    sending the password to the server.

    SRP-6a protocol:
    ─────────────────
    a. Client sends: username + A (a Diffie-Hellman value derived from
       a random client secret 'a')

    b. Server responds: salt + B (a DH value derived from the stored
       password verifier and a random server secret 'b')

    c. Client computes: shared key K = H(A, B, client_secret, password)
    d. Server computes: same shared key K = H(B, A, server_secret, verifier)

    e. Client sends: M1 = H(A, B, K)       — proof it knows K
    f. Server sends: M2 = H(A, M1, K)      — proof it also knows K

    After this exchange, both sides have K. The password never travels
    the wire — only DH values and their hash proofs.

  Step 3: Validate phone number
    Apple sends an SMS to the phone number.
    User enters the 6-digit code.
    Apple marks the phone number as validated for this Apple ID.

  Step 4: Upload public keys to IDS
    Device sends HTTPS request to IDS server:
    {
      apple_id: "alice@icloud.com",
      phone:    "+15551234567",
      devices: [
        {
          push_token:    "A1B2C3..." (32-byte APNs token),
          encryption_key_pub: <RSA-1280 DER-encoded public key>,
          signing_key_pub:    <EC P-256 public key>
        }
      ]
    }

    IDS stores this mapping and makes it available to other registered
    Apple ID users who perform lookups.

  Step 5: IDS lookup (sender fetches recipient keys)
    GET https://identity.apple.com/lookup
    Body: { "uris": ["+15559876543", "bob@icloud.com"] }

    Response:
    {
      "results": {
        "+15559876543": [
          {
            "push-token": "B1B2B3...",
            "encryption-key": "MIIBIj...",   ← RSA-1280 DER, base64
            "signing-key": "MFkwEw...",      ← EC P-256 DER, base64
            "session-token": "...",          ← validity proof from Apple
          },
          { ... },  ← Bob's second device
          { ... }   ← Bob's third device
        ]
      }
    }
```

## Key Concepts: iMessage End-to-End Encryption

### Overview of the Encryption Model

```
  iMessage Encryption Model
  ══════════════════════════

  ┌────────────────────────────────────────────────────────────────────┐
  │ Message body encryption: AES-128-CTR with a random per-message key │
  │ Key encryption: RSA-1280-OAEP per recipient device                 │
  │ Authentication: ECDSA-P256 signature over message digest            │
  └────────────────────────────────────────────────────────────────────┘

  Compared to Signal Protocol:
  ─────────────────────────────
  iMessage uses RSA-based key encapsulation. This is simpler and well-understood
  but has two significant weaknesses vs Signal:
  1. No forward secrecy — if Alice's RSA private key is compromised,
     ALL past messages can be decrypted (assuming attacker stored them).
  2. No break-in recovery — no ratchet means no rotation of encryption keys.

  Why did Apple choose RSA instead of Signal? Likely timeline: iMessage
  launched in 2011, Signal Protocol was published in 2013. Apple has been
  slow to update the encryption model since.
```

### Per-Message Key Generation

```
  The 88-Byte Key Blob
  ═════════════════════

  Apple generates a random 88-byte key blob for each message:
    Bytes  0-39:  HMAC key  (40 bytes) — for message authentication
    Bytes 40-55:  AES key   (16 bytes) — for message body encryption
    Bytes 56-87:  padding   (32 bytes) — zeroed

  Derivation of actual AES key and IV from the blob:
    Encryption:
      payload_key = key_blob[40:56]    ← 16 bytes of AES key
      payload_iv  = RAND(16)           ← random IV
      ciphertext  = AES-128-CTR(payload_key, payload_iv, message_body)

    Authentication (over the ciphertext, not plaintext):
      mac = HMAC-SHA256(key_blob[0:40], payload_iv || ciphertext)
      message_digest = SHA1(payload_iv || ciphertext || mac)

  The 88-byte blob is then encrypted with RSA for each recipient device.
  This is the "encrypted key" that travels alongside the ciphertext.
```

### Multi-Device Encryption Example

Alice sends "Hello" from her iPhone to Bob (3 devices: iPhone, iPad, MacBook).
Alice also has a MacBook — it needs a copy so it can show the sent message.

```
  Sending "Hello" from Alice's iPhone to Bob's 3 devices
  ════════════════════════════════════════════════════════

  Alice's IDS lookup result:
    Bob's iPhone:  (enc_key_bob_iphone,  sign_key_bob_iphone,  token_bob_iphone)
    Bob's iPad:    (enc_key_bob_ipad,    sign_key_bob_ipad,    token_bob_ipad)
    Bob's MacBook: (enc_key_bob_mac,     sign_key_bob_mac,     token_bob_mac)
    Alice's MacBook:(enc_key_alice_mac,  sign_key_alice_mac,   token_alice_mac)

  Step 1: Generate message key blob
    key_blob = RAND(88)   ← 88 random bytes

  Step 2: Encrypt message body
    iv         = RAND(16)
    ciphertext = AES-128-CTR(key_blob[40:56], iv, b"Hello")
    mac        = HMAC-SHA256(key_blob[0:40], iv || ciphertext)
    body_digest = SHA1(iv || ciphertext || mac)

  Step 3: Sign the digest (for recipient authentication)
    signature = ECDSA-P256-Sign(alice_signing_key_priv, body_digest)

  Step 4: Encrypt key_blob for EACH recipient device
    ┌───────────────────────────────────────────────────────────────┐
    │ Destination         │ Encrypted key                           │
    ├─────────────────────┼─────────────────────────────────────────┤
    │ Bob's iPhone        │ RSA-OAEP-Encrypt(enc_key_bob_iphone,    │
    │                     │                  key_blob)              │
    ├─────────────────────┼─────────────────────────────────────────┤
    │ Bob's iPad          │ RSA-OAEP-Encrypt(enc_key_bob_ipad,      │
    │                     │                  key_blob)              │
    ├─────────────────────┼─────────────────────────────────────────┤
    │ Bob's MacBook       │ RSA-OAEP-Encrypt(enc_key_bob_mac,       │
    │                     │                  key_blob)              │
    ├─────────────────────┼─────────────────────────────────────────┤
    │ Alice's MacBook     │ RSA-OAEP-Encrypt(enc_key_alice_mac,     │
    │  (sender's device!) │                  key_blob)              │
    └─────────────────────┴─────────────────────────────────────────┘

  Total: 4 separate RSA encryptions of the same key_blob.
  The message body (ciphertext) is the SAME for all recipients.

  Step 5: Deliver via APNs
    For each recipient device, send an APNs payload to their token:

    APNs delivery 1 → token_bob_iphone:
    {
      "aps": { "content-available": 1 },
      "cT": "SGVsbG8=",           ← ciphertext (base64)
      "eK": "<RSA-enc for iphone>",   ← encrypted key_blob (base64)
      "sig": "<ECDSA signature>",
      "sId": "alice@icloud.com",  ← sender identity
      "dI": "msg-uuid-1234"
    }

    APNs delivery 2 → token_bob_ipad:    (same cT, different eK)
    APNs delivery 3 → token_bob_mac:     (same cT, different eK)
    APNs delivery 4 → token_alice_mac:   (same cT, different eK)

  Step 6: Bob's iPhone receives and decrypts
    1. APNs wakes Messages app on iPhone.
    2. App reads the payload: extract cT and eK.
    3. Decrypt eK with Alice's RSA private key:
         key_blob = RSA-OAEP-Decrypt(alice_enc_key_priv, eK)
    4. Derive payload_key = key_blob[40:56], hmac_key = key_blob[0:40]
    5. Verify MAC: HMAC-SHA256(hmac_key, iv || ciphertext) == stored mac
    6. Decrypt: AES-128-CTR-Decrypt(payload_key, iv, ciphertext) → "Hello"
    7. Verify signature: ECDSA-P256-Verify(alice_signing_key_pub, body_digest, sig)
    8. Display "Hello" in Messages.

  ┌──────────────────────────────────────────────────────────────────────┐
  │ Why does Alice's MacBook also get an encrypted copy?                  │
  │                                                                      │
  │ Alice opens Messages on her MacBook. She expects to see the message  │
  │ she just sent from her iPhone in her sent history.                   │
  │ The message body is end-to-end encrypted — Apple cannot decrypt it   │
  │ and re-encrypt it for Alice's MacBook.                               │
  │ So Alice's iPhone must proactively send an encrypted copy to every   │
  │ device Alice owns.                                                   │
  │                                                                      │
  │ This is called "multi-device sender fanout" and is the core reason   │
  │ iMessage's encryption model differs from Signal. Signal handles this │
  │ with the Double Ratchet + device linking; iMessage uses RSA fanout.  │
  └──────────────────────────────────────────────────────────────────────┘
```

### Group iMessage Encryption

```
  Group Chats in iMessage
  ════════════════════════

  iMessage groups are NOT a distinct protocol-level concept with shared
  group keys (unlike WhatsApp sender keys). Instead, every iMessage in
  a group is encrypted pairwise, just like a 1-to-1 message but sent
  to more devices.

  Group with Alice, Bob, Carol (total 4 devices):
    Alice: 1 device
    Bob:   2 devices
    Carol: 1 device

  When Alice sends to the group:
    Alice encrypts key_blob separately for:
      Bob's iPhone, Bob's iPad, Carol's iPhone,
      AND Alice's own device (for sent message sync).
    = 4 RSA encryptions, 4 APNs deliveries.

  Limitation: O(n) encryption operations where n = total device count.
  For a large group of 20 people each with 3 devices = 60 encryptions
  per message. Compare to WhatsApp sender keys = 1 encryption.

  Note: iMessage does not display a "group is end-to-end encrypted"
  indicator at the protocol level for groups. The group concept is
  entirely client-side — the protocol is just a set of pairwise messages
  with a shared "group ID" in the payload to allow grouping.
```

### Forward Secrecy Analysis

```
  iMessage Forward Secrecy: None
  ════════════════════════════════

  Forward secrecy means: if today's keys are compromised,
  yesterday's messages remain safe.

  iMessage does NOT have forward secrecy:
    - Alice's RSA-1280 private key is stored on her device.
    - Every message ever sent to Alice was encrypted with her RSA public key.
    - If an attacker obtains Alice's RSA private key (e.g., via malware,
      physical device access, or a court order), they can decrypt ALL
      stored iMessage ciphertext — past and future.

  Signal Protocol comparison:
    - Each message uses a unique derived key (Double Ratchet).
    - Derived keys are deleted immediately after use.
    - Compromising key #100 reveals nothing about messages 1-99.

  This is the most significant security criticism of iMessage.
  Apple has acknowledged this limitation but has not redesigned
  the encryption model as of 2025.

  Practical note: Because Apple controls the key directory (IDS),
  Apple could theoretically inject a malicious key for a user and
  receive copies of all future messages. This is a known concern —
  key transparency mechanisms (like what Apple added in Messages in
  iCloud with Contact Key Verification in 2023) address it partially.
```

## Key Concepts: iMessage Wire Format

### APNs Payload for iMessage

The iMessage app registers for APNs under the topic `com.apple.madrid`
(the internal codename for iMessage during development). All iMessage
APNs payloads use this topic.

```
  iMessage APNs Payload Structure
  ════════════════════════════════

  The aps object is minimal for iMessage — most data is in custom fields
  outside aps, encoded as a binary plist:

  Outer JSON (as received by APNs client on device):
  {
    "aps": {
      "content-available": 1    // wake the app silently
      // No alert, badge, or sound — iMessage handles these itself
    },
    // Custom fields (these are binary plist data, base64-encoded in JSON):
    "D": "<binary plist>"       // the actual iMessage payload
  }

  The "D" field is a binary plist with these keys:
  ┌──────────────┬──────────────────────────────────────────────────────┐
  │ Key          │ Description                                          │
  ├──────────────┼──────────────────────────────────────────────────────┤
  │ t            │ Message type. Integer.                               │
  │              │   100 = iMessage text                                │
  │              │   101 = delivered receipt                            │
  │              │   102 = read receipt                                 │
  │              │   108 = typing indicator (start)                     │
  │              │   109 = typing indicator (stop)                      │
  ├──────────────┼──────────────────────────────────────────────────────┤
  │ P            │ Encrypted payload (Data type). The ciphertext.       │
  ├──────────────┼──────────────────────────────────────────────────────┤
  │ K            │ Encrypted message key (Data). RSA-OAEP output.       │
  ├──────────────┼──────────────────────────────────────────────────────┤
  │ sP           │ Sender's signing key certificate (for verification). │
  ├──────────────┼──────────────────────────────────────────────────────┤
  │ S            │ ECDSA signature of message digest.                   │
  ├──────────────┼──────────────────────────────────────────────────────┤
  │ U            │ Message UUID (Data, 16 bytes). The unique message ID. │
  ├──────────────┼──────────────────────────────────────────────────────┤
  │ c            │ Content type. String.                                │
  │              │   "text/plain" = plain iMessage text                 │
  │              │   "audio/amr"  = audio message                       │
  ├──────────────┼──────────────────────────────────────────────────────┤
  │ gid          │ Group ID (UUID). Identifies the group conversation.  │
  ├──────────────┼──────────────────────────────────────────────────────┤
  │ gn           │ Group name (String). Human name of the group.        │
  ├──────────────┼──────────────────────────────────────────────────────┤
  │ r            │ Reply-to message ID (UUID Data). For reply threads.  │
  └──────────────┴──────────────────────────────────────────────────────┘
```

### Binary Plist Format

Apple uses binary property lists (binary plists) to encode structured data
for efficiency. The format is defined by Apple and used throughout macOS/iOS.

```
  Binary Plist (bplist00) Format
  ════════════════════════════════

  ┌────────────────────────────────────────────────────────────────────┐
  │  Header: "bplist00" (8 bytes, ASCII magic)                         │
  ├────────────────────────────────────────────────────────────────────┤
  │  Objects (variable):                                               │
  │    Each object begins with a 1-byte marker:                        │
  │    ┌────────┬──────────────────────────────────────────────────────┐│
  │    │ Marker │ Object type                                          ││
  │    ├────────┼──────────────────────────────────────────────────────┤│
  │    │ 0x00   │ null                                                 ││
  │    │ 0x08   │ false (bool)                                         ││
  │    │ 0x09   │ true  (bool)                                         ││
  │    │ 0x1N   │ int, where N = byte count exponent (2^N bytes follow)││
  │    │ 0x2N   │ real (float), where N = byte count exponent          ││
  │    │ 0x33   │ date (8-byte big-endian float, Apple epoch)          ││
  │    │ 0x4N   │ data (binary), N = length or 0xF = use int object    ││
  │    │ 0x5N   │ ASCII string, N = length                             ││
  │    │ 0x6N   │ UTF-16 string, N = length in characters              ││
  │    │ 0xAN   │ array, N = count (or 0xF = use int object)            ││
  │    │ 0xDN   │ dict, N = count of key-value pairs                   ││
  │    └────────┴──────────────────────────────────────────────────────┘│
  ├────────────────────────────────────────────────────────────────────┤
  │  Offset Table: array of offsets to each object                     │
  │  (each offset is `object_ref_size` bytes)                          │
  ├────────────────────────────────────────────────────────────────────┤
  │  Trailer (32 bytes):                                               │
  │    offset_table_offset_size: 1 byte                                │
  │    object_ref_size:          1 byte                                │
  │    num_objects:              8 bytes (big-endian int64)            │
  │    root_object_index:        8 bytes (big-endian int64)            │
  │    offset_table_offset:      8 bytes (big-endian int64)            │
  └────────────────────────────────────────────────────────────────────┘

  Example: binary plist encoding of {"t": 100, "U": <16 zero bytes>}

  62706C69 73743030   "bplist00"  (header magic)

  D2                  dict with 2 pairs (0xD0 | 2)
  51                  string, length 1  (0x50 | 1)
  74                  "t"
  10 64              int8 value 100  (0x10=int, 1 byte; 0x64=100)
  51                  string, length 1
  55                  "U"
  4F 10              data, length follows as int
  10                  int8 value
  10                  = 16 (length of the UUID data)
  00000000 00000000  16 zero bytes (the UUID)
  00000000 00000000

  [offset table]
  [trailer]
```

### The Madrid Topic

```
  Why "com.apple.madrid"?
  ═══════════════════════

  iMessage was developed internally at Apple under the codename "Madrid"
  (not to be confused with BlackBerry's server project also called Madrid).
  Apple's internal codenames often refer to cities.

  The APNs topic `com.apple.madrid` identifies iMessage notifications.
  When Apple's APNs server sees a notification with this topic, it knows
  to route it to the Messages app on iOS/macOS.

  Topics for Apple's system services (not public knowledge but reverse-
  engineered from device behavior):
  ┌─────────────────────────────────┬────────────────────────────────────┐
  │ Topic                           │ Service                            │
  ├─────────────────────────────────┼────────────────────────────────────┤
  │ com.apple.madrid                │ iMessage                           │
  │ com.apple.iCloud.FMF            │ Find My Friends                    │
  │ com.apple.iCloud.fmip.voiceRoute│ Find My iPhone                     │
  │ com.apple.maps.notifications    │ Maps / Navigation                   │
  │ com.apple.siri.updates          │ Siri knowledge updates             │
  └─────────────────────────────────┴────────────────────────────────────┘
```

## Key Concepts: SMS Fallback

### How iMessage Detects Whether to Use iMessage or SMS

```
  iMessage vs SMS Detection
  ══════════════════════════

  When Alice types in Bob's number and starts a conversation:

  Step 1: IDS lookup
    Alice's device asks IDS: "Is +1-555-9876 registered for iMessage?"
    IDS query: GET https://identity.apple.com/lookup
               Body: {"uris": ["+15559876543"]}

    Case A: Bob has iMessage registered
      IDS returns: { "+15559876543": [ {device records...} ] }
      Result: Alice's device uses iMessage (blue bubbles)

    Case B: Bob has NOT registered iMessage
      IDS returns: { "+15559876543": [] } (empty list)
      Result: Alice's device uses SMS (green bubbles)

  Step 2: Delivery and fallback
    After sending an iMessage, if delivery via APNs fails
    (device offline, APNs error) after ~10 minutes:
      Option A: Retry iMessage (default for most failures)
      Option B: Send as SMS (if "Send as SMS" is enabled in Settings)

  The "blue bubble / green bubble" cultural shorthand:
  ────────────────────────────────────────────────────
  Blue  = iMessage (end-to-end encrypted, delivered over Apple's network)
  Green = SMS/MMS (carrier-delivered, no E2E encryption)

  Technically:
    Blue = sent via com.apple.madrid APNs topic
    Green = handed off to the telephony subsystem (CTCarrier API)

  Phone number validation for iMessage eligibility:
  ──────────────────────────────────────────────────
  Apple validates ownership of a phone number by:
  1. Sending a 6-digit SMS code to the number.
  2. User enters the code in Settings > Messages > Send & Receive.
  3. Apple records: this phone number is associated with this Apple ID
     and device push token.
  Subsequent IDS lookups for that number return iMessage device records.
```

## Key Concepts: FaceTime Relay

### ICE, STUN, and TURN

FaceTime is a video/audio call service. Direct peer-to-peer (P2P) connections
between devices are always preferred because they reduce latency and avoid
Apple's servers seeing audio/video. But NAT (Network Address Translation)
makes P2P connections difficult — most devices sit behind routers with private
IP addresses and the outside world cannot initiate connections to them directly.

```
  NAT Traversal Problem
  ══════════════════════

  Alice's device: LAN IP 192.168.1.5, behind router at 203.0.113.10
  Bob's device:   LAN IP 10.0.0.42,  behind router at 198.51.100.20

  Direct connection attempt:
    Alice tries to connect to 198.51.100.20:some_port
    Bob's router drops the packet — it hasn't set up a port mapping
    for Alice's address. NAT traversal fails.

  STUN (Session Traversal Utilities for NAT):
    Purpose: discover your public IP address and the NAT binding.
    Protocol: UDP to stun.apple.com (or similar), get back your
              external IP:port as seen by the STUN server.
    Limitation: only works when both sides' NATs are not "symmetric"
               (symmetric NAT assigns different ports for different
               destinations — STUN is useless there).

  ICE (Interactive Connectivity Establishment):
    ICE gathers multiple "candidate" addresses:
    ┌─────────────────────┬──────────────────────────────────────────┐
    │ Candidate type      │ Description                              │
    ├─────────────────────┼──────────────────────────────────────────┤
    │ host                │ Local LAN address (e.g., 192.168.1.5:5000)│
    │ server-reflexive    │ Address seen by STUN server (external IP) │
    │ relay               │ Address on TURN relay server             │
    └─────────────────────┴──────────────────────────────────────────┘

    ICE tries all combinations of Alice's candidates and Bob's candidates
    in priority order (host > server-reflexive > relay).
    First working pair becomes the connection path.

  TURN (Traversal Using Relays around NAT):
    Last resort. If direct P2P fails, both sides connect to Apple's TURN
    relay server. All audio/video flows through the relay.
    Apple uses:
      turn.apple.com (various geographically distributed servers)
    Protocol: TURN over UDP (typically) or TCP with TLS fallback.
```

```
  FaceTime Connection Establishment
  ════════════════════════════════════

  Step 1: Signaling via APNs
    Alice's device sends a FaceTime call invitation to Bob's device
    via APNs (topic: com.apple.facetime, push type: voip).
    The invitation contains Alice's ICE candidates and SDP offer.

  Step 2: ICE candidate gathering (both sides)
    Alice gathers:
      host:     192.168.1.5:5004
      reflexive: 203.0.113.10:49152 (from STUN)
      relay:    turn.apple.com:3478 with relay address

    Bob gathers (similarly):
      host:     10.0.0.42:5004
      reflexive: 198.51.100.20:49153
      relay:    turn.apple.com:3478

  Step 3: ICE connectivity checks
    Both sides exchange candidate lists (over the APNs signaling channel).
    Both try all candidate pairs with STUN Binding requests (connectivity checks).

    Typical result:
    ┌─────────────────────────────────────────────────────────────────┐
    │  If same LAN:        host ↔ host (fastest, LAN speeds)          │
    │  If different NATs:  reflexive ↔ reflexive (if NAT permits)      │
    │  If symmetric NAT:   relay ↔ relay (Apple's TURN servers)        │
    └─────────────────────────────────────────────────────────────────┘

  Step 4: DTLS handshake over the chosen ICE path
    FaceTime uses DTLS (Datagram TLS) over UDP for encrypted media.
    DTLS-SRTP generates keys for SRTP (Secure Real-time Transport Protocol).

  Step 5: SRTP audio/video streams
    Audio: typically AAC-LD (Low Delay) or Opus codec
    Video: H.264 (older) or HEVC (H.265, modern)
    Both sent as SRTP packets (RTP + DTLS-derived encryption keys).

  Apple's relay infrastructure:
    Apple operates TURN relay servers in Apple data centers worldwide.
    About 30-40% of FaceTime calls go through relay (the rest are P2P).
    Apple claims they cannot see call content (SRTP encrypted) even
    on relay paths.
```

## Algorithms

### APNs: JWT Token Generation

```
  generate_jwt_token(team_id, key_id, private_key_p8):
    header = base64url({
      "alg": "ES256",
      "kid": key_id     // 10-character key ID from App Store Connect
    })

    now = unix_timestamp()
    payload = base64url({
      "iss": team_id,   // 10-character team ID
      "iat": now        // issued-at timestamp
    })

    signing_input = header + "." + payload
    signature = ECDSA_P256_SHA256_sign(private_key_p8, signing_input)
    return signing_input + "." + base64url(signature)

  Note: The JWT is valid for 45 minutes from iat.
  Providers should cache and reuse the JWT, regenerating when near expiry.
  Generating a new JWT per notification is wasteful.

  Example JWT (decoded):
    Header:  {"alg": "ES256", "kid": "ABC1234567"}
    Payload: {"iss": "TEAM123456", "iat": 1700000000}
    Signature: [64 bytes ECDSA P-256 signature]
```

### iMessage: Encrypt and Send

```
  imessage_send(sender_keys, recipient_ids, message_text):

    # Step 1: Generate per-message material
    key_blob    = RAND(88)
    payload_key = key_blob[40:56]
    hmac_key    = key_blob[0:40]
    iv          = RAND(16)

    # Step 2: Encrypt message body
    ciphertext  = AES_128_CTR_encrypt(payload_key, iv, message_text.utf8)
    mac         = HMAC_SHA256(hmac_key, iv || ciphertext)
    body_digest = SHA1(iv || ciphertext || mac)

    # Step 3: Sign
    signature = ECDSA_P256_sign(sender_keys.signing_priv, body_digest)

    # Step 4: Look up all recipient devices from IDS
    all_devices = []
    for identity in recipient_ids:
      devices = IDS.lookup(identity)   # HTTP to identity.apple.com
      all_devices.extend(devices)

    # Also add sender's own devices (for multi-device sync)
    sender_devices = IDS.lookup(sender_keys.identity)
    all_devices.extend(sender_devices)

    # Step 5: For each device, encrypt key_blob and deliver via APNs
    for device in all_devices:
      enc_key = RSA_OAEP_encrypt(device.encryption_key_pub, key_blob)

      payload = binary_plist({
        "t":  100,             # message type = iMessage text
        "U":  message_uuid,    # 16-byte UUID
        "P":  iv || ciphertext || mac,   # encrypted body
        "K":  enc_key,         # encrypted key blob
        "S":  signature,       # ECDSA signature
        "c":  "text/plain"     # content type
      })

      apns_send(
        token=device.push_token,
        topic="com.apple.madrid",
        push_type="background",
        priority=10,
        payload={
          "aps": {"content-available": 1},
          "D": payload
        }
      )

    return message_uuid
```

### IDS Lookup with Caching

```
  IDS lookups are cached aggressively — sending a message to a contact
  should not require a network round trip every time.

  ids_lookup(identity, cache):
    # Check cache first
    if identity in cache and not cache[identity].expired:
      return cache[identity].devices

    # Fetch from IDS
    response = HTTPS_GET(
      "https://identity.apple.com/lookup",
      headers={"Authorization": "X-Apple-ID " + signed_auth_token},
      body=json({"uris": [identity]})
    )

    if response.status != 200:
      raise IdsLookupError(response)

    devices = parse_device_records(response.json)

    # Cache result with TTL (Apple controls TTL via response headers)
    cache[identity] = CacheEntry(devices=devices, ttl=response.ttl)

    return devices

  Cache invalidation triggers:
    - Contact Key Verification mismatch (key changed — possible attack)
    - User explicitly triggers re-lookup ("Send as iMessage" retry)
    - TTL expiry
    - IDS push notification that a contact's key has changed
```

### Binary Plist Encoder

```
  encode_bplist(value):
    objects = []
    collect_objects(value, objects)   # DFS collection
    offsets = []
    body = ByteBuffer()

    body.write(b"bplist00")           # magic header

    for obj in objects:
      offsets.append(len(body))
      write_object(body, obj, objects)

    offset_table_start = len(body)
    ref_size = byte_size_for(len(objects))
    offset_size = byte_size_for(offset_table_start)

    for offset in offsets:
      body.write_uint(offset, offset_size)

    # Trailer (32 bytes)
    body.write_uint8(0)               # padding
    body.write_uint8(0)               # padding
    body.write_uint8(0)               # padding
    body.write_uint8(0)               # padding
    body.write_uint8(0)               # padding
    body.write_uint8(0)               # padding
    body.write_uint8(offset_size)     # size of each offset entry
    body.write_uint8(ref_size)        # size of each object reference
    body.write_uint64(len(objects))   # total object count
    body.write_uint64(0)              # root object index (always 0)
    body.write_uint64(offset_table_start)

    return body.bytes()

  write_object(buf, obj, all_objects):
    if obj is None:         buf.write(0x00)
    elif obj is False:      buf.write(0x08)
    elif obj is True:       buf.write(0x09)
    elif isinstance(obj, int):
      if obj fits in 1 byte: buf.write(0x10); buf.write_uint8(obj)
      elif fits in 2 bytes:  buf.write(0x11); buf.write_uint16_be(obj)
      elif fits in 4 bytes:  buf.write(0x12); buf.write_uint32_be(obj)
      else:                  buf.write(0x13); buf.write_uint64_be(obj)
    elif isinstance(obj, float):
      buf.write(0x23)               # 8-byte real
      buf.write_float64_be(obj)
    elif isinstance(obj, bytes):
      write_count_marker(buf, 0x40, len(obj))
      buf.write(obj)
    elif isinstance(obj, str):
      encoded = obj.encode("utf-8")
      write_count_marker(buf, 0x50, len(obj))  # ASCII marker
      buf.write(encoded)
    elif isinstance(obj, list):
      write_count_marker(buf, 0xA0, len(obj))
      for item in obj:
        buf.write_ref(all_objects.index(item))
    elif isinstance(obj, dict):
      write_count_marker(buf, 0xD0, len(obj))
      for k in obj.keys():
        buf.write_ref(all_objects.index(k))
      for v in obj.values():
        buf.write_ref(all_objects.index(v))
```

## Test Strategy

### APNs Tests

```
  1. Binary Protocol (Historical)
  ─────────────────────────────────
  test_enhanced_notification_encode:
    token   = b"\xA1" * 32
    payload = b'{"aps":{"alert":"test"}}'
    notif_id = 42
    expiry   = 1700000000
    frame = encode_enhanced_notification(token, payload, notif_id, expiry, priority=10)

    assert frame[0] == 0x01            # command byte
    frame_len = struct.unpack(">I", frame[1:5])[0]
    assert frame_len == len(frame) - 5

    # Verify item 1 (device token)
    assert frame[5] == 0x01            # item ID
    assert struct.unpack(">H", frame[6:8])[0] == 32   # length
    assert frame[8:40] == b"\xA1" * 32

  test_error_response_decode:
    raw = bytes([0x08, 0x08, 0x00, 0x00, 0x01, 0x23])
    err = decode_error_response(raw)
    assert err.command == 0x08
    assert err.status  == 0x08         # "invalid token" status code
    assert err.notif_id == 0x123

  test_payload_over_4kb_rejected:
    large_payload = {"aps": {"alert": "x" * 5000}}
    expect: PayloadTooLargeError raised before sending

  2. HTTP/2 Protocol
  ───────────────────
  test_jwt_token_valid_format:
    token = generate_jwt(team_id="TEAM123456", key_id="KEY1234567",
                         private_key=test_key)
    parts = token.split(".")
    assert len(parts) == 3
    header  = json_decode(base64url_decode(parts[0]))
    payload = json_decode(base64url_decode(parts[1]))
    assert header["alg"] == "ES256"
    assert header["kid"] == "KEY1234567"
    assert payload["iss"] == "TEAM123456"
    sig_valid = ECDSA_verify(test_key.public, parts[0]+"."+parts[1], parts[2])
    assert sig_valid

  test_notification_request_headers:
    req = build_apns_request(token=device_token, payload={"aps":{}},
                             topic="com.example.myapp", push_type="alert")
    assert req.method == "POST"
    assert req.path == f"/3/device/{device_token}"
    assert req.headers["apns-topic"] == "com.example.myapp"
    assert req.headers["apns-push-type"] == "alert"

  test_bad_device_token_response_handling:
    Mock APNs response: HTTP 410, body={"reason":"Unregistered"}
    handler = send_notification(token, payload)
    assert handler.result == "unregistered"
    assert token_removed_from_db(token)

  test_too_many_requests_backoff:
    Mock APNs response: HTTP 429
    sender = APNSSender(backoff_strategy=ExponentialBackoff)
    sender.send(notification)
    assert sender.retry_count == 1
    assert sender.next_retry_delay >= 1.0  # seconds
```

### IDS Tests

```
  test_ids_registration_payload:
    keys = generate_imessage_keys()
    payload = build_ids_registration(
      apple_id="alice@icloud.com",
      phone="+15551234567",
      push_token=test_token,
      keys=keys
    )
    assert "encryption-key" in payload
    assert "signing-key" in payload
    assert payload["push-token"] == test_token

  test_ids_lookup_returns_device_records:
    mock_response = {"+15559876543": [
      {"push-token": "TOKEN1", "encryption-key": "KEY1", "signing-key": "SKEY1"}
    ]}
    devices = ids_lookup("+15559876543", mock_ids_server)
    assert len(devices) == 1
    assert devices[0].push_token == "TOKEN1"

  test_ids_lookup_cache_hit:
    ids_lookup("+15559876543", cache=cold_cache)  # hits network
    ids_lookup("+15559876543", cache=warm_cache)  # must not hit network
    assert mock_network.call_count == 1

  test_ids_lookup_empty_means_sms:
    mock_response = {"+15559876543": []}
    result = route_message("+15559876543", mock_ids)
    assert result.transport == "sms"
```

### iMessage E2E Encryption Tests

```
  test_key_generation:
    keys = generate_imessage_keys()
    assert keys.encryption_key.key_size == 1280   # RSA-1280
    assert keys.signing_key.curve == "P-256"
    # Private keys should not be exportable in serialized form
    assert "private" not in keys.to_public_bundle()

  test_message_encrypt_decrypt_round_trip:
    alice_keys = generate_imessage_keys()
    plaintext  = "Hello, Bob!"

    (ciphertext, enc_key, iv, mac, sig) = imessage_encrypt(
      alice_keys.signing_priv,
      alice_keys.encryption_pub,  # encrypting to self for test
      plaintext
    )

    recovered = imessage_decrypt(
      alice_keys.encryption_priv,
      alice_keys.signing_pub,
      ciphertext, enc_key, iv, mac, sig
    )
    assert recovered == plaintext

  test_multi_device_fanout:
    alice_keys = generate_imessage_keys()
    bob_device_1 = generate_imessage_keys().public_bundle()
    bob_device_2 = generate_imessage_keys().public_bundle()
    alice_mac    = generate_imessage_keys().public_bundle()

    deliveries = imessage_send_to_devices(
      sender=alice_keys,
      recipients=[bob_device_1, bob_device_2, alice_mac],
      text="Hello"
    )

    assert len(deliveries) == 3  # 3 APNs deliveries
    # All deliveries have same ciphertext (body encrypted once)
    ciphertexts = [d.payload["cT"] for d in deliveries]
    assert all(c == ciphertexts[0] for c in ciphertexts)
    # All deliveries have DIFFERENT encrypted keys
    enc_keys = [d.payload["eK"] for d in deliveries]
    assert len(set(enc_keys)) == 3

  test_rsa_oaep_encrypt_decrypt:
    key_pair  = RSA_generate(1280)
    blob      = RAND(88)
    encrypted = RSA_OAEP_encrypt(key_pair.public, blob)
    recovered = RSA_OAEP_decrypt(key_pair.private, encrypted)
    assert recovered == blob

  test_signature_verification_failure:
    alice_keys = generate_imessage_keys()
    mallory_keys = generate_imessage_keys()
    (ciphertext, enc_key, iv, mac, sig) = imessage_encrypt(
      mallory_keys.signing_priv,  # Mallory signs, claims to be Alice
      alice_keys.encryption_pub,
      "hello"
    )
    expect: SignatureVerificationError on decrypt with alice_keys.signing_pub

  test_mac_tamper_detection:
    alice_keys = generate_imessage_keys()
    (ciphertext, enc_key, iv, mac, sig) = imessage_encrypt(
      alice_keys.signing_priv, alice_keys.encryption_pub, "hello"
    )
    tampered_ciphertext = flip_byte(ciphertext, 10)
    expect: MACVerificationError on decrypt

  test_group_chat_pairwise_encryption:
    alice = generate_imessage_keys()
    group_members = [generate_imessage_keys() for _ in range(4)]

    deliveries = imessage_group_send(sender=alice,
                                     members=group_members,
                                     text="Group hello")
    # 4 members + 1 sender = 5 deliveries (or more if multi-device)
    assert len(deliveries) >= 4
    # Each delivery has individually encrypted key
    enc_keys = [d.enc_key for d in deliveries]
    assert len(set(enc_keys)) == len(deliveries)
```

### Binary Plist Tests

```
  test_bplist_encode_decode_string:
    original = "Hello, iMessage!"
    encoded  = encode_bplist(original)
    assert encoded[0:8] == b"bplist00"
    decoded = decode_bplist(encoded)
    assert decoded == original

  test_bplist_encode_decode_dict:
    original = {"t": 100, "c": "text/plain"}
    encoded  = encode_bplist(original)
    decoded  = decode_bplist(encoded)
    assert decoded == original

  test_bplist_encode_data:
    data     = b"\x00\x01\x02" * 100
    original = {"P": data, "U": b"\xFF" * 16}
    encoded  = encode_bplist(original)
    decoded  = decode_bplist(encoded)
    assert decoded["P"] == data
    assert decoded["U"] == b"\xFF" * 16

  test_bplist_nested_dict:
    original = {"aps": {"alert": "test", "badge": 3}}
    encoded  = encode_bplist(original)
    decoded  = decode_bplist(encoded)
    assert decoded["aps"]["badge"] == 3

  test_bplist_bool_encoding:
    assert decode_bplist(encode_bplist(True))  is True
    assert decode_bplist(encode_bplist(False)) is False

  test_bplist_roundtrip_imessage_payload:
    payload = {
      "t": 100,
      "U": b"\xDE\xAD\xBE\xEF" * 4,
      "P": b"\x00" * 256,     # simulated ciphertext
      "K": b"\xFF" * 160,     # simulated RSA output
      "c": "text/plain"
    }
    assert decode_bplist(encode_bplist(payload)) == payload
```

## Error Handling and Edge Cases

```
  IDS Key Change Detection
  ══════════════════════════

  When Alice's device fetches Bob's IDS records and gets a DIFFERENT
  public key than last time, this is a security-critical event.
  Possibilities:
    a. Bob got a new device (legitimate)
    b. Bob re-installed Messages (legitimate)
    c. Apple (or an attacker) injected a fraudulent key (attack)

  Modern iMessage behavior (with Contact Key Verification, iOS 17+):
    - Users can compare "safety numbers" (hash of their IDS keys) in person
    - Mismatch is shown as a warning: "This person's security keys changed"
    - If CKV (Contact Key Verification) is enabled, unexpected key changes
      trigger a large visible warning

  APNs Delivery Guarantees:
  ══════════════════════════

  APNs provides "at most once" delivery (not "at least once"):
    - If the device is offline and the notification expires (apns-expiration
      has passed), Apple discards it. The provider is NOT notified.
    - If APNs is temporarily overloaded, notifications may be dropped.
    - APNs stores at most ONE notification per app per device while offline.
      New notifications overwrite the stored one.

  iMessage handling:
    - Sender waits for a "delivered" receipt (type=101 payload).
    - If no receipt after ~2 minutes, sender resends the APNs push.
    - After a configurable timeout, iMessage gives up and (optionally)
      falls back to SMS.
    - "Delivered" in iMessage does NOT mean the user read it — it means
      the APNs delivery succeeded (the device received the push).
    - "Read" is a separate receipt (type=102) sent when the user opens
      the conversation.

  FaceTime Relay Fallback:
  ══════════════════════════

  ICE fails if:
    - All STUN candidates fail (symmetric NAT on both sides)
    - TURN credentials expire or TURN server is unreachable
    - Firewall blocks UDP entirely

  If ICE fails within 5 seconds:
    - FaceTime falls back to TCP-based relay (TURN over TCP)
    - If that also fails: call setup fails with "FaceTime Unavailable"

  ICE candidate priority order (highest to lowest):
    1. host (local LAN) — lowest latency
    2. server-reflexive (public IP via STUN) — NAT traversal
    3. peer-reflexive (discovered during connectivity checks)
    4. relay (TURN) — highest latency but always works if TURN is reachable
```
