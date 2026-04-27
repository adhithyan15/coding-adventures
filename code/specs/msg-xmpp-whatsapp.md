# MSG-XMPP-WHATSAPP — XMPP and WhatsApp

## Overview

Imagine two people on opposite ends of the world who want to have a conversation
in real time. A naïve approach would be polling: Alice asks the server every
second "Any messages for me?" This wastes enormous bandwidth, drains phone
batteries, and still feels slow. The better approach is a **persistent
connection**: Alice opens a channel to the server and the server pushes new
messages down that channel the moment they arrive.

XMPP and WhatsApp both solve this problem, but from different angles.

**Analogy: XMPP is the postal service of messaging.**
Every participant has an address (a JID, like `alice@example.com`). Servers are
like post offices: they accept mail from local users, route it to the right
destination post office, and deliver it to recipients. Any two post offices can
exchange mail directly — this is the **federated** model, the same architecture
that makes email work. You can self-host an XMPP server and talk to anyone on
any other XMPP server.

**Analogy: WhatsApp is like a private courier with a secret handshake.**
WhatsApp controls all the servers. To connect, your device performs a cryptographic
ceremony (the Noise handshake) that proves it is a registered device, establishes
an encrypted tunnel, and then speaks a compact binary dialect that carries the
same concepts as XMPP stanzas — but uses one-tenth of the bytes.

This document covers:

1. **XMPP fundamentals** — streams, stanzas, roster, service discovery, MUC
2. **WhatsApp's XMPP variant** — Noise Protocol, binary framing, protobuf messages
3. **WhatsApp E2E encryption** — Signal Protocol, X3DH, Double Ratchet, sender keys
4. **WhatsApp media** — out-of-band CDN upload/download

## Architecture

### XMPP Federated Architecture

```
  ┌──────────────────────────────────────────────────────────────────────┐
  │                       The XMPP Federation                            │
  │                                                                      │
  │  alice@example.com                     bob@other.org                 │
  │  ┌──────────────┐                      ┌──────────────┐              │
  │  │ Alice's      │  TCP port 5222        │ Bob's        │              │
  │  │ XMPP Client  │──────────────────────▶│ XMPP Client  │             │
  │  │ (Jabber,     │  (client-to-server)   │ (Conversations│            │
  │  │  Gajim, etc.)│                       │  etc.)        │            │
  │  └──────┬───────┘                       └──────┬────────┘            │
  │         │ TCP 5222                             │ TCP 5222            │
  │         ▼                                      ▼                     │
  │  ┌──────────────┐   S2S TCP 5269        ┌──────────────┐             │
  │  │ example.com  │◀────────────────────▶ │  other.org   │             │
  │  │ XMPP Server  │  (server-to-server)   │ XMPP Server  │             │
  │  │              │  RFC 7590              │              │             │
  │  └──────────────┘                       └──────────────┘             │
  │                                                                      │
  │  Key insight: servers route stanzas, like email MTA routing.         │
  │  Any XMPP server can talk to any other without central coordination. │
  └──────────────────────────────────────────────────────────────────────┘
```

### WhatsApp Architecture

```
  ┌──────────────────────────────────────────────────────────────────────┐
  │                     WhatsApp Architecture                            │
  │                                                                      │
  │  Alice's Phone                         Bob's Phone                   │
  │  ┌──────────────┐                      ┌──────────────┐              │
  │  │ WhatsApp App │                      │ WhatsApp App │              │
  │  │              │                      │              │              │
  │  │ ┌──────────┐ │                      │ ┌──────────┐ │              │
  │  │ │Signal    │ │                      │ │Signal    │ │              │
  │  │ │Protocol  │ │                      │ │Protocol  │ │              │
  │  │ │(E2E enc) │ │                      │ │(E2E enc) │ │              │
  │  │ └────┬─────┘ │                      │ └────┬─────┘ │              │
  │  │      │       │                      │      │       │              │
  │  │ ┌────▼─────┐ │                      │ ┌────▼─────┐ │              │
  │  │ │WA Binary │ │    Noise Protocol    │ │WA Binary │ │              │
  │  │ │  Codec   │ │◀────────────────────▶│ │  Codec   │ │              │
  │  │ └────┬─────┘ │    (ChaCha20-Poly)   │ └────┬─────┘ │              │
  │  └──────┼───────┘                      └──────┼───────┘              │
  │         │                                     │                      │
  │         ▼  TCP port 443                       ▼  TCP 443             │
  │  ┌─────────────────────────────────────────────────────────────┐     │
  │  │                 WhatsApp Edge Servers                        │     │
  │  │   (controlled entirely by Meta — no federation)             │     │
  │  │                                                              │     │
  │  │   ┌──────────────┐    ┌──────────────┐   ┌───────────────┐  │     │
  │  │   │ Noise/Framing│    │  Key Server  │   │   CDN (media) │  │     │
  │  │   │  Gateway     │    │ (prekey dist)│   │  upload/DL    │  │     │
  │  │   └──────────────┘    └──────────────┘   └───────────────┘  │     │
  │  └─────────────────────────────────────────────────────────────┘     │
  └──────────────────────────────────────────────────────────────────────┘
```

## Key Concepts: XMPP

### JIDs: Jabber IDs

Every XMPP entity has an address called a **JID** (Jabber ID). The format
mirrors email addresses but adds an optional third component called a
**resource**:

```
  Full JID format:
  ┌──────────────────────────────────────────────────────────────┐
  │  user@domain/resource                                        │
  │                                                              │
  │  alice@example.com/mobile    ← Alice's mobile client        │
  │  alice@example.com/laptop    ← Alice's laptop client        │
  │  alice@example.com           ← bare JID (any resource)      │
  │  conference.example.com      ← a server component (no user) │
  │  room@conference.example.com/nick ← MUC occupant JID        │
  └──────────────────────────────────────────────────────────────┘

  Components:
  ┌─────────────┬──────────────────────────────────────────────────┐
  │ Part        │ Description                                      │
  ├─────────────┼──────────────────────────────────────────────────┤
  │ localpart   │ The username. Like the part before @ in email.   │
  │ (user)      │ Optional — servers and components have no user.  │
  ├─────────────┼──────────────────────────────────────────────────┤
  │ domainpart  │ The server hostname. Required. Routes stanzas    │
  │ (domain)    │ to the correct server in the federation.         │
  ├─────────────┼──────────────────────────────────────────────────┤
  │ resourcepart│ Identifies a specific client session. One user   │
  │ (resource)  │ can have multiple clients online simultaneously. │
  │             │ The server delivers to the highest-priority one  │
  │             │ when the bare JID is used as destination.        │
  └─────────────┴──────────────────────────────────────────────────┘
```

**Why resources?** Alice might be logged in from her phone, laptop, and tablet
simultaneously. The resource lets Bob address a message specifically to her
phone, or to all her devices at once (bare JID).

### The XML Stream

XMPP is fundamentally different from HTTP. HTTP is **request/response** — you
send one message, get one reply, connection closes. XMPP is a **persistent XML
stream** — an XML document that never closes (until logout), and both sides
can send elements at any time.

```
  HTTP (request-response):
  Client → Server: GET /api/messages
  Server → Client: 200 OK\r\n{...json...}
  Connection closes.
  [repeat for each message]

  XMPP (persistent stream, RFC 6120):
  Client opens TCP connection.
  Client → Server: <stream:stream to="example.com" ...>
  Server → Client: <stream:stream from="example.com" ...>
  [stream stays OPEN]
  Client → Server: <iq type="get">...</iq>      (any time)
  Server → Client: <iq type="result">...</iq>   (any time)
  Server → Client: <message from="bob">...</message>  (server-initiated!)
  Client → Server: <presence/>
  Server → Client: <presence from="carol"/>
  ... (forever, until logout)
  Client → Server: </stream:stream>
  Server → Client: </stream:stream>
  Connection closes.
```

The persistent stream is what enables real-time push. The server can send
a `<message>` stanza to Alice the instant Bob sends it — no polling needed.

### Stream Negotiation

Before exchanging stanzas, client and server negotiate security and
authentication. The sequence is defined in RFC 6120:

