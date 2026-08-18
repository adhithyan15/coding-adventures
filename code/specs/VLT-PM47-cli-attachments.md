# VLT-PM47 — Chunked Encrypted Attachments

## Status

Normative Phase 1B contract for storing a file inside a vault, listing what an
item carries, and writing one attachment back out to a byte-identical plaintext
file.

`VLT-PM00-local-first-password-manager.md` §23 item 11 bundles four daily-use
conveniences — "password generator, TOTP display, clipboard, attachments and
packing". `VLT-PM44-cli-password-generate.md` shipped the generator,
`VLT-PM45-cli-totp-code.md` the TOTP display, and
`VLT-PM46-cli-clipboard.md` the clipboard. This document is the fourth and
last: **attachments**. It closes item 11 for the local product. Item 11's
trailing word, *packing*, is a provider-adapter concern that both §10.7 and
§13.5 of VLT-PM00 place in the first cloud phase, and §2.4 below states why
that is not a gap in this slice.

Depends on: VLT-PM00 §6, §8.1, §9, §9.1, §10.2, §10.4, §10.7, §14.4, §14.6,
§19.4; VLT02; VLT14; VLT-PM03; VLT-PM05 §6, §7.2, §13; VLT-PM08; VLT-PM15 §2,
§5; VLT-PM25; VLT-PM41; VLT-PM42.

## 1. Why this slice exists, and what it costs to get wrong

Every other thing a vault-pm item holds is a *field*: a password, a note body, a
card number, a TOTP seed. Fields are small, and the product's whole storage
design is shaped by that assumption — one item revision is one canonical CBOR
map, sealed into one AEAD frame, addressed by one hash. An attachment breaks the
assumption. A scanned recovery sheet, a `.pem`, an exported authenticator
backup: these are the things a person actually needs beside a credential, and
none of them is a field.

The temptation is to treat a file as a very large field and put its bytes in the
revision. That does not work, and the reason it does not work is the single most
expensive thing this campaign has already learned. §3 states it in full.

The second temptation is to invent a resumable-upload protocol, because "large
write" sounds like it needs one. That also does not work — not because it is too
hard, but because it would be a *second* durable-write ceremony beside the one
VLT-PM41 swept exhaustively and VLT-PM42 taught to finish itself. §5 states why
an attachment write is one ordinary mutation and must remain one.

## 2. Scope

### 2.1 What ships

VLT-PM00 §14.4 already names the grammar, and this slice implements three of its
four lines:

```text
vault-pm [--vault NAME] attachment add ITEM PATH
vault-pm [--vault NAME] attachment list ITEM
vault-pm [--vault NAME] attachment export ITEM ATTACHMENT PATH
```

The verb is the top-level `attachment` of §14.4, not an `item attach`
sub-verb. The command table in that section is the product's own published
grammar and predates this document; a slice that shipped a different spelling
would leave §14.4 wrong about a feature that exists, which is the exact
condition VLT-PM46 §1 called "a product whose documentation is wrong".

One deviation from §14.4, stated so the table can be corrected rather than
quietly contradicted: **the destination path on `attachment export` is
required, not optional.** §14.4 renders it `[PATH]`, which implies a default —
and the only available default is the stored attachment name, resolved against
the process working directory. §4.6 explains why a name authored by a
synchronising peer must never reach a filesystem path resolver, and a defaulted
destination is precisely that. §14.4's optional bracket is removed.

### 2.2 What is deferred, and why

- **`attachment remove`.** §14.4's fourth line. Removing an `AttachmentId` from
  the OR-set is trivial — VLT-PM03's `ObservedSet` already supports it and
  VLT-PM00 §9.1 already merges it. The bytes are the problem: chunk and manifest
  objects are immutable repository objects, so a `remove` deletes a reference and
  leaves every byte of the file in the store until a garbage collection that
  VLT-PM00 §19.4 specifies and this product has not built. A person who removes a
  scanned passport from a password manager and is told "removed" has been told
  something false. Shipping the reference deletion without the sweep would
  manufacture that sentence. `attachment remove` lands with `gc run`.
- **Packing.** See §2.4.
- **Compression.** An attachment ciphertext's length is provider-visible
  (VLT-PM00 §7 concedes sizes). Compressing before sealing would make the
  *compressibility* of the plaintext provider-visible too, which is a strictly
  larger leak than its size, and the classic form of that mistake has its own
  literature. If compression ever ships it needs its own analysis, not a flag.
- **Cross-item deduplication.** A content-derived blob identity would let the
  store see that two items hold the same file. VLT14's own API documentation
  names "content-derived `blob_id` + deterministic `dek`" as the unsafe
  combination; the safe variants all re-encrypt, so dedup buys nothing at this
  layer and costs a metadata channel.
- **Streaming export.** §4.7 buffers one whole attachment in memory. The ceiling
  in §3.4 is what makes that defensible, and §8.2 records what a streaming host
  write would have to add.
- **Attachments on portable export/import.** VLT-PM17/VLT-PM18's snapshot
  format carries records, not blobs. An attachment is not lost — it stays in
  the source vault — but it does not travel, and **`export` says so** rather
  than leaving the operator to discover it at restore time. §8.3 records the
  shape of the change and §6.6 the notice.

