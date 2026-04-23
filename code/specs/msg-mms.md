# MSG-MMS — MMS (Multimedia Messaging Service)

## Overview

MMS is the natural successor to SMS. Where SMS is a postcard — limited to 160
characters of plain text — MMS is a parcel that can carry photos, audio clips,
video, slideshows, and rich-formatted text. MMS was standardized by the Open
Mobile Alliance (OMA) in the early 2000s and is the protocol behind the
"picture message" button in every phone's messaging app.

**Analogy:** Sending an MMS is like ordering something from an online shop
using a text-message notification system:

1. You tap "Send" in the messaging app (you are the sender).
2. Your phone uploads the photo + message to a **MMSC** (Multimedia Messaging
   Service Centre) — this is your carrier's server. The upload uses HTTP over
   your cellular data connection.
3. The MMSC sends the recipient's phone a **WAP Push notification** — a tiny
   SMS-like alert saying "You have a new MMS waiting. Click here to download."
4. The recipient's phone automatically (or on user action) fetches the MMS
   from the MMSC using an HTTP GET request.
5. The phone decodes the MMSC's response — a binary blob using OMA's binary
   encoding — and displays the photo slideshow.

This two-step upload/download design is important. The message itself never
travels inside the WAP Push notification — only a URL does. This allowed early
GPRS phones (which had tiny RAM) to download only the parts they could handle.

**Why couldn't SMS just carry images?**

```
SMS maximum payload:     140 bytes (1120 bits)
Smallest JPEG thumbnail: ~2,000 bytes
PNG screenshot:          ~50,000 bytes
15-second video clip:    ~3,000,000 bytes
```

SMS is simply too small. Even if you base64-encoded a tiny image and split it
across 20+ concatenated SMS parts, the overhead would be enormous and there
would be no way to signal "these parts form an image, display them together."
MMS needed a completely new protocol stack.

The governing standard is **OMA MMS 1.3** (Open Mobile Alliance Multimedia
Messaging Service version 1.3), built on top of **WAP** (Wireless Application
Protocol). The binary encoding comes from **WSP** (Wireless Session Protocol),
the WAP equivalent of HTTP.

## Layer Position

```
User Application (Messages.app, third-party MMS client)
│
│   "Send this photo to +442071234567"
▼
MMS Application Layer (this package)
│   - Builds MMS PDU: binary-encoded headers + multipart body
│   - Encodes SMIL (the slideshow layout)
│   - Attaches image/audio/video parts with MIME types
│   - Sends via HTTP POST to MMSC URL
│   - Receives WAP Push notifications from MMSC
│   - Fetches pending MMS via HTTP GET
│   - Decodes binary WSP headers and multipart body
│
▼
APN Configuration Layer
│   - MMS requires the MMS APN (separate from internet APN)
│   - Phone requests "mms" APN context from modem
│   - Gets a carrier-internal IP address on the MMS subnet
│   - MMSC URL is only reachable via this APN
│   (e.g., mms.verizon.com only resolves from Verizon's MMS subnet)
│
▼
WAP Push Reception
│   - MMSC notifies phone of pending MMS via WAP Push
│   - WAP Push arrives as a special binary SMS (DCS=0xF5, port 2948)
│   - Contains X-Mms-Message-Type: m-notification-ind
│   - Contains the Content-Location URL for fetching
│
▼
HTTP over Cellular (GPRS / UMTS / LTE)
│   - Standard HTTP/1.1 for upload and download
│   - User-Agent identifies the device to the MMSC
│   - Content-Type: application/vnd.wap.mms-message
│
▼
MM4 (inter-carrier relay) — when sender and recipient are on different carriers
│   - SMTP-based relay between carrier MMSCs
│   - MM4_Forward.REQ / MM4_Forward.RES message types
│   - Standard email infrastructure repurposed for MMS relay
│
▼
MMSC (Multimedia Messaging Service Centre)
│   - Carrier's MMS server (e.g., mms.t-mobile.com)
│   - Stores messages, resizes oversized media
│   - Routes to remote MMSC via MM4
│   - Tracks delivery/read reports
│
▼
Radio Network (GPRS / UMTS / LTE)
    - MMS uses the packet-switched (data) network, not the circuit-switched
      voice/SMS network. This is why MMS requires data to be enabled.
```

**Depends on:** Cellular data connectivity (APN context), SMS for WAP Push
delivery (the notification is a binary SMS), HTTP for content transfer.

**Used by:** Consumer messaging apps, group MMS, A2P MMS (marketing messages
with images), carrier interoperability (MM4), enterprise messaging platforms.

## Key Concepts

### The MMS APN: A Separate Data Lane

**APN (Access Point Name)** is the gateway name your phone uses to connect to
the internet via the cellular network. Think of it as the address of the door
you knock on to get internet access. Most phones have two APNs configured:

```
APN configuration example (AT&T):
══════════════════════════════════

  Internet APN:
    Name:     AT&T Internet
    APN:      phone         (or "broadband")
    Purpose:  Regular internet browsing, app downloads, streaming

  MMS APN:
    Name:     AT&T MMS
    APN:      mms
    MMSC URL: http://mmsc.mobile.att.net
    MMS Proxy: proxy.mobile.att.net:80
    Purpose:  ONLY for sending/receiving MMS

Why are they separate?
  The MMSC server lives on a special carrier-internal subnet.
  It is NOT reachable from the public internet.
  The MMS APN gives your phone an IP address on that internal subnet.
  The internet APN gives you a NAT'd public-facing IP.

  When your phone sends MMS:
  1. Suspend (or establish alongside) the internet APN context
  2. Activate the MMS APN context — get an IP on carrier's MMS subnet
  3. POST the MMS to MMSC URL
  4. Deactivate MMS APN (or keep for subsequent messages)
  5. Resume internet APN

  Many LTE phones can have multiple APN contexts active simultaneously,
  so modern phones do not need to suspend internet access to send MMS.

AT modem commands for APN:
  AT+CGDCONT=1,"IP","mms"          # configure context 1 with MMS APN
  AT+CGACT=1,1                     # activate context 1
  AT+CGPADDR=1                     # query assigned IP address
```

### Reference Points: MM1 and MM4

OMA defines reference points (interfaces) between components of the MMS system.

```
MMS Reference Points
══════════════════════

  Phone A              MMSC A              MMSC B              Phone B
  (T-Mobile)           (T-Mobile)          (AT&T)              (AT&T)
      │                    │                   │                   │
      │    MM1 (HTTP)      │                   │                   │
      ├───────────────────►│                   │                   │
      │  m-send-req PDU    │    MM4 (SMTP)     │                   │
      │                    ├──────────────────►│                   │
      │                    │  MM4_Forward.REQ  │  WAP Push         │
      │                    │                   ├──────────────────►│
      │                    │                   │  m-notification   │
      │                    │                   │                   │
      │                    │                   │  HTTP GET (MM1)   │
      │                    │                   │◄──────────────────┤
      │                    │                   │  m-retrieve-conf  │
      │                    │                   ├──────────────────►│
      │                    │    MM4 ack        │                   │
      │    m-send-conf     │◄──────────────────┤                   │
      │◄───────────────────┤                   │                   │

  MM1: Between handset and MMSC. Uses HTTP.
  MM4: Between two MMSCs (inter-carrier). Uses SMTP.

  MM2, MM3: Internal MMSC interfaces (not visible to implementors).
  MM5: MMSC to HLR (for subscriber info lookup).
  MM6, MM7: Value-added services (e.g., enterprise MMS API).
```

