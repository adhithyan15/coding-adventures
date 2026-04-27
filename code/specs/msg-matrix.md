# MSG-MATRIX — Matrix Protocol

## Overview

Matrix is an open, federated, real-time communication protocol. It lets anyone
run a server (called a homeserver) and communicate with users on any other
homeserver — just like email, but for messaging, voice, and video.

**Analogy:** Matrix is to messaging what email is to mail. When you send an
email from a Gmail account to an Outlook account, the two providers cooperate
to deliver your message. Neither company owns "email." Anyone can run an email
server. Matrix works the same way for chat: you can host your own server at
`matrix.mycompany.com`, and your users can message people on `matrix.org`,
`mozilla.org`, or any other homeserver in the world. No single company controls
who can participate.

The contrast with centralized messengers is sharp: if WhatsApp shuts down, all
WhatsApp users lose access to their messages and contacts. If matrix.org shuts
down, users on matrix.org can migrate to another homeserver — their contacts on
other servers still exist, and the conversation history in federated rooms
remains accessible from those other servers.

The project has three main components:

1. **The Matrix Specification** — a set of documents (and MSCs — Matrix Spec
   Changes, the RFC-equivalent) describing every API, event format, and
   algorithm. The spec lives at spec.matrix.org.
2. **Homeservers** — server software that stores room events, serves the
   Client-Server API, and interconnects with other homeservers via the
   Federation API. Reference implementation: Synapse (Python). Others: Dendrite
   (Go), Conduit (Rust).
3. **Clients** — applications that talk to a homeserver via the Client-Server
   API. Examples: Element (Electron/mobile), Cinny (web), FluffyChat (Flutter).

## Architecture

```
  ┌─────────────────────────────────────────────────────────────────────┐
  │                         The Matrix Network                          │
  │                                                                     │
  │  ┌──────────────────────┐        ┌──────────────────────────────┐   │
  │  │   Homeserver A       │        │   Homeserver B               │   │
  │  │   matrix.org         │◄──────►│   mozilla.org                │   │
  │  │                      │  Fed.  │                              │   │
  │  │  ┌────────────────┐  │  API   │  ┌────────────────────────┐  │   │
  │  │  │  Room State    │  │        │  │  Room State (replica)  │  │   │
  │  │  │  (event DAG)   │  │        │  │  (event DAG)           │  │   │
  │  │  └────────────────┘  │        │  └────────────────────────┘  │   │
  │  │                      │        │                              │   │
  │  │  ┌────────────────┐  │        │  ┌────────────────────────┐  │   │
  │  │  │  Media Repo    │  │        │  │  Media Repo            │  │   │
  │  │  └────────────────┘  │        │  └────────────────────────┘  │   │
  │  └──────────┬───────────┘        └──────────────┬───────────────┘   │
  │             │ Client-Server API                  │ Client-Server API  │
  │             ▼                                    ▼                   │
  │  ┌──────────────────────┐        ┌──────────────────────────────┐   │
  │  │  Element (web/mobile)│        │  FluffyChat (mobile)         │   │
  │  │  @alice:matrix.org   │        │  @bob:mozilla.org            │   │
  │  └──────────────────────┘        └──────────────────────────────┘   │
  └─────────────────────────────────────────────────────────────────────┘

  Federation API:  HTTPS + ed25519 server signatures (mutual auth)
  Client-Server:   HTTPS + Bearer access tokens
  Both:            REST over JSON
```

A **room** is the central concept — a shared conversation space that can span
multiple homeservers. Every homeserver that has a user in the room holds a
complete replica of that room's event history. There is no "master copy": all
servers are peers.

## Key Concepts

### The Room Event DAG

A Matrix room is not a simple list of messages. It is a **Directed Acyclic
Graph (DAG)** of events. Each event points to the events that came before it.

**Why a DAG instead of a linear list?** In a federated system, two homeservers
can independently accept new events at the same time, without knowing about
each other's events yet. When they reconnect, both events are valid — they
happened in parallel branches of history. A linear list would force you to
pick one ordering and discard the other. A DAG lets both events exist, with
the next event authored after reconnection merging the two branches.

```
Simple linear history (single server):

  E1 ──► E2 ──► E3 ──► E4
  (create) (join) (msg) (msg)


Split-brain scenario (two servers, federated room):

  Server A creates E3, Server B creates E3' at the same time.
  Neither knows about the other's event yet.

  E1 ──► E2 ──► E3        (Server A's view)
                E3'       (Server B's view, same prev: E2)

  Later, when servers sync, E4 is authored. It has prev_events
  pointing to BOTH E3 and E3':

  E1 ──► E2 ──► E3 ──┐
              └── E3' ──►  E4   (merges both branches)

  This is legal and expected. The DAG represents reality faithfully.
```

Every event in the DAG has two critical pointer fields:

- **prev_events**: the event IDs of the events immediately preceding this
  one. Usually one event (linear history), sometimes two or more (merging
  branches after a fork).
- **auth_events**: the event IDs of the state events that authorized this
  event. For example, a message event's auth_events include the sender's
  `m.room.member` event (proving they are in the room) and the
  `m.room.power_levels` event (proving they have permission to send).

### Event Types

Matrix events fall into two categories: **state events** and **message events**.

**State events** define the current configuration of the room. Each state event
has a `type` and a `state_key`. The combination `(type, state_key)` uniquely
identifies one "slot" in the room state. Only the most recent state event for
each `(type, state_key)` pair is considered "current state."

```
State Event Slots
═════════════════

  (m.room.create, "")                — room creation event; always first
  (m.room.name, "")                  — display name of the room
  (m.room.topic, "")                 — room topic
  (m.room.avatar, "")                — room avatar image URL
  (m.room.join_rules, "")            — who can join (public, invite, knock)
  (m.room.history_visibility, "")    — who can see history
  (m.room.power_levels, "")          — who can do what (numeric levels)
  (m.room.encryption, "")            — encryption algorithm (if E2E enabled)
  (m.room.member, "@alice:matrix.org") — Alice's membership (join/invite/leave)
  (m.room.member, "@bob:mozilla.org")  — Bob's membership
  (m.space.child, "!roomid:server")   — space child room link
```

**Message events** are one-off events that do not replace each other. Each one
is a new entry in the DAG history.

```
Message Event Types
═══════════════════

  m.room.message        — a text, image, file, or audio message
  m.room.redaction      — marks another event as deleted
  m.reaction            — emoji reaction to an event
  m.room.encrypted      — an E2E-encrypted event payload
  m.sticker             — a sticker image
  m.call.invite         — WebRTC call invitation
  m.call.answer         — WebRTC call answer
  m.call.hangup         — WebRTC call termination
```

### The Event Format

Every Matrix event — whether a message, a membership change, or room creation
— shares a common structure.