### 2.3 What is *not* reused, and why the reuse map still holds

VLT-PM43 established that `vault-pm` does not build on `vault-sealed-store` or
`vault-key-custody`; it has its own envelope in `vault-pm-format` and its own
codec in `vault-pm-application`. That remains true and this slice does not
change it.

It does not follow that VLT14 is irrelevant, and two statements in VLT-PM00
settle the question directly. §6's reuse map assigns "attachments →
`vault-attachments` → chunk AEAD". §8.1 says item bodies and attachments
"retain their own VLT01/VLT14 DEKs **beneath this repository envelope**". Those
are not a suggestion to reimplement VLT14's framing; they are an instruction to
nest it. So this slice reuses `coding_adventures_vault_attachments` exactly as
VLT-PM45 reused `vault-auth`'s RFC 6238 engine: the cryptographic core is
imported, and everything about *where the bytes live* is vault-pm's own.

### 2.4 "Packing" is a backend word, and it is not due yet

Item 11 says "attachments and packing". Read on its own that phrase looks like
two features. VLT-PM00 uses the word in exactly two places and both are about
providers:

- §10.7: "A backend **may** pack many small objects into a larger immutable blob
  when individual-object overhead is material… Attachment chunks may use packs
  **in the first cloud phase**."
- §13.5: the Google Drive adapter "must not create one provider file per chunk",
  and the pack layer is "provider-neutral" but sized by Drive's constraints.

Packing is therefore a property of a storage adapter, not of the attachment
format: the logical contract stays "one immutable value per object ID" (§10.7's
first sentence) whether or not a backend groups those values on the wire. The
only adapter this product has is `storage-fs`, where 64 KiB files carry no
per-object overhead worth a layer, and the phase that introduces the adapter
that does need one is Phase 2 item 15.

This slice therefore ships attachments and states packing's absence as a
scheduled Phase 2 concern rather than an omission. §8.1 records what the pack
layer must not be allowed to change about §4 when it arrives.

## 3. The size ceiling is the design

### 3.1 Two ceilings, and the one that binds

`vault-pm-application` refuses any sealed plaintext over `MAX_PLAINTEXT_BYTES`
= **16 MiB**. `canonical-cbor` independently refuses to *emit* any single CBOR
value over `MAX_ENCODED_SIZE` = **1 MiB**. VLT-PM05 §13.1 states the
consequence: the looser gate is ours, the tighter gate is the codec's, and the
range between them is a set of values that are legal to hold, legal to decode,
and illegal to re-encode.

That gap is not theoretical. It cost this campaign two changes — #11945 and
#11931 — to convert six `try_encode(...).expect(...)` call sites into closed
errors, after a 1 MiB password was shown to abort the process on every later
command against the same vault. The catalog encode reached the ceiling at under
twenty thousand ordinary items with no hostile peer involved at all.

**So the binding number for anything an attachment touches is 1 MiB, and it is a
per-CBOR-value number.** A design that puts a file in one value is a design
whose maximum attachment is under a megabyte and whose failure mode above that
is the bug class this product has already paid for twice.

### 3.2 Why not one big object

The 16 MiB frame bound is not the constraint people expect it to be. Three
independent things sit between a file and a sealed frame:

1. the record encode, which is CBOR and therefore capped at 1 MiB;
2. the **revision framing** around the record — VLT-PM05 §13.1's second failure
   point, which can exceed the ceiling even when the record cleared it; and
3. the catalog re-encode, which every mutation performs.

A 4 MiB attachment stored as a field would clear none of them. Raising
`MAX_ENCODED_SIZE` is not available either: it is a denial-of-service bound on a
deliberately zero-dependency codec that every vault-pm object passes through,
and loosening it to accommodate attachments would loosen it for a peer-authored
catalog too.

### 3.3 Chunks, and why 64 KiB

The attachment plaintext is split into fixed 64 KiB pieces. Each piece is sealed
by VLT14's chunk AEAD, serialised as one small CBOR map, and sealed *again* as
one ordinary vault-pm repository object (§4.2 explains why both layers earn
their place). One chunk object's plaintext is therefore:

```text
64 KiB ciphertext + 16-byte tag + 16-byte blob id + index + final flag
+ version + kind + CBOR framing        ≈  65,600 bytes
```

against `MAX_ENCODED_SIZE` = 1,048,576. **Sixteen times the headroom, on the
one bound that binds.** The value is a constant of the format, not a function
of the file, so no attachment of any permitted size can move it.

64 KiB is VLT14's `CHUNK_SIZE` and is not re-litigated here. It is age v1's
chunk size, it bounds working-set memory on a phone, and adopting a different
one would mean either forking VLT14 or passing a size VLT14's encryptor rejects.

### 3.4 The ceilings this slice imposes