```
  Stream Negotiation Sequence
  ════════════════════════════

  Step 1: Client opens stream
  ─────────────────────────────
  CLIENT → SERVER:
  <?xml version='1.0'?>
  <stream:stream
    xmlns='jabber:client'
    xmlns:stream='http://etherx.jabber.org/streams'
    to='example.com'
    version='1.0'>

  Step 2: Server replies with its feature list
  ─────────────────────────────────────────────
  SERVER → CLIENT:
  <?xml version='1.0'?>
  <stream:stream
    xmlns='jabber:client'
    xmlns:stream='http://etherx.jabber.org/streams'
    from='example.com'
    id='abc123'
    version='1.0'>
  <stream:features>
    <starttls xmlns='urn:ietf:params:xml:ns:xmpp-tls'>
      <required/>
    </starttls>
  </stream:features>

  Step 3: Client upgrades to TLS (STARTTLS)
  ──────────────────────────────────────────
  CLIENT → SERVER:
  <starttls xmlns='urn:ietf:params:xml:ns:xmpp-tls'/>

  SERVER → CLIENT:
  <proceed xmlns='urn:ietf:params:xml:ns:xmpp-tls'/>

  [Both sides perform TLS handshake. From here, all bytes are encrypted.]

  Step 4: Client re-opens the stream over TLS
  ─────────────────────────────────────────────
  CLIENT → SERVER:
  <stream:stream to='example.com' version='1.0' ...>

  SERVER → CLIENT:
  <stream:stream id='def456' ...>
  <stream:features>
    <mechanisms xmlns='urn:ietf:params:xml:ns:xmpp-sasl'>
      <mechanism>SCRAM-SHA-1</mechanism>
      <mechanism>PLAIN</mechanism>
    </mechanisms>
  </stream:features>

  Step 5: SASL Authentication (SCRAM-SHA-1 shown)
  ─────────────────────────────────────────────────
  CLIENT → SERVER:
  <auth xmlns='urn:ietf:params:xml:ns:xmpp-sasl'
        mechanism='SCRAM-SHA-1'>
    [base64 of client-first-message]
  </auth>

  SERVER → CLIENT:
  <challenge xmlns='urn:ietf:params:xml:ns:xmpp-sasl'>
    [base64 of server-first-message with nonce+salt+iterations]
  </challenge>

  CLIENT → SERVER:
  <response xmlns='urn:ietf:params:xml:ns:xmpp-sasl'>
    [base64 of client-final-message with ClientProof]
  </response>

  SERVER → CLIENT:
  <success xmlns='urn:ietf:params:xml:ns:xmpp-sasl'>
    [base64 of server-final-message with ServerSignature]
  </success>

  Step 6: Client re-opens stream (third time)
  ─────────────────────────────────────────────
  CLIENT → SERVER:
  <stream:stream to='example.com' ...>

  SERVER → CLIENT:
  <stream:stream id='ghi789' ...>
  <stream:features>
    <bind xmlns='urn:ietf:params:xml:ns:xmpp-bind'/>
    <session xmlns='urn:ietf:params:xml:ns:xmpp-session'/>
  </stream:features>

  Step 7: Resource binding
  ─────────────────────────
  CLIENT → SERVER:
  <iq type='set' id='bind1'>
    <bind xmlns='urn:ietf:params:xml:ns:xmpp-bind'>
      <resource>mobile</resource>
    </bind>
  </iq>

  SERVER → CLIENT:
  <iq type='result' id='bind1'>
    <bind xmlns='urn:ietf:params:xml:ns:xmpp-bind'>
      <jid>alice@example.com/mobile</jid>
    </bind>
  </iq>

  [Client is now fully authenticated and bound. Stanza exchange begins.]
```

### SASL Mechanisms

```
  SASL Mechanism Comparison
  ══════════════════════════

  ┌──────────────────┬────────────────────────────────────────────────┐
  │ Mechanism        │ Description                                    │
  ├──────────────────┼────────────────────────────────────────────────┤
  │ PLAIN            │ Sends username + password in cleartext (base64 │
  │                  │ encoded). MUST only be used over TLS. Simple   │
  │                  │ but the server learns the plaintext password.  │
  ├──────────────────┼────────────────────────────────────────────────┤
  │ SCRAM-SHA-1      │ Salted Challenge Response. Both sides prove    │
  │                  │ knowledge of the password without transmitting │
  │                  │ it. Server stores salted hash, not plaintext.  │
  │                  │ Mutual authentication: client also verifies    │
  │                  │ the server (prevents MITM).                    │
  ├──────────────────┼────────────────────────────────────────────────┤
  │ SCRAM-SHA-256    │ Same as SCRAM-SHA-1 but with SHA-256. Stronger.│
  ├──────────────────┼────────────────────────────────────────────────┤
  │ EXTERNAL         │ Authentication via TLS client certificate.     │
  │                  │ Client presents a certificate and the server   │
  │                  │ checks it against a known CA. Used for         │
  │                  │ server-to-server (S2S) connections.            │
  └──────────────────┴────────────────────────────────────────────────┘
```

### The Three Stanza Types

XMPP has exactly three kinds of messages, called **stanzas**. Every stanza
you ever encounter is one of these three:

```
  The XMPP Stanza Trinity
  ════════════════════════

  ┌─────────────┬──────────────────────────────────────────────────────┐
  │ Stanza      │ Purpose                                              │
  ├─────────────┼──────────────────────────────────────────────────────┤
  │ <message>   │ One-way delivery. "Here is some content." No reply   │
  │             │ required. Used for chat messages, notifications.     │
  ├─────────────┼──────────────────────────────────────────────────────┤
  │ <presence>  │ Broadcast availability. "I am here, here is my       │
  │             │ status." Used for online/offline/away state.         │
  ├─────────────┼──────────────────────────────────────────────────────┤
  │ <iq>        │ Request/response. "Info/Query." Like HTTP GET/POST.  │
  │ (info/query)│ Must receive exactly one response: result or error.  │
  │             │ Used for roster, service discovery, configuration.   │
  └─────────────┴──────────────────────────────────────────────────────┘

  All stanzas share common attributes:
  ┌───────┬───────────────────────────────────────────────────────────┐
  │ Attr  │ Description                                               │
  ├───────┼───────────────────────────────────────────────────────────┤
  │ to    │ Destination JID. If absent, the stanza goes to the server.│
  │ from  │ Sender JID. Stamped by the server, not trusted from client│
  │ id    │ Unique identifier. Links replies to requests in <iq>.     │
  │ type  │ Sub-type, varies by stanza kind (see below).              │
  │ xml   │ The language of text content (xml:lang="en").             │
  │ :lang │                                                           │
  └───────┴───────────────────────────────────────────────────────────┘
```

### The `<message>` Stanza

```
  Message stanza types:
  ┌────────────┬────────────────────────────────────────────────────────┐
  │ type=      │ Meaning                                                │
  ├────────────┼────────────────────────────────────────────────────────┤
  │ chat       │ One-to-one conversation. The most common type.         │
  │ groupchat  │ Message in a multi-user chat room.                    │
  │ headline   │ Automated alert/notification (e.g., news feed).       │
  │ normal     │ Single message (like email) without ongoing thread.   │
  │ error      │ Error reply to a previous message.                    │
  └────────────┴────────────────────────────────────────────────────────┘

  Wire example — Alice sends "Hello!" to Bob:

  <message
    to='bob@example.com'
    from='alice@example.com/mobile'
    id='msg001'
    type='chat'>
    <body>Hello!</body>
    <thread>thread-abc-123</thread>
  </message>

  Child elements of <message>:
  ┌──────────────┬──────────────────────────────────────────────────────┐
  │ Element      │ Description                                          │
  ├──────────────┼──────────────────────────────────────────────────────┤
  │ <body>       │ The human-readable message text. Can appear multiple │
  │              │ times with different xml:lang for localization.      │
  ├──────────────┼──────────────────────────────────────────────────────┤
  │ <subject>    │ Subject line. Commonly used in MUC rooms.            │
  ├──────────────┼──────────────────────────────────────────────────────┤
  │ <thread>     │ Conversation thread identifier. Lets clients group   │
  │              │ related messages together.                           │
  ├──────────────┼──────────────────────────────────────────────────────┤
  │ Extensions   │ XML extension elements in other namespaces. Examples:│
  │              │ XEP-0184 delivery receipts, XEP-0333 markers,        │
  │              │ XEP-0380 explicit message encryption indicator.      │
  └──────────────┴──────────────────────────────────────────────────────┘
```

