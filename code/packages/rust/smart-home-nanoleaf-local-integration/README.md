# smart-home-nanoleaf-local-integration

This package connects Nanoleaf lights directly to D23 over the local network:

- `_nanoleafapi._tcp.local` mDNS discovery through `smart-home-discovery`;
- physical-presence token pairing through `POST /api/v1/new`;
- bounded, authenticated local HTTP inspection with token material kept outside runtime metadata; and
- capability-caged power, brightness, RGB, and color-temperature commands followed by a fresh state read and confirmed D23 state installation.

Run `cargo run -p smart-home-nanoleaf-local-integration -- discover` to scan the
local network. Put a device in pairing mode, then run `pair <host> [port]` and
store the emitted token in a vault. Set `NANOLEAF_AUTH_TOKEN` and optionally
`NANOLEAF_CREDENTIAL_REF` before `inspect <host> [port]`.

This polling slice does not claim Nanoleaf event-stream support or cloud API
coverage.