| Bound | Value | Derived from |
|---|---:|---|
| `ATTACHMENT_CHUNK_BYTES` | 65,536 | VLT14 `CHUNK_SIZE` |
| `MAX_ATTACHMENT_BYTES` | 16,777,216 | `= MAX_PLAINTEXT_BYTES` |
| `MAX_ATTACHMENT_CHUNKS` | 256 | `= MAX_ATTACHMENT_BYTES.div_ceil(CHUNK_BYTES)` |
| `MAX_ATTACHMENT_NAME_BYTES` | 255 | §4.5 |
| attachments per item | 64 | VLT-PM03 `MAX_ATTACHMENTS`, unchanged |

Every one of these is *computed* in source from the constant to its right rather
than written twice. A second literal is a second thing that can drift, and the
whole subject of this section is what happens when two bounds disagree.

`MAX_ATTACHMENT_BYTES` equals `MAX_PLAINTEXT_BYTES` on purpose, and the equality
is the argument: **an attachment can never be larger than a plaintext this
product already accepts in a single sealed frame.** Attachments do not become a
bigger door than records. A number chosen for user comfort — "photos are about
this big" — would have had no such property and would have had to be defended
against every future change to the frame bound separately.

Four independent bounds confirm 256 chunks is not close to anything:

| Bound | Value | Consumed by one maximal attachment |
|---|---:|---:|
| `MAX_ENCODED_SIZE` per chunk object | 1,048,576 | 65,600 (6.3%) |
| `MAX_ADDED_OBJECTS` per commit | 4,096 | 260 (6.3%) |
| `MAX_PUBLICATION_OBJECTS` per journal | 4,096 | 260 (6.3%) |
| VLT14 `MAX_CHUNK_COUNT` | 16,777,216 | 256 (0.0015%) |

The 260 is 256 chunks plus the manifest, the new item revision, the rebuilt
catalog, and the audit event — the complete object set of one attach, which §5
requires to be one publication.

### 3.5 Rejection is closed, at both ends

Two separate refusals, for two separate adversaries.

**Local.** A source file larger than `MAX_ATTACHMENT_BYTES` is rejected before
any entropy is drawn, any object is sealed, and any prompt is shown. The class
is `BoundExceeded`, surfaced as CLI exit 2. Nothing is published; the vault is
byte-for-byte unchanged.

**Peer.** Every field a synchronising peer could author is bounded on the decode
side, independently of what the encoder would have produced:

- a manifest declaring more than `MAX_ATTACHMENT_CHUNKS` chunk references is
  `BoundExceeded` **before** any buffer is allocated;
- a manifest declaring a `total_plaintext_len` above `MAX_ATTACHMENT_BYTES` is
  `BoundExceeded` before any buffer is allocated — the declared length is never
  a capacity argument that has not first been bounded;
- a chunk whose ciphertext exceeds `CHUNK_SIZE` is refused by VLT14's decryptor;
- a chunk out of order, from another blob, promoted to final, or submitted after
  final is refused by VLT14's AAD binding;
- a stream that ends without a final chunk is `Truncated`;
- a reassembled length that disagrees with the manifest's declared length is an
  integrity failure (§4.4 explains why this check is not redundant);
- a reassembled SHA-256 that disagrees with the manifest's is an integrity
  failure.

**None of these is a panic.** That is the entire lesson of #11945 restated for a
new surface: an oversized or malformed value an untrusted peer is free to author
must never abort the process, because the offending object persists and the
abort then repeats on every later command against the same vault. Every encode
in this slice is `try_encode` mapped through the existing `map_encode_error`,
and every decode returns `BoundExceeded` or `IntegrityFailure`.

## 4. Storage design

### 4.1 Three kinds of object, one publication

An attachment is stored as **chunk objects**, exactly one **manifest object**,
and a **reference in the item revision**. All of them are ordinary vault-pm
repository objects: canonical CBOR plaintext with the universal `{1: version,
2: kind, …}` header, sealed by `seal_object` under a fresh random per-object
DEK wrapped under the vault's object-wrap key, framed as `VPO1`, and addressed
by SHA-256 of the framed ciphertext. Nothing about their storage, addressing,
publication, verification, or garbage collection is new.

Two kind codes are added to VLT-PM05 §6's registry:

| Code | Name | Purpose |
|---:|---|---|
| 6 | `AttachmentManifestV1` | one attachment's metadata and chunk references |
| 7 | `AttachmentChunkV1` | one VLT14 sealed chunk |

The kind is bound into both AEAD associated-data strings, so a chunk frame
presented where a manifest is expected fails as `IntegrityFailure` rather than
decoding into the wrong shape.

### 4.2 The chunk, and why it is encrypted twice

```text
AttachmentChunkV1 {
    1: version = 1,
    2: kind = 7,
    3: blob_id       = 16 bytes,
    4: chunk_index   = uint < MAX_ATTACHMENT_CHUNKS,
    5: is_final      = bool,
    6: ciphertext    = bstr, length <= 65,536,
    7: tag           = 16 bytes,
}
```

Fields 3 through 7 are exactly VLT14's `EncryptedChunk`. The inner AEAD is
XChaCha20-Poly1305 under the per-blob DEK, with nonce `blob_id || index_be ||
0x00000000` and associated data `"VAT1" || blob_id || index_be || is_final ||
0x00…`. The outer AEAD is vault-pm's ordinary object envelope.