### The `<presence>` Stanza

```
  Presence stanza types:
  ┌─────────────────┬─────────────────────────────────────────────────┐
  │ type=           │ Meaning                                         │
  ├─────────────────┼─────────────────────────────────────────────────┤
  │ (no type attr)  │ Available. "I am online."                       │
  │ unavailable     │ Going offline. "I am signing out."              │
  │ subscribe       │ Request to add to roster and see presence.      │
  │ subscribed      │ Accept a subscribe request.                     │
  │ unsubscribe     │ Stop receiving someone's presence.              │
  │ unsubscribed    │ Deny or revoke subscribe permission.            │
  │ probe           │ Server asking another server "is this user on?" │
  │ error           │ Error from a previous presence stanza.          │
  └─────────────────┴─────────────────────────────────────────────────┘

  Presence child elements:
  ┌──────────────┬──────────────────────────────────────────────────────┐
  │ Element      │ Description                                          │
  ├──────────────┼──────────────────────────────────────────────────────┤
  │ <show>       │ Sub-status: away, chat (actively chatting), dnd      │
  │              │ (do not disturb), xa (extended away). Absent = chat. │
  ├──────────────┼──────────────────────────────────────────────────────┤
  │ <status>     │ Human-readable status message. "In a meeting."       │
  ├──────────────┼──────────────────────────────────────────────────────┤
  │ <priority>   │ Integer -128 to 127. When bare JID is addressed,    │
  │              │ server delivers to resource with highest priority.   │
  └──────────────┴──────────────────────────────────────────────────────┘

  Wire examples:

  Alice goes online with status "Working from home":
  <presence from='alice@example.com/mobile'>
    <show>chat</show>
    <status>Working from home</status>
    <priority>5</priority>
  </presence>

  Alice goes away:
  <presence from='alice@example.com/mobile'>
    <show>away</show>
    <status>Be back in 20 min</status>
  </presence>

  Alice signs out:
  <presence from='alice@example.com/mobile' type='unavailable'/>
```

### The `<iq>` Stanza

```
  IQ stanza types:
  ┌────────┬────────────────────────────────────────────────────────────┐
  │ type=  │ Meaning                                                    │
  ├────────┼────────────────────────────────────────────────────────────┤
  │ get    │ Request. "Give me this information." Like HTTP GET.         │
  │ set    │ Command. "Change this thing." Like HTTP POST/PUT.           │
  │ result │ Success reply to get or set. Must echo the id attribute.   │
  │ error  │ Error reply. Contains an <error> child with the error type. │
  └────────┴────────────────────────────────────────────────────────────┘

  IQ rules:
  - Every get or set MUST receive exactly one result or error in reply.
  - The id attribute is used to correlate requests with replies.
  - A client should not send a second IQ with the same id before the
    first has been answered (though servers can handle concurrency).

  Wire example — roster fetch:
  CLIENT → SERVER:
  <iq type='get' id='roster1'>
    <query xmlns='jabber:iq:roster'/>
  </iq>

  SERVER → CLIENT:
  <iq type='result' id='roster1' from='example.com'>
    <query xmlns='jabber:iq:roster' ver='ver7'>
      <item jid='bob@example.com' name='Bob' subscription='both'>
        <group>Friends</group>
      </item>
      <item jid='carol@other.org' name='Carol' subscription='to'/>
    </query>
  </iq>
```

### Roster: The Contact List

The **roster** is XMPP's contact list, stored on the server. This is important:
when you install XMPP on a new device, your contacts are already there because
they live on the server, not the device.

```
  Roster Item Subscription States
  ══════════════════════════════════

  Two directions of subscription are tracked independently:
    - Does Alice receive Bob's presence updates? (Alice "subscribed to" Bob)
    - Does Bob receive Alice's presence updates? (Bob "subscribed to" Alice)

  Combined states:
  ┌──────────────┬────────────────────────────────────────────────────┐
  │ subscription │ Meaning                                            │
  ├──────────────┼────────────────────────────────────────────────────┤
  │ none         │ No subscription in either direction.               │
  ├──────────────┼────────────────────────────────────────────────────┤
  │ from         │ Contact subscribes to MY presence.                 │
  │              │ They see when I'm online. I don't see them.        │
  ├──────────────┼────────────────────────────────────────────────────┤
  │ to           │ I subscribe to the CONTACT's presence.             │
  │              │ I see when they're online. They don't see me.      │
  ├──────────────┼────────────────────────────────────────────────────┤
  │ both         │ Mutual subscription. Both see each other.          │
  │              │ The normal "friends" state.                        │
  └──────────────┴────────────────────────────────────────────────────┘

  Subscription negotiation (Alice adds Bob as a contact):

  1. Alice adds Bob to roster (local, subscription=none):
     CLIENT → SERVER:
     <iq type='set' id='add1'>
       <query xmlns='jabber:iq:roster'>
         <item jid='bob@example.com' name='Bob'/>
       </query>
     </iq>

  2. Alice sends subscribe request:
     CLIENT → SERVER:
     <presence to='bob@example.com' type='subscribe'/>

  3. Bob's server delivers the request. Bob accepts:
     BOB CLIENT → BOB SERVER:
     <presence to='alice@example.com' type='subscribed'/>
     <presence to='alice@example.com' type='subscribe'/>  (Bob also wants to see Alice)

  4. Alice's server notifies her and auto-replies:
     SERVER → ALICE CLIENT:
     <presence from='bob@example.com' type='subscribed'/>

     ALICE CLIENT → SERVER:
     <presence to='bob@example.com' type='subscribed'/>

  5. Both now have subscription='both'.
```

### Service Discovery (XEP-0030)

How does a client know what features a server or another client supports?
XMPP has no hardcoded capability list — instead, entities **advertise**
their capabilities via a standard query mechanism.

```
  Service Discovery: disco#info and disco#items
  ═══════════════════════════════════════════════

  disco#info — what does this entity support?
  ────────────────────────────────────────────
  CLIENT → SERVER:
  <iq type='get' to='example.com' id='disco1'>
    <query xmlns='http://jabber.org/protocol/disco#info'/>
  </iq>

  SERVER → CLIENT:
  <iq type='result' from='example.com' id='disco1'>
    <query xmlns='http://jabber.org/protocol/disco#info'>
      <identity category='server' type='im' name='Example XMPP'/>
      <feature var='http://jabber.org/protocol/disco#info'/>
      <feature var='http://jabber.org/protocol/disco#items'/>
      <feature var='urn:xmpp:mam:2'/>          ← MAM supported
      <feature var='http://jabber.org/protocol/muc'/>  ← MUC supported
      <feature var='urn:ietf:params:xml:ns:xmpp-session'/>
    </query>
  </iq>

  disco#items — what sub-items/components does this entity have?
  ──────────────────────────────────────────────────────────────
  CLIENT → SERVER:
  <iq type='get' to='example.com' id='disco2'>
    <query xmlns='http://jabber.org/protocol/disco#items'/>
  </iq>

  SERVER → CLIENT:
  <iq type='result' from='example.com' id='disco2'>
    <query xmlns='http://jabber.org/protocol/disco#items'>
      <item jid='conference.example.com' name='Conference Rooms'/>
      <item jid='pubsub.example.com' name='Publish-Subscribe'/>
      <item jid='proxy.example.com' name='SOCKS5 Proxy'/>
    </query>
  </iq>
```

### Multi-User Chat (XEP-0045)