### MM1: The Handset-to-MMSC Flow in Detail

```
MM1 Send Flow (Phone A sends MMS):
════════════════════════════════════

  Step 1: Phone builds the MMS PDU
    - Binary-encoded headers (m-type: m-send-req, transaction-id, etc.)
    - Multipart body: SMIL + image + text parts
    - Content-Type: application/vnd.wap.mms-message

  Step 2: Phone HTTP POSTs to MMSC
    POST http://mmsc.t-mobile.com HTTP/1.1
    Content-Type: application/vnd.wap.mms-message
    User-Agent: Android/MmsService
    Content-Length: <length>

    <binary MMS PDU>

  Step 3: MMSC responds with m-send-conf
    HTTP/1.1 200 OK
    Content-Type: application/vnd.wap.mms-message

    <binary m-send-conf PDU>
    (contains: response-status=ok, transaction-id, message-id)

MM1 Receive Flow (Phone B receives MMS):
═════════════════════════════════════════

  Step 1: MMSC B sends WAP Push to Phone B
    (This is a binary SMS delivered to port 2948)
    Contains: m-notification-ind PDU
    Key fields:
      m-message-class: personal
      m-message-size: 45000
      m-content-location: http://mmsc.att.com/mms/retrieve?id=abc123
      m-expiry: (timestamp)

  Step 2: Phone B auto-fetches or waits for user
    GET http://mmsc.att.com/mms/retrieve?id=abc123 HTTP/1.1
    Accept: */*, application/vnd.wap.mms-message
    x-wap-profile: http://...   (device capability profile)

  Step 3: MMSC B responds with m-retrieve-conf
    HTTP/1.1 200 OK
    Content-Type: application/vnd.wap.mms-message

    <binary m-retrieve-conf PDU>
    (contains: all headers + multipart body with SMIL + image)

  Step 4: Phone B sends m-notifyresp-ind (delivery receipt)
    POST http://mmsc.att.com HTTP/1.1
    <binary m-notifyresp-ind PDU>
    (contains: transaction-id, status=retrieved)
```

### WAP Push Notification: How the Phone Knows a Message is Waiting

WAP Push is a binary SMS that carries a URL. The phone's SMS daemon watches
for SMS messages on UDP port 2948 (the WAP Push port). When one arrives, it
triggers the MMS client.

```
WAP Push PDU structure:
════════════════════════

  A WAP Push is a binary SMS with:
    DCS = 0xF5 (8-bit, class 1, port addressing active)
    The UD contains a WSP (Wireless Session Protocol) PDU

  The WSP PDU:
  ┌─────────────────────────────────────────────────────────────────┐
  │ Transaction ID  (1 byte): 0x00 (connectionless)                 │
  │ PDU Type        (1 byte): 0x06 (Push)                           │
  │ Headers Length  (uintvar): length of headers section             │
  │ Content-Type   (encoded): application/vnd.wap.mms-message        │
  │   or for notification: application/vnd.wap.sic (Service Ind.)   │
  │ X-WAP-Application-ID: x-wap-application:mms.ua                  │
  │ [Body: binary MMS notification PDU]                             │
  └─────────────────────────────────────────────────────────────────┘

  The X-WAP-Application-ID header is what tells the phone "route this
  WAP Push to the MMS client, not some other WAP application."

  Binary WAP Push bytes for an MMS notification (hexdump):
  ┌──────────────────────────────────────────────────────────────────┐
  │ 00         Transaction ID = 0                                    │
  │ 06         WSP PDU type = Push                                   │
  │ 01         Headers length = 1 byte                               │
  │ AE         Content-Type short integer for                        │
  │            application/vnd.wap.mms-message (0x2E | 0x80)        │
  │ [body: binary m-notification-ind PDU follows]                    │
  └──────────────────────────────────────────────────────────────────┘

  uintvar (variable-length integer): WSP uses a variable-length
  integer encoding where bit 7 of each byte signals "more bytes
  follow" and bits 6-0 carry 7 bits of value. Similar to protobuf
  varints but big-endian:

  Value 128:  0x81 0x00   (1000 0001, 0000 0000)
              └─more──┘   └─last─┘
              7 bits: 0000001, then 0000000 → 0b0000001_0000000 = 128

  Value 300:  0x82 0x2C   (1000 0010, 0010 1100)
              7 bits: 0000010, then 0101100 → 0b0000010_0101100 = 300

  Value 127:  0x7F        (single byte, no continuation)
```

### MMS Binary Header Encoding

This is the heart of the protocol. OMA MMS uses a binary encoding for all
headers — not plain-text HTTP headers. This saves bandwidth on slow GPRS
connections (max ~40 kbps in practice).

**Why binary headers instead of text?** The string "Content-Type:
application/vnd.wap.mms-message\r\n" is 47 bytes. The binary equivalent is
2 bytes: 0x84 (field code for Content-Type in WSP) followed by 0x9E (short
integer code for the MIME type). A factor-of-20 compression on header data.

