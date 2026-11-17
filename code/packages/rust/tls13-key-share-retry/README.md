# tls13-key-share-retry

`tls13-key-share-retry` is an allocation-free composition boundary for the
public group transition requested by an RFC 9846 `HelloRetryRequest`.

It accepts the retry's exact two-byte `selected_group`, authorizes that group
through the existing data-driven named-group policy, requires the group to
have appeared in the first ClientHello `supported_groups` list, and requires
it not to have appeared in that ClientHello's key-share offer. A second step
requires exactly one replacement share for the selected group. Success retains
all exact decoded inputs as non-copyable evidence. A final step decodes the
ServerHello share directly against that evidence's retained replacement offer,
preventing an unrelated offer from being substituted between the two checks.
Stable errors and Debug values contain no group identifiers, public key bytes,
positions, or lengths.

```rust
use tls13_key_share::{decode_client_key_shares, KeyShareLimits};
use tls13_key_share_retry::{
    bind_retried_client_key_share, bind_retry_selected_group,
    bind_retry_server_key_share,
};
use tls13_named_group_policy::{NamedGroup, NamedGroupPolicy};
use tls13_supported_groups::{decode_supported_groups, NamedGroupListLimits};

let policy = NamedGroupPolicy::new(&[NamedGroup::X25519])?;
let supported = decode_supported_groups(&[0x00, 0x02, 0x00, 0x1d], NamedGroupListLimits::new(1)?)?;
let initial = decode_client_key_shares(&[0x00, 0x00], &policy, KeyShareLimits::new(1)?)?;
let selected = bind_retry_selected_group(&[0x00, 0x1d], &policy, supported, initial)?;

let mut replacement = [0_u8; 38];
replacement[..6].copy_from_slice(&[0x00, 0x24, 0x00, 0x1d, 0x00, 0x20]);
let retried = decode_client_key_shares(&replacement, &policy, KeyShareLimits::new(1)?)?;
let bound = bind_retried_client_key_share(selected, retried)?;
assert_eq!(bound.selected_group().parameters().group(), NamedGroup::X25519);

let mut server = [0_u8; 36];
server[..4].copy_from_slice(&[0x00, 0x1d, 0x00, 0x20]);
let final_share = bind_retry_server_key_share(&bound, &server, &policy)?;
assert_eq!(final_share.server_share().key_exchange().len(), 32);
# Ok::<(), Box<dyn std::error::Error>>(())
```

This package does not prove entropy or key freshness, generate or own a private
key, perform key agreement, locate extensions in a handshake, compare other
ClientHello fields, update a transcript, reject a second retry, sequence a
handshake, validate certificates, establish trust, check revocation, perform
I/O, authorize HTTP/OAuth transport, or make concrete HTTPS or live-provider
use ready.

## Development

```bash
bash BUILD
cargo clippy -p tls13-key-share-retry --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p tls13-key-share-retry --no-deps
```