```
  MUC Architecture
  ══════════════════

  A MUC room is hosted by a conference component (e.g., conference.example.com).
  Rooms have JIDs of the form:  roomname@conference.example.com

  Room roles:
  ┌──────────────┬──────────────────────────────────────────────────────┐
  │ Role         │ Permissions                                          │
  ├──────────────┼──────────────────────────────────────────────────────┤
  │ visitor      │ Can receive messages. Cannot send in moderated rooms. │
  ├──────────────┼──────────────────────────────────────────────────────┤
  │ participant  │ Can send messages in the room.                       │
  ├──────────────┼──────────────────────────────────────────────────────┤
  │ moderator    │ Can kick, mute participants. Grant/revoke visitor.   │
  └──────────────┴──────────────────────────────────────────────────────┘

  Room affiliations (permanent, survive session end):
  ┌──────────────┬──────────────────────────────────────────────────────┐
  │ Affiliation  │ Meaning                                              │
  ├──────────────┼──────────────────────────────────────────────────────┤
  │ owner        │ Created the room. Can destroy, configure, set admins.│
  │ admin        │ Can ban members, configure room.                     │
  │ member       │ Can enter member-only rooms.                         │
  │ none         │ No special affiliation.                              │
  │ outcast      │ Banned from the room.                                │
  └──────────────┴──────────────────────────────────────────────────────┘

  Joining a room (Alice enters "dev" room with nick "alice"):
  ──────────────────────────────────────────────────────────
  CLIENT → SERVER:
  <presence to='dev@conference.example.com/alice'>
    <x xmlns='http://jabber.org/protocol/muc'/>
  </presence>

  Room broadcasts Alice's join to all occupants:
  SERVER → ALL:
  <presence from='dev@conference.example.com/alice'>
    <x xmlns='http://jabber.org/protocol/muc#user'>
      <item affiliation='member' role='participant'/>
      <status code='110'/>  ← code 110 = this is your presence
    </x>
  </presence>

  Sending a groupchat message:
  ────────────────────────────
  CLIENT → SERVER:
  <message to='dev@conference.example.com' type='groupchat'>
    <body>Anyone seen the build failure?</body>
  </message>

  Room reflects to all occupants:
  SERVER → ALL:
  <message from='dev@conference.example.com/alice' type='groupchat'>
    <body>Anyone seen the build failure?</body>
  </message>
```

### Message Archive Management (XEP-0313)

```
  MAM: Fetching Message History
  ══════════════════════════════

  Problem: User A logs on after 3 days offline. They missed 500 messages.
  Solution: MAM lets clients fetch archived messages from the server.

  Fetch messages after a specific ID:
  ────────────────────────────────────
  CLIENT → SERVER:
  <iq type='set' id='mam1'>
    <query xmlns='urn:xmpp:mam:2' queryid='q1'>
      <x xmlns='jabber:x:data' type='submit'>
        <field var='FORM_TYPE'>
          <value>urn:xmpp:mam:2</value>
        </field>
        <field var='after-id'>
          <value>msg-id-1234</value>
        </field>
      </x>
      <set xmlns='http://jabber.org/protocol/rsm'>
        <max>50</max>
      </set>
    </query>
  </iq>

  Server returns archived messages wrapped in <result>:
  ──────────────────────────────────────────────────────
  SERVER → CLIENT:
  <message id='arc001'>
    <result xmlns='urn:xmpp:mam:2' queryid='q1' id='msg-id-1235'>
      <forwarded xmlns='urn:xmpp:forward:0'>
        <delay xmlns='urn:xmpp:delay' stamp='2024-01-15T10:32:00Z'/>
        <message from='bob@example.com' to='alice@example.com'
                 type='chat'>
          <body>Hey, are you there?</body>
        </message>
      </forwarded>
    </result>
  </message>
  ... (more archived messages) ...

  SERVER → CLIENT:  (final notification when done)
  <iq type='result' id='mam1'>
    <fin xmlns='urn:xmpp:mam:2' complete='true'>
      <set xmlns='http://jabber.org/protocol/rsm'>
        <first>msg-id-1235</first>
        <last>msg-id-1284</last>
        <count>50</count>
      </set>
    </fin>
  </iq>
```

## Key Concepts: WhatsApp

### Why WhatsApp Modified XMPP

WhatsApp started in 2009 using standard XMPP (Ejabberd server). As the user
base grew to hundreds of millions of devices, several problems emerged:

1. **XML is verbose.** The string `from='alice@s.whatsapp.net'` is 28 bytes.
   A 1-byte token lookup in a dictionary is 28x more efficient.

2. **XML parsing is slow.** At 100M+ messages per day, XML DOM parsing burned
   CPU unnecessarily.

3. **XMPP's STARTTLS+SASL model was not designed for mobile.** Mobile clients
   disconnect and reconnect constantly. SASL credential re-exchange on every
   reconnect is expensive.

4. **WhatsApp needed its own E2E encryption.** Standard XMPP has no built-in
   E2E encryption layer. Adding Signal Protocol required designing a custom
   message envelope format.

The result: WhatsApp kept the _concepts_ of XMPP (stanza types, JIDs, presence)
but replaced the wire format entirely with a binary encoding and replaced
STARTTLS+SASL with the Noise Protocol Framework.

### The Noise Protocol Handshake

**Analogy:** Imagine two spies who need to meet in a public place. They each
know each other's face (static public key) from a photograph given to them by
headquarters. When they meet:
1. One spy shows a temporary disguise (ephemeral key).
2. The other spy shows their disguise AND proves they recognize the first spy.
3. The first spy proves they recognize the second spy.

Now they can talk privately, and both are certain they're talking to the real
spy, not an impersonator. This is the Noise XX pattern.

```
  Noise XX Handshake (WhatsApp uses this to replace STARTTLS+SASL)
  ══════════════════════════════════════════════════════════════════

  Notation:
    e  = ephemeral key (generated fresh for this session)
    s  = static key (long-lived device key, registered with WhatsApp)
    ee = DH(client_e, server_e)    — ephemeral-ephemeral shared secret
    es = DH(client_e, server_s)    — client ephemeral + server static
    se = DH(client_s, server_e)    — client static + server ephemeral
    →  = client sends to server
    ←  = server sends to client

  Message 1 (client → server): Advertise client ephemeral key
  ────────────────────────────────────────────────────────────
  → e

  The client generates a fresh ephemeral key pair (e_pub, e_priv).
  Sends e_pub to the server.
  The server is now "warmed up" to derive secrets with the client.

  Message 2 (server → client): Server ephemeral + prove server identity
  ──────────────────────────────────────────────────────────────────────
  ← e, ee, s, es

  Server generates its own ephemeral key (se_pub, se_priv).
  Sends se_pub (e).
  Derives ee = DH(se_priv, client_e_pub). Now both have ee.
  Sends s_pub encrypted with AEAD key derived from ee.
  Derives es = DH(se_priv, client_s_pub). Now both have es.
  The encrypted s is authenticated: client can verify the server identity.

  Message 3 (client → server): Prove client identity
  ─────────────────────────────────────────────────────
  → s, se

  Client sends its static public key s_pub encrypted with AEAD key
  derived from ee + es.
  Derives se = DH(client_s_priv, server_e_pub). Now both have se.
  Both now derive the final transport keys from ee + es + se.

  Result:
  ┌──────────────────────────────────────────────────────────────────┐
  │ After Noise XX completes:                                        │
  │   - Client has proven it holds the registered static key pair    │
  │   - Server has proven it holds its own static key pair           │
  │   - Both share two symmetric keys:                               │
  │       send_key = HKDF(chaining_key, "send")                      │
  │       recv_key = HKDF(chaining_key, "recv")                      │
  │   - All subsequent frames are ChaCha20-Poly1305 encrypted        │
  └──────────────────────────────────────────────────────────────────┘
```

### WhatsApp Frame Framing

After Noise, every message is encapsulated in a simple frame:

```
  WhatsApp Wire Frame Format
  ══════════════════════════

  ┌─────────────────────────────────────────────────────────────────┐
  │                     Frame on the wire                           │
  ├───────────────────────────┬─────────────────────────────────────┤
  │  Length (3 bytes, big-    │  Encrypted payload (variable)       │
  │  endian, unsigned)        │  ChaCha20-Poly1305 ciphertext       │
  │  e.g., 0x00 0x01 0x2F    │  + 16-byte Poly1305 MAC             │
  │  = 303 bytes of payload   │                                     │
  └───────────────────────────┴─────────────────────────────────────┘

  Why 3-byte length (max 16 MB per frame)?
  WhatsApp media is sent out-of-band (CDN), so in-band messages are
  text/receipts only and fit easily in 3 bytes. 4 bytes would add
  25% overhead to very short receipts. 2 bytes (max 64 KB) is too
  small for large group messages with many recipients.

  Decryption of a received frame:
    1. Read 3 bytes → parse as big-endian uint24 → call this L
    2. Read L bytes → this is the ciphertext
    3. Decrypt with ChaCha20-Poly1305 using recv_key + nonce
    4. Nonce = per-connection counter, incremented after each frame
    5. If MAC verification fails → close connection immediately
```

### The WA Binary Protocol

Inside each decrypted frame is a **Node**. Nodes are the binary equivalent of
XMPP stanzas.

```
  Node Structure
  ══════════════

  A Node has:
    tag        — what kind of stanza this is (message, receipt, ack, ...)
    attrs      — key-value attributes (from, to, id, type, ...)
    content    — child nodes OR binary data OR empty

  Wire encoding:
  ┌──────────────────────────────────────────────────────────────────┐
  │  List8/List16 header: *list-size*                                │
  │    0xF8 = list8 (count fits in 1 byte, up to 255 items)          │
  │    0xF9 = list16 (count fits in 2 bytes)                         │
  │    0x00 = empty list (no children)                               │
  ├──────────────────────────────────────────────────────────────────┤
  │  Tag byte (the node type, looked up in token dictionary)         │
  ├──────────────────────────────────────────────────────────────────┤
  │  Attribute count byte (number of key-value pairs × 2, as items)  │
  ├──────────────────────────────────────────────────────────────────┤
  │  Attributes: [key token][value token] × count                    │
  ├──────────────────────────────────────────────────────────────────┤
  │  Content type + content:                                         │
  │    0x00       = no content                                       │
  │    list8/16   = child nodes (recurse)                            │
  │    0xFB + u8  = binary data (1-byte length)                      │
  │    0xFC + u32 = binary data (4-byte length)                      │
  └──────────────────────────────────────────────────────────────────┘

  Token dictionary (excerpt — common strings get 1-byte IDs):
  ┌────────┬────────────────────────────────────────────────────────┐
  │ Token  │ String                                                 │
  ├────────┼────────────────────────────────────────────────────────┤
  │ 0x03   │ "id"                                                   │
  │ 0x06   │ "from"                                                  │
  │ 0x07   │ "group"                                                 │
  │ 0x08   │ "groups"                                               │
  │ 0x0A   │ "message"                                              │
  │ 0x10   │ "to"                                                   │
  │ 0x12   │ "type"                                                  │
  │ 0x14   │ "notification"                                         │
  │ 0x15   │ "receipt"                                              │
  │ 0x16   │ "participant"                                           │
  │ ...    │ (dictionary has ~250 entries)                           │
  └────────┴────────────────────────────────────────────────────────┘

  JID encoding — JIDs get special treatment:
  ┌───────────────────────────────────────────────────────────────┐
  │ 0xFA = JID pair                                               │
  │   [user bytes] [server token or bytes]                        │
  │                                                               │
  │ Example: 15551234567@s.whatsapp.net                           │
  │   0xFA                                                        │
  │   0xFB 0x0B "15551234567"   ← user part, binary string 11 ch │
  │   0x18                     ← token for "s.whatsapp.net"       │
  └───────────────────────────────────────────────────────────────┘

  Concrete example — a text message node decoded:
  ────────────────────────────────────────────────
  tag:   "message"
  attrs: {
    id:   "3EB0123456789",
    from: "15551234567@s.whatsapp.net",
    to:   "15559876543@s.whatsapp.net",
    type: "text"
  }
  content: [
    <Node tag="body" attrs={} content=b"Hello World">,
    <Node tag="enc" attrs={type: "msg", v: "2"} content=<signal_ciphertext>>
  ]
```

### WhatsApp Protobuf Messages

Inside the `<enc>` node is a protobuf-encoded `WebMessageInfo`. This is the
Signal Protocol ciphertext, which when decrypted reveals a `Message` protobuf.

```
  WebMessageInfo (simplified proto definition)
  ═════════════════════════════════════════════

  message MessageKey {
    string remote_jid   = 1;  // who this message is with
    bool   from_me      = 2;  // did we send it?
    string id           = 3;  // message ID (random hex)
    string participant  = 4;  // in groups: actual sender JID
  }

  message Message {
    oneof content {
      string           conversation      = 1;  // plain text
      ImageMessage     image_message     = 3;
      VideoMessage     video_message     = 4;
      AudioMessage     audio_message     = 5;
      DocumentMessage  document_message  = 6;
      StickerMessage   sticker_message   = 26;
      // ... 40+ message types
    }
  }

  message ImageMessage {
    string url              = 1;  // CDN URL
    string mimetype         = 2;  // "image/jpeg"
    string caption          = 3;  // optional caption text
    bytes  file_sha256      = 4;  // SHA-256 of plaintext
    uint64 file_length      = 5;  // byte size
    uint32 height           = 6;
    uint32 width            = 7;
    bytes  media_key        = 10; // AES key for decrypting CDN blob
    bytes  file_enc_sha256  = 32; // SHA-256 of ciphertext on CDN
    string direct_path      = 36; // CDN path component
  }

  message WebMessageInfo {
    MessageKey key          = 1;
    Message    message      = 2;
    uint64     message_timestamp = 3;
    enum Status {
      PENDING         = 0;
      SERVER_ACK      = 1;
      DELIVERY_ACK    = 2;
      READ            = 3;
      PLAYED          = 4;
    }
    Status     status       = 4;
    string     push_name    = 5;  // sender's display name
  }

  Wire format — a text message "Hello" in protobuf hex:
  ──────────────────────────────────────────────────────

  WebMessageInfo {key: {id: "ABC", from_me: false}, message: {conversation: "Hello"}}

  Byte-by-byte:
    0A             field 1 (key), type LEN
    1A             length 26
      0A             field 1 (remote_jid), type LEN
      12             length 18
        31 35 35 35...  "15551234567@s.whatsapp.net"  (18 bytes with domain)
      10             field 2 (from_me), type VARINT
      00             false
      1A             field 3 (id), type LEN
      03             length 3
        41 42 43   "ABC"
    12             field 2 (message), type LEN
    07             length 7
      0A             field 1 (conversation), type LEN
      05             length 5
        48 65 6C 6C 6F  "Hello"
    18             field 3 (message_timestamp), type VARINT
    C0 84 3D       varint-encoded unix timestamp
```

## Key Concepts: WhatsApp E2E Encryption

### Signal Protocol Overview

WhatsApp's end-to-end encryption is built on the Signal Protocol, which
combines two sub-protocols:

```
  Signal Protocol = X3DH + Double Ratchet
  ════════════════════════════════════════

  X3DH (Extended Triple Diffie-Hellman):
    - Used ONCE to establish the initial shared secret between two devices
    - Works even if the recipient is offline (asynchronous key agreement)
    - Requires a key server (WhatsApp's servers) to distribute prekeys

  Double Ratchet:
    - Used for EVERY message after session establishment
    - Provides forward secrecy: compromise of today's key doesn't expose
      yesterday's messages
    - Provides break-in recovery: compromise of key K doesn't expose
      messages encrypted after key K+n (n > 0)
```

### Device Registration

Before Alice can send an encrypted message to Bob, both must have registered
their keys with WhatsApp's servers.