```
Binary Header Field Codes (OMA MMS 1.3 Table)
═══════════════════════════════════════════════

  Each header field has an assigned code byte with the high bit set (0x80+).

  ┌──────────┬──────────────────────────────────────────────────────────┐
  │ Code     │ Header Field                                             │
  ├──────────┼──────────────────────────────────────────────────────────┤
  │ 0x8C     │ X-Mms-Message-Type  (m-type)                             │
  │ 0x98     │ X-Mms-Transaction-Id                                     │
  │ 0x8D     │ X-Mms-MMS-Version                                        │
  │ 0x97     │ X-Mms-Subject                                            │
  │ 0x89     │ X-Mms-From                                               │
  │ 0x97     │ X-Mms-To                                                 │
  │ 0x81     │ X-Mms-Bcc                                                │
  │ 0x82     │ X-Mms-Cc                                                 │
  │ 0x85     │ X-Mms-Date                                               │
  │ 0x86     │ X-Mms-Delivery-Report                                    │
  │ 0x87     │ X-Mms-Delivery-Time                                      │
  │ 0x8A     │ X-Mms-Message-Class                                      │
  │ 0x8E     │ X-Mms-Message-Size                                       │
  │ 0x88     │ X-Mms-Expiry                                             │
  │ 0x83     │ X-Mms-Content-Location                                   │
  │ 0x8B     │ X-Mms-Message-Id                                         │
  │ 0x96     │ X-Mms-Response-Status                                    │
  │ 0x84     │ Content-Type  (of the whole PDU body)                    │
  │ 0x91     │ X-Mms-Read-Report                                        │
  │ 0x95     │ X-Mms-Report-Allowed                                     │
  └──────────┴──────────────────────────────────────────────────────────┘

  Message-Type values (for field 0x8C):
  ┌──────────┬─────────────────────────────────────────────────────────┐
  │ Value    │ Message type                                            │
  ├──────────┼─────────────────────────────────────────────────────────┤
  │ 0x80     │ m-send-req        (handset → MMSC, send request)        │
  │ 0x81     │ m-send-conf       (MMSC → handset, send confirmation)   │
  │ 0x82     │ m-notification-ind (MMSC → handset, WAP Push notif.)    │
  │ 0x83     │ m-notifyresp-ind  (handset → MMSC, notification ack)   │
  │ 0x84     │ m-retrieve-conf   (MMSC → handset, fetched MMS)        │
  │ 0x85     │ m-acknowledge-ind (handset → MMSC, retrieve ack)       │
  │ 0x86     │ m-delivery-ind    (MMSC → sender, delivery report)     │
  │ 0x87     │ m-read-rec-ind    (handset → MMSC, read report)        │
  │ 0x88     │ m-read-orig-ind   (MMSC → sender, read report relay)   │
  └──────────┴─────────────────────────────────────────────────────────┘

  MMS Version values (for field 0x8D):
  ┌──────────┬─────────────────────────────────────────────────────────┐
  │ 0x90     │ MMS 1.0                                                 │
  │ 0x91     │ MMS 1.1                                                 │
  │ 0x92     │ MMS 1.2                                                 │
  │ 0x93     │ MMS 1.3   ← most common today                           │
  └──────────┴─────────────────────────────────────────────────────────┘
```

#### Value Encoding Rules

```
How header values are encoded:
════════════════════════════════

  The encoding depends on the value type:

  1. Short Integer (values 0..127):
     Single byte with high bit SET: value | 0x80
     Example: MMS version 1.3 = 0x13 | 0x80 = 0x93

  2. Text string (ASCII, null-terminated):
     Bytes of the string followed by 0x00
     Example: "abc123" → 0x61 0x62 0x63 0x31 0x32 0x33 0x00

  3. Encoded string (text with character set):
     Charset-specific encoding, preceded by charset code

  4. Long integer (>127):
     One byte for the byte count, then the value bytes (big-endian)
     Example: 45000 = 0x0000AFB8 → 0x03 0x00 0xAF 0xB8
              (0x03 = 3 bytes follow)

  5. Variable-length integer (uintvar):
     Used for lengths and some counts. 7 bits per byte, MSB continuation.

  6. Date/Time:
     Long integer containing seconds since 1970-01-01 00:00:00 UTC (Unix time)

  7. Encoded address:
     For phone numbers: "/TYPE=PLMN" suffix
     "+12125551234/TYPE=PLMN" or just "+12125551234"
     For email: standard email address

  Decoding a binary header stream:
  ─────────────────────────────────
  Read byte B:
    B >= 0x80: this is a field code. Next bytes are the value.
    B == 0x7F: value-length follows as uintvar, then value bytes
    B < 0x20:  integer = B, done
    B < 0x80:  text starts here (ASCII chars until 0x00)
    B == 0x22: quoted string: read until closing 0x22
```

### SMIL: The Slideshow Layout Language

SMIL (Synchronized Multimedia Integration Language, pronounced "smile") is an
XML format that describes how the parts of an MMS should be displayed. Think of
it as the HTML of MMS — it lays out the visual presentation.

```
SMIL anatomy:
══════════════

  An MMS with one image and a caption looks like this in SMIL:
  ─────────────────────────────────────────────────────────────────────
  <?xml version="1.0"?>
  <!DOCTYPE smil PUBLIC "-//W3C//DTD SMIL 2.0//EN"
            "http://www.w3.org/2001/SMIL20/SMIL20.dtd">
  <smil>
    <head>
      <layout>
        <root-layout width="176" height="220" background-color="#ffffff"/>
        <region id="Image" top="0" left="0"
                height="160" width="176" fit="meet"/>
        <region id="Text"  top="160" left="0"
                height="60"  width="176"/>
      </layout>
    </head>
    <body>
      <par dur="5000ms">
        <img src="photo.jpg" region="Image"/>
        <text src="caption.txt" region="Text"/>
      </par>
    </body>
  </smil>
  ─────────────────────────────────────────────────────────────────────

  Key SMIL elements:
  ┌──────────────────┬─────────────────────────────────────────────────┐
  │ Element          │ Purpose                                         │
  ├──────────────────┼─────────────────────────────────────────────────┤
  │ <smil>           │ Root element                                    │
  │ <head>           │ Presentation metadata (layout)                  │
  │ <layout>         │ Defines display regions                         │
  │ <root-layout>    │ Overall dimensions of the MMS canvas            │
  │ <region>         │ A named rectangular area on the canvas          │
  │ <body>           │ The sequence of slides                          │
  │ <par>            │ One "parallel" slide — elements shown together  │
  │ <seq>            │ Sequential container (rarely used in MMS)       │
  │ <img>            │ Image part (references another MIME part)       │
  │ <text>           │ Text part (references another MIME part)        │
  │ <audio>          │ Audio part                                      │
  │ <video>          │ Video part                                      │
  └──────────────────┴─────────────────────────────────────────────────┘

  <par dur="5000ms">: This slide is shown for 5 seconds.
  src="photo.jpg": References the attachment named "photo.jpg".
                   This matches the Content-ID or Content-Location header
                   of another MIME part in the multipart body.

  Multi-slide SMIL (two slides):
  ─────────────────────────────────────────────────────────────────────
  <body>
    <par dur="3000ms">
      <img src="slide1.jpg" region="Image"/>
      <text src="text1.txt" region="Text"/>
    </par>
    <par dur="4000ms">
      <img src="slide2.jpg" region="Image"/>
      <text src="text2.txt" region="Text"/>
    </par>
  </body>
  ─────────────────────────────────────────────────────────────────────
  Slide 1 shows for 3 seconds, then Slide 2 shows for 4 seconds.
```

### MMS PDU Structure: Multipart Body