Encrypting twice is not belt-and-braces; the two layers authenticate different
things:

- The **outer** layer binds the bytes to *this vault* and *this object kind*,
  and gives the object an address. It says nothing whatever about which
  attachment a chunk belongs to or where in it the chunk sits.
- The **inner** layer binds the bytes to *this blob* at *this index* with *this*
  finality. It is what makes "chunk 4 of attachment A substituted for chunk 4 of
  attachment B" a tag failure rather than a silent wrong answer.

Content addressing alone does not supply the inner property. It guarantees that
an object with a given ID has given bytes; it does not stop a manifest — or a
tampered revision, or a future pack index — from listing the wrong IDs in the
wrong order. VLT14 makes that misordering unforgeable without the DEK. The cost
is 16 bytes per 64 KiB, or 0.02%.

**Nonce discipline.** VLT14 derives chunk nonces from a counter, which is
against `vault-pm-application`'s convention that every nonce is caller-injected
independent randomness. The convention is not violated, because the two live at
different layers: the *outer* object frames keep their independent random
nonces, and the *inner* counter nonces are safe precisely under VLT14's stated
condition — a `(blob_id, dek)` pair never reused across streams. This slice
satisfies it the strong way: both the 128-bit blob id and the 256-bit DEK are
drawn fresh from the caller-supplied entropy block for every single attach, so
no two streams in the product's history can collide even on the same file. The
crate's own `new_random` constructor, which would reach for the OS CSPRNG
directly and break `vault-pm-application`'s determinism, is not used;
`from_dek` is.

### 4.3 The manifest

```text
AttachmentManifestV1 {
    1: version = 1,
    2: kind = 6,
    3: attachment_id      = 16 bytes,
    4: dek                = 32 bytes,
    5: name               = text, 1..=255 bytes,
    6: total_plaintext_len = uint <= MAX_ATTACHMENT_BYTES,
    7: content_sha256     = 32 bytes,
    8: chunks             = array of ObjectId, 1..=MAX_ATTACHMENT_CHUNKS,
    9: created_at_ms      = uint,
}
```

**The attachment id *is* the blob id.** Both are 128-bit opaque random values
with the same lifetime and the same uniqueness requirement, and one value
carries both meanings: it names the attachment in VLT-PM03's OR-set, and it is
the value VLT14 binds into every chunk's associated data. Keeping them separate
would have added a field whose only job was to be checked equal to another
field, and would have created a state — the two disagreeing — with no defined
meaning. Because they are one value, VLT14's AAD binds the chunk to the
*attachment identity a person sees*, not to a private alias.

The DEK is stored in field 4 in the clear *inside this object's plaintext*,
which is sealed under the vault's object-wrap key. VLT14's documentation says
the host must wrap the DEK before storage; the enclosing object envelope is that
wrap, and it is the same envelope every record body already relies on. §8.1 of
VLT-PM00 is explicit that attachment DEKs live "beneath this repository
envelope". A future recipient-sharing feature can rewrap one manifest to share
one attachment without touching a chunk.

`chunks` is ordered by chunk index — position *i* is chunk *i* — and its length
is the chunk count. A duplicate object ID inside it is an integrity failure: two
identical chunk ciphertexts cannot occur, because every chunk has a distinct
nonce and a distinct outer random DEK.

Encoded size at the maximum: 256 references at 34 bytes plus a name and the
fixed fields is **about 9 KiB**, against the 1 MiB ceiling. The manifest is the
reason the revision does not grow with attachment size.

### 4.4 Why the length and hash are checked even though they are signed

The manifest sits inside an AEAD-sealed object whose ID is pinned by the item
revision, whose ID is pinned by the catalog, whose ID is pinned by an Ed25519
commit signed by a certified device. Tampering with `total_plaintext_len`
requires forging that chain. So why re-derive it?

Because VLT14 v1 does **not** authenticate it. The crate commits `0` rather
than the real total in every chunk's associated data — it cannot know the total
until the stream ends, and buffering the whole stream to learn it is the thing
chunking exists to avoid. The crate's own documentation states the consequence
and names the caller's duty: verify `decryptor.finish()? == total_plaintext_len`
after the last chunk. This slice does that, and hashes the reassembled plaintext
as well. Neither check is load-bearing against a network adversary; both are
load-bearing against *this product's own future bugs*, which are the failure
mode a byte-identical-round-trip guarantee actually has to survive.

### 4.5 Names are validated, never sanitised

`name` is the source file's **base name only**. The directory it came from is
never stored: VLT-PM15 §5 forbids a source path in an audit event, and there is
no reason for the vault to hold one either.

A name must be 1..=255 bytes of UTF-8, must not be `.` or `..`, and must
contain none of:

- a Unicode **Cc** control character — which is what blocks ESC, so a terminal
  escape sequence cannot ride in a name, and blocks tab and newline, so the
  tab-separated listing cannot be forged;
- a Unicode **Cf** format character — the bidirectional marks and overrides
  (U+061C, U+200E–U+200F, U+202A–U+202E), the isolates (U+2066–U+2069), the
  zero-width and joining characters, and U+FEFF;