```
Matrix Event Structure
══════════════════════

  ┌─────────────────────┬──────────────────────────────────────────────────┐
  │ Field               │ Description                                      │
  ├─────────────────────┼──────────────────────────────────────────────────┤
  │ type                │ Event type string, e.g. "m.room.message"         │
  │                     │ Namespaced: m.* (Matrix), org.* (custom)         │
  ├─────────────────────┼──────────────────────────────────────────────────┤
  │ room_id             │ The room this event belongs to.                  │
  │                     │ Format: !<localpart>:<homeserver>                │
  │                     │ Example: !jNBbFgSWJBjEtmVquB:matrix.org          │
  ├─────────────────────┼──────────────────────────────────────────────────┤
  │ sender              │ The user who sent this event.                    │
  │                     │ Format: @<localpart>:<homeserver>                │
  │                     │ Example: @alice:matrix.org                       │
  ├─────────────────────┼──────────────────────────────────────────────────┤
  │ event_id            │ Unique identifier for this event.                │
  │                     │ Room version 4+: URL-safe base64 of SHA-256      │
  │                     │ hash of the canonical JSON of the event.         │
  │                     │ Format: $<hash>                                  │
  │                     │ Example: $oGO4s_HoAGaAHk5OwzokJEuVXbKhKGLrFIi  │
  ├─────────────────────┼──────────────────────────────────────────────────┤
  │ origin_server_ts    │ Millisecond Unix timestamp set by the            │
  │                     │ originating homeserver. NOT trustworthy for      │
  │                     │ ordering (servers can lie); use the DAG for      │
  │                     │ causal ordering.                                  │
  ├─────────────────────┼──────────────────────────────────────────────────┤
  │ content             │ The event payload — a JSON object whose          │
  │                     │ schema depends on the event type.                │
  ├─────────────────────┼──────────────────────────────────────────────────┤
  │ state_key           │ Present only on state events. The slot key.     │
  │                     │ Empty string "" for room-level state.            │
  │                     │ User ID for membership events.                   │
  ├─────────────────────┼──────────────────────────────────────────────────┤
  │ prev_events         │ List of event IDs of the DAG parents.           │
  │                     │ Creates the DAG edges (pointing backward).       │
  ├─────────────────────┼──────────────────────────────────────────────────┤
  │ auth_events         │ List of event IDs of the state events that      │
  │                     │ authorized this event. Verified at acceptance.   │
  ├─────────────────────┼──────────────────────────────────────────────────┤
  │ depth               │ Integer: approximate topological position in     │
  │                     │ the DAG. depth = max(prev depths) + 1.           │
  │                     │ The room create event has depth 1.               │
  ├─────────────────────┼──────────────────────────────────────────────────┤
  │ hashes              │ Content hashes for event integrity verification: │
  │                     │   {"sha256": "<base64 of SHA-256 of redacted    │
  │                     │               event JSON>"}                      │
  ├─────────────────────┼──────────────────────────────────────────────────┤
  │ signatures          │ ed25519 signatures by the originating server:   │
  │                     │   {"server.org": {"ed25519:key_id": "<sig>"}}   │
  ├─────────────────────┼──────────────────────────────────────────────────┤
  │ unsigned            │ Extra data not included in the event hash or     │
  │                     │ signatures. Safe for servers to add/modify.      │
  │                     │ Contains: age, redacted_because, transaction_id  │
  └─────────────────────┴──────────────────────────────────────────────────┘
```

#### Event ID: The Reference Hash

In room version 4 and above, an event's ID is derived from its content. This
makes events **self-certifying**: if you have the event and the event ID, you
can verify they match.

```
Computing an event_id (room version 4+):

  1. Take the event JSON (the "canonical" form, see below).
  2. Remove the "unsigned" field (it is mutable, not part of identity).
  3. Compute SHA-256 of the UTF-8 bytes of this canonical JSON.
  4. Encode with URL-safe base64 (no padding).
  5. Prepend "$".

  event_id = "$" + base64url_nopad(sha256(canonical_json(event)))

  Canonical JSON rules:
    - Object keys sorted lexicographically (Unicode code point order).
    - No extra whitespace (no spaces, no indentation, no newlines).
    - Unicode strings as-is (no \uXXXX escapes unless necessary).
    - Numbers: integers as integers, no trailing .0.
```

### Power Levels and Authorization

The `m.room.power_levels` state event is the permission system. It maps users
and event types to integer "power levels." A user with power level 50 can do
everything a user with power level 50 or below is allowed to do.

```
m.room.power_levels event content (typical defaults):

{
  "ban": 50,            // power level needed to ban a user
  "invite": 0,          // power level needed to invite
  "kick": 50,           // power level needed to kick
  "redact": 50,         // power level needed to redact others' events
  "state_default": 50,  // default power level for state events
  "events_default": 0,  // default power level for message events
  "users_default": 0,   // default power level for new users
  "users": {
    "@alice:matrix.org": 100,   // Alice is room admin
    "@bot:matrix.org": 50       // bot is moderator
  },
  "events": {
    "m.room.name": 50,          // changing room name needs level 50
    "m.room.power_levels": 100  // only admins can change power levels
  }
}
```

When a server receives an event, it checks auth_events. For example, for a
message event:
1. The sender's `m.room.member` event must show `membership: join`.
2. The sender's power level (from `m.room.power_levels`) must be at or above
   `events_default` (or the specific override for that event type).
3. If the event is a state event, the sender's power level must be at or above
   `state_default` (or the specific override).

### State Resolution: The Hard Problem

**The fundamental problem:** Two homeservers in the same federated room both
accept different, conflicting state events concurrently. When they reconnect,
whose state wins?

**Analogy:** Imagine two editors both editing the same Wikipedia article
offline. When they reconnect, which version of the article is "correct"? Git
handles this with merges and explicit conflict resolution. Matrix handles it
with a deterministic algorithm that all servers compute independently and
arrive at the same answer — no coordination needed.

#### State Resolution v1 (Historical, Broken)

The original algorithm (used in room versions 1 and 2) compared conflicting
state events by:
1. Looking up the power levels of the senders.
2. If power levels differ, the higher-power sender's event wins.
3. If power levels are equal, the event with the lexicographically greater
   event ID wins.

This was broken in subtle ways:

```
The v1 Bug (simplified):

  Suppose Alice is room admin (power level 100).
  Alice sends two conflicting state events E1 and E2.
  E1: sets m.room.join_rules to "public"
  E2: sets m.room.join_rules to "invite"

  Under v1, the algorithm compares Alice's power level in E1's auth chain
  vs Alice's power level in E2's auth chain. If Alice had DEMOTED herself
  in E2's auth chain... the algorithm could be tricked into accepting a
  demoted user's event, even if that demotion itself was unauthorized.

  Specifically: a user could author an event that appears to change their
  own power level, and the v1 algorithm would use that self-authored power
  level to resolve the conflict — circular and exploitable.
```

#### State Resolution v2 (Current, Room Version 2+)

State resolution v2 is the algorithm Matrix uses today. It is complex but
correct. The key insight: rather than comparing sender power levels directly,
the algorithm uses the **auth chain** — the transitive closure of all events
that authorized an event — as the ground truth.