```
MMS PDU = Binary Headers + Multipart Body
══════════════════════════════════════════

  ┌─────────────────────────────────────────────────────────────────┐
  │                    MMS PDU (m-send-req)                         │
  │                                                                 │
  │  ┌─────────────────────────────────────────────────────────┐   │
  │  │               BINARY HEADERS                            │   │
  │  │  0x8C 0x80     X-Mms-Message-Type: m-send-req          │   │
  │  │  0x98 "t12345\0"  X-Mms-Transaction-Id: t12345         │   │
  │  │  0x8D 0x93     X-Mms-MMS-Version: 1.3                  │   │
  │  │  0x89 "+12125551234/TYPE=PLMN\0"  X-Mms-From           │   │
  │  │  0x97 "+442071234567/TYPE=PLMN\0" X-Mms-To             │   │
  │  │  0x97 "Check this out!\0"         X-Mms-Subject         │   │
  │  │  0x86 0x80     X-Mms-Delivery-Report: yes               │   │
  │  │  0x84 <content-type-encoding>     Content-Type          │   │
  │  └─────────────────────────────────────────────────────────┘   │
  │                                                                 │
  │  ┌─────────────────────────────────────────────────────────┐   │
  │  │               MULTIPART BODY                            │   │
  │  │                                                         │   │
  │  │  Number of parts: 3 (uintvar)                           │   │
  │  │                                                         │   │
  │  │  ┌─────────────────────────────────────────────────┐   │   │
  │  │  │  Part 0: SMIL presentation                      │   │   │
  │  │  │  Headers length: (uintvar)                      │   │   │
  │  │  │  Data length: (uintvar)                         │   │   │
  │  │  │  Content-Type: application/smil                 │   │   │
  │  │  │  Content-ID: <smil>                             │   │   │
  │  │  │  Data: <?xml ...><smil>...</smil>               │   │   │
  │  │  └─────────────────────────────────────────────────┘   │   │
  │  │                                                         │   │
  │  │  ┌─────────────────────────────────────────────────┐   │   │
  │  │  │  Part 1: JPEG image                             │   │   │
  │  │  │  Content-Type: image/jpeg                       │   │   │
  │  │  │  Content-ID: <photo.jpg>                        │   │   │
  │  │  │  Data: [binary JPEG bytes]                      │   │   │
  │  │  └─────────────────────────────────────────────────┘   │   │
  │  │                                                         │   │
  │  │  ┌─────────────────────────────────────────────────┐   │   │
  │  │  │  Part 2: Text caption                           │   │   │
  │  │  │  Content-Type: text/plain; charset=utf-8        │   │   │
  │  │  │  Content-ID: <caption.txt>                      │   │   │
  │  │  │  Data: "Look at this sunset!"                   │   │   │
  │  │  └─────────────────────────────────────────────────┘   │   │
  │  └─────────────────────────────────────────────────────────┘   │
  └─────────────────────────────────────────────────────────────────┘
```

#### Multipart Part Encoding

```
Each multipart part structure:
════════════════════════════════

  ┌──────────────────────────────────────────────────────────────────┐
  │ headers-length  (uintvar) — byte count of the headers section    │
  │ data-length     (uintvar) — byte count of the data section       │
  │ content-type   — encoded content type (short int or text)        │
  │ other-headers  — additional headers (Content-ID, Content-Location│
  │                  Content-Disposition, etc.) binary-encoded       │
  │ data           — raw bytes of the part content                   │
  └──────────────────────────────────────────────────────────────────┘

  Content-Type binary encoding for common MIME types:
  ┌──────────────┬──────────────────────────────────────────────────┐
  │ Short code   │ MIME type                                        │
  ├──────────────┼──────────────────────────────────────────────────┤
  │ 0x83         │ application/vnd.wap.multipart.related            │
  │ 0xA3         │ application/vnd.wap.multipart.mixed              │
  │ 0x9E         │ application/vnd.wap.mms-message                  │
  │ 0xB4         │ application/smil                                 │
  │ 0x24         │ image/jpeg                                       │
  │ 0x25         │ image/gif                                        │
  │ 0x1B         │ image/png (assigned by OMA)                      │
  │ 0x03         │ text/html                                        │
  │ 0x83         │ text/plain                                       │
  │ 0x15         │ audio/mpeg                                       │
  │ 0x22         │ video/3gpp                                       │
  └──────────────┴──────────────────────────────────────────────────┘

  If the MIME type is not in the table, it is sent as a text string.

  Content-ID and Content-Location:
    Content-ID is enclosed in angle brackets: <photo.jpg>
    Content-Location is a plain filename or URL: photo.jpg
    SMIL references parts by matching src="photo.jpg" to Content-ID
    "<photo.jpg>" or Content-Location "photo.jpg".

  Multipart type: multipart/related vs multipart/mixed
    multipart/related: parts are related to each other (SMIL + attachments)
                       The first part MUST be the SMIL presentation.
                       This is the standard for MMS with layout.
    multipart/mixed:   standalone attachments, no SMIL layout.
                       Used for simple image-only messages on some carriers.
```

### Complete PDU: m-send-req (Simple Image MMS)