```
  Registration — What Each Device Generates and Uploads
  ═══════════════════════════════════════════════════════

  ┌──────────────────────────────────────────────────────────────────┐
  │ Key Material Generated on Device (NEVER leaves the device)        │
  │                                                                   │
  │ identity_key_pair      = generate_curve25519_key_pair()           │
  │   identity_key_pub  → uploaded to WhatsApp server                 │
  │   identity_key_priv → stored in device secure storage             │
  │                                                                   │
  │ signed_prekey_pair     = generate_curve25519_key_pair()           │
  │   signed_prekey_pub → uploaded                                    │
  │   signed_prekey_sig = sign(signed_prekey_pub, identity_key_priv) │
  │   signed_prekey_priv → stored on device                          │
  │                                                                   │
  │ one_time_prekeys[100]  = [generate_curve25519_key_pair() × 100]  │
  │   one_time_prekey_pubs → batch uploaded to server                 │
  │   one_time_prekey_privs → stored on device                       │
  └──────────────────────────────────────────────────────────────────┘

  Server stores per-device:
    identity_key_pub          (permanent)
    signed_prekey_pub         (rotated ~weekly)
    signed_prekey_sig         (signature verification)
    one_time_prekeys[]        (consumed one by one; server requests
                               new batch when running low)
```

### X3DH: Establishing a Session

When Alice wants to message Bob for the first time:

```
  X3DH Key Agreement
  ═══════════════════

  Alice fetches Bob's prekey bundle from WhatsApp server:
  {
    bob_identity_key_pub:  IK_B
    bob_signed_prekey_pub: SPK_B
    bob_signed_prekey_sig: sig_B
    bob_one_time_prekey:   OPK_B   (if available)
  }

  Alice verifies: verify_signature(SPK_B, sig_B, IK_B) == true

  Alice generates an ephemeral key pair: EK_A = generate_key_pair()

  Alice computes the shared secret via 4 DH operations:
  ─────────────────────────────────────────────────────
    DH1 = DH(IK_A_priv,  SPK_B)   ← Alice's identity × Bob's signed prekey
    DH2 = DH(EK_A_priv,  IK_B)    ← Alice's ephemeral × Bob's identity
    DH3 = DH(EK_A_priv,  SPK_B)   ← Alice's ephemeral × Bob's signed prekey
    DH4 = DH(EK_A_priv,  OPK_B)   ← Alice's ephemeral × Bob's one-time prekey

  master_secret = HKDF(DH1 || DH2 || DH3 || DH4, "WhatsApp Keys")

  Why 4 DH operations? Each adds a layer of security:
    DH1 provides authentication (identity keys involved on both sides)
    DH2 provides forward secrecy for identity (ephemeral on Alice's side)
    DH3 provides forward secrecy for signed prekey
    DH4 provides one-time key protection (if OPK is used)

  Alice sends Bob an X3DH initial message containing:
  {
    alice_identity_key_pub:  IK_A
    alice_ephemeral_key_pub: EK_A
    bob_one_time_prekey_id:  OPK_B_id  (so Bob knows which OPK was used)
    ciphertext: encrypt(master_secret, first_message)
  }

  Bob receives this message (even if he was offline) and can reconstruct
  master_secret by performing the same 4 DH operations in reverse:
    DH1 = DH(SPK_B_priv, IK_A)
    DH2 = DH(IK_B_priv,  EK_A)
    DH3 = DH(SPK_B_priv, EK_A)
    DH4 = DH(OPK_B_priv, EK_A)
```

### Double Ratchet: Per-Message Keys

After X3DH establishes the root key, the Double Ratchet takes over:

```
  Double Ratchet Overview
  ═══════════════════════

  The "double" refers to two interleaved ratchets:

  1. Diffie-Hellman Ratchet (outer ratchet):
     Every time Alice sends a message and receives a reply, she advances
     the DH ratchet by generating a new ephemeral key. This produces a new
     chain key. Even if an attacker captures Alice's current chain key,
     they cannot compute past chain keys (forward secrecy) and cannot
     predict future chain keys without Bob's new DH key (break-in recovery).

  2. Symmetric Ratchet (inner ratchet):
     Within a single "sending chain", each message uses a different key.
     The chain advances with a simple KDF: next_key = KDF(current_key).
     Once a message key is used, it is deleted.

  State maintained per session:
  ┌──────────────────────────────────────────────────────────────────┐
  │ RatchetState {                                                    │
  │   root_key:         bytes        ← updated on each DH ratchet   │
  │   send_chain_key:   bytes        ← KDF-ratcheted on each send   │
  │   recv_chain_key:   bytes        ← KDF-ratcheted on each recv   │
  │   send_ratchet_key: KeyPair      ← our current DH ratchet key   │
  │   recv_ratchet_key: PubKey       ← their last DH ratchet pubkey │
  │   send_msg_number:  uint32       ← messages sent in this chain  │
  │   recv_msg_number:  uint32       ← messages received in chain   │
  │   skipped_keys:     Map          ← out-of-order message keys    │
  │ }                                                                │
  └──────────────────────────────────────────────────────────────────┘

  Message header (prepended to each Double Ratchet ciphertext):
  ┌──────────────────┬──────────────────────────────────────────────┐
  │ Field            │ Description                                  │
  ├──────────────────┼──────────────────────────────────────────────┤
  │ dh_pub           │ Sender's current DH ratchet public key       │
  │ n                │ Message number in current sending chain      │
  │ pn               │ Length of previous sending chain             │
  └──────────────────┴──────────────────────────────────────────────┘
```

### Group Messaging: Sender Keys

Double Ratchet works well for 1-to-1 chat but is inefficient for groups.
If a group has 256 members and Alice sends a message, she would need to
encrypt separately for each recipient — 256 encryptions and 256 transmissions.

WhatsApp solves this with **Sender Keys** (also called the "Sender Key"
distribution mechanism from the Signal Protocol):

```
  Group E2E with Sender Keys
  ═══════════════════════════

  Group setup (first time Alice sends to group G):
  ─────────────────────────────────────────────────
  1. Alice generates a random SenderKey for herself in group G:
       sk_alice = generate_sender_key()

  2. Alice distributes sk_alice to every member of G.
     She sends it individually to each member using that member's
     1-to-1 Double Ratchet session. Each distribution is a separate
     encrypted message. This is the expensive step — done once.

  3. Every member now has:
       sk_alice   — so they can decrypt Alice's group messages
       sk_bob     — if Bob also sent a distribution, they have his key
       ...

  Sending a group message (after setup):
  ─────────────────────────────────────────────
  1. Alice ratchets her SenderKey forward:
       (message_key, next_sk_alice) = ratchet(sk_alice)

  2. Alice encrypts message_body with message_key (AES-256-CBC + HMAC):
       ciphertext = AES_CBC_HMAC(message_key, message_body)

  3. Alice sends ONE ciphertext to the WhatsApp server, which fans it out
     to all group members. Only ONE encryption/transmission!

  4. Each member decrypts with the message_key they derive from their
     stored copy of sk_alice.

  SenderKey ratchet state:
  ┌──────────────────────────────────────────────────────────────────┐
  │ SenderKeyState {                                                  │
  │   sender_key_id:   uint32   ← ID of this sender key             │
  │   iteration:       uint32   ← how many messages sent so far     │
  │   chain_key:       bytes    ← current chain key                  │
  │   signing_key:     KeyPair  ← for authenticating sender identity │
  │ }                                                                │
  └──────────────────────────────────────────────────────────────────┘

  Limitation: SenderKeys do NOT provide break-in recovery for groups.
  If an attacker captures sk_alice at iteration N, they can decrypt all
  future group messages from Alice (since the chain is deterministic
  from that point). This is an accepted trade-off for efficiency.
```

### WhatsApp Media Encryption and CDN Flow