```
State Resolution v2 Algorithm
══════════════════════════════

  Inputs:
    - Two (or more) competing state sets S1, S2, ...
      Each state set is: {(type, state_key) → event_id}

  Step 1: Classify events as "conflicted" or "unconflicted"

    For each (type, state_key) slot:
      If all state sets agree on the same event → unconflicted
      If state sets disagree → conflicted

    Unconflicted state forms the base. It is trusted without further work.

  Step 2: Compute auth_chain_union

    For each conflicted event, collect all events in its auth chain
    (recursively following auth_events pointers) plus the event itself.
    Take the union across all conflicted events.

    auth_chain_union = ⋃ auth_chains of all conflicted events

  Step 3: Filter auth_chain_union

    Keep only events in auth_chain_union that themselves are state events
    and appear in the conflicted set. These form the "conflicted auth events."

  Step 4: Resolve conflicted auth events first (recursively)

    The conflicted auth events must be resolved before we can resolve the
    other conflicted events. This recursion bottoms out because auth chains
    are finite and acyclic.

  Step 5: Apply the "reverse topological power ordering"

    With the auth events resolved, we can now compute power levels at the
    "time" of each conflicted event. Sort conflicted events by:
      (a) Power level of sender at event time (descending: higher power first)
      (b) Origin server timestamp (ascending: older events first)
      (c) Event ID (lexicographic, for determinism)

  Step 6: Iteratively apply conflicted events in this order

    Start with the unconflicted base state. For each conflicted event in
    the sorted order:
      - Check whether this event would be authorized by the current
        partial state (i.e., run auth checks against the state-so-far).
      - If authorized: apply it (update the state slot).
      - If not authorized: skip it.

  Result: a single resolved state set that all servers compute identically.
```

**Concrete example — split-brain scenario:**

```
Setup:
  Room has Alice (admin, power 100) and Bob (moderator, power 50).
  Both servers have this state.

  Server A (while offline from Server B):
    Bob kicks Carol: Carol's membership → "leave"
    Bob changes room name to "Team Chat"

  Server B (while offline from Server A):
    Alice re-invites Carol: Carol's membership → "invite"
    Alice changes room name to "Project Alpha"

  When servers reconnect, the conflicted slots are:
    (m.room.member, @carol) — Bob says "leave", Alice says "invite"
    (m.room.name, "")       — Bob says "Team Chat", Alice says "Project Alpha"

  Resolution:
    For the membership conflict:
      Bob's power level = 50, Alice's power level = 100
      Alice has higher power → Carol's membership = "invite" wins

    For the room name conflict:
      Alice's power level = 100 (either way)
      Both events have the same sender power level.
      Fall back to origin_server_ts → the older event wins.
      (Or event ID lexicographic comparison if timestamps are equal.)
```

### The Room Version System

Why do room versions exist? Because Matrix is a federated protocol. You cannot
run a "flag day" where all servers simultaneously switch to a new algorithm.
Some servers will be running old software.

Room versions solve this: each room has a version number embedded in the
`m.room.create` event. All servers must understand the algorithms for the
versions of rooms they participate in. Upgrading a room creates a new room
(with a tombstone event in the old room and a `m.room.create` pointing to it).

```
Current room versions (as of Matrix spec v1.x):

  Version 1  — Original. State resolution v1. Deprecated.
  Version 2  — State resolution v2. Deprecated.
  Version 3  — Event IDs as hashes. Deprecated.
  Version 4  — URL-safe base64 event IDs. Reference implementation default.
  Version 5  — Stricter auth rules for power level events. Deprecated.
  Version 6  — Stricter JSON validation. Deprecated.
  Version 7  — Knock join rule support. Deprecated.
  Version 8  — Restricted join rule (spaces). Deprecated.
  Version 9  — Knock+restricted rule. Deprecated.
  Version 10 — Catch-up auth rules fix. Current stable.
  Version 11 — Room creation redaction changes. Current stable.
```

## Client-Server API

The Client-Server API is REST over HTTPS, versioned at `/_matrix/client/v3/`.
Clients authenticate with a Bearer access token in the Authorization header.

```
Authorization: Bearer <access_token>
```

### Authentication: Login

```
POST /_matrix/client/v3/login

Request body:
{
  "type": "m.login.password",
  "identifier": {
    "type": "m.id.user",
    "user": "alice"
  },
  "password": "hunter2",
  "device_id": "MYDEVICE",           // optional; server generates if absent
  "initial_device_display_name": "My Laptop"
}

Response:
{
  "access_token": "syt_YWxpY2U_AbCdEfGhIjKl_12345",
  "device_id": "MYDEVICE",
  "user_id": "@alice:matrix.org",
  "home_server": "matrix.org",       // deprecated but still present
  "well_known": {
    "m.homeserver": {
      "base_url": "https://matrix.org"
    }
  }
}
```

Clients can also authenticate via SSO (single sign-on). The flow redirects the
user to the homeserver's SSO provider, which issues a login token that the
client exchanges for an access token via `m.login.token`.

### Sync: The Core Mechanism

The `/sync` endpoint is the heartbeat of every Matrix client. It returns
everything that has changed since the last sync.

```
GET /_matrix/client/v3/sync?since=<next_batch>&timeout=30000

  since      — token from the previous sync response (omit for initial sync)
  timeout    — long-poll duration in milliseconds. The server holds the
               connection open and returns when there are new events OR the
               timeout expires.
  filter     — JSON filter to limit which rooms/events are returned
  full_state — if true, return full room state (not just state changes)
```

**The sync response** is the most complex data structure in the Client-Server
API. It contains everything the client needs to update its local state.

```json
{
  "next_batch": "s72595_4483_1934",
  "account_data": {
    "events": [
      {
        "type": "m.direct",
        "content": {
          "@bob:mozilla.org": ["!directroom:matrix.org"]
        }
      }
    ]
  },
  "rooms": {
    "join": {
      "!jNBbFgSWJBjEtmVquB:matrix.org": {
        "summary": {
          "m.heroes": ["@bob:mozilla.org"],
          "m.joined_member_count": 2,
          "m.invited_member_count": 0
        },
        "state": {
          "events": [
            {
              "type": "m.room.name",
              "sender": "@alice:matrix.org",
              "state_key": "",
              "content": { "name": "Project Alpha" },
              "event_id": "$abc123",
              "origin_server_ts": 1609459200000
            }
          ]
        },
        "timeline": {
          "events": [
            {
              "type": "m.room.message",
              "sender": "@bob:mozilla.org",
              "content": {
                "msgtype": "m.text",
                "body": "Hello from Mozilla!"
              },
              "event_id": "$def456",
              "origin_server_ts": 1609459260000
            },
            {
              "type": "m.room.message",
              "sender": "@alice:matrix.org",
              "content": {
                "msgtype": "m.text",
                "body": "Hi Bob! Great to connect."
              },
              "event_id": "$ghi789",
              "origin_server_ts": 1609459320000
            }
          ],
          "limited": true,
          "prev_batch": "t27-54992_4_0_2_0_0_0_0_0"
        },
        "ephemeral": {
          "events": [
            {
              "type": "m.typing",
              "content": {
                "user_ids": ["@carol:matrix.org"]
              }
            },
            {
              "type": "m.receipt",
              "content": {
                "$def456": {
                  "m.read": {
                    "@alice:matrix.org": {
                      "ts": 1609459310000
                    }
                  }
                }
              }
            }
          ]
        },
        "account_data": {
          "events": [
            {
              "type": "m.fully_read",
              "content": {
                "event_id": "$def456"
              }
            }
          ]
        },
        "unread_notifications": {
          "notification_count": 1,
          "highlight_count": 0
        }
      }
    },
    "invite": {
      "!invited_room:example.com": {
        "invite_state": {
          "events": [
            {
              "type": "m.room.member",
              "sender": "@carol:example.com",
              "state_key": "@alice:matrix.org",
              "content": { "membership": "invite" },
              "event_id": "$invite_evt"
            }
          ]
        }
      }
    },
    "leave": {}
  },
  "to_device": {
    "events": [
      {
        "type": "m.room_key",
        "sender": "@bob:mozilla.org",
        "content": {
          "algorithm": "m.megolm.v1.aes-sha2",
          "room_id": "!jNBbFgSWJBjEtmVquB:matrix.org",
          "session_id": "XRvzMNPbNNmRuuLsxELW1g",
          "session_key": "AgAAAABTyn3o..."
        }
      }
    ]
  },
  "device_lists": {
    "changed": ["@bob:mozilla.org"],
    "left": []
  }
}
```