```
Scenario: send a JPEG photo with caption "Hi!" from +12125551234 to +442071234567

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
BINARY HEADER SECTION
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

  8C 80        X-Mms-Message-Type: m-send-req (0x80)
               └── 0x8C = field code for X-Mms-Message-Type
               └── 0x80 = short integer 0 with high bit set = 0x80 = m-send-req

  98           X-Mms-Transaction-Id:
  74 31 32 33 34 35 00
               "t12345" null-terminated ASCII
               └── 0x98 = field code for Transaction-Id

  8D 93        X-Mms-MMS-Version: 1.3
               └── 0x8D = field code, 0x93 = short integer 0x13 | 0x80 = 1.3

  89           X-Mms-From:
  2B 31 32 31 32 35 35 35 31 32 33 34 2F 54 59 50 45 3D 50 4C 4D 4E 00
               "+12125551234/TYPE=PLMN\0"
               └── 0x89 = field code for From

  97           X-Mms-To:
  2B 34 34 32 30 37 31 32 33 34 35 36 37 2F 54 59 50 45 3D 50 4C 4D 4E 00
               "+442071234567/TYPE=PLMN\0"
               └── 0x97 = field code for To

  86 80        X-Mms-Delivery-Report: yes
               └── 0x86 = field code, 0x80 = short-integer "yes"

  84           Content-Type: (of the whole PDU body)
  BA 81 83     encoded value for application/vnd.wap.multipart.related
               with start="<smil>" type="application/smil"
               (complex encoding — see below)

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Content-Type encoding for multipart/related:
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

  When the Content-Type has parameters (like multipart/related with
  start= and type= parameters), the encoding becomes:

  Value-length (uintvar): total byte count of what follows
  Well-known-media (short int): 0x33 = multipart/related (0xB3 with high bit)
  Parameters:
    "start" parameter:
      0x1A  (well-known token for "start")
      3C 73 6D 69 6C 3E 00   "<smil>\0"
    "type" parameter:
      0x11  (well-known token for "type")
      0xB4  (short integer for application/smil)

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
MULTIPART BODY
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

  03              Number of parts = 3 (uintvar)

  ─── Part 0: SMIL ────────────────────────────────────────────────

  SMIL content (UTF-8 text):
  │ <?xml version="1.0"?>
  │ <smil><head><layout>
  │   <root-layout width="176" height="220"/>
  │   <region id="Image" top="0" left="0" height="160" width="176"/>
  │   <region id="Text" top="160" left="0" height="60" width="176"/>
  │ </layout></head>
  │ <body><par dur="5000ms">
  │   <img src="photo.jpg" region="Image"/>
  │   <text src="caption.txt" region="Text"/>
  │ </par></body></smil>

  Let SMIL byte count = S (approx 280 bytes)

  Part 0 wire encoding:
  [headers-length: uintvar]  ← byte count of headers that follow
  [data-length:    uintvar]  ← = S
  B4                          Content-Type: application/smil (short int)
  98                          Content-ID header field code (0x98... actually
                              in part headers, field codes differ from PDU
                              headers — use Content-ID = 0xC0 in WSP)
  3C 73 6D 69 6C 3E 00       "<smil>\0"   Content-ID value
  [SMIL bytes]                the actual SMIL XML

  ─── Part 1: JPEG image ──────────────────────────────────────────

  Assume JPEG is 12,000 bytes (tiny thumbnail).

  [headers-length: uintvar]
  [data-length: 0x82 0xEE 0x48]   uintvar encoding of 12000
  24                               Content-Type: image/jpeg (short int)
  [Content-ID field code]
  70 68 6F 74 6F 2E 6A 70 67 00   "photo.jpg\0"
  [12000 bytes of JPEG data]

  ─── Part 2: Text caption ────────────────────────────────────────

  Caption text: "Hi!" (3 bytes)

  [headers-length: uintvar]
  03                               data-length = 3
  83                               Content-Type: text/plain (short int)
  ...charset parameter...          (text/plain; charset=utf-8)
  [Content-ID field code]
  63 61 70 74 69 6F 6E 2E 74 78 74 00   "caption.txt\0"
  48 69 21                         "Hi!" in UTF-8

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
WAP Push notification bytes (binary SMS carrying m-notification-ind):
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

  Outer binary SMS:
    DCS = 0xF5 (8-bit data, WAP Push port addressing)
    TP destination port: 2948 (0x0B84) — the WAP Push port
    TP source port: 2948

  WSP Push PDU (the SMS payload):
  00        Transaction ID = 0
  06        WSP PDU type = Push
  01        Headers-Length = 1 (one byte of headers follows)
  AE        Content-Type: application/vnd.wap.mms-message (0x2E | 0x80)

  m-notification-ind binary PDU (the WSP body):
  8C 82     X-Mms-Message-Type: m-notification-ind (0x82)
  98        X-Mms-Transaction-Id:
  6E 31 00  "n1\0"
  8D 93     X-Mms-MMS-Version: 1.3
  8E        X-Mms-Message-Size:
  03        3 bytes follow
  00 2E E0  = 12000 decimal (the MMS total size)
  88        X-Mms-Expiry:
  04        4 bytes follow
  67 FB DE 00  Unix timestamp of expiry (seconds since epoch)
  83        X-Mms-Content-Location:
  68 74 74 70 3A 2F 2F 6D 6D 73 63 2E 61 74 74 2E 63 6F 6D 2F 72 65 74
  72 69 65 76 65 3F 69 64 3D 61 62 63 31 32 33 00
            "http://mmsc.att.com/retrieve?id=abc123\0"
```

### MM4: Inter-carrier SMTP Relay

When Alice (T-Mobile US) sends an MMS to Bob (AT&T US), T-Mobile's MMSC cannot
directly deliver to Bob — it must relay to AT&T's MMSC. MM4 is the inter-MMSC
protocol. Remarkably, it is built on top of SMTP (email).

```
MM4 inter-carrier flow:
════════════════════════

  MMSC A (T-Mobile)           MMSC B (AT&T)
      │                            │
      │  SMTP EHLO mm4.t-mobile.com│
      ├───────────────────────────►│
      │  220 OK                    │
      │◄───────────────────────────┤
      │                            │
      │  MAIL FROM: <mmsc@t-mob>   │
      │  RCPT TO: <mmsc@att.com>   │
      │  DATA                      │
      ├───────────────────────────►│
      │                            │
      │  [MM4_Forward.REQ email]   │
      ├───────────────────────────►│
      │  250 Message accepted      │
      │◄───────────────────────────┤
      │                            │
      │  [MMSC B delivers to Bob]  │
      │                            │
      │  [MM4_Forward.RES email]   │
      │◄───────────────────────────┤
      │  (acknowledgment)          │

  MM4 email structure (RFC 2822 message):
  ─────────────────────────────────────────
  From: mmsc@t-mobile.com
  To: mmsc@att.com
  Message-ID: <mm4-req-abc123@t-mobile.com>
  MIME-Version: 1.0
  X-Mms-3GPP-MMS-Version: 6.10.0
  X-Mms-Message-Type: MM4_Forward.REQ
  X-Mms-Transaction-Id: t12345
  X-Mms-Message-Id: 20260422153000-abc123
  X-Mms-Originator-Address: +12125551234/TYPE=PLMN
  X-Mms-Recipient-Address: +442071234567/TYPE=PLMN
  X-Mms-Ack-Request: Yes
  Content-Type: application/vnd.3gpp.mms-message; boundary="boundary123"

  --boundary123
  Content-Type: application/smil
  Content-ID: <smil>

  <?xml version="1.0"?>
  <smil>...</smil>

  --boundary123
  Content-Type: image/jpeg
  Content-ID: <photo.jpg>
  Content-Transfer-Encoding: base64

  /9j/4AAQSkZJRgABAQAAAQABAAD/2wBDA...
  [base64-encoded JPEG data]
  --boundary123--

  Note: MM4 uses MIME multipart (like email attachments) with base64 encoding,
  NOT the binary WSP multipart encoding used in MM1. SMTP was designed for
  text, so binary data must be base64-encoded.

  MM4_Forward.RES (acknowledgment back from MMSC B to MMSC A):
  ─────────────────────────────────────────────────────────────
  From: mmsc@att.com
  To: mmsc@t-mobile.com
  In-Reply-To: <mm4-req-abc123@t-mobile.com>
  X-Mms-Message-Type: MM4_Forward.RES
  X-Mms-Transaction-Id: t12345
  X-Mms-Request-Status-Code: Ok
```

### Size Limits and Media Resizing

```
Carrier-imposed size limits (approximate, varies by carrier and year):
════════════════════════════════════════════════════════════════════════

  ┌──────────────────────┬────────────────────────────────────────────┐
  │ Carrier              │ Typical MMS size limit                     │
  ├──────────────────────┼────────────────────────────────────────────┤
  │ AT&T (US)            │ 1 MB (1,048,576 bytes)                     │
  │ Verizon (US)         │ 1.2 MB                                     │
  │ T-Mobile (US)        │ 1 MB                                       │
  │ EE (UK)              │ 300 KB                                      │
  │ Vodafone (UK)        │ 300 KB                                      │
  │ Older carriers       │ 100 KB (early 2000s)                       │
  └──────────────────────┴────────────────────────────────────────────┘

  These limits are enforced by the MMSC. If your m-send-req PDU exceeds
  the limit, the MMSC returns an error response-status in m-send-conf.

  How Android handles oversized images:
  ─────────────────────────────────────
  1. User selects 4000×3000 pixel photo (8 MB JPEG)
  2. MmsService checks configured carrier limit (e.g., 1 MB)
  3. MmsService scales the image down iteratively:
     a. Try quality=90% → still too big
     b. Try quality=80%, scale to 1600×1200 → measure size
     c. If still too big, try quality=70%, scale to 1024×768
     d. Continue until under limit
  4. Attach the resized image to the MMS PDU

  Audio/Video limits:
  ─────────────────────────────────────
  - Audio: AMR-NB (Adaptive Multi-Rate Narrowband) at 4.75 kbps
    = ~34 KB per minute. Maximum typical: ~15-20 seconds.
  - Video: H.263 baseline or 3GPP MPEG-4 at 15fps, 176×144 (QCIF)
    typically limited to 10-15 seconds at low bitrate.

  Image format support (minimum requirement from OMA):
  - image/jpeg  — required
  - image/gif   — required (including animated)
  - image/png   — required in MMS 1.3
  - image/bmp   — optional
  - image/wbmp  — optional (WAP Bitmap, monochrome, very old)
```