- **Zl** or **Zp**, the line and paragraph separators; or
- `/` or `\`.

Anything else is rejected with `InvalidInput`. It is rejected rather than
repaired, because a sanitiser is a function whose output the author did not
choose, and the interesting inputs to one are exactly the hostile ones.

The Cf and Zl/Zp half of that list is there because **this validator also runs
on decode**, on a name a synchronising peer authored. `char::is_control` is Cc
only; a name carrying U+202E renders as a different name, and §6.2's listing is
how the operator chooses which attachment to export. Ordinary non-ASCII names
are untouched — the rejection is of characters that change what a name *looks
like*, not of anybody's alphabet.

The rendering layer escapes independently (§6.2). Two gates, because a
validator must not be the only thing between peer-authored text and a
terminal.

The 255-byte limit is the widest name every filesystem this product targets
accepts, so a name that survives ingest is a name that could have been a file.
That is a property about the *source*, and §4.6 is emphatic that it is not a
promise about the destination.

### 4.6 A stored name never reaches a path resolver

**The exported file's location is the operator's `PATH` argument and nothing
else.** The stored name is displayed by `attachment list` and is otherwise
inert. It is never joined to a directory, never used as a default, never
consulted to pick an extension.

This is the reason §2.1 removes §14.4's optional destination. `name` is
attacker-controlled in the only threat model that matters here — VLT-PM00 §7's
malicious-peer adversary, who authors a revision on another device and syncs
it. Validation in §4.5 rejects the separators that make classic traversal work,
but the defence that does not depend on a validator being exhaustive is
structural: no code path turns a stored name into a path. A future GUI that
offers "save as <name>" must put the name through its platform's save dialog,
where a human sees the resolved destination, and not through a join.

### 4.7 The tenth field on the item revision

VLT-PM03's `ItemDocument` has carried `attachments: ObservedSet<AttachmentId>`
since the domain was written, and VLT-PM05 §6.1 has carried it as integer key
`9` of the live-state map. That set is the CRDT membership — VLT-PM00 §9.1's
"OR-set of immutable attachment IDs" — and it is unchanged.

What it cannot express is *where the manifest is*. So the live-state map gains
one field:

```text
10: attachment_manifests = array of {
        1: attachment_id = 16 bytes,
        2: manifest      = ObjectId,
    }, ascending by attachment_id
```

Three properties, all enforced on decode:

1. **Field 10 is present if and only if the field-9 OR-set has at least one
   retained value.** An item with no attachments encodes exactly the nine keys
   it encoded before this slice, byte for byte. Every revision written by every
   earlier version of this product is still decodable, and every revision this
   product writes for an item without attachments is still identical to what it
   would have written yesterday.
2. **Its key set equals the OR-set's retained values exactly.** A manifest
   pointer with no membership, or membership with no manifest pointer, is
   `IntegrityFailure`. There is no state in which an attachment half-exists.
3. Ascending, unique, and bounded by `MAX_ATTACHMENTS`.

Property 1 is what makes this a compatible extension rather than a format break,
and it is available only because nothing before this slice could put a value in
the field-9 set — the type existed, the merge existed, the bound existed, and no
ceremony ever added an id. The strict "exactly these keys" decoder is otherwise
preserved: unknown keys, duplicate keys, and wrong types are still refused.

Size: 64 attachments cost about 3.4 KiB, against the 1 MiB revision ceiling.
Storing manifests inline instead — names, hashes, DEKs and 256 chunk references
each — would have cost up to 570 KiB and put the revision encode back in the
range §3.1 is about.

### 4.8 Merge, history, and collection

- **Merge.** Field 9 merges as before. Field 10 is derived from the merged set:
  the union of both sides' maps, which cannot conflict, because an
  `AttachmentId` is immutable and globally unique, so two revisions that both
  know an id necessarily point at the same manifest. Two that disagree are
  `IntegrityFailure`, not a conflict to resolve.
- **History and restore.** Revisions are immutable, so an older revision carries
  the attachment set it had. Restoring it restores that set. The chunk and
  manifest objects of an attachment dropped by a later revision are still in the
  store, so the restore is real rather than a dangling pointer.
- **Garbage collection.** `plan_gc` marks reachable every object named by a
  reachable commit's `added_objects`, plus its catalog root, tombstone root and
  device certificate. Chunks and manifests are published in `added_objects`, so
  they are reachable by construction — no second hop through the manifest is
  required, and no change to GC is needed. VLT-PM00 §19.4's "in-progress
  attachment manifests" clause is about a resumable-upload design this slice
  deliberately does not have (§5).

## 5. Crash resumability is inherited, not built

**An attachment write is one ordinary mutation.** It is not a streaming upload
with its own progress state, and this is the single most important structural
decision in this document.

VLT-PM05 §7.2 says every mutation constructs the complete randomized frames,
the signed commit and the signed announcement *first*, then records them in a
`PendingPublication` journal by an atomic compare-exchange, and only then writes
to the repository; success is a second compare-exchange to `Active`. An
attachment write does exactly that, with 256 more frames in the same journal:

```text
prepare:  seal 0..N chunk frames, the manifest frame, the new revision frame,
          the rebuilt catalog frame, the audit-event frame, the signed commit,
          the signed announcement