The sync response has these top-level sections:

```
Sync Response Sections
══════════════════════

  ┌──────────────────────┬──────────────────────────────────────────────┐
  │ Section              │ Description                                  │
  ├──────────────────────┼──────────────────────────────────────────────┤
  │ next_batch           │ Opaque token — pass as "since" next call.    │
  │                      │ The server's "here is where you are in the   │
  │                      │ stream."                                      │
  ├──────────────────────┼──────────────────────────────────────────────┤
  │ rooms.join           │ Rooms the user is currently in. Each has:    │
  │                      │   state: state events that changed since last │
  │                      │     sync (or full state if initial/full_state)│
  │                      │   timeline: message events since last sync   │
  │                      │   ephemeral: typing, receipts (not in DAG)  │
  │                      │   account_data: per-room account data        │
  ├──────────────────────┼──────────────────────────────────────────────┤
  │ rooms.invite         │ Rooms the user has been invited to.          │
  │                      │   invite_state: minimal state needed to      │
  │                      │   display the invitation.                    │
  ├──────────────────────┼──────────────────────────────────────────────┤
  │ rooms.leave          │ Rooms the user has left or been kicked from. │
  ├──────────────────────┼──────────────────────────────────────────────┤
  │ account_data         │ User-wide data: m.direct (DM room map),      │
  │                      │ m.push_rules, m.ignored_user_list            │
  ├──────────────────────┼──────────────────────────────────────────────┤
  │ to_device            │ Messages sent directly to this device (not   │
  │                      │ via a room). Used for Olm key delivery.      │
  ├──────────────────────┼──────────────────────────────────────────────┤
  │ device_lists         │ Device changes: which users' device lists    │
  │                      │ need to be re-queried (for E2E key sync).    │
  ├──────────────────────┼──────────────────────────────────────────────┤
  │ device_one_time_keys │ Count of remaining one-time keys on the     │
  │ _count               │ server (so the client knows when to upload   │
  │                      │ more).                                        │
  └──────────────────────┴──────────────────────────────────────────────┘
```

**Ephemeral events** — typing notifications and read receipts — are not stored
in the room DAG. They are best-effort, can be lost, and are not part of the
event history. This is intentional: there is no need to store "Alice was typing
at 3pm" permanently.

### Sending Events

```
Sending a message event (idempotent):
  PUT /_matrix/client/v3/rooms/{roomId}/send/{eventType}/{txnId}

  roomId:    !jNBbFgSWJBjEtmVquB:matrix.org
  eventType: m.room.message
  txnId:     client-generated random string, e.g. "m1609459400.1"
             Same txnId = same event (retry safety)

  Body: the event content object
  {
    "msgtype": "m.text",
    "body": "Hello, Matrix!"
  }

  Response:
  { "event_id": "$oGO4s_HoAGaAHk5OwzokJEuVXbKhKGLrFIi..." }
```

The `txnId` provides **idempotency**: if the client sends the request and the
network drops before it receives the response, it can resend with the same
`txnId`. The server returns the same `event_id` without creating a duplicate
event.

```
Sending a state event:
  PUT /_matrix/client/v3/rooms/{roomId}/state/{eventType}/{stateKey}

  Example: change room name
  PUT /rooms/!abc:matrix.org/state/m.room.name/
  Body: { "name": "New Room Name" }

  Example: set room topic
  PUT /rooms/!abc:matrix.org/state/m.room.topic/
  Body: { "topic": "Discussion about project goals" }
```

### Room Lifecycle

```
Room Creation:
  POST /_matrix/client/v3/createRoom
  {
    "preset": "private_chat",       // private_chat | public_chat | trusted_private_chat
    "name": "My Project Room",
    "topic": "Discussion about Project X",
    "invite": ["@bob:mozilla.org"],
    "initial_state": [
      {
        "type": "m.room.encryption",
        "state_key": "",
        "content": { "algorithm": "m.megolm.v1.aes-sha2" }
      }
    ]
  }

  Response: { "room_id": "!abc123:matrix.org" }

Joining a Room:
  POST /_matrix/client/v3/rooms/{roomId}/join
  — or —
  POST /_matrix/client/v3/join/{roomIdOrAlias}

  For public rooms, this is immediate.
  For invite-only rooms, the user must first be invited.

Inviting a User:
  POST /_matrix/client/v3/rooms/{roomId}/invite
  { "user_id": "@carol:example.com" }

Leaving a Room:
  POST /_matrix/client/v3/rooms/{roomId}/leave
```

### Media

Matrix has a separate media repository subsystem. Binary files are not stored
as event content (that would be too large for JSON). Instead, they are uploaded
separately and referenced by URI.

```
Upload:
  POST /_matrix/media/v3/upload?filename=photo.jpg
  Content-Type: image/jpeg
  Body: <raw binary bytes>

  Response: { "content_uri": "mxc://matrix.org/AbCdEfGh" }

  The "mxc://" URI is Matrix-specific. It encodes:
    mxc://<server_name>/<media_id>

Download:
  GET /_matrix/media/v3/download/{serverName}/{mediaId}
  GET /_matrix/media/v3/thumbnail/{serverName}/{mediaId}?width=128&height=128&method=scale

  Thumbnail generation is done server-side.
```

When sending an image message:
```json
{
  "msgtype": "m.image",
  "body": "photo.jpg",
  "url": "mxc://matrix.org/AbCdEfGh",
  "info": {
    "mimetype": "image/jpeg",
    "size": 102400,
    "w": 800,
    "h": 600,
    "thumbnail_url": "mxc://matrix.org/ThumbnailId",
    "thumbnail_info": {
      "mimetype": "image/jpeg",
      "size": 8192,
      "w": 128,
      "h": 96
    }
  }
}
```

## Server-Server (Federation) API

The Federation API is how homeservers talk to each other. It is HTTPS with
**mutual authentication** — both servers verify each other's identity using
ed25519 signing keys.

### Server Discovery

