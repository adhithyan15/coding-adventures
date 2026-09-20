# der-tlv (TypeScript)

`@coding-adventures/der-tlv` provides bounded canonical DER framing with
`Uint8Array.subarray` views, BigInt wire-length arithmetic, explicit limits,
stable payload-blind errors, and an iterative sibling cursor. It does not
interpret ASN.1 values, parse X.509, perform cryptography, or access the host.

Public decode operations validate every configured bound. Cursors retain a
frozen snapshot, so non-finite, fractional, negative, oversized, or later
mutated JavaScript values cannot disable the input, value, tag, or sibling-work
limits. They also capture a fixed input extent so a resizable backing buffer
cannot append work after construction, using typed-array intrinsics so
shadowed accessors cannot disguise the actual extent or escape decoded ranges.