CAS #1:   Active -> PendingPublication{ every frame above }
publish:  put_immutable each object, then the commit, then the announcement
CAS #2:   PendingPublication -> Active{ new pins }
```

Everything VLT-PM41 and VLT-PM42 proved about that machine therefore holds here
with no new argument:

- A crash before CAS #1 leaves the vault byte-identical. Nothing was written.
- A crash between CAS #1 and CAS #2 leaves the exact frames on disk. The next
  ordinary command republishes the *identical bytes* — same object IDs, same
  commit, same reserved device counter — and finishes. This is VLT-PM42's
  pending-publication recovery, unmodified.
- A crash during publication leaves some chunk objects present and no commit
  referencing them. They are §10.4's "unreachable immutable objects that GC may
  remove later" — the same residue an interrupted item create leaves, at a
  larger count. They are not an orphaned partial blob that nothing can clean up,
  because the thing that cleans them up is the thing that cleans up every other
  interrupted publication.
- A torn state is impossible for the same reason it is impossible for an item
  edit: publication never exposes a commit whose dependencies were not published
  first, and the revision that names the manifest is published in the same
  commit as the manifest and the chunks.

The alternative — a resumable upload that writes chunks across several commits
and tracks progress — would have introduced a durable state VLT-PM41's matrix
does not cover, a second recovery path beside VLT-PM42's, and a genuinely
orphanable partial blob. It buys the ability to attach a file larger than one
publication can carry. §3.4 sets the ceiling below that point on purpose, so the
capability is not needed and the state does not exist.

**The one new durable write outside the storage backend** is the export
artifact (§6.3). It gets a `DurableStep` of its own so VLT-PM41's drill can
count and kill it, exactly as `export.artifact` did for portable export.

## 6. Ceremonies

### 6.1 `attachment add` is an item update

Ordering, with every step load-bearing:

1. Parse; reject a missing or unreadable source, a directory, or a file over
   `MAX_ATTACHMENT_BYTES`. Reject an invalid name (§4.5). **Before any prompt.**
2. Reserve the wall clock and the entropy block (§6.4).
3. Unlock, recovering a pending publication if one is present.
4. Require the item to exist, be live, and have exactly one current candidate.
   More than one is `ConflictRequired`; the operator resolves the conflict
   first, as with every other edit.
5. Chunk, seal, build the manifest, build the new revision with field 9 extended
   and field 10 rebuilt, rebuild the catalog, sign the commit.
6. Publish through `publish_mutation`.
7. Print the new `AttachmentId`.

The audit action is **`ItemUpdate`**, item-scoped, with the pre-attach revision
as `selected_revision` and the new revision as `result_revision` — because
attaching a file *is* an update of the item document, and VLT-PM15 §2's closed
registry deliberately names user-visible operations at item granularity rather
than one code per field. §8.4 records the argument for a distinct code and why
it is not taken here.

The event carries no name, no size, no hash, and no source path. VLT-PM15 §5
forbids "plaintext attachment name or source/destination path" outright, and the
event has no field one could ride in.

### 6.2 `attachment list` is an item read

Returns, per attachment: the `AttachmentId`, the name, the plaintext byte
length, and the SHA-256 of the plaintext content. It reads the item's manifests
and nothing else — no chunk object is fetched, so listing a 16 MiB attachment
costs one small object read.

**The name is rendered quoted and escaped**, through the same helper every
other stored string in this CLI passes through. §4.5 validates it on decode as
well as on ingest, so this is the second of two gates rather than the only one;
it is here because a validator is a statement about what was stored and the
escape is a statement about what reaches a terminal, and the second is the one
the operator is reading when they choose an attachment to export.

The audit action is **`ItemRead`**, item-scoped, non-secret. It requires no
confirmation ceremony for the same reason `item show` does not: what it renders
is the redacted projection, and a name is the same class of metadata as a title,
which this product has always shown.

The content hash is rendered because it is what makes the round-trip guarantee
checkable by a person: export the file, hash it, compare. It is a hash of
plaintext the operator can already obtain, so it discloses nothing new.

### 6.3 `attachment export` is a disclosure

An exported file is plaintext on disk. That is what an export *is* — the point
is to hand the person their file. The disclosure question is not whether the
plaintext lands unencrypted; it is whether the ceremony above it matches every
other release of vault-held secret content, and whether anything *else* leaks.

The ceremony is VLT-PM25's, unchanged in every respect but the sentence:

| Step | `item reveal` | `attachment export` |
|---|---|---|
| destination validated before any prompt | n/a | required (§6.5) |
| audit time / randomness reserved pre-unlock | identical | identical |
| unlock | identical | identical |
| interactive TTY confirmation | required | required |
| disclosure intent | `InteractiveReveal { confirmed }` | identical |
| durable `ItemRead` published **before** release | identical | identical |
| refusal publishes `Denied` | identical | identical |
| delivery | controlling terminal | the named file |

The intent stays `InteractiveReveal` for VLT-PM46 §3.0's reason: the chain
records *that* an item was read, not the door it left by. The prompt does not,
for VLT-PM46 §3.1's reason — a consent ceremony that misdescribes what it is
consenting to manufactures a record of an agreement nobody made:

```text
Write this attachment's contents to a plaintext file? Type yes to continue:
```

Same exact-lowercase-`yes` rule, same `Denied` outcome on refusal or on a host
failure collecting the answer.

**Publish before release, literally.** Reassembly happens inside the application
boundary, before the audit event is published; the reassembled bytes are held in
a local binding across the publication and are returned to the caller only after
it succeeds. If publication fails, the plaintext is dropped, zeroized, and never
reaches the host. This is the identical structure `audited_current_item_totp_code`
uses, and it is shared with it rather than copied: the outcome table that maps
situation to audit result is one function.

### 6.4 A portable export announces what it did not carry

An attachment does not survive `export` → `import`: the snapshot carries
records, and the import path drops the attachment set rather than producing an
item that claims attachments the target vault has no bytes for.

A backup an operator believes carries their recovery codes and does not is
worse than no backup, so `export` from a vault holding at least one attachment
writes a fixed sentence to standard error:

```text
vault-pm: portable export does not carry attachments
```

The same shape as VLT-PM42's recovery notice, and for the same reasons.
Standard output is unchanged, so nothing that parses it has to learn a new
line, and the exit class of a command that also had something to mention is the
exit class of the command. The sentence is fixed at compile time and names no
vault, item, count, or path — the condition that produced it is an aggregate of
the kind VLT-PM00 §20 permits, and the sentence carries not even that.

### 6.5 Entropy is caller-supplied and sized by the file

`vault-pm-application` draws no randomness of its own. Every mutation takes one
owned wipe-on-drop entropy block whose size is a `const`. An attachment's block
cannot be a `const`, because its size depends on how many chunks the file has —
the same situation as `PortableImportRandomnessV1`, which is variable-length and
validated by a function. The layout, in the order it is consumed:

```text
attachment / blob id      16
blob DEK                  32
per chunk object          80 x chunk_count
manifest object           80
revision object           80
catalog object            80
commit object             80
trace id                  32
audit-event object        80
                          = 480 + 80 x chunk_count