Before Server A can talk to Server B (`mozilla.org`), it must find B's address.
The discovery process follows this chain:

```
Discovery Chain for "mozilla.org":

  1. Check for a .well-known file:
     GET https://mozilla.org/.well-known/matrix/server
     Response: { "m.server": "matrix.mozilla.org:8448" }
     → Use matrix.mozilla.org:8448

  2. If .well-known not found or has no port:
     Look up SRV DNS record:
     _matrix._tcp.mozilla.org → SRV 10 0 8448 matrix.mozilla.org

  3. If no SRV record:
     Fall back to mozilla.org:8448 (default Matrix federation port)

  4. If the .well-known specifies an IP:port directly:
     Use it, but verify the TLS certificate matches the original domain.
```

### Server Authentication: Signing Keys

Every homeserver has one or more ed25519 key pairs for signing federation
requests and events.

```
Key Publication:
  GET /_matrix/key/v2/server
  Response:
  {
    "server_name": "matrix.org",
    "valid_until_ts": 1640000000000,
    "verify_keys": {
      "ed25519:a_RXGa": {
        "key": "<base64-encoded public key>"
      }
    },
    "old_verify_keys": {
      "ed25519:auto": {
        "expired_ts": 1609459200000,
        "key": "<base64-encoded old public key>"
      }
    },
    "signatures": {
      "matrix.org": {
        "ed25519:a_RXGa": "<base64 signature over canonical JSON>"
      }
    }
  }
```

**Authorization header for federation requests:**

Requests between servers are authenticated with an `Authorization` header
containing an `X-Matrix` signature:

```
Authorization: X-Matrix origin="matrix.org",
                         destination="mozilla.org",
                         key="ed25519:a_RXGa",
                         sig="<base64 sig over canonical JSON of {method, uri, destination, origin, content}>"
```

### The Federation Transaction

Events are sent between servers in **transactions**. A transaction is a batch
of PDUs (Persistent Data Units, i.e., room events) and EDUs (Ephemeral Data
Units, i.e., typing notifications, presence).

```
PUT /_matrix/federation/v1/send/{txnId}

Transaction JSON format:
{
  "origin": "matrix.org",
  "origin_server_ts": 1609459400000,
  "pdus": [
    {
      "type": "m.room.message",
      "sender": "@alice:matrix.org",
      "room_id": "!jNBbFgSWJBjEtmVquB:matrix.org",
      "event_id": "$oGO4s_HoAGaAHk5OwzokJEuVXbKhKGLrFIi",
      "origin_server_ts": 1609459380000,
      "content": {
        "msgtype": "m.text",
        "body": "Hello from matrix.org!"
      },
      "prev_events": ["$previous_event_id"],
      "auth_events": [
        "$room_create_event_id",
        "$alice_member_event_id",
        "$power_levels_event_id"
      ],
      "depth": 42,
      "hashes": { "sha256": "abc123..." },
      "signatures": {
        "matrix.org": { "ed25519:a_RXGa": "<sig>" }
      }
    }
  ],
  "edus": [
    {
      "edu_type": "m.typing",
      "content": {
        "room_id": "!jNBbFgSWJBjEtmVquB:matrix.org",
        "user_id": "@alice:matrix.org",
        "typing": true
      }
    }
  ]
}
```

```
PDU (Persistent Data Unit):    a room event that becomes part of the DAG.
                                Must be validated, auth-checked, stored.
EDU (Ephemeral Data Unit):     ephemeral data. Not stored, best-effort,
                                dropped if the server is offline.
```

### Receiving Events: Validation

When a server receives a PDU, it must:

```
PDU Validation Steps
════════════════════

  1. Check event_id matches SHA-256 hash of canonical JSON.
  2. Check signatures — the originating server's ed25519 signature must
     verify over the canonical JSON of the event.
  3. Fetch unknown auth_events from the originating server if needed.
  4. Run auth checks using auth_events:
     - Is the sender a member of the room?
     - Do they have sufficient power level?
     - If state event: do they have permission to set this state?
  5. Run state resolution if this event creates a DAG fork.
  6. Accept and store the event.
  7. Send to local clients via /sync.

  If any step fails: reject the event (do not store, do not forward).
```

### Backfilling

If a server is offline for a period and misses some events, it can fetch them
retroactively:

```
GET /_matrix/federation/v1/backfill/{roomId}?v={eventId}&limit=100

  v=     — start fetching backward from this event ID
  limit= — how many events to return

  Returns: a Transaction object with the historical PDUs.
```

### The Join Protocol

Joining a federated room requires a two-step handshake. This ensures the
joining server gets a valid copy of the room state and that the join event
is properly authorized.

```
Join Protocol Sequence
══════════════════════

  Joining server (J):  matrix.org — user @alice wants to join
  Resident server (R): mozilla.org — already in the room

  Step 1: make_join — get a draft event to sign
  ─────────────────────────────────────────────
  J → R: GET /_matrix/federation/v2/make_join/{roomId}/{userId}
         ?ver=10&ver=11   (supported room versions)

  R → J: {
    "room_version": "10",
    "event": {
      "type": "m.room.member",
      "room_id": "!room:mozilla.org",
      "sender": "@alice:matrix.org",
      "state_key": "@alice:matrix.org",
      "content": { "membership": "join" },
      "prev_events": ["$current_forward_extremity"],
      "auth_events": ["$create", "$alice_prev_member", "$power_levels"],
      "depth": 105
      // Note: no event_id, no signatures, no hashes yet
    }
  }

  Step 2: send_join — J signs the event and sends it back
  ────────────────────────────────────────────────────────
  J: computes hashes, signs with its own key.
  J → R: PUT /_matrix/federation/v2/send_join/{roomId}/{eventId}
         Body: the signed join event

  R → J: {
    "event": { <the join event, now with room's signatures> },
    "state": [ <full current state of the room> ],
    "auth_chain": [ <all events in the auth chain of the state> ],
    "servers_in_room": ["matrix.org", "mozilla.org", "example.com"]
  }

  J: validates state, runs state resolution, stores room.
  J: sends the join event to all servers in the room.
```

### Key Notary

Servers can ask other servers to vouch for a third server's keys. This is
useful when a server's keys have rotated and a historical event was signed
with an old key.

```
GET /_matrix/key/v2/query
Body:
{
  "server_keys": {
    "matrix.org": {
      "ed25519:a_RXGa": {
        "minimum_valid_until_ts": 1609459000000
      }
    }
  }
}

Response: keys with signatures from both the key server and matrix.org itself.
```

## End-to-End Encryption (Olm/Megolm)

Matrix homeservers see all event metadata (room IDs, sender IDs, timestamps,
event types) even when E2E encryption is enabled. E2E encryption hides only
the event **content** from the server.

**Analogy:** E2E in Matrix is like sending sealed envelopes through the post
office. The post office sees the sender, recipient, timestamp, and envelope
size — but not what is inside. The homeserver always sees that @alice sent
@bob a message in !room at 3pm. It just cannot read what the message said.

### libolm

The crypto library is **libolm** — a C library with bindings in every major
language. It implements two protocols:

- **Olm** — a 1:1 encrypted session (like Signal's X3DH + Double Ratchet)
  used for device-to-device key delivery.
- **Megolm** — a group session protocol (like Signal's Sender Keys) used
  for encrypting actual room messages.

### Olm: 1:1 Device Sessions

Olm is used exclusively for delivering encryption keys (room keys) from one
device to another. Room messages themselves use Megolm.

**Key concepts:**

```
Key Types in Olm
═════════════════

  Each device has:
  ┌─────────────────────┬────────────────────────────────────────────────┐
  │ Key                 │ Description                                    │
  ├─────────────────────┼────────────────────────────────────────────────┤
  │ Identity key        │ Long-term Curve25519 key pair. Published to    │
  │ (Curve25519)        │ the homeserver. Identifies the device.         │
  ├─────────────────────┼────────────────────────────────────────────────┤
  │ Signing key         │ Long-term Ed25519 key pair. Used to sign       │
  │ (Ed25519)           │ device keys and messages.                      │
  ├─────────────────────┼────────────────────────────────────────────────┤
  │ One-time keys       │ Short-lived Curve25519 key pairs. Generated    │
  │ (Curve25519)        │ in batches (typically 100 at a time). Each     │
  │                     │ can only be used once (for the key exchange).  │
  ├─────────────────────┼────────────────────────────────────────────────┤
  │ Fallback key        │ A "last resort" one-time key used when all     │
  │ (Curve25519)        │ one-time keys are exhausted. Can be reused.    │
  └─────────────────────┴────────────────────────────────────────────────┘
```

**Starting an Olm session (Alice → Bob's device):**

```
Olm Session Setup
══════════════════

  Phase 1: Bob publishes keys
  ───────────────────────────
  Bob's device generates:
    - Identity key pair: B_id (Curve25519)
    - Signing key pair: B_sign (Ed25519)
    - One-time key pairs: OTK_1, OTK_2, ..., OTK_100

  Bob's device uploads to homeserver:
  POST /_matrix/client/v3/keys/upload
  {
    "device_keys": {
      "user_id": "@bob:mozilla.org",
      "device_id": "BOBDEVICE",
      "algorithms": ["m.olm.v1.curve25519-aes-sha2", "m.megolm.v1.aes-sha2"],
      "keys": {
        "curve25519:BOBDEVICE": "<B_id public key>",
        "ed25519:BOBDEVICE": "<B_sign public key>"
      },
      "signatures": {
        "@bob:mozilla.org": {
          "ed25519:BOBDEVICE": "<sig over canonical JSON of device_keys>"
        }
      }
    },
    "one_time_keys": {
      "curve25519:OTK_1": "<OTK_1 public key>",
      ...
    }
  }

  Phase 2: Alice claims a one-time key
  ─────────────────────────────────────
  Alice's device:
  POST /_matrix/client/v3/keys/claim
  {
    "one_time_keys": {
      "@bob:mozilla.org": {
        "BOBDEVICE": "curve25519"
      }
    }
  }
  Response: { "one_time_keys": { "@bob:mozilla.org": { "BOBDEVICE":
    { "curve25519:OTK_1": "<OTK_1 public key>" } } } }

  Phase 3: Alice creates an Olm session
  ───────────────────────────────────────
  Alice runs X3DH key agreement:
    A_id     = Alice's identity key (Curve25519)
    A_eph    = Alice's new ephemeral key (generated for this session)
    B_id     = Bob's identity key
    OTK      = Bob's one-time key

    DH1 = DH(A_id, B_id)       // both parties' identity keys
    DH2 = DH(A_eph, B_id)      // Alice's ephemeral + Bob's identity
    DH3 = DH(A_eph, OTK)       // Alice's ephemeral + Bob's one-time key

    shared_secret = HKDF(DH1 || DH2 || DH3)

  Alice can now send an encrypted message to Bob's device.
  The first message is a "PreKeyMessage" — it carries Alice's ephemeral
  public key so Bob can reproduce the key agreement.
```

**Olm message types:**

```
  PreKeyMessage (type 0):
    Contains: Alice's identity key, Alice's ephemeral key, ciphertext
    Used for: the first message in a new Olm session
    Bob uses these keys to run X3DH in reverse and derive shared_secret.

  Message (type 1):
    Contains: just ciphertext (Double Ratchet message)
    Used for: all subsequent messages in the session
```

### Megolm: Group Sessions for Room Messages

Olm is too expensive for group messaging (you would need one Olm message per
device). Megolm is designed for one-to-many encryption in rooms.

**Analogy:** In a conference call, one person speaks and everyone hears.
Megolm works the same way: Alice generates one group session, distributes the
session key to every device in the room (using Olm), and then encrypts her
messages with that one session. Any device with the session key can decrypt.

```
Megolm Session Structure
════════════════════════

  OutboundGroupSession (Alice's side):
  ┌────────────────────┬─────────────────────────────────────────────────┐
  │ Field              │ Description                                     │
  ├────────────────────┼─────────────────────────────────────────────────┤
  │ session_id         │ Unique ID for this session. Ed25519 public key. │
  ├────────────────────┼─────────────────────────────────────────────────┤
  │ session_key        │ The initial ratchet state (random 32 bytes).    │
  │                    │ This is what gets distributed to other devices. │
  ├────────────────────┼─────────────────────────────────────────────────┤
  │ message_index      │ Counter: how many messages have been encrypted  │
  │                    │ with this session. Starts at 0.                 │
  ├────────────────────┼─────────────────────────────────────────────────┤
  │ ratchet            │ 4 SHA-256 hash chains (R0, R1, R2, R3).        │
  │                    │ Advancing the ratchet is irreversible.          │
  └────────────────────┴─────────────────────────────────────────────────┘

  InboundGroupSession (Bob's side, after receiving the key):
  ┌────────────────────┬─────────────────────────────────────────────────┐
  │ Field              │ Description                                     │
  ├────────────────────┼─────────────────────────────────────────────────┤
  │ session_id         │ Matches OutboundGroupSession.session_id.        │
  ├────────────────────┼─────────────────────────────────────────────────┤
  │ session_key        │ The initial ratchet state received from sender. │
  ├────────────────────┼─────────────────────────────────────────────────┤
  │ known_indices      │ Set of message_index values already decrypted.  │
  │                    │ Replay protection: reject duplicate indices.    │
  └────────────────────┴─────────────────────────────────────────────────┘
```

**The Megolm Ratchet:**

```
Megolm Ratchet Advancement
══════════════════════════

  State: 4 chains of 32 bytes each: R0, R1, R2, R3

  Advance():
    R0 = HMAC-SHA256(R0, 0x00)
    R1 = HMAC-SHA256(R1, 0x01)
    R2 = HMAC-SHA256(R2, 0x02)
    R3 = HMAC-SHA256(R3, 0x03)

    Every 256 steps: reset from higher chain:
      at i % 256 == 0:   R3 = HMAC-SHA256(R3, 0x03); R2 = R3; R1 = R2; R0 = R1
      at i % 65536 == 0: similar cascade from even higher

  Encryption key for message i:
    keys = HKDF(R0_i || R1_i || R2_i || R3_i, "MEGOLM_KEYS")
    → AES-256-CBC key + HMAC-SHA256 key + IV

  Property: knowing the state at message N does not reveal messages 0..N-1.
  This is one-way (forward secrecy within the session).
  However: knowing state at N reveals ALL future messages N, N+1, N+2, ...
  Megolm provides forward secrecy only if sessions are rotated.
```

**Key Distribution Flow:**

```
Megolm Key Distribution
════════════════════════

  1. Alice creates an OutboundGroupSession for room !R
     - Generates random session_key
     - Derives session_id (Ed25519 public key from session_key)

  2. Alice queries devices in the room:
     GET /keys/query: returns device keys for all room members

  3. For each device D (including her own other devices):
     a. Alice creates an Olm session to D (if not already established)
     b. Alice encrypts an m.room_key event via Olm:
        {
          "type": "m.room_key",
          "content": {
            "algorithm": "m.megolm.v1.aes-sha2",
            "room_id": "!R:matrix.org",
            "session_id": "<session_id>",
            "session_key": "<session_key>"
          }
        }
     c. Alice sends this as a to-device event via:
        PUT /sendToDevice/m.room.encrypted/{txnId}
        {
          "messages": {
            "@bob:mozilla.org": {
              "BOBDEVICE": {
                "algorithm": "m.olm.v1.curve25519-aes-sha2",
                "ciphertext": { "<bob_curve25519_key>": {
                  "type": 0,   // PreKeyMessage on first send
                  "body": "<base64 ciphertext>"
                }}
              }
            }
          }
        }

  4. Alice now encrypts all room messages with the OutboundGroupSession.

  5. When Bob's device receives the m.room_key to-device event:
     - Decrypts with its Olm session to Alice's device
     - Creates an InboundGroupSession from the session_key
     - Can now decrypt all Alice's Megolm messages in !R
```

**The encrypted room event:**

```json
{
  "type": "m.room.encrypted",
  "room_id": "!jNBbFgSWJBjEtmVquB:matrix.org",
  "sender": "@alice:matrix.org",
  "content": {
    "algorithm": "m.megolm.v1.aes-sha2",
    "sender_key": "<Alice's Curve25519 identity key>",
    "device_id": "ALICEDEVICE",
    "session_id": "XRvzMNPbNNmRuuLsxELW1g",
    "ciphertext": "<base64 of Megolm-encrypted inner event>"
  },
  "event_id": "$encrypted_event_id",
  "origin_server_ts": 1609459380000
}
```

The `ciphertext` decrypts to a full event JSON:
```json
{
  "type": "m.room.message",
  "content": {
    "msgtype": "m.text",
    "body": "This message is end-to-end encrypted."
  },
  "room_id": "!jNBbFgSWJBjEtmVquB:matrix.org"
}
```

### Device Management

```
Upload device keys and one-time keys:
  POST /_matrix/client/v3/keys/upload
  → Device registers with the homeserver.

Query another user's devices and keys:
  POST /_matrix/client/v3/keys/query
  { "device_keys": { "@bob:mozilla.org": [] } }
  → Returns Bob's devices and their keys.

Claim one-time keys (for starting an Olm session):
  POST /_matrix/client/v3/keys/claim
  { "one_time_keys": { "@bob:mozilla.org": { "BOBDEVICE": "curve25519" } } }
  → Consumes one of Bob's one-time keys.
```

### Cross-Signing

When a user has multiple devices, they need a way to verify that all their
devices belong to the same person. Cross-signing uses three additional key
layers:

```
Cross-Signing Key Hierarchy
════════════════════════════

  Master Key (MSK)
  ├── Self-Signing Key (SSK)    — signs the user's own device keys
  │   ├── Device 1 (phone)      ← signed by SSK
  │   ├── Device 2 (laptop)     ← signed by SSK
  │   └── Device 3 (desktop)    ← signed by SSK
  └── User-Signing Key (USK)   — signs other users' master keys
      ├── @bob's MSK            ← signed by USK (means "I trust Bob")
      └── @carol's MSK          ← signed by USK

  Verification chain:
    If I trust Alice's MSK (via safety number / QR code verification),
    and Alice's SSK is signed by Alice's MSK,
    and Alice's device key is signed by Alice's SSK,
    → I can trust all of Alice's devices without manual per-device verification.
```

### Key Backup

Users lose their Megolm session keys if they lose their device. Key backup
allows storing keys on the server, encrypted with a recovery key.

```
Key Backup Flow:
  1. Client generates a recovery key (random 256 bits).
  2. Client uploads a backup version:
     POST /_matrix/client/v3/room_keys/version
     {
       "algorithm": "m.megolm_backup.v1.curve25519-aes-sha2",
       "auth_data": {
         "public_key": "<ephemeral Curve25519 pubkey>",
         "signatures": { "@alice:matrix.org": { "ed25519:MSK": "<sig>" } }
       }
     }
  3. Client uploads session keys:
     PUT /_matrix/client/v3/room_keys/keys/{roomId}/{sessionId}
     {
       "first_message_index": 0,
       "forwarded_count": 0,
       "is_verified": true,
       "session_data": {
         "ephemeral": "<per-key ephemeral pubkey>",
         "ciphertext": "<AES-encrypted session_key>",
         "mac": "<HMAC>"
       }
     }
  4. To restore: client downloads backup, decrypts with recovery key.
```

## Spaces

Spaces are rooms that contain other rooms. They provide a folder-like
hierarchy for organizing rooms.

```
Space Hierarchy Example
═══════════════════════

  !acme-company:matrix.org  (Space: "ACME Company")
  ├── !engineering:matrix.org     (Space: "Engineering")
  │   ├── !backend:matrix.org    (Room: "Backend Team")
  │   ├── !frontend:matrix.org   (Room: "Frontend Team")
  │   └── !devops:matrix.org     (Room: "DevOps")
  ├── !marketing:matrix.org       (Space: "Marketing")
  │   └── !campaigns:matrix.org  (Room: "Campaign Planning")
  └── !general:matrix.org         (Room: "General Discussion")
```

Space relationships are represented as state events:

```
In the parent space room:
  State event type: m.space.child
  State key: !backend:matrix.org   (the child room ID)
  Content:
  {
    "via": ["matrix.org"],    // servers to use to join the child
    "suggested": true         // this child is suggested to new space members
  }

In the child room:
  State event type: m.space.parent
  State key: !acme-company:matrix.org   (the parent space ID)
  Content:
  {
    "via": ["matrix.org"],
    "canonical": true         // this is the "canonical" parent
  }
```

Computing space membership: a client fetches the state of the space room,
finds all `m.space.child` state events, and recursively fetches the children.
The server endpoint `/_matrix/client/v1/rooms/{roomId}/hierarchy` simplifies
this into a single paginated API call.

## Algorithms

### Canonical JSON

Matrix's deterministic JSON serialization (for hashing and signing):

```
canonical_json(value):
  if value is null:    return "null"
  if value is bool:    return "true" or "false"
  if value is number:  return integer representation (no decimal for integers)
  if value is string:  return JSON-encoded string (escape only \, ", and control chars)
  if value is array:   return "[" + join(canonical_json(each), ",") + "]"
  if value is object:
    sorted_keys = sort(keys(value))  // lexicographic Unicode code point order
    pairs = ['"' + key + '":' + canonical_json(value[key]) for key in sorted_keys]
    return "{" + join(pairs, ",") + "}"
```

### Computing the Event ID

```
compute_event_id(event):
  // Remove mutable fields
  event_copy = copy(event)
  delete event_copy["unsigned"]
  delete event_copy["signatures"]   // not included in hash input
  delete event_copy["hashes"]       // not included in hash input

  // Hash the redacted event (what remains after removing content-sensitive fields)
  redacted = redact(event_copy)     // removes non-essential content fields
  hash_bytes = sha256(canonical_json(redacted))
  return "$" + base64url_nopad(hash_bytes)
```

### State Resolution v2 (Full Algorithm)

```
resolve_state(state_sets):
  // state_sets: list of {(type, state_key) → event} dicts

  // Step 1: partition into conflicted and unconflicted
  unconflicted = {}
  conflicted = []
  for each (type, state_key) slot:
    values = [s.get(slot) for s in state_sets]
    unique_values = deduplicate(values)
    if len(unique_values) == 1:
      unconflicted[slot] = unique_values[0]
    else:
      conflicted.extend(unique_values)

  // Step 2: compute auth chain union for conflicted events
  auth_chain_union = {}
  for event in conflicted:
    auth_chain_union.update(get_auth_chain(event))  // recursive

  // Step 3: filter to auth events that are also conflicted
  conflicted_auth_events = [e for e in auth_chain_union
                            if e in conflicted and is_state_event(e)]

  // Step 4: resolve conflicted auth events first (recursion on smaller set)
  partial_state = resolve_state([state_set restricted to auth chain events])

  // Step 5: compute power levels at each event's time
  for event in conflicted:
    event.resolved_power = power_level_of(event.sender, partial_state)

  // Step 6: sort conflicted events for iterative application
  sort key = (-resolved_power, origin_server_ts, event_id)
  sorted_conflicted = sort(conflicted, key=sort_key)

  // Step 7: iteratively apply to build resolved state
  resolved = copy(unconflicted)
  for event in sorted_conflicted:
    if auth_check(event, resolved):  // would this event be authorized?
      slot = (event.type, event.state_key)
      resolved[slot] = event

  return resolved
```

### Megolm Encryption

```
megolm_encrypt(session, plaintext):
  // session: OutboundGroupSession
  i = session.message_index

  // Derive keys from current ratchet state
  ratchet_state = R0_i || R1_i || R2_i || R3_i
  keys = HKDF(ratchet_state, salt="", info="MEGOLM_KEYS", len=80)
  aes_key = keys[0:32]      // AES-256 key
  mac_key  = keys[32:64]    // HMAC-SHA256 key
  iv       = keys[64:80]    // AES IV

  // Serialize the Megolm message
  message_body = encode_megolm_message(i, ciphertext=AES-CBC(aes_key, iv, plaintext))
  mac = HMAC-SHA256(mac_key, message_body)[0:8]  // first 8 bytes

  // Sign the full message with Ed25519
  full_message = message_body || mac
  sig = Ed25519_sign(session.signing_key, full_message)

  // Advance ratchet
  session.message_index += 1
  advance_ratchet(session.ratchet)

  return base64(full_message || sig)
```

## Test Strategy

### Event Serialization Tests

```
test_canonical_json_sorted_keys:
  input:  {"z": 1, "a": 2, "m": 3}
  expect: {"a":2,"m":3,"z":1}

test_canonical_json_nested:
  input:  {"b": {"y": 1, "x": 2}, "a": true}
  expect: {"a":true,"b":{"x":2,"y":1}}

test_event_id_computation:
  given:  a room version 4 event with known content
  expect: event_id matches $<sha256 of canonical JSON of redacted event>

test_signature_verification:
  given:  an event with a valid ed25519 signature
  expect: verify_signature() returns true
  given:  same event with tampered content
  expect: verify_signature() returns false
```

### State Resolution Tests

```
test_unconflicted_state_passes_through:
  given:  two state sets agreeing on all (type, state_key) slots
  expect: resolved state equals the agreed state, no algorithm applied

test_higher_power_level_wins:
  given:  conflicted m.room.name — admin (100) vs moderator (50)
  expect: admin's version of the state wins

test_equal_power_falls_back_to_timestamp:
  given:  conflicted state event from two users with equal power level
  expect: the event with the smaller origin_server_ts wins

test_fork_merge:
  given:  DAG where events E3 and E3' both have E2 as prev_event
  given:  E4 has prev_events = [E3, E3']
  expect: E4 is accepted, state resolved from both branches

test_circular_auth_chain_rejected:
  given:  an event whose auth_events chain eventually references itself
  expect: validation fails with an auth chain loop error
```

### Federation Tests

```
test_make_join_send_join_round_trip:
  given:  server J wanting to join a room on server R
  when:   J calls make_join, signs the event, calls send_join
  expect: R returns valid room state; J can decrypt all state events

test_signature_on_pdu_verified:
  given:  a PDU from server.org with valid ed25519 signature
  expect: receiving server accepts it
  given:  same PDU with corrupted signature
  expect: receiving server rejects it

test_backfill_returns_historical_events:
  given:  a room with 200 events
  when:   GET /backfill with limit=20 and v=event_100
  expect: returns 20 events ending at event_100 (backward from that point)
```

### Crypto Tests

```
test_olm_session_established_and_message_decrypted:
  given:  Alice creates an OutboundSession to Bob using Bob's one-time key
  when:   Alice encrypts "hello" and Bob decrypts
  expect: Bob gets "hello"

test_megolm_message_index_replay_rejected:
  given:  Bob has an InboundGroupSession that has seen message_index 5
  when:   Bob receives another message with message_index 5 (replay)
  expect: decryption fails with "replayed message" error

test_room_key_distribution_via_olm:
  given:  Alice generates a Megolm session
  when:   Alice distributes the session_key to Bob's device via to-device Olm
  expect: Bob can decrypt Alice's subsequent Megolm messages in the room

test_canonical_json_matches_reference:
  given:  the test vectors from the Matrix specification
  expect: canonical_json() output matches spec byte-for-byte

test_event_hashing_self_certifying:
  given:  an event with known content
  expect: sha256(canonical_json(redacted_event)) == decode_base64(event_id[1:])
```

### Sync Tests

```
test_incremental_sync_returns_only_new_events:
  given:  a room with 10 events; client synced through event 5
  when:   client calls /sync?since=<token after event 5>
  expect: timeline contains only events 6-10

test_timeline_limited_with_prev_batch:
  given:  a room with 1000 events; client missed 500 since last sync
  when:   client syncs with timeout=0
  expect: timeline.limited = true, timeline.prev_batch is set
  when:   client fetches /rooms/{id}/messages?from=prev_batch
  expect: can page through all missed events

test_to_device_events_delivered_once:
  given:  a to-device event sent to Alice's device
  when:   Alice syncs twice
  expect: to-device event appears in first sync only (consumed on delivery)
```