```
  Media Upload Flow (Alice sends a photo)
  ═══════════════════════════════════════

  ┌────────────────────────────────────────────────────────────────┐
  │                                                                │
  │  1. Alice generates random media_key (32 bytes)                │
  │                                                                │
  │  2. Derive encryption material from media_key:                 │
  │       expanded = HKDF(media_key, "WhatsApp Image Keys", 112)   │
  │       iv       = expanded[0:16]                                │
  │       aes_key  = expanded[16:48]                               │
  │       mac_key  = expanded[48:80]                               │
  │       (ref_key = expanded[80:112]  ← used for thumbnails)      │
  │                                                                │
  │  3. Encrypt the image:                                         │
  │       ciphertext = AES-256-CBC(aes_key, iv, image_bytes)       │
  │       mac = HMAC-SHA256(mac_key, iv || ciphertext)             │
  │       upload_blob = ciphertext || mac[0:10]                    │
  │                                                                │
  │  4. POST upload_blob to WhatsApp media upload URL              │
  │       Server returns direct_path and CDN URL                   │
  │                                                                │
  │  5. Alice computes:                                            │
  │       file_sha256     = SHA256(image_bytes)   ← of plaintext   │
  │       file_enc_sha256 = SHA256(upload_blob)   ← of ciphertext  │
  │                                                                │
  │  6. Alice sends an ImageMessage protobuf containing:           │
  │       url, direct_path, media_key, file_sha256, file_enc_sha256│
  │     The media_key is encrypted in the Signal session envelope. │
  │                                                                │
  └────────────────────────────────────────────────────────────────┘

  Media Download Flow (Bob receives the photo)
  ═════════════════════════════════════════════

  ┌────────────────────────────────────────────────────────────────┐
  │                                                                │
  │  1. Bob receives ImageMessage protobuf (decrypted via Signal)  │
  │       Extracts: url, media_key, file_enc_sha256                │
  │                                                                │
  │  2. Bob fetches ciphertext from CDN:                           │
  │       GET https://mmg.whatsapp.net/{direct_path}               │
  │       Response body = upload_blob (ciphertext || mac)          │
  │                                                                │
  │  3. Bob verifies integrity:                                     │
  │       assert SHA256(upload_blob) == file_enc_sha256            │
  │       (proves the CDN didn't corrupt or tamper with the file)  │
  │                                                                │
  │  4. Bob derives decryption material from media_key:            │
  │       expanded = HKDF(media_key, "WhatsApp Image Keys", 112)   │
  │       iv, aes_key, mac_key = expanded[0:16, 16:48, 48:80]      │
  │                                                                │
  │  5. Verify MAC:                                                │
  │       expected_mac = HMAC-SHA256(mac_key, iv || ciphertext)    │
  │       assert expected_mac[0:10] == upload_blob[-10:]           │
  │                                                                │
  │  6. Decrypt:                                                   │
  │       image_bytes = AES-256-CBC-decrypt(aes_key, iv, ciphertext│
  │                                                                │
  │  7. Verify plaintext integrity:                                 │
  │       assert SHA256(image_bytes) == file_sha256                │
  │                                                                │
  └────────────────────────────────────────────────────────────────┘

  Why is media out-of-band?
  ─────────────────────────
  "In-band" would mean Base64-encoding the image inside the protobuf,
  sending it through the WA binary protocol, through the Noise frame,
  through the WhatsApp server, and to the recipient.

  The problems with in-band media:
  1. The WhatsApp server would have to buffer the entire image in RAM
     while the recipient is offline. Billions of photos × megabytes = ∞
  2. Forwarded messages: if Alice forwards Bob's photo to Carol, Carol
     would download from the CDN URL — same encrypted blob, same CDN cache.
     In-band would require Alice to re-upload. CDN deduplication is free.
  3. Multiple group recipients: server sends ONE CDN URL to 256 people.
     In-band would be 256 copies of the blob.
```

## Algorithms

### XMPP: Parsing a Stanza from a TCP Stream

```
  XMPP is an XML stream — we cannot use a DOM parser (which needs the
  complete document). We use a streaming SAX-style parser.

  xmpp_read_stanza(stream):
    parser = SaxParser()
    buffer = ByteBuffer()
    depth = 0

    loop:
      chunk = stream.read(4096)
      if chunk is empty:
        raise ConnectionClosed

      buffer.append(chunk)
      events = parser.feed(buffer)

      for event in events:
        if event.type == START_ELEMENT:
          depth += 1
          if depth == 1:
            # This is <stream:stream> — the stream open element
            handle_stream_open(event)
          elif depth == 2:
            # This is the start of a top-level stanza
            current_stanza = new_stanza(event)

        elif event.type == END_ELEMENT:
          depth -= 1
          if depth == 1:
            # We just closed a top-level stanza — it's complete
            return current_stanza
          elif depth == 0:
            # </stream:stream> — client is logging out
            raise StreamClosed

        elif event.type == TEXT:
          if current_stanza is not None:
            current_stanza.add_text(event.text)
```

### XMPP: Routing a Stanza

```
  route_stanza(stanza, local_domain):
    dest_jid = parse_jid(stanza.to)

    if dest_jid is None:
      # No 'to' attribute → send back to sender's server
      deliver_to_server_component(stanza)
      return

    if dest_jid.domain == local_domain:
      # Local delivery
      sessions = find_active_sessions(dest_jid.local)
      if sessions is empty:
        store_offline(stanza)
        return
      if dest_jid.resource is not None:
        # Full JID — deliver to specific session
        session = find_session(dest_jid.resource)
        if session:
          session.send(stanza)
        else:
          send_error(stanza, "item-not-found")
      else:
        # Bare JID — deliver to highest-priority session
        best = max(sessions, key=lambda s: s.priority)
        best.send(stanza)
    else:
      # Remote delivery — route via S2S
      s2s_route(stanza, dest_jid.domain)
```

### WhatsApp: Encoding a Node to Binary

```
  encode_node(node):
    out = ByteBuffer()

    # Write list header for [tag, attrs..., content]
    num_attrs = len(node.attrs)
    list_size = 1 + num_attrs * 2 + (1 if node.content else 0)
    write_list_header(out, list_size)

    # Write tag token
    write_token(out, DICT[node.tag])

    # Write attributes (key-value pairs as alternating tokens)
    for key, value in node.attrs.items():
      write_token(out, DICT[key])
      write_value(out, value)

    # Write content
    if node.content is None:
      pass  # no content byte needed (list_size accounts for it)
    elif isinstance(node.content, bytes):
      out.write(0xFB)     # binary content flag
      out.write_varint(len(node.content))
      out.write(node.content)
    elif isinstance(node.content, list):
      write_list_header(out, len(node.content))
      for child in node.content:
        out.write(encode_node(child))

    return out.bytes()

  write_list_header(out, size):
    if size == 0:
      out.write(0x00)
    elif size <= 255:
      out.write(0xF8)
      out.write(size & 0xFF)
    else:
      out.write(0xF9)
      out.write((size >> 8) & 0xFF)
      out.write(size & 0xFF)
```

### WhatsApp: Double Ratchet Encrypt

```
  ratchet_encrypt(state, plaintext):
    # Step 1: Symmetric ratchet — derive message key from chain key
    message_key   = HMAC-SHA256(state.send_chain_key, b"\x01")
    state.send_chain_key = HMAC-SHA256(state.send_chain_key, b"\x02")

    # Step 2: Build message header
    header = {
      dh_pub: state.send_ratchet_key.public,
      n:      state.send_msg_number,
      pn:     state.prev_send_count
    }
    state.send_msg_number += 1

    # Step 3: Encrypt
    aead_key = HKDF(message_key, "WhatsApp Message Keys", 80)
    enc_key  = aead_key[0:32]
    mac_key  = aead_key[32:64]
    iv       = aead_key[64:80]

    header_bytes = serialize(header)
    ciphertext   = AES-256-CBC(enc_key, iv, plaintext)
    mac          = HMAC-SHA256(mac_key, header_bytes || ciphertext)

    return header_bytes || ciphertext || mac[0:8]
```

### WhatsApp: Noise XX Handshake

```
  noise_xx_initiator(static_key_pair, server_static_pub):
    # Initialize Noise state
    state = NoiseState("Noise_XX_25519_AESGCM_SHA256")
    state.initialize_as_initiator()

    # Message 1: → e
    e = generate_key_pair()
    msg1 = e.public
    state.mix_hash(msg1)
    send(msg1)

    # Message 2: ← e, ee, s, es
    msg2 = receive()
    server_e_pub = msg2[0:32]
    state.mix_hash(server_e_pub)
    state.mix_key(DH(e.private, server_e_pub))  # mix ee
    encrypted_server_s = msg2[32:]
    server_s_pub = state.decrypt_and_hash(encrypted_server_s)
    state.mix_key(DH(e.private, server_s_pub))  # mix es

    # Message 3: → s, se
    encrypted_client_s = state.encrypt_and_hash(static_key_pair.public)
    state.mix_key(DH(static_key_pair.private, server_e_pub))  # mix se
    send(encrypted_client_s)

    # Derive transport keys
    (send_key, recv_key) = state.split()
    return (send_key, recv_key)
```