```

`attachment_random_bytes(plaintext_len)` computes it and is the only place the
arithmetic appears; the ceremony asserts the consumed offset equals the declared
size, as every existing ceremony does.

### 6.6 The destination

`attachment export` writes through a host capability modelled exactly on
`write_portable_export`: refuse an existing destination rather than overwrite
it, create with owner-only permissions where the platform has them, write and
`fsync`, and **remove the partial file if any step fails**. A half-written
plaintext file left behind by a failed export is a leak with no owner, and the
existing portable-export path already refuses to leave one.

Three residuals, recorded rather than assumed away. On Unix `create_new` is
`O_CREAT | O_EXCL`, which refuses an existing path including a dangling
symbolic link, and `0600` applies from the instant the file exists; both
statements are narrower on Windows, where `CREATE_NEW` resolves a reparse point
and `OpenOptions` exposes no mode — a gap that matters more for this plaintext
than for the encrypted portable artifact, and that needs a Windows security
descriptor to close. The cleanup re-resolves the path rather than acting on the
descriptor. And a `SIGKILL` delivered *inside* the write still leaves a partial
file: the drill brackets the call and can prove both of its landing points
clean, but not that one. Removing that last residual means writing to a
temporary and renaming, which changes what the destination is during the write.

The **source** is opened `O_NONBLOCK | O_NOCTTY` on Unix, because the check
that would reject a FIFO cannot run until the open returns and opening a FIFO
for reading blocks until a writer appears — without the flag, naming a named
pipe hangs the command instead of being refused. The read buffer is sized one
byte past the file's declared length so that a file which grows between the
metadata read and the read itself cannot make `read_to_end` reallocate: a
reallocation copies the plaintext already read into a new allocation and frees
the old one *unwiped*, which is exactly what the `Zeroizing` buffer exists to
prevent.

The destination is validated before the passphrase prompt. A person who named a
path that already exists should learn it immediately, not after typing their
master passphrase — VLT-PM44 §2.3, VLT-PM45 §2.3, and VLT-PM46 §3.2 all place
this check in the same position for the same reason.

## 7. What this cannot do

- **The exported file is not protected.** Once written it is an ordinary file
  under ordinary permissions, and this product does not track it, clear it, or
  know if it is copied. `0600` at creation is the whole of the protection.
- **Sizes are visible to the provider.** A vault holding one 12 MiB attachment
  shows 188 objects of ~65 KiB. VLT-PM00 §7 already concedes sizes and access
  patterns; attachments make the concession legible. Padding is §24's deferred
  metadata-hiding decision.
- **Attachment count is visible in the same way.** An item with attachments has
  a longer revision than one without, by a few tens of bytes per attachment.
- **A very large attachment is a very large commit.** 260 objects publish
  serially through `put_immutable`. On `storage-fs` this is fast; on a Phase 2
  cloud adapter it is exactly the situation §13.5's pack layer exists for.

## 8. Deferred

### 8.1 Packing, and the invariant it must not break

When the Drive adapter arrives with §13.5's pack layer, it groups encrypted
objects into provider files and keeps an encrypted pack index of byte ranges.
It must not be allowed to become an attachment feature. Two invariants:

- One immutable value per object ID, as §10.7's own first sentence says. A
  packed chunk is the same object, in a different container.
- The pack index is not a substitute for VLT14's per-chunk AAD. A pack index
  that could reorder chunks undetectably would undo §4.2's inner layer.

### 8.2 Streaming

§4.7's design already streams on the *write* side in every sense that matters —
each chunk is sealed independently and no chunk depends on another. What is
buffered is the sealing loop's input and the export's output. Making export
stream to disk needs: a host capability that yields a handle rather than taking
a buffer; a partial-file cleanup contract for a failure part-way through; and a
decision about what a torn export means when the hash check can only run at the
end. Worth doing when the ceiling rises; not worth doing at 16 MiB.

### 8.3 Attachments in portable export

VLT-PM17's snapshot carries records. Carrying attachments needs a snapshot
format change, a `candidate_count`-style completeness assertion over blobs, and
an answer to what VLT-PM19/VLT-PM20 restore verification compares. VLT-PM17's
own acceptance criterion — that import creates new encryption identities —
implies re-chunking under a new DEK, which is a real transformation and not a
copy.

### 8.4 A distinct audit action

`ItemAttachmentAdd` would tell an audit reader that an item update was an
attachment ingest rather than a password change. That is genuine information,
and it is deliberately not added: VLT-PM15's registry is closed, its §2 lists
attachment export as an *access* and never names an attachment-specific code,
and widening it is a change to a signed, verified format that should be made
once, for a stated reason, rather than as a side effect of a feature. If the
distinction is wanted, it is a VLT-PM15 amendment.

### 8.5 `attachment remove`

§2.2. Lands with `gc run`.

## 9. Acceptance gates

1. A file larger than one chunk round-trips **byte-identically**: attach a
   multi-chunk file, export it, compare every byte. Proven at a size that is not
   a chunk multiple, so the final short chunk is exercised, and at an exact
   multiple, so the empty-final-chunk case is exercised.
2. The encrypted store contains no plaintext byte of the attachment and no
   plaintext byte of its name, verified by scanning the whole tree.
3. A file at exactly `MAX_ATTACHMENT_BYTES` is accepted; one byte more is
   refused with `BoundExceeded`, before any prompt, leaving the vault
   byte-for-byte unchanged.
4. Every peer-authored malformation in §3.5 returns a closed error and **no
   test in this slice may provoke a panic** — including a manifest declaring a
   chunk count and a total length far above the ceilings, which must be refused
   before allocation.
5. A revision with no attachments encodes to the exact bytes it encoded before
   this slice, and a revision written before this slice still decodes.
6. `attachment export` publishes a durable `ItemRead` before the plaintext
   reaches the host, publishes `Denied` on refusal, and writes nothing on
   refusal.
7. The audit chain records the attach and the export and contains neither the
   attachment name, the file bytes, nor either path.
8. VLT-PM41's drill kills a real `SIGKILL`ed `attachment add` at its
   characteristic landing points; at every one the vault is either unchanged or
   finishes on the next ordinary command, `status` and `doctor` report a named
   recoverable state, and no plaintext reaches disk.
9. An export whose write fails leaves no partial file at the destination, an
    existing destination is refused with its bytes intact, a symbolic link is
    refused rather than followed, and a named-pipe source is refused rather
    than waited on.
9a. A portable export of a vault holding an attachment writes the fixed
    attachments-not-carried notice to standard error, one that does not writes
    nothing there, and standard output is identical in both cases.
10. `cargo fmt --check`, `cargo clippy --all-targets -D warnings`, and
    `cargo doc` are clean on every touched crate, and each crate's own `BUILD`
    line passes.

## 10. References

### Internal

`VLT-PM00-local-first-password-manager.md`, `VLT-PM03-domain.md`,
`VLT-PM05-application.md`, `VLT-PM08-cli-host.md`,
`VLT-PM15-operation-audit.md`, `VLT-PM17-cli-portable-export.md`,
`VLT-PM25-cli-secret-reveal.md`, `VLT-PM41-cli-crash-fault-matrix.md`,
`VLT-PM42-cli-pending-publication-recovery.md`,
`VLT-PM43-cli-passphrase-rotation.md`, `VLT-PM44-cli-password-generate.md`,
`VLT-PM45-cli-totp-code.md`, `VLT-PM46-cli-clipboard.md`,
`VLT02-vault-records.md`, `VLT14-vault-attachments.md`.

### Code

`code/packages/rust/vault-attachments`,
`code/packages/rust/vault-pm-application`,
`code/packages/rust/vault-pm-audit`,
`code/packages/rust/vault-pm-cli`,
`code/packages/rust/vault-pm-cli-host`,
`code/packages/rust/vault-pm-crash-injection`,
`code/packages/rust/vault-pm-domain`,
`code/programs/rust/vault-pm-cli`,
`code/programs/rust/vault-pm-cli-drill`.
