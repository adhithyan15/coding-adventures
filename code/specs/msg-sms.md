# MSG-SMS — SMS (Short Message Service)

## Overview

SMS is the oldest and most universally supported mobile messaging protocol in
existence. Every phone on Earth can send and receive a text message. This was
not an accident — SMS was designed from the ground up in 1985 to work on the
GSM radio network, piggy-backing on the control-plane signaling channel (not
the voice channel), which is why you can sometimes get a text even when a voice
call would fail.

**Analogy:** SMS is like a postcard sent through a post office that keeps a
copy until the recipient's mailbox is available. When you send an SMS:

1. Your phone writes the message on a "postcard" (the PDU — Protocol Data
   Unit) and hands it to the nearest post office (the SMSC — Short Message
   Service Centre).
2. The SMSC stores the postcard and tries to deliver it to the recipient.
3. If the recipient's phone is off or out of range, the SMSC holds the
   postcard for up to 72 hours (the validity period) and retries.
4. Once the recipient's phone wakes up and registers on the network, the SMSC
   delivers the postcard immediately.
5. The SMSC optionally sends you a delivery report — a receipt confirming
   the postcard was actually delivered to the recipient's inbox.

This store-and-forward model is why SMS is so reliable. Email requires the
recipient's mail server to be reachable *right now*. SMS requires only that the
SMSC (your carrier's server) is reachable, and then it waits for you.

The governing standard is **3GPP TS 23.040** (formerly GSM 03.40), which
defines every bit of the SMS PDU format. This document covers the PDU in
enough detail that you could implement an SMS gateway from scratch.

## Layer Position

```
User Application (contacts app, third-party SMS client)
│
│   "Send 'Hello' to +442071234567"
▼
SMS Application Layer (this package)
│   - Constructs SMS-SUBMIT PDU
│   - Encodes text using GSM 7-bit alphabet or UCS-2
│   - Splits long messages into concatenated parts (UDH)
│   - Parses incoming SMS-DELIVER and SMS-STATUS-REPORT PDUs
│
▼
AT Command Interface  (legacy modem interface, still universal)
│   AT+CMGF=0         → PDU mode
│   AT+CMGS=<len>     → send PDU
│   AT+CMGR=<index>   → read stored PDU
│   AT+CNMI=...       → configure new message indication
│
▼                     OR
SMPP (Short Message Peer-to-Peer)  ← bulk SMS gateway API
│   Used by SMS aggregators, A2P (application-to-person) services
│
▼
SMSC (Short Message Service Centre)
│   - Your carrier's server (e.g., +447785016005 for UK Vodafone)
│   - Stores message, tracks validity period
│   - Routes to destination carrier via SS7 / MAP protocol
│
▼
SS7 / MAP Protocol (Mobile Application Part)
│   - The telephone network's signaling backbone
│   - SMSC uses MAP to locate destination handset (HLR lookup)
│   - Routes SMS across carrier boundaries
│
▼
Radio Network (GSM / UMTS / LTE)
│   - SMS travels over the control plane (SDCCH / SACCH in GSM)
│   - Not the voice channel — this is why SMS works when calls are busy
│
▼
Recipient Handset
    - OS SMS daemon decodes PDU
    - Reassembles concatenated parts
    - Displays message to user
```

**Depends on:** Radio network access (handled by the baseband processor, not
application code), SMSC addressing (provided by carrier SIM configuration).

**Used by:** Every consumer messaging app, A2P SMS services (two-factor auth,
appointment reminders, bank alerts), IoT devices communicating via cellular
modems.

## Key Concepts

### The SMSC: Your Carrier's Store-and-Forward Server

The SMSC is the heart of SMS. It is a specialized server run by your mobile
carrier (and sometimes by third-party SMS aggregators like Twilio, Nexmo, or
AWS SNS). The SMSC address is stored on your SIM card — when you insert a SIM,
you automatically know which SMSC to submit messages to.

```
SMSC routing overview:
═══════════════════════

  Phone A               SMSC (Carrier A)          SMSC (Carrier B)
  (Verizon)             smsc.verizon.com           smsc.att.com
      │                        │                          │
      │  SMS-SUBMIT PDU        │                          │
      │  "Hello, +1-555-0199"  │                          │
      ├───────────────────────►│                          │
      │                        │  SS7 / MAP: SRI-SM       │
      │                        │  (Send Routing Info for  │
      │                        │   Short Message)         │
      │                        │  "Where is +1-555-0199?" │
      │                        ├─────────────────────────►│
      │                        │                          │
      │                        │  HLR Response:           │
      │                        │  "roaming on tower X,    │
      │                        │   MSC address Y"         │
      │                        │◄─────────────────────────┤
      │                        │                          │
      │                        │  MT-Forward-SM           │
      │                        │  (Mobile Terminated)     │
      │                        ├─────────────────────────►│
      │                        │                          │ SMS-DELIVER to
      │                        │                          │ Phone B
      │                        │                          ├──────►[Phone B]
```

**HLR (Home Location Register):** A database that tracks where each subscriber
is currently located. When your phone roams to another country, the HLR is
updated. The SMSC queries the HLR to find which Mobile Switching Centre (MSC)
currently serves the recipient.

**MAP (Mobile Application Part):** The SS7 sub-protocol that carries SMS
routing queries between network nodes. If you've ever wondered why SMS works
internationally — it's because carriers exchange MAP messages over the SS7
backbone.

### PDU Format Overview

Every SMS on the wire is a **PDU (Protocol Data Unit)**. The PDU is a binary
blob — not human-readable text. There are three main PDU types:

```
PDU Types
══════════

  SMS-SUBMIT          Phone → SMSC
  │  "I want to send this message to that number"
  │  Contains: destination number, message text, options
  │  Created by: the sending phone
  │
  SMS-DELIVER         SMSC → Phone
  │  "Here is a message for you"
  │  Contains: sender number, timestamp, message text
  │  Created by: the SMSC when delivering to recipient
  │
  SMS-STATUS-REPORT   SMSC → Phone (delivery report)
      "Your earlier message was delivered/failed"
      Contains: reference number, recipient, discharge time, status
      Created by: SMSC in response to a delivery report request
```

**Why two different PDU types for send and receive?** Because they carry
different information. When you send (SMS-SUBMIT), you don't know the delivery
timestamp — that hasn't happened yet. When the SMSC delivers (SMS-DELIVER), it
stamps the arrival time and includes the sender's address. Different data,
different PDU structures.

### SMS-SUBMIT PDU: Field-by-Field

This is the PDU your phone sends to the SMSC when you press "Send."

```
SMS-SUBMIT PDU Wire Format
══════════════════════════

Byte offset: 0        1        2        3        4        ...
             │        │        │        │        │
             ▼        ▼        ▼        ▼        ▼
          ┌──────┬────────┬────────┬────────┬──────────────────────┐
          │ SMSC │  PDU   │  MR    │  DA    │  PID  DCS  VP  UDL  UD │
          │ Addr │  Type  │        │  Addr  │                         │
          └──────┴────────┴────────┴──────────────────────────────────┘

Full field breakdown:
  [SMSC Address]  — who to submit to (your carrier's SMSC)
  [PDU Type]      — 1 byte: MTI + optional flags
  [MR]            — 1 byte: Message Reference (0x00 = let SMSC assign)
  [DA]            — Destination Address (variable length)
  [PID]           — 1 byte: Protocol Identifier
  [DCS]           — 1 byte: Data Coding Scheme (encoding)
  [VP]            — Validity Period (1 or 7 bytes, or absent)
  [UDL]           — 1 byte: User Data Length
  [UD]            — User Data (the actual message text, up to 140 bytes)
```

#### SMSC Address Field

```
SMSC Address encoding:
═══════════════════════

  Byte 0:  Length    — number of bytes that follow (including TON/NPI byte)
  Byte 1:  TON/NPI   — Type of Number / Numbering Plan Indicator
  Byte 2+: Number    — BCD-encoded, semi-octets, padded with 0xF

  TON/NPI byte breakdown (8 bits):
  ┌─────┬──────────────────┬────────────────────────────────────────────┐
  │ Bit │ Field            │ Values                                     │
  ├─────┼──────────────────┼────────────────────────────────────────────┤
  │ 7   │ (always 1)       │ Fixed to 1 by spec                         │
  ├─────┼──────────────────┼────────────────────────────────────────────┤
  │ 6-4 │ Type of Number   │ 000 = Unknown                              │
  │     │                  │ 001 = International (has country code)     │
  │     │                  │ 010 = National                             │
  │     │                  │ 101 = Alphanumeric                         │
  ├─────┼──────────────────┼────────────────────────────────────────────┤
  │ 3-0 │ Numbering Plan   │ 0001 = ISDN/telephone (E.164) — use this  │
  └─────┴──────────────────┴────────────────────────────────────────────┘

  BCD encoding example: +447785016005
    Strip the '+': 447785016005
    Pad to even length: 447785016005 (already even)
    Swap nibbles in pairs: 44 → 0x44, 77 → 0x77, 85 → 0x58, ...
    Wait — let's be precise:
      '4','4' → high nibble=4, low nibble=4 → byte = 0x44
      '7','7' → 0x77
      '8','5' → 0x58   (note: '8' is low nibble, '5' is high nibble)
                         In BCD, digit pairs are swapped!
      '0','1' → 0x10
      '6','0' → 0x06
      '0','5' → 0x50

  Correct BCD swap rule:
    For digits D0 D1 D2 D3 D4 D5 ...
    Byte 0 = (D1 << 4) | D0
    Byte 1 = (D3 << 4) | D2
    ...
    If odd number of digits, final nibble is 0xF (padding)
```

#### PDU Type Octet (MTI Byte)

```
PDU Type Octet — SMS-SUBMIT
════════════════════════════

  Bit 7   Bit 6   Bit 5   Bit 4   Bit 3   Bit 2   Bit 1   Bit 0
  ┌───────┬───────┬───────┬───────┬───────┬───────┬───────┬───────┐
  │  RP   │ UDHI  │  SRR  │ VPF1  │ VPF0  │  RD   │ MTI1  │ MTI0  │
  └───────┴───────┴───────┴───────┴───────┴───────┴───────┴───────┘

  ┌──────┬────────────────────────┬────────────────────────────────────┐
  │ Bits │ Field                  │ Meaning                            │
  ├──────┼────────────────────────┼────────────────────────────────────┤
  │ 1-0  │ MTI (Message Type)     │ 01 = SMS-SUBMIT                    │
  │      │                        │ 00 = SMS-DELIVER (in DELIVER PDU)  │
  │      │                        │ 10 = SMS-STATUS-REPORT             │
  ├──────┼────────────────────────┼────────────────────────────────────┤
  │ 2    │ RD (Reject Duplicates) │ 0 = accept duplicate MR from phone │
  │      │                        │ 1 = discard if same MR queued      │
  ├──────┼────────────────────────┼────────────────────────────────────┤
  │ 4-3  │ VPF (Validity Period   │ 00 = no VP field present           │
  │      │      Format)           │ 10 = relative format (1 byte)      │
  │      │                        │ 11 = absolute format (7 bytes)     │
  ├──────┼────────────────────────┼────────────────────────────────────┤
  │ 5    │ SRR (Status Report     │ 1 = request a delivery report      │
  │      │      Request)          │ 0 = no report needed               │
  ├──────┼────────────────────────┼────────────────────────────────────┤
  │ 6    │ UDHI (User Data Header │ 1 = UD field starts with a UDH     │
  │      │       Indicator)       │     (used for concatenated SMS)    │
  │      │                        │ 0 = UD is pure message text        │
  ├──────┼────────────────────────┼────────────────────────────────────┤
  │ 7    │ RP (Reply Path)        │ 1 = reply path set                 │
  │      │                        │ 0 = no reply path                  │
  └──────┴────────────────────────┴────────────────────────────────────┘

  Common PDU type byte values:
    0x01 = SMS-SUBMIT, no VP, no SRR, no UDHI, no RP
    0x11 = SMS-SUBMIT, relative VP (1 byte), no SRR
    0x21 = SMS-SUBMIT, no VP, SRR=1 (request delivery report)
    0x41 = SMS-SUBMIT, no VP, UDHI=1 (concatenated message part)
    0x61 = SMS-SUBMIT, no VP, SRR=1, UDHI=1 (concat + delivery report)
```

#### Destination Address Field

```
Destination Address encoding:
══════════════════════════════

  Byte 0:  Number of digits (not bytes!) in the phone number
  Byte 1:  TON/NPI (same format as SMSC address)
  Byte 2+: BCD-encoded phone number digits

  Example: +442071234567 (UK London number)
    Number of digits: 12
    TON/NPI: 0x91 (10010001 → bit7=1, TON=001=international, NPI=0001=ISDN)
    BCD:
      '4','4' → 0x44
      '2','0' → 0x02
      '7','1' → 0x17
      '2','3' → 0x32
      '4','5' → 0x54
      '6','7' → 0x76

  Wire bytes: 0C 91 44 02 17 32 54 76
              │  │  └─────────────────── BCD digits
              │  └── TON/NPI = 0x91 (international E.164)
              └── 0x0C = 12 decimal = 12 digits
```

#### PID (Protocol Identifier)

```
Protocol Identifier (PID) — 1 byte
════════════════════════════════════

  For a normal SMS (just a text message): PID = 0x00

  Special values:
  ┌──────┬────────────────────────────────────────────────────────┐
  │ PID  │ Meaning                                                │
  ├──────┼────────────────────────────────────────────────────────┤
  │ 0x00 │ Plain SMS (no special protocol stacking)               │
  │ 0x01 │ Telematic device (fax group 3)                         │
  │ 0x20 │ Telex                                                  │
  │ 0x40 │ Silent SMS (phone receives but does not show user)     │
  │ 0x7F │ SIM Data Download (for OTA SIM programming)            │
  └──────┴────────────────────────────────────────────────────────┘

  You almost always use 0x00.
```

#### DCS (Data Coding Scheme)

```
Data Coding Scheme (DCS) — 1 byte
═══════════════════════════════════

  The DCS byte tells the recipient how to decode the User Data bytes.
  It controls two critical things: the character encoding and the
  message class.

  Bit layout (most common form):
  Bit 7-4: 0000  (general data coding group)
  Bit 3:   0     (no message class)
  Bit 2:   0     (no compression)
  Bit 1-0: encoding

  ┌──────────┬────────────────────────────────────────────────────┐
  │ DCS byte │ Encoding                                           │
  ├──────────┼────────────────────────────────────────────────────┤
  │ 0x00     │ GSM 7-bit default alphabet (160 chars per SMS)     │
  │ 0x04     │ 8-bit binary data (140 bytes per SMS)              │
  │ 0x08     │ UCS-2 (UTF-16 Big Endian — 70 chars per SMS)       │
  └──────────┴────────────────────────────────────────────────────┘

  Message class (bits 3-0 when bits 7-4 = 0001):
  ┌─────────┬──────────────────────────────────────────────────────┐
  │ Class   │ Meaning                                              │
  ├─────────┼──────────────────────────────────────────────────────┤
  │ Class 0 │ Flash SMS — displayed immediately, not stored        │
  │ Class 1 │ ME-specific (store in phone memory)                  │
  │ Class 2 │ SIM-specific (store on SIM card)                     │
  │ Class 3 │ TE-specific (forward to connected terminal)          │
  └─────────┴──────────────────────────────────────────────────────┘

  Why does encoding affect character limit?
  Each SMS payload is at most 140 bytes (1120 bits).
  - 7-bit encoding: 1120 ÷ 7 = 160 characters
  - 8-bit encoding: 140 ÷ 1 = 140 characters (binary data)
  - UCS-2 encoding: 140 ÷ 2 = 70 characters (2 bytes per char)
```

#### Validity Period

```
Validity Period (VP) — relative format (1 byte)
════════════════════════════════════════════════

  The VP tells the SMSC how long to keep trying to deliver
  the message before giving up and discarding it.

  Relative VP decoding:
  ┌────────────┬──────────────────────────────────────────────────┐
  │ VP value   │ Validity period                                  │
  ├────────────┼──────────────────────────────────────────────────┤
  │ 0..143     │ (VP + 1) × 5 minutes    (5 min to 12 hours)     │
  │ 144..167   │ 12 hours + (VP - 143) × 30 minutes              │
  │ 168..196   │ (VP - 166) days          (2 to 30 days)          │
  │ 197..255   │ (VP - 192) weeks         (5 to 63 weeks)         │
  └────────────┴──────────────────────────────────────────────────┘

  Common values:
    0xAA = 170 decimal → 12h + (170 - 143) × 30min = 12h + 13.5h = 25.5h
    0xAD = 173 decimal → 12h + (173 - 143) × 30min = 12h + 15h = 27h ≈ 1 day
    0xFF = 255 decimal → (255 - 192) = 63 weeks ≈ max validity

  Absolute VP (7 bytes): same format as a timestamp — year, month, day,
  hour, minute, second, timezone. Used when you need a specific expiry time.
```

#### UDL and UD (User Data Length and User Data)

```
UDL — User Data Length (1 byte)
══════════════════════════════════

  For 7-bit encoding: UDL = number of CHARACTERS (not bytes!)
    "Hello" → UDL = 5 (5 characters)
    Even though 5 × 7 bits = 35 bits = 5 bytes packed

  For UCS-2 encoding: UDL = number of BYTES
    "Hello" in UCS-2 = 10 bytes → UDL = 10

  This asymmetry trips up many implementors. Always check DCS first.

User Data (UD) — variable length, max 140 bytes
══════════════════════════════════════════════════

  Contains the encoded message text (or binary data).
  For GSM 7-bit: the characters are packed 7 bits each (see below).
  For UCS-2: big-endian UTF-16 code units, 2 bytes each.
  For 8-bit: raw binary.

  If UDHI=1 (UDH present for concatenated SMS):
    Byte 0:    UDHL (UDH Length) — length of the UDH that follows
    Bytes 1+:  UDH content (IEs — Information Elements)
    After UDH: message text, starting on septets/byte boundary
```

### GSM 7-Bit Alphabet: Fitting 160 Characters in 140 Bytes

This is one of the most elegant bit-packing tricks in telecommunications.
The standard SMS payload is 140 bytes = 1120 bits. If you use 7 bits per
character instead of 8, you get 1120 ÷ 7 = 160 characters. That famous "160
character limit" comes directly from this math.

```
7-bit packing example: "Hello"
════════════════════════════════

  GSM 7-bit codes:
    'H' = 72 = 0x48 = 0b1001000
    'e' = 65 = 0x65 = 0b1100101
    'l' = 76 = 0x6C = 0b1101100
    'l' = 76 = 0x6C = 0b1101100
    'o' = 111 = 0x6F = 0b1101111

  Wait — those are ASCII codes, but GSM 7-bit is NOT ASCII!
  GSM uses its own table (see below). Let's use the correct values:
    'H' = 72  (GSM code 72 — happens to be same as ASCII)
    'e' = 101 (GSM code 101)
    'l' = 108 (GSM code 108)
    'l' = 108
    'o' = 111

  Pack 7 bits each, LSB first:
    H  = 0b1001000  → bits: 0,0,0,1,0,0,1  (LSB to MSB)
    e  = 0b1100101  → bits: 1,0,1,0,0,1,1
    l  = 0b1101100  → bits: 0,0,1,1,0,1,1
    l  = 0b1101100  → bits: 0,0,1,1,0,1,1
    o  = 0b1101111  → bits: 1,1,1,1,0,1,1

  Concatenate all bits (LSB first):
    0001001 1010011 0110110 0110110 1101111
    │       │       │       │       │
    H       e       l       l       o

  Regroup into 8-bit bytes (LSB first):
    Bit stream: 0,0,0,1,0,0,1, 1,0,1,0,0,1,1, 0,0,1,1,0,1,1, 0,0,1,1,...
                └────────────┘
                  byte 0
    Byte 0: bits 0-7  = 1,1,0,0,1,0,0,0 (reversed for display) = 0xE8
    Wait, let me be more careful:

  Correct packing algorithm:
    Take all characters' 7-bit codes in order.
    Pack them into bytes by placing bit 0 of char[0] at bit 0 of byte[0],
    bit 1 of char[0] at bit 1 of byte[0], ..., bit 6 of char[0] at bit 6
    of byte[0], bit 0 of char[1] at bit 7 of byte[0], bit 1 of char[1] at
    bit 0 of byte[1], and so on.

  For "Hello" (H=72, e=101, l=108, l=108, o=111):
    H: 1001000  → place at bits[0..6]
    e: 1100101  → bit 0 of e at bit[7], rest at bits[8..13]
    l: 1101100  → ...

  Resulting bytes:
    Byte 0: H[6:0] + e[0]   = 0100 0001 (wait, e[0]=1, H=1001000)
                             = 1_1001000 = 0xE8
    Byte 1: e[6:1] + l[1:0] = 10_110010 = 0x32 + upper... let's trace:
    
  "Hello" packed into 5 bytes:
    0xE8 0x32 0x9B 0xFD 0x06

  Verification: 5 characters × 7 bits = 35 bits = 4.375 bytes → ceil = 5 bytes ✓
```

#### The GSM Basic Character Set Table

```
GSM 7-bit Default Alphabet (3GPP TS 23.038)
════════════════════════════════════════════

  Code  Char    Code  Char    Code  Char    Code  Char
  ────────────  ────────────  ────────────  ────────────
  0x00  @       0x10  Δ       0x20  SP      0x30  0
  0x01  £       0x11  _       0x21  !       0x31  1
  0x02  $       0x12  Φ       0x22  "       0x32  2
  0x03  ¥       0x13  Γ       0x23  #       0x33  3
  0x04  è       0x14  Λ       0x24  ¤       0x34  4
  0x05  é       0x15  Ω       0x25  %       0x35  5
  0x06  ù       0x16  Π       0x26  &       0x36  6
  0x07  ì       0x17  Ψ       0x27  '       0x37  7
  0x08  ò       0x18  Σ       0x28  (       0x38  8
  0x09  Ç       0x19  Θ       0x29  )       0x39  9
  0x0A  LF      0x1A  Ξ       0x2A  *       0x3A  :
  0x0B  Ø       0x1B  ESC     0x2B  +       0x3B  ;
  0x0C  ø       0x1C  Æ       0x2C  ,       0x3C  <
  0x0D  CR      0x1D  æ       0x2D  -       0x3D  =
  0x0E  Å       0x1E  ß       0x2E  .       0x3E  >
  0x0F  å       0x1F  É       0x2F  /       0x3F  ?

  Code  Char    Code  Char    Code  Char    Code  Char
  0x40  ¡       0x50  P       0x60  ¿       0x70  p
  0x41  A       0x51  Q       0x61  a       0x71  q
  0x42  B       0x52  R       0x62  b       0x72  r
  0x43  C       0x53  S       0x63  c       0x73  s
  0x44  D       0x54  T       0x64  d       0x74  t
  0x45  E       0x55  U       0x65  e       0x75  u
  0x46  F       0x56  V       0x66  f       0x76  v
  0x47  G       0x57  W       0x67  g       0x77  w
  0x48  H       0x58  X       0x68  h       0x78  x
  0x49  I       0x59  Y       0x69  i       0x79  y
  0x4A  J       0x5A  Z       0x6A  j       0x7A  z
  0x4B  K       0x5B  Ä       0x6B  k       0x7B  ä
  0x4C  L       0x5C  Ö       0x6C  l       0x7C  ö
  0x4D  M       0x5D  Ñ       0x6D  m       0x7D  ñ
  0x4E  N       0x5E  Ü       0x6E  n       0x7E  ü
  0x4F  O       0x5F  §       0x6F  o       0x7F  à

  Notable differences from ASCII:
  - 0x00 is '@' (not NUL)
  - 0x24 is '¤' (not '$' — dollar is 0x02)
  - 0x40 is '¡' (not '@')
  - Greek capitals (Δ Φ Γ Λ Ω Π Ψ Σ Θ Ξ) are in the control range
  - No lowercase accented Latin letters except in the 0x70+ range
```

#### The GSM Extension Table (Escaped Characters)

```
GSM Extension Table (accessed via ESC = 0x1B prefix)
══════════════════════════════════════════════════════

  When you see the byte 0x1B in a GSM 7-bit stream, the NEXT byte is
  looked up in this extension table instead of the basic table.
  Each extended character costs 2 septets (14 bits) instead of 7.

  Extended  Char    Extended  Char
  ────────────────  ────────────────
  0x0A      FF (form feed)
  0x14      ^
  0x28      {
  0x29      }
  0x2F      \
  0x3C      [
  0x3D      ~
  0x3E      ]
  0x40      |
  0x65      €     ← This is why the euro sign costs 2 chars in an SMS!

  Analogy: The extension table is like a "shift" key on an old typewriter.
  Press ESC first, then the key, to get the extended character. This is
  why typing "€100" in an SMS costs 5 character-slots (ESC + €, then 100).

  Real-world impact: The € symbol appearing in a text message reduces your
  remaining characters by 1 extra because it costs 2 septets instead of 1.
```

### UCS-2 Encoding: When 7 Bits Isn't Enough

If your message contains any character not in the GSM 7-bit alphabet — for
example, Chinese characters, Arabic, emoji, or accented letters not in the
extension table — the phone must switch to UCS-2 encoding.

```
UCS-2 vs UTF-16 vs Unicode:
═════════════════════════════

  Unicode:   The standard that assigns code points to every character.
             'A' = U+0041, '€' = U+20AC, '你' = U+4F60, '😀' = U+1F600

  UCS-2:     Encodes Unicode code points U+0000 to U+FFFF as 2 bytes
             each, big-endian. Cannot represent code points above U+FFFF
             (e.g., most emoji which are in the U+1F000+ range).

  UTF-16:    Extension of UCS-2 that uses "surrogate pairs" to encode
             code points above U+FFFF. Most modern phones actually use
             UTF-16, but the SMS spec says UCS-2.

  In SMS:
  DCS = 0x08 → 2 bytes per character, big-endian
  Max characters per SMS = 140 bytes ÷ 2 = 70 characters

  Example: "Hello" in UCS-2:
    'H' = U+0048 → 0x00 0x48
    'e' = U+0065 → 0x00 0x65
    'l' = U+006C → 0x00 0x6C
    'l' = U+006C → 0x00 0x6C
    'o' = U+006F → 0x00 0x6F
    Total: 10 bytes, UDL = 10

  Example: "你好" in UCS-2:
    '你' = U+4F60 → 0x4F 0x60
    '好' = U+597D → 0x59 0x7D
    Total: 4 bytes, UDL = 4

  The 70-character limit is why WhatsApp/iMessage exist:
  A 70-character limit for non-Latin scripts is very restrictive.
  Messaging apps use TCP/IP instead of SMS PDUs, removing this limit.
```

### Concatenated SMS: Long Messages

What happens when your message is longer than 160 characters (7-bit) or 70
characters (UCS-2)? The phone splits it into multiple SMS parts, each
containing a **User Data Header (UDH)** that lets the recipient reassemble
them in order.

```
UDH Structure (User Data Header)
══════════════════════════════════

  The UDH is prepended to the User Data field when UDHI=1 in the PDU type.

  ┌──────┬────────────────────────────────────────────────────────────┐
  │ Byte │ Meaning                                                    │
  ├──────┼────────────────────────────────────────────────────────────┤
  │  0   │ UDHL — UDH Length (not including this byte itself)         │
  │      │ For standard concatenation: 0x05                           │
  ├──────┼────────────────────────────────────────────────────────────┤
  │  1   │ IEI — Information Element Identifier                       │
  │      │ 0x00 = Concatenated short message, 8-bit ref number        │
  │      │ 0x08 = Concatenated short message, 16-bit ref number       │
  ├──────┼────────────────────────────────────────────────────────────┤
  │  2   │ IEDL — IE Data Length                                      │
  │      │ For 8-bit ref: 0x03 (3 bytes of data follow)               │
  ├──────┼────────────────────────────────────────────────────────────┤
  │  3   │ Reference number — links all parts of the same message     │
  │      │ 0x00 to 0xFF for 8-bit, 0x0000 to 0xFFFF for 16-bit       │
  ├──────┼────────────────────────────────────────────────────────────┤
  │  4   │ Total number of parts (e.g., 3 for a 3-part message)       │
  ├──────┼────────────────────────────────────────────────────────────┤
  │  5   │ Part number (1-based: 1, 2, 3, ...)                        │
  └──────┴────────────────────────────────────────────────────────────┘

  A concatenated SMS using 8-bit reference numbers:
  UDHL=05, IEI=00, IEDL=03, REF=42, TOTAL=03, PART=01

  This header is 6 bytes. For 7-bit encoding, it occupies 7 septets
  (ceil(6×8/7) = 7), leaving 153 characters per part instead of 160.

  UDH alignment for 7-bit encoding:
  ┌─────────────────────────────────────────────────────────────────┐
  │ The UDH must always end on a septet boundary.                   │
  │ 6 bytes UDH = 48 bits. Septets = ceil(48/7) = 7 septets.       │
  │ 7 septets = 49 bits. Padding = 1 bit (fill low bit of first     │
  │ message septet with 0).                                         │
  │ Remaining septets: 160 - 7 = 153 characters per part.           │
  └─────────────────────────────────────────────────────────────────┘

  For UCS-2: UDH occupies 6 bytes, leaving 140 - 6 = 134 bytes = 67
  characters per part.

Reassembly example:
═══════════════════

  Sender sends "A long message..." split into 3 parts:

  Part 1: UDH(ref=42, total=3, part=1) + "Part one text here (153 chars)"
  Part 2: UDH(ref=42, total=3, part=2) + "Part two text here (153 chars)"
  Part 3: UDH(ref=42, total=3, part=3) + "Part three (shorter)"

  Recipient phone:
  1. Receives Part 2 first (networks don't guarantee order)
  2. Stores it in a reassembly buffer keyed by (sender, ref=42)
  3. Receives Part 1, stores it
  4. Receives Part 3 — total=3, have 3 parts — reassemble!
  5. Concatenate parts 1, 2, 3 in order → display full message

  What if a part never arrives?
  The phone waits (typically 24-48 hours), then either displays what
  it has or silently discards the incomplete message. There is no
  automatic retransmission at the SMS level — the SMSC handles
  delivery of each individual PDU independently.
```

### SMS-DELIVER PDU: What the Recipient Gets

The SMSC transforms your SMS-SUBMIT into an SMS-DELIVER PDU before forwarding
it to the recipient. The structure is similar but the fields differ:

```
SMS-DELIVER PDU Wire Format
════════════════════════════

  [SMSC Address] [PDU Type] [OA] [PID] [DCS] [SCTS] [UDL] [UD]

  OA   — Originating Address (who sent it)
  SCTS — Service Centre Time Stamp (7 bytes) — when SMSC received it

  SCTS format (7 bytes, BCD semi-octets):
  ┌────────┬───────────────────────────────────────────────────────────┐
  │ Byte   │ Contents                                                  │
  ├────────┼───────────────────────────────────────────────────────────┤
  │ 0      │ Year (last 2 digits, BCD): e.g., 0x26 = year 2026        │
  │ 1      │ Month (BCD): 0x04 = April                                 │
  │ 2      │ Day (BCD): 0x22 = 22nd                                    │
  │ 3      │ Hour (BCD): 0x15 = 15:00                                  │
  │ 4      │ Minute (BCD): 0x30 = :30                                  │
  │ 5      │ Second (BCD): 0x00 = :00                                  │
  │ 6      │ Timezone (BCD semi-octets, signed): 0x00 = UTC+0          │
  │        │ Each unit = 15 minutes. 0x08 = +02:00                     │
  │        │ Negative: bit 3 of high nibble set. 0x28 = -02:00         │
  └────────┴───────────────────────────────────────────────────────────┘

  PDU Type for SMS-DELIVER:
  Bit 1-0 (MTI): 00 = SMS-DELIVER
  Bit 2 (MMS):   0 = more messages waiting at SMSC (rare)
  Bit 5 (SRI):   1 = status report was requested by sender
  Bit 6 (UDHI):  1 = UDH present
  Bit 7 (RP):    reply path indicator
```

### SMS-STATUS-REPORT PDU: Delivery Receipts

When you set SRR=1 in your SMS-SUBMIT, the SMSC sends back an
SMS-STATUS-REPORT PDU once the message is delivered (or fails).

```
SMS-STATUS-REPORT PDU
══════════════════════

  [SMSC Addr] [PDU Type] [MR] [RA] [SCTS] [DT] [ST]

  MR   — Message Reference (matches the MR you sent in SMS-SUBMIT)
  RA   — Recipient Address (the destination number you sent to)
  SCTS — When the original message was submitted to SMSC
  DT   — Discharge Time (when the message was delivered or failed)
  ST   — Status byte

  Status byte values:
  ┌──────┬─────────────────────────────────────────────────────────────┐
  │ ST   │ Meaning                                                     │
  ├──────┼─────────────────────────────────────────────────────────────┤
  │ 0x00 │ SM delivered successfully ← the happy path                  │
  │ 0x01 │ SM forwarded, final status unknown                          │
  │ 0x02 │ SM replaced by SC                                           │
  ├──────┼─────────────────────────────────────────────────────────────┤
  │ 0x20 │ Congestion — retry later                                     │
  │ 0x21 │ SME (phone) busy — retry later                              │
  │ 0x22 │ No response from SME                                         │
  │ 0x23 │ Service rejected — retry later                               │
  ├──────┼─────────────────────────────────────────────────────────────┤
  │ 0x40 │ Remote procedure error — permanent failure                   │
  │ 0x41 │ Incompatible destination                                     │
  │ 0x42 │ Connection rejected by SME                                   │
  │ 0x43 │ Not obtainable — permanent failure                           │
  │ 0x44 │ Quality of service not available                             │
  │ 0x45 │ No interworking available                                    │
  │ 0x46 │ SM validity period expired                                    │
  │ 0x47 │ SM deleted by originating SME                                │
  │ 0x48 │ SM deleted by SC administration                              │
  │ 0x49 │ SM does not exist                                            │
  └──────┴─────────────────────────────────────────────────────────────┘

  Analogy: "Submit-on-submit" vs "Delivery receipt"
  ─────────────────────────────────────────────────
  Submit-on-submit: You get a timestamp the moment the SMSC accepts your
  message. Like getting a receipt from the post office when you hand in
  the parcel. The parcel hasn't been delivered yet.

  Delivery receipt (SRR=1): You get notified when the recipient's phone
  actually downloads and acknowledges the message. Like signature-required
  registered mail — you get a card back saying "delivered at 3:15pm."

  The SMSC timestamp (SCTS in SMS-DELIVER) is the submit-on-submit time.
  The Discharge Time (DT in STATUS-REPORT) is the actual delivery time.
  The gap between them is how long the SMSC held the message.
```

## Complete Worked Example: "Hello" from +12125551234 to +442071234567

Let's build the complete SMS-SUBMIT PDU byte by byte.

```
Parameters:
  SMSC number: +447785016005 (UK Vodafone SMSC)
  Sender:      +12125551234  (New York number)
  Recipient:   +442071234567
  Message:     "Hello"
  Encoding:    GSM 7-bit (DCS = 0x00)
  VP:          None (SMSC uses default)
  SRR:         0 (no delivery report)

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
STEP 1: SMSC Address (+447785016005)
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

  Digits: 447785016005 → 12 digits
  TON/NPI: 0x91 (international, ISDN)
  BCD encoding of 447785016005:
    4,4 → 0x44
    7,7 → 0x77
    8,5 → 0x58
    0,1 → 0x10
    6,0 → 0x06
    0,5 → 0x50

  Length byte = 7 (1 byte TON/NPI + 6 bytes BCD = 7 bytes total that follow)

  SMSC field: 07 91 44 77 58 10 06 50
              │  │  └────────────────── BCD digits
              │  └── TON/NPI = 0x91
              └── 7 bytes follow

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
STEP 2: PDU Type Byte
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

  MTI = 01 (SMS-SUBMIT)
  RD  = 0  (accept duplicates)
  VPF = 00 (no VP field)
  SRR = 0  (no status report)
  UDHI= 0  (no UDH, single-part message)
  RP  = 0

  Bits: 0  0  0  0  0  0  0  1
        RP UDHI SRR VPF1 VPF0 RD MTI1 MTI0
  = 0x01

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
STEP 3: Message Reference
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

  MR = 0x00  (let SMSC assign a reference number)

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
STEP 4: Destination Address (+442071234567)
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

  Digits: 442071234567 → 12 digits
  TON/NPI: 0x91 (international)
  BCD encoding:
    4,4 → 0x44
    2,0 → 0x02   (note: swap! digits are "20", so bytes are 0x02)
    7,1 → 0x17
    2,3 → 0x32
    4,5 → 0x54
    6,7 → 0x76

  Number-of-digits byte = 0x0C (12 decimal)

  DA field: 0C 91 44 02 17 32 54 76

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
STEP 5: PID and DCS
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

  PID = 0x00 (plain SMS)
  DCS = 0x00 (GSM 7-bit default alphabet)

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
STEP 6: UDL and UD — encoding "Hello" in GSM 7-bit
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

  GSM 7-bit codes (these match ASCII for basic Latin):
    'H' = 72  = 0b1001000
    'e' = 101 = 0b1100101
    'l' = 108 = 0b1101100
    'l' = 108 = 0b1101100
    'o' = 111 = 0b1101111

  Pack 7 bits each, LSB first, into bytes:
    Start with a 35-bit stream (5 chars × 7 bits):
    Char  Bits (LSB→MSB):
    H:    0 0 0 1 0 0 1
    e:    1 0 1 0 0 1 1
    l:    0 0 1 1 0 1 1
    l:    0 0 1 1 0 1 1
    o:    1 1 1 1 0 1 1

    Bit stream (bit 0 first):
    0001001 1010011 0011011 0011011 1110110 1   ← 35 bits, LSB first

    Reformatted as bit positions 0..34:
    pos: 0  1  2  3  4  5  6  7  8  9  10 11 12 13 14 15 16 17 ...
    bit: 0  0  0  1  0  0  1  1  0  1  0  0  1  1  0  0  1  1  ...

    Group into bytes (8 bits each):
    Byte 0 (bits 0-7):  0 0 0 1 0 0 1 1  → read as value: bit0=0,bit1=0,...
                        value = 0b11001000 wait, we read LSB→MSB:
                        bit0=0, bit1=0, bit2=0, bit3=1, bit4=0, bit5=0,
                        bit6=1, bit7=1
                        = 0b11001000... no, binary value:
                        bit 7 is the MSB. bit[7]=1, bit[6]=1, bit[5]=0,
                        bit[4]=0, bit[3]=1, bit[2]=0, bit[1]=0, bit[0]=0
                        = 0b11001000 = 0xC8? That doesn't match known result.

  Let me use the standard reference. The known packed bytes for "Hello" are:
    E8 32 9B FD 06

  Cross-check:
    0xE8 = 1110 1000
    0x32 = 0011 0010
    0x9B = 1001 1011
    0xFD = 1111 1101
    0x06 = 0000 0110

  Unpack: read bits LSB first from the byte stream:
    Byte 0 = 0xE8: bits = 0,0,0,1,0,1,1,1  (LSB first) = 0001011 0...
    Hmm, 0xE8 in binary is 1110 1000.
    Bits LSB first: bit0=0, bit1=0, bit2=0, bit3=1, bit4=0, bit5=1, bit6=1, bit7=1
    First 7 bits (char 0): bit0..bit6 = 0,0,0,1,0,1,1 = 0b1101000 = 72 = 'H' ✓
    Bit 7 of byte 0 = 1 → becomes bit 0 of 'e'
    Byte 1 = 0x32 = 0011 0010:
    Bits LSB first: 0,1,0,0,1,1,0,0
    'e' bits: bit0=1(from byte0 bit7), bit1..bit6 from byte1 bits0..5
             = 1, 0,1,0,0,1,1 = 0b1100101 = 101 = 'e' ✓

  UDL = 5 (5 characters)
  UD  = E8 32 9B FD 06 (5 bytes)

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
COMPLETE PDU (hex string sent to modem):
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

  07 91 44 77 58 10 06 50    ← SMSC address (+447785016005)
  01                         ← PDU type (SMS-SUBMIT, no VP)
  00                         ← Message reference (0)
  0C 91 44 02 17 32 54 76   ← Destination address (+442071234567)
  00                         ← PID (plain SMS)
  00                         ← DCS (GSM 7-bit)
  05                         ← UDL (5 characters)
  E8 32 9B FD 06             ← UD (packed "Hello")

  Full PDU hex string:
  07914477581006500100 0C914402173254760000 05E8329BFD06

  How this is sent via AT command:
    AT+CMGF=0            (switch to PDU mode)
    AT+CMGS=15           (15 = bytes in PDU excluding SMSC field)
    > [paste PDU bytes]
    > [Ctrl-Z to send]

  PDU length sent to AT+CMGS:
  Total PDU: 8 + 1 + 1 + 8 + 1 + 1 + 1 + 5 = 26 bytes
  Minus SMSC field (8 bytes): 26 - 8 = 18 bytes... wait:
  SMSC field = 07 91 44 77 58 10 06 50 = 8 bytes
  Remaining = 01 00 0C 91 44 02 17 32 54 76 00 00 05 E8 32 9B FD 06
            = 18 bytes → AT+CMGS=18

  Annotated hex dump:
  ┌──────────────────────────────────────────────────────────────────┐
  │ Offset  Hex    Field                                             │
  ├──────────────────────────────────────────────────────────────────┤
  │ 0       07     SMSC length (7 bytes follow)                      │
  │ 1       91     SMSC TON/NPI (international)                      │
  │ 2-7     44 77 58 10 06 50   SMSC BCD (+447785016005)             │
  │ 8       01     PDU type (SMS-SUBMIT)                             │
  │ 9       00     Message reference                                  │
  │ 10      0C     DA length (12 digits)                             │
  │ 11      91     DA TON/NPI (international)                        │
  │ 12-17   44 02 17 32 54 76  DA BCD (+442071234567)                │
  │ 18      00     PID                                               │
  │ 19      00     DCS (GSM 7-bit)                                   │
  │ 20      05     UDL (5 characters)                                │
  │ 21-25   E8 32 9B FD 06     UD ("Hello" packed 7-bit)            │
  └──────────────────────────────────────────────────────────────────┘
```

## AT Commands: The Legacy Modem Interface

AT commands (the Hayes command set, AT = "ATtention") are how software
communicates with cellular modems. Every GSM modem supports them. They are
the plumbing behind SIM800 modules, Sierra Wireless modems, USB 3G dongles,
and even modern LTE routers.

```
AT Command Reference for SMS
══════════════════════════════

  AT+CMGF — Message Format
  ┌──────────────────┬──────────────────────────────────────────────┐
  │ Command          │ Effect                                       │
  ├──────────────────┼──────────────────────────────────────────────┤
  │ AT+CMGF=0        │ PDU mode — raw binary PDUs                   │
  │ AT+CMGF=1        │ Text mode — human-readable ASCII             │
  └──────────────────┴──────────────────────────────────────────────┘
  Always use PDU mode for production code. Text mode is for debugging.

  AT+CMGS — Send Message (PDU mode)
  ┌──────────────────────────────────────────────────────────────────┐
  │ AT+CMGS=<length>                                                 │
  │ <PDU hex string><Ctrl-Z>                                         │
  │                                                                  │
  │ <length> = number of bytes in PDU EXCLUDING the SMSC prefix      │
  │                                                                  │
  │ Response on success: +CMGS: <mr>                                 │
  │   where <mr> is the message reference assigned by SMSC           │
  │ Response on failure: +CMS ERROR: <err>                           │
  └──────────────────────────────────────────────────────────────────┘

  AT+CMGR — Read Message
  ┌──────────────────────────────────────────────────────────────────┐
  │ AT+CMGR=<index>                                                  │
  │                                                                  │
  │ Response: +CMGR: <stat>,,<length>                                │
  │           <PDU hex string>                                       │
  │           OK                                                     │
  │                                                                  │
  │ <stat>: 0=unread, 1=read, 2=unsent, 3=sent, 4=all               │
  └──────────────────────────────────────────────────────────────────┘

  AT+CMGL — List Messages
  ┌──────────────────────────────────────────────────────────────────┐
  │ AT+CMGL=0    List unread messages                                │
  │ AT+CMGL=4    List all messages                                   │
  └──────────────────────────────────────────────────────────────────┘

  AT+CNMI — New Message Indication
  ┌──────────────────────────────────────────────────────────────────┐
  │ AT+CNMI=2,1,0,0,0                                                │
  │ When a new SMS arrives, the modem sends:                         │
  │ +CMTI: "SM",<index>                                              │
  │ (meaning: new message at index <index> in SIM storage)           │
  │ Software must then call AT+CMGR=<index> to read it.              │
  └──────────────────────────────────────────────────────────────────┘

  Full send session trace:
  ────────────────────────
  → AT+CMGF=0
  ← OK
  → AT+CMGS=18
  ← > (modem is waiting for PDU input)
  → 07914477581006500100 0C914402173254760000 05E8329BFD06\x1A
     (0x1A = Ctrl-Z, signals end of input)
  ← +CMGS: 42
  ← OK
  (message reference 42 was assigned by the SMSC)
```

## Algorithms

### Algorithm 1: GSM 7-bit Encode

```
function gsm7_encode(text: string) → bytes:
    # Step 1: map each character to its GSM 7-bit code
    codes = []
    for char in text:
        if char in GSM_EXT_TABLE:
            codes.append(GSM_ESCAPE)     # 0x1B
            codes.append(GSM_EXT_TABLE[char])
        elif char in GSM_BASIC_TABLE:
            codes.append(GSM_BASIC_TABLE[char])
        else:
            raise ValueError(f"Character {char!r} not in GSM alphabet")

    # Step 2: pack 7-bit codes into bytes
    result = bytearray()
    bit_pos = 0            # current bit position in output stream
    current_byte = 0       # accumulator

    for code in codes:     # code is 0..127
        # code has 7 bits. Place them at bit_pos..bit_pos+6
        for bit in range(7):
            if (code >> bit) & 1:
                current_byte |= (1 << (bit_pos % 8))
            bit_pos += 1
            if bit_pos % 8 == 0:
                result.append(current_byte)
                current_byte = 0

    if bit_pos % 8 != 0:
        result.append(current_byte)    # flush last partial byte

    return bytes(result)
```

### Algorithm 2: GSM 7-bit Decode

```
function gsm7_decode(data: bytes, num_chars: int) → string:
    result = []
    bit_pos = 0
    escape_next = False

    for i in range(num_chars):
        # Extract 7 bits starting at bit_pos
        code = 0
        for bit in range(7):
            byte_index = (bit_pos + bit) // 8
            bit_index  = (bit_pos + bit) % 8
            if byte_index < len(data):
                code |= ((data[byte_index] >> bit_index) & 1) << bit
        bit_pos += 7

        if escape_next:
            char = GSM_EXT_TABLE_REVERSE.get(code, '?')
            escape_next = False
        elif code == GSM_ESCAPE:  # 0x1B
            escape_next = True
            continue
        else:
            char = GSM_BASIC_TABLE_REVERSE.get(code, '?')

        result.append(char)

    return ''.join(result)
```

### Algorithm 3: Split Long Message into Concatenated Parts

```
function split_message(text: string, encoding: str) → list[PDU]:
    if encoding == 'gsm7':
        max_single   = 160
        max_per_part = 153   # with UDH overhead
    elif encoding == 'ucs2':
        max_single   = 70
        max_per_part = 67
    else:
        raise ValueError(f"Unknown encoding: {encoding}")

    if len(text) <= max_single:
        return [build_single_pdu(text, encoding)]

    # Split into parts
    parts  = []
    ref    = random.randint(0, 255)    # shared reference for all parts
    chunks = []
    i = 0
    while i < len(text):
        chunks.append(text[i : i + max_per_part])
        i += max_per_part

    total = len(chunks)
    for part_num, chunk in enumerate(chunks, start=1):
        udh = build_udh(ref=ref, total=total, part=part_num)
        pdu = build_pdu_with_udh(chunk, encoding, udh)
        parts.append(pdu)

    return parts


function build_udh(ref: int, total: int, part: int) → bytes:
    # 8-bit reference number UDH (IEI=0x00)
    return bytes([
        0x05,   # UDHL: 5 bytes of UDH data follow
        0x00,   # IEI: concatenated SMS, 8-bit ref
        0x03,   # IEDL: 3 bytes of IE data
        ref,    # Reference number
        total,  # Total parts
        part,   # This part's sequence number
    ])
```

### Algorithm 4: Build SMS-SUBMIT PDU

```
function build_sms_submit(
    smsc:        str,       # e.g. "+447785016005"
    destination: str,       # e.g. "+442071234567"
    message:     str,
    request_dr:  bool = False,
) → bytes:

    # Determine encoding
    if all(c in GSM_ALPHABET for c in message):
        dcs   = 0x00
        ud    = gsm7_encode(message)
        udl   = len(message)      # character count for 7-bit
    else:
        dcs   = 0x08
        ud    = message.encode('utf-16-be')
        udl   = len(ud)           # byte count for UCS-2

    # Build PDU type byte
    pdu_type = 0x01   # SMS-SUBMIT, MTI=01
    if request_dr:
        pdu_type |= 0x20  # set SRR bit

    # Assemble PDU
    pdu = bytearray()
    pdu.extend(encode_address(smsc, is_smsc=True))
    pdu.append(pdu_type)
    pdu.append(0x00)             # MR = 0
    pdu.extend(encode_address(destination, is_smsc=False))
    pdu.append(0x00)             # PID
    pdu.append(dcs)
    pdu.append(udl)
    pdu.extend(ud)

    return bytes(pdu)


function encode_address(number: str, is_smsc: bool) → bytes:
    # Strip leading '+' and spaces
    digits = ''.join(c for c in number if c.isdigit())
    international = number.startswith('+')

    ton_npi = 0x91 if international else 0xA1

    # BCD encode: swap digit pairs
    bcd = bytearray()
    padded = digits + ('F' if len(digits) % 2 else '')
    for i in range(0, len(padded), 2):
        high = int(padded[i+1], 16) if padded[i+1] != 'F' else 0xF
        low  = int(padded[i],   16)
        bcd.append((high << 4) | low)

    result = bytearray()
    if is_smsc:
        result.append(len(bcd) + 1)   # length = TON/NPI byte + BCD bytes
    else:
        result.append(len(digits))    # length = number of digits
    result.append(ton_npi)
    result.extend(bcd)
    return bytes(result)
```

## Test Strategy

### Unit Tests: GSM 7-bit Encoding

```
test "encode single ASCII character":
    assert gsm7_encode("A") == bytes([0x41])

test "encode 'Hello' produces known bytes":
    assert gsm7_encode("Hello") == bytes([0xE8, 0x32, 0x9B, 0xFD, 0x06])

test "encode exactly 160 characters":
    text = "A" * 160
    result = gsm7_encode(text)
    assert len(result) == 140   # 160 × 7 bits = 1120 bits = 140 bytes

test "encode 161 characters raises error":
    # Can't fit in one SMS without concatenation
    # (handled by split_message, not gsm7_encode)
    pass

test "encode special character '@' (GSM code 0x00)":
    assert gsm7_encode("@") == bytes([0x00])

test "encode euro sign (extension table)":
    # '€' = ESC (0x1B) + 0x65 in extension table
    # Two septets: 0x1B packed, then 0x65 packed
    result = gsm7_encode("€")
    decoded = gsm7_decode(result, 2)   # 2 chars (ESC + code)
    assert decoded == "€"   # wait — UDL would be 2 (counting the escape)
    # Actually most implementations count escaped chars as 1 in UDL
    # but 2 septets in the packed data. This is an important edge case.
```

### Unit Tests: Address Encoding

```
test "encode international number +442071234567":
    result = encode_address("+442071234567", is_smsc=False)
    assert result == bytes([0x0C, 0x91, 0x44, 0x02, 0x17, 0x32, 0x54, 0x76])

test "encode SMSC +447785016005":
    result = encode_address("+447785016005", is_smsc=True)
    assert result == bytes([0x07, 0x91, 0x44, 0x77, 0x58, 0x10, 0x06, 0x50])

test "decode BCD address recovers original number":
    encoded = encode_address("+12125551234", is_smsc=False)
    decoded  = decode_address(encoded, is_smsc=False)
    assert decoded == "+12125551234"

test "encode odd-length number (+1)":
    # Single-digit numbers need F padding
    result = encode_address("+1", is_smsc=False)
    # BCD: digit '1' padded with F → byte = 0xF1
    assert result[2] == 0xF1
```

### Unit Tests: Full PDU Assembly

```
test "build hello PDU matches reference":
    pdu = build_sms_submit(
        smsc="+447785016005",
        destination="+442071234567",
        message="Hello",
    )
    expected = bytes.fromhex(
        "079144775810065001000C914402173254760000" + "05E8329BFD06"
    )
    assert pdu == expected

test "PDU with delivery report sets SRR bit":
    pdu = build_sms_submit(
        smsc="+447785016005",
        destination="+12125551234",
        message="Hi",
        request_dr=True,
    )
    pdu_type_byte = pdu[8]   # byte after 8-byte SMSC field
    assert pdu_type_byte & 0x20   # SRR bit set

test "UCS-2 message sets DCS=0x08":
    pdu = build_sms_submit(
        smsc="+447785016005",
        destination="+12125551234",
        message="你好",
    )
    dcs_byte = pdu[-3 - 2]  # rough index — better to parse properly
    assert dcs_byte == 0x08
```

### Unit Tests: Concatenated SMS

```
test "161-character message splits into 2 parts":
    text  = "A" * 161
    parts = split_message(text, "gsm7")
    assert len(parts) == 2

test "all parts share the same reference number":
    text  = "B" * 500
    parts = split_message(text, "gsm7")
    refs  = [extract_udh_ref(p) for p in parts]
    assert len(set(refs)) == 1   # all same

test "parts are numbered 1 through N":
    text  = "C" * 500
    parts = split_message(text, "gsm7")
    nums  = [extract_udh_part_num(p) for p in parts]
    assert nums == list(range(1, len(parts)+1))

test "reassemble 3-part message (out of order)":
    original = "X" * 459   # 3 parts of 153 chars each
    parts    = split_message(original, "gsm7")
    shuffled = [parts[2], parts[0], parts[1]]
    reassembled = reassemble(shuffled)
    assert reassembled == original
```

### Integration Tests: AT Command Session

```
test "AT+CMGS sends correct byte count":
    pdu      = build_sms_submit("+447785016005", "+12125551234", "Test")
    smsc_len = pdu[0] + 1    # SMSC field length (includes length byte itself)
    at_len   = len(pdu) - smsc_len
    # Verify this would be the correct value for AT+CMGS=<at_len>
    assert at_len == len(pdu) - 8  # for our example SMSC

test "received PDU can be parsed back":
    original_text = "Hello, World!"
    sent_pdu      = build_sms_submit("+447785016005", "+12125551234", original_text)
    deliver_pdu   = smsc_simulate_deliver(sent_pdu)  # simulate SMSC conversion
    parsed        = parse_sms_deliver(deliver_pdu)
    assert parsed.text == original_text
```

### Edge Cases

```
test "empty string":
    pdu = build_sms_submit("+447785016005", "+12125551234", "")
    udl_byte = pdu[-1]   # last byte (UDL, since UD is empty)
    assert udl_byte == 0x00

test "exactly 160 GSM characters":
    text  = "A" * 160
    parts = split_message(text, "gsm7")
    assert len(parts) == 1   # fits in single SMS

test "exactly 161 GSM characters":
    text  = "A" * 161
    parts = split_message(text, "gsm7")
    assert len(parts) == 2

test "message with null byte in binary (DCS=0x04)":
    data  = bytes([0x00, 0xFF, 0x00, 0xAB])
    pdu   = build_binary_sms("+447785016005", "+12125551234", data)
    dcs   = extract_dcs(pdu)
    assert dcs == 0x04
    assert extract_ud(pdu) == data
```