### Delivery and Read Reports

```
Delivery report flow:
══════════════════════

  When sender sets X-Mms-Delivery-Report: yes in m-send-req:

  1. Recipient phone sends m-notifyresp-ind with status=Retrieved
     after successfully downloading the MMS.

  2. MMSC B forwards this as an m-delivery-ind back to MMSC A:
     X-Mms-Message-Type: m-delivery-ind
     X-Mms-Message-Id: (matches original message-id)
     X-Mms-To: +12125551234/TYPE=PLMN  (original sender)
     X-Mms-Date: (delivery timestamp)
     X-Mms-Status: Retrieved

  3. MMSC A delivers m-delivery-ind to sender's phone via WAP Push
     or piggybacks it in next contact.

  Status values in m-delivery-ind:
  ┌────────────────────┬────────────────────────────────────────────┐
  │ Value              │ Meaning                                    │
  ├────────────────────┼────────────────────────────────────────────┤
  │ Expired            │ Recipient didn't fetch before expiry       │
  │ Retrieved          │ Successfully downloaded by recipient       │
  │ Rejected           │ Recipient rejected (e.g., content filter)  │
  │ Deferred           │ Recipient deferred (manual download off)   │
  │ Unrecognized       │ Recipient doesn't understand MMS           │
  │ Indeterminate      │ Status unknown                             │
  │ Forwarded          │ Message was forwarded to another party     │
  │ Unreachable        │ Recipient not reachable                    │
  └────────────────────┴────────────────────────────────────────────┘

Read report flow:
══════════════════

  When sender sets X-Mms-Read-Report: yes in m-send-req:

  1. Recipient phone, when user actually opens/reads the MMS, sends
     an m-read-rec-ind PDU to the MMSC.
  2. MMSC relays it as m-read-orig-ind to original sender's MMSC.
  3. Sender's phone receives it and can mark the message as "Read."

  Note: Many phones and carriers do NOT implement read reports.
  It is commonly disabled or ignored in practice.
```

## Algorithms

### Algorithm 1: Encode Binary MMS Header

```
function encode_mms_header(field_code: int, value) → bytes:
    result = bytearray()
    result.append(field_code | 0x80)   # field code always has high bit set

    if isinstance(value, int) and value <= 127:
        # Short integer encoding
        result.append(value | 0x80)
    elif isinstance(value, int):
        # Long integer encoding
        raw = value.to_bytes(max(1, (value.bit_length() + 7) // 8), 'big')
        result.append(len(raw))   # length byte
        result.extend(raw)
    elif isinstance(value, str):
        # Null-terminated text string
        result.extend(value.encode('ascii'))
        result.append(0x00)
    elif isinstance(value, bytes):
        # Raw bytes with length prefix
        result.append(len(value))
        result.extend(value)

    return bytes(result)


function encode_uintvar(value: int) → bytes:
    """Variable-length integer: 7 bits per byte, MSB = continuation."""
    if value == 0:
        return bytes([0x00])
    result = []
    while value > 0:
        result.append(value & 0x7F)
        value >>= 7
    result.reverse()
    # Set continuation bit on all bytes except the last
    for i in range(len(result) - 1):
        result[i] |= 0x80
    return bytes(result)


function decode_uintvar(data: bytes, offset: int) → (int, int):
    """Returns (value, new_offset)."""
    value = 0
    while True:
        byte = data[offset]
        offset += 1
        value = (value << 7) | (byte & 0x7F)
        if not (byte & 0x80):
            break
    return value, offset
```

### Algorithm 2: Build m-send-req PDU

```
function build_m_send_req(
    sender:      str,         # "+12125551234"
    recipient:   str,         # "+442071234567"
    subject:     str,
    smil:        str,         # SMIL XML string
    attachments: list[Attachment],  # each has content_type, content_id, data
    transaction_id: str = None,
    request_delivery_report: bool = True,
) → bytes:

    if transaction_id is None:
        transaction_id = "t" + str(random.randint(100000, 999999))

    # Build SMIL part
    smil_bytes = smil.encode('utf-8')
    smil_part  = encode_part(
        content_type='application/smil',
        content_id='<smil>',
        data=smil_bytes,
    )

    # Build attachment parts
    att_parts = [
        encode_part(a.content_type, a.content_id, a.data)
        for a in attachments
    ]

    all_parts = [smil_part] + att_parts

    # Build body: number-of-parts + concatenated part bytes
    body = encode_uintvar(len(all_parts))
    for part in all_parts:
        body += part

    # Content-Type for the PDU = multipart/related with parameters
    content_type_field = encode_multipart_related_content_type(
        start='<smil>',
        type='application/smil',
    )

    # Build headers
    headers = bytearray()
    headers += encode_mms_header(0x0C, 0x80)        # Message-Type: m-send-req
    headers += encode_mms_header(0x18, transaction_id)  # Transaction-Id
    headers += encode_mms_header(0x0D, 0x93)        # MMS-Version: 1.3
    headers += encode_mms_header(0x09, sender + "/TYPE=PLMN")   # From
    headers += encode_mms_header(0x17, recipient + "/TYPE=PLMN") # To
    if subject:
        headers += encode_mms_header(0x17, subject) # Subject (wrong code, see note)
    if request_delivery_report:
        headers += encode_mms_header(0x06, 0x80)    # Delivery-Report: yes
    headers += content_type_field                   # Content-Type

    return bytes(headers) + bytes(body)


function encode_part(content_type: str, content_id: str, data: bytes) → bytes:
    # Encode the part headers
    part_headers = bytearray()

    # Content-Type header (field 0x04 in part context)
    if content_type in CONTENT_TYPE_SHORT_CODES:
        ct_bytes = bytes([CONTENT_TYPE_SHORT_CODES[content_type] | 0x80])
    else:
        ct_bytes = content_type.encode('ascii') + b'\x00'
    part_headers.extend(ct_bytes)

    # Content-ID header
    part_headers.append(0xC0)   # well-known header: Content-ID
    part_headers.extend(content_id.encode('ascii') + b'\x00')

    # Now build the full part
    result = bytearray()
    result.extend(encode_uintvar(len(part_headers)))   # headers-length
    result.extend(encode_uintvar(len(data)))            # data-length
    result.extend(part_headers)
    result.extend(data)
    return bytes(result)
```

