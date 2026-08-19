# `coding_adventures_vault_pm_agent_protocol`

This crate is the bounded wire format for the VLT-PM48 local vault-pm agent.
It performs no I/O and depends on nothing but
`coding_adventures_zeroize`: given a byte slice, it decodes a request or
response; given a request or response, it encodes a byte slice. Framing
(reading a socket, timeouts, retries) and everything about what the bytes
*mean* to a vault live one layer up, in `vault-pm-agent-host`.

## Why hand-rolled instead of a serialization crate

This workspace does not carry `serde` as a general-purpose dependency.
`vault-pm-format` and `vault-pm-cli-host`'s `ClipboardClearRequest` are both
exact, hand-checked byte layouts rather than derived ones, and this protocol
follows the same convention: one fixed version byte, one fixed tag byte per
message, and explicitly bounded fields, so the complete set of bytes a local
peer can ever send is readable in this one file.

## What crosses this wire, and what does not

The agent retains exactly one thing across process boundaries: a master
passphrase a person already typed once, so a later one-shot `vault-pm`
invocation does not have to ask again. Decrypted item fields, the vault root
key, search results, and TOTP codes never cross this wire — see
`VLT-PM48-local-agent-ipc.md` §3 for the full argument that an agent-cached
passphrase is no worse an exposure than `vault-pm shell`'s own in-memory
buffer already is.

Six requests (`Ping`, `Unlock`, `GetPassphrase`, `Lock`, `Status`,
`Shutdown`), five distinct response shapes. Every carried string is
length-prefixed and bounded to the same ceilings the rest of this product
already uses — 64 bytes for a vault name (matching `vault-pm-config`'s
`ConfigName`), 1,024 bytes for a passphrase (matching
`vault-pm-cli-host::MAX_SECRET_BYTES`) — and a whole frame is capped well
below what a hostile or malformed peer could use to force an unbounded
allocation. The length prefix is checked by the transport layer *before* a
payload buffer is ever allocated; this crate's own `MAX_FRAME_BYTES` is the
ceiling it checks against.

Decoding is closed rather than tolerant: a version mismatch, an unknown tag,
a truncated field, or trailing bytes after a complete message are all
refused with the same single `ProtocolError`. There is exactly one producer
of this format per version — this same crate — so a byte sequence that is
not exactly what `encode` would have written is never guessed at.

## Reuse

VLT-PM00 §17 names "local agent lifecycle and permission-checked IPC" as a
desktop-specific responsibility for a later phase. This crate's independence
from sockets, threads, and `vault-pm-application` is deliberate preparation
for that reuse — a future desktop agent can share this wire format without
inheriting the CLI's transport, and vice versa.

## Verification

Eleven tests cover every request and response round-tripping through
`encode`/`decode`, redacted `Debug` formatting for every variant, malformed
and truncated input being refused rather than guessed at, and every
oversized field or frame being rejected at encode time. Tarpaulin's LLVM
engine measures 162 of 176 lines covered (92.05%).

```bash
bash BUILD
cargo clippy -p coding_adventures_vault_pm_agent_protocol --all-targets -- -D warnings
```