## Test Strategy

### XMPP Tests

```
  1. Stream Negotiation
  ─────────────────────
  test_stream_open_produces_correct_xml:
    Given: client opens stream to "example.com"
    Expect: <?xml ...?><stream:stream xmlns=... to='example.com' version='1.0'>

  test_starttls_negotiation:
    Given: server offers <starttls><required/>
    When:  client sends <starttls/>
    Then:  server responds <proceed/>
    Then:  TLS handshake completes
    Then:  stream is re-opened

  test_sasl_plain_auth_success:
    Given: encoded credentials for alice/password123
    When:  client sends <auth mechanism='PLAIN'>base64</auth>
    Then:  server verifies and responds <success/>

  test_sasl_plain_auth_failure:
    Given: wrong password
    Then:  server responds <failure><not-authorized/></failure>

  2. Stanza Parsing
  ──────────────────
  test_parse_message_stanza:
    Input XML: <message to='bob' from='alice' type='chat'><body>Hi</body></message>
    Expect: MessageStanza{to="bob", from="alice", type=chat, body="Hi"}

  test_parse_iq_get:
    Input XML: <iq type='get' id='r1'><query xmlns='jabber:iq:roster'/></iq>
    Expect: IqStanza{type=get, id="r1", query_ns="jabber:iq:roster"}

  test_parse_presence_unavailable:
    Input XML: <presence type='unavailable'/>
    Expect: PresenceStanza{type=unavailable}

  test_partial_stanza_buffering:
    Feed: "<message to='bob'"         → no stanza returned
    Feed: " from='alice'>"             → no stanza returned
    Feed: "<body>Hello</body>"         → no stanza returned
    Feed: "</message>"                 → MessageStanza{to=bob, from=alice, body=Hello}

  3. Roster Management
  ─────────────────────
  test_roster_fetch_round_trip:
    Send: IQ get with jabber:iq:roster
    Receive: IQ result with two roster items
    Expect: parsed roster has 2 contacts with correct subscription states

  test_subscribe_flow:
    Simulate full subscribe/subscribed exchange
    Verify subscription state reaches 'both'

  4. MUC
  ───────
  test_muc_join_produces_presence:
    When: client sends <presence to='room@conf.example.com/nick'>
    Then: server broadcasts join presence to all occupants

  test_groupchat_message_routed_to_all:
    Given: room with 3 occupants
    When: one occupant sends groupchat message
    Then: all 3 receive it with from='room@conf.../sender_nick'
```

### WhatsApp Tests

```
  1. Binary Protocol
  ───────────────────
  test_token_lookup_round_trip:
    For each token in dictionary:
      encode token → decode → verify original string

  test_node_encode_decode_round_trip:
    node = Node(tag="message", attrs={"id": "abc", "type": "text"},
                content=[Node(tag="body", content=b"Hello")])
    encoded = encode_node(node)
    decoded = decode_node(encoded)
    assert decoded == node

  test_jid_encoding:
    jid = "15551234567@s.whatsapp.net"
    encoded = encode_jid(jid)
    assert encoded[0] == 0xFA
    assert decode_jid(encoded) == jid

  test_binary_node_with_binary_content:
    node = Node(tag="enc", attrs={"type": "msg"}, content=b"\x00" * 256)
    Verify encode/decode preserves binary data including null bytes

  2. Noise Handshake
  ───────────────────
  test_noise_xx_key_agreement:
    client_static = generate_key_pair()
    server_static = generate_key_pair()
    (c_send, c_recv) = noise_xx_initiator(client_static, server_static.public)
    (s_send, s_recv) = noise_xx_responder(server_static, client_static.public)
    assert c_send == s_recv
    assert c_recv == s_send

  test_noise_frame_encrypt_decrypt:
    send_key, recv_key = (random_key(), random_key())
    plaintext = b"Hello WhatsApp"
    frame = encrypt_frame(send_key, nonce=0, plaintext)
    recovered = decrypt_frame(recv_key, nonce=0, frame)
    assert recovered == plaintext

  3. E2E Encryption
  ──────────────────
  test_x3dh_session_establishment:
    alice_bundle = generate_registration_bundle()
    bob_bundle = generate_registration_bundle()
    # Alice initiates session to Bob
    (session_a, initial_message) = x3dh_initiate(alice_bundle, bob_bundle.prekey_bundle)
    session_b = x3dh_respond(bob_bundle, initial_message)
    assert session_a.root_key == session_b.root_key

  test_double_ratchet_message_round_trip:
    session_a, session_b = establish_sessions()
    ciphertext = ratchet_encrypt(session_a, b"Hello Bob")
    plaintext = ratchet_decrypt(session_b, ciphertext)
    assert plaintext == b"Hello Bob"

  test_double_ratchet_forward_secrecy:
    # Send 10 messages, capture keys after message 5
    # Verify messages 1-5 cannot be decrypted after key deletion
    ...

  test_media_encryption_round_trip:
    image_bytes = load_test_image()
    media_key = generate_media_key()
    (upload_blob, metadata) = encrypt_media(media_key, image_bytes, "image")
    decrypted = decrypt_media(media_key, upload_blob, metadata)
    assert decrypted == image_bytes

  test_media_sha256_integrity_check:
    media_key = generate_media_key()
    (upload_blob, metadata) = encrypt_media(media_key, b"real image", "image")
    corrupted_blob = flip_byte(upload_blob, 42)
    expect_exception(IntegrityError):
      decrypt_media(media_key, corrupted_blob, metadata)

  4. Sender Keys (Group E2E)
  ───────────────────────────
  test_sender_key_distribution_and_decrypt:
    alice_key = generate_sender_key()
    # Simulate distribution: encode alice's key as if sent over 1-1 session
    encoded = encode_sender_key_distribution(alice_key)
    restored = decode_sender_key_distribution(encoded)
    plaintext = b"Group message"
    ciphertext = sender_key_encrypt(alice_key, plaintext)
    recovered = sender_key_decrypt(restored, ciphertext)
    assert recovered == plaintext
```

## Error Handling

```
  XMPP Defined Conditions (RFC 6120 §8.3.3):
  ════════════════════════════════════════════

  ┌────────────────────────────┬──────────────────────────────────────────┐
  │ Error                      │ Meaning                                  │
  ├────────────────────────────┼──────────────────────────────────────────┤
  │ <bad-request>              │ Malformed stanza. Parser rejection.       │
  │ <conflict>                 │ Resource already in use.                  │
  │ <feature-not-implemented>  │ Requested feature not supported.          │
  │ <forbidden>                │ Insufficient permissions.                 │
  │ <gone>                     │ User has moved; alternate address given.  │
  │ <internal-server-error>    │ Server-side fault.                        │
  │ <item-not-found>           │ Addressed entity does not exist.          │
  │ <not-acceptable>           │ Stanza does not meet criteria.            │
  │ <not-allowed>              │ Action not permitted.                     │
  │ <not-authorized>           │ Not authenticated.                        │
  │ <recipient-unavailable>    │ Recipient not online and no offline store. │
  │ <remote-server-not-found>  │ Cannot route to remote domain.            │
  │ <service-unavailable>      │ Service offline or not configured.        │
  └────────────────────────────┴──────────────────────────────────────────┘

  Example error stanza:
  <message type='error' from='example.com' to='alice@example.com' id='msg001'>
    <error type='cancel'>
      <item-not-found xmlns='urn:ietf:params:xml:ns:xmpp-stanzas'/>
      <text>bob@example.com does not exist</text>
    </error>
  </message>

  WhatsApp error handling:
  ═════════════════════════
  - Noise MAC failure → immediate TCP close (no error message, prevents oracle attacks)
  - Unknown token in binary decode → skip node, log warning
  - Signal decrypt failure → display "Message could not be decrypted" to user
  - CDN integrity failure → retry download (may be CDN corruption), then error
  - Key server timeout → queue message, retry on reconnect
```
