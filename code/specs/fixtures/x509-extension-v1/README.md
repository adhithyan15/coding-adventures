# Generic X.509 Extension v1 fixtures

This directory is the language-neutral contract for decoding one RFC 5280
`Extension` value above the repository-owned typed DER ASN.1 layer.

Each case supplies one complete DER element. A consumer must frame the root
through its package-native `der-asn1` decoder and pass that same decoder state
to its generic extension decoder. Success projects validated OID arcs as exact
decimal strings, the `critical` Boolean, byte-exact opaque value contents, and
the shared successful-element count.

Portable errors use one of eight extension categories. Structure and typed
child failures additionally project the exact nested DER ASN.1 category;
framing failures also project the exact DER TLV category. Every offset is local
to the complete Extension element. Results and public diagnostics must not
retain or format hostile input bytes.

The value inside `extnValue` is opaque. A byte sequence that is malformed DER
therefore remains a successful generic Extension value. Extension registries,
semantic decoders, certificate policy, signatures, trust, revocation, TLS,
transport, and ambient authority are outside this corpus.

`consumers.json` is the closed target registry for all 15 established lanes.
Registry membership records an obligation; completion requires each listed
package's real production source, package-native fixture test, capability
manifest, documentation, metadata, and BUILD fronts.