### Algorithm 3: Decode Incoming MMS PDU

```
function decode_mms_pdu(data: bytes) → MmsPdu:
    offset = 0
    headers = {}

    # Parse binary headers until we hit Content-Type
    while offset < len(data):
        field_code = data[offset]
        offset += 1

        if field_code < 0x80:
            # Unknown or text header — skip (shouldn't happen in well-formed PDU)
            continue

        field_code &= 0x7F   # strip high bit to get logical field number

        # Parse value based on next byte
        value, offset = decode_mms_value(data, offset)
        headers[field_code] = value

        # Content-Type signals end of headers, body follows
        if field_code == 0x04:   # Content-Type
            break

    # Parse multipart body
    num_parts, offset = decode_uintvar(data, offset)
    parts = []
    for _ in range(num_parts):
        headers_len, offset = decode_uintvar(data, offset)
        data_len,    offset = decode_uintvar(data, offset)
        part_headers_end = offset + headers_len
        part_data_end    = part_headers_end + data_len

        part_headers = decode_part_headers(data[offset:part_headers_end])
        part_data    = data[part_headers_end:part_data_end]

        parts.append(MmsPart(headers=part_headers, data=part_data))
        offset = part_data_end

    return MmsPdu(headers=headers, parts=parts)


function decode_mms_value(data: bytes, offset: int) → (Any, int):
    byte = data[offset]
    if byte >= 0x80:
        # Short integer: value is (byte & 0x7F)
        return byte & 0x7F, offset + 1
    elif byte < 0x20:
        # Integer: the byte itself is the value
        return byte, offset + 1
    elif byte == 0x1F:
        # Value-length encoded: next is uintvar length, then bytes
        length, offset = decode_uintvar(data, offset + 1)
        value = int.from_bytes(data[offset:offset+length], 'big')
        return value, offset + length
    else:
        # Text string: null-terminated
        end = data.index(0x00, offset)
        value = data[offset:end].decode('ascii', errors='replace')
        return value, end + 1
```

### Algorithm 4: WAP Push Parser

```
function parse_wap_push(sms_ud: bytes) → WapPushPdu:
    """Parse the User Data of a WAP Push binary SMS."""
    offset = 0

    # WSP Push header
    transaction_id = sms_ud[offset]; offset += 1
    pdu_type       = sms_ud[offset]; offset += 1

    if pdu_type != 0x06:
        raise ValueError(f"Expected WSP Push (0x06), got {pdu_type:#x}")

    # Headers length
    headers_length, offset = decode_uintvar(sms_ud, offset)
    headers_end = offset + headers_length

    # Content-Type (first header)
    content_type, offset = decode_wsp_content_type(sms_ud, offset)

    # Additional headers (X-WAP-Application-ID, etc.)
    wap_headers = {}
    while offset < headers_end:
        field, value, offset = decode_wsp_header(sms_ud, offset)
        wap_headers[field] = value

    # Body = the MMS notification PDU
    body = sms_ud[headers_end:]

    # Parse the body as an MMS PDU (m-notification-ind)
    mms = decode_mms_pdu(body)

    return WapPushPdu(
        content_type=content_type,
        wap_headers=wap_headers,
        mms=mms,
    )


function extract_content_location(notification: MmsPdu) → str:
    """Get the URL to fetch the MMS from."""
    # X-Mms-Content-Location is field code 0x03
    location = notification.headers.get(0x03)
    if not location:
        raise ValueError("No Content-Location in MMS notification")
    return location   # URL like "http://mmsc.att.com/retrieve?id=abc123"
```

## Test Strategy

### Unit Tests: uintvar Encoding

```
test "encode 0":
    assert encode_uintvar(0) == bytes([0x00])

test "encode 127 (single byte max)":
    assert encode_uintvar(127) == bytes([0x7F])

test "encode 128 (two bytes needed)":
    assert encode_uintvar(128) == bytes([0x81, 0x00])

test "encode 300":
    assert encode_uintvar(300) == bytes([0x82, 0x2C])

test "encode 2097151 (max 3-byte value)":
    assert encode_uintvar(2097151) == bytes([0xFF, 0xFF, 0x7F])

test "decode round-trip":
    for v in [0, 1, 127, 128, 255, 300, 16383, 16384, 100000]:
        encoded = encode_uintvar(v)
        decoded, offset = decode_uintvar(encoded, 0)
        assert decoded == v
        assert offset == len(encoded)
```

### Unit Tests: Binary Header Encoding

```
test "short integer encoding":
    # MMS version 1.3 = 0x93
    result = encode_mms_header(0x8D, 0x13)   # field=0x8D, value=0x13
    assert result == bytes([0x8D, 0x93])

test "message type m-send-req":
    result = encode_mms_header(0x8C, 0x00)   # field=m-type, value=m-send-req
    assert result == bytes([0x8C, 0x80])

test "text string encoding":
    result = encode_mms_header(0x98, "t12345")
    assert result == bytes([0x98]) + b"t12345\x00"

test "long integer encoding for size":
    # 45000 bytes = 0x0000AFB8
    result = encode_mms_header(0x8E, 45000)
    # Should be: field byte, then 0x03 (3 bytes), then 0x00 0xAF 0xB8
    # Wait: 45000 = 0xAFB8 which is only 2 bytes. Let's check:
    # 45000 in hex: 45000 / 256 = 175.78... → 0xAFB8
    # 0xAFB8 = 44984... no. 45000 = 0xAFC8
    assert result[1] == 0x02   # 2 bytes follow
    assert result[2] == 0xAF
    assert result[3] == 0xC8

test "decode encoded header":
    encoded = encode_mms_header(0x98, "t12345")
    field, value, offset = decode_mms_header(encoded, 0)
    assert field == 0x18   # 0x98 & 0x7F = 0x18
    assert value == "t12345"
```

### Unit Tests: Multipart Part Encoding

```
test "SMIL part round-trip":
    smil = b"<smil><body><par/></body></smil>"
    encoded = encode_part("application/smil", "<smil>", smil)
    decoded  = decode_part(encoded)
    assert decoded.content_type == "application/smil"
    assert decoded.content_id   == "<smil>"
    assert decoded.data         == smil

test "JPEG part preserves binary data":
    jpeg = bytes(range(256)) * 50   # 12,800 bytes of fake JPEG
    encoded = encode_part("image/jpeg", "<photo.jpg>", jpeg)
    decoded  = decode_part(encoded)
    assert decoded.data == jpeg

test "headers-length field is accurate":
    smil = b"<smil/>"
    encoded = encode_part("application/smil", "<smil>", smil)
    headers_len, off = decode_uintvar(encoded, 0)
    data_len,    off = decode_uintvar(encoded, off)
    # Part headers end at off + headers_len, data follows
    actual_data = encoded[off + headers_len : off + headers_len + data_len]
    assert actual_data == smil

test "zero-byte text part":
    encoded = encode_part("text/plain", "<t.txt>", b"")
    decoded  = decode_part(encoded)
    assert decoded.data == b""
```

