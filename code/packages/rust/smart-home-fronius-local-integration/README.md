# smart-home-fronius-local-integration

First-party, read-only Fronius Solar API v1 integration for D23.

It discovers likely Fronius Data Managers over mDNS or accepts a manual HTTP
origin, fetches realtime Power Flow telemetry, validates the API status, and
installs normalized site and inverter sensor entities into the smart-home
runtime. Authorization is checked before any LAN request is made.

```bash
cargo run -p smart-home-fronius-local-integration -- discover
cargo run -p smart-home-fronius-local-integration -- inspect 192.0.2.20
```