### Unit Tests: Full PDU Assembly

```
test "m-send-req contains required headers":
    pdu = build_m_send_req(
        sender="+12125551234",
        recipient="+442071234567",
        subject="Test",
        smil="<smil/>",
        attachments=[],
    )
    decoded = decode_mms_pdu(pdu)
    assert decoded.headers.get(MESSAGE_TYPE) == M_SEND_REQ
    assert decoded.headers.get(MMS_VERSION)  == 0x13
    assert decoded.headers.get(FROM)         == "+12125551234/TYPE=PLMN"
    assert decoded.headers.get(TO)           == "+442071234567/TYPE=PLMN"

test "number of parts correct":
    attachments = [
        Attachment("image/jpeg", "<photo.jpg>", b"\xFF\xD8" + b"\x00" * 1000),
        Attachment("text/plain", "<cap.txt>", b"Hi!"),
    ]
    pdu = build_m_send_req(
        sender="+12125551234",
        recipient="+442071234567",
        subject="",
        smil="<smil/>",
        attachments=attachments,
    )
    decoded = decode_mms_pdu(pdu)
    assert len(decoded.parts) == 3   # SMIL + 2 attachments

test "SMIL is first part":
    pdu     = build_m_send_req("+1555", "+1444", "", "<smil/>", [])
    decoded = decode_mms_pdu(pdu)
    assert decoded.parts[0].content_type == "application/smil"

test "round-trip PDU encode/decode":
    original_smil = "<smil><body><par dur='3s'><img src='x.jpg'/></par></body></smil>"
    jpeg_data     = b"\xFF\xD8" + b"\xAB" * 5000
    pdu = build_m_send_req(
        sender="+12125551234",
        recipient="+442071234567",
        subject="Sunset",
        smil=original_smil,
        attachments=[Attachment("image/jpeg", "<x.jpg>", jpeg_data)],
    )
    decoded = decode_mms_pdu(pdu)
    assert decoded.parts[0].data.decode('utf-8') == original_smil
    assert decoded.parts[1].data == jpeg_data
```

### Unit Tests: WAP Push Parsing

```
test "parse WAP Push extracts content-location":
    # Build a synthetic WAP Push notification
    notification_pdu = build_m_notification_ind(
        transaction_id="n1",
        message_size=45000,
        content_location="http://mmsc.att.com/retrieve?id=abc123",
        expiry=1783800000,
    )
    wap_push = build_wap_push(notification_pdu)

    # Parse it
    parsed = parse_wap_push(wap_push)
    url = extract_content_location(parsed.mms)
    assert url == "http://mmsc.att.com/retrieve?id=abc123"

test "WAP Push transaction ID byte is 0x00":
    notification_pdu = build_m_notification_ind(
        transaction_id="test",
        message_size=1000,
        content_location="http://example.com/mms",
        expiry=1783800000,
    )
    wap_push = build_wap_push(notification_pdu)
    assert wap_push[0] == 0x00   # transaction ID
    assert wap_push[1] == 0x06   # PDU type = Push

test "WSP content type byte 0xAE":
    notification_pdu = build_m_notification_ind("x", 100, "http://x.com", 0)
    wap_push = build_wap_push(notification_pdu)
    # Headers-length = 1, then Content-Type byte
    headers_len, off = decode_uintvar(wap_push, 2)
    assert headers_len == 1
    assert wap_push[off] == 0xAE   # application/vnd.wap.mms-message
```

### Integration Tests: HTTP Send/Receive

```
test "HTTP POST to MMSC has correct Content-Type":
    pdu = build_m_send_req("+1555", "+1444", "", "<smil/>", [])
    request = build_http_request(
        method="POST",
        url="http://mmsc.carrier.com",
        body=pdu,
    )
    assert request.headers["Content-Type"] == "application/vnd.wap.mms-message"
    assert request.headers["Content-Length"] == str(len(pdu))

test "receive and decode m-send-conf":
    # Simulate MMSC response
    conf = build_m_send_conf(
        transaction_id="t12345",
        response_status=0x80,   # Ok
        message_id="20260422153000-abc123",
    )
    decoded = decode_mms_pdu(conf)
    assert decoded.headers.get(MESSAGE_TYPE)    == M_SEND_CONF
    assert decoded.headers.get(RESPONSE_STATUS) == 0x00   # Ok
    assert decoded.headers.get(MESSAGE_ID)      == "20260422153000-abc123"

test "fetch MMS with HTTP GET":
    url = "http://mmsc.att.com/retrieve?id=abc123"
    # Verify the GET request is well-formed for a carrier MMSC
    request = build_fetch_request(url)
    assert request.method == "GET"
    assert request.headers["Accept"] == "*/*, application/vnd.wap.mms-message"
    # User-Agent should identify the device
    assert "MmsService" in request.headers["User-Agent"]
```

### Edge Cases and Error Handling

```
test "oversized PDU rejected":
    # 2 MB JPEG — exceeds 1 MB carrier limit
    huge_jpeg = b"\xFF\xD8" + b"\x00" * (2 * 1024 * 1024)
    with assert_raises(MmsSizeExceededError):
        build_m_send_req("+1555", "+1444", "", "<smil/>",
                         [Attachment("image/jpeg", "<huge.jpg>", huge_jpeg)],
                         max_size=1024*1024)

test "recipient with no data capability":
    # Carrier returns error response-status in m-send-conf
    error_conf = build_m_send_conf(
        transaction_id="t12345",
        response_status=0x82,   # Unable-to-resolve-recipient
        message_id="",
    )
    decoded = decode_mms_pdu(error_conf)
    assert is_error_status(decoded.headers[RESPONSE_STATUS])

test "expired notification (past expiry)":
    conf = build_m_notification_ind(
        transaction_id="old",
        message_size=1000,
        content_location="http://mmsc.att.com/retrieve?id=expired",
        expiry=1000,   # Unix timestamp in the distant past
    )
    parsed = parse_wap_push(build_wap_push(conf))
    expiry = parsed.mms.headers.get(EXPIRY)
    assert expiry < current_unix_time()   # message is expired

test "SMIL with two slides":
    smil = '''<smil><body>
      <par dur="3s"><img src="a.jpg" region="r"/></par>
      <par dur="3s"><img src="b.jpg" region="r"/></par>
    </body></smil>'''
    slides = parse_smil_slides(smil)
    assert len(slides) == 2
    assert slides[0].duration == 3000   # milliseconds
    assert slides[0].image_src == "a.jpg"
    assert slides[1].image_src == "b.jpg"

test "uintvar edge: value requiring 4 bytes":
    v = 2097152   # 0x200000 — first value needing 4 bytes
    encoded = encode_uintvar(v)
    assert len(encoded) == 4
    decoded, _ = decode_uintvar(encoded, 0)
    assert decoded == v
```
