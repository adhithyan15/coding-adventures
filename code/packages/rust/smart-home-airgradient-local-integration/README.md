# smart-home-airgradient-local-integration

First-party, read-only AirGradient local monitor integration for D23.

It resolves the documented `airgradient_<serial>.local` mDNS hostname or
accepts a manual host. A bounded local HTTP client reads
`/measures/current`, verifies the returned serial, model, and firmware, and
installs normalized PM, CO2, temperature, humidity, VOC, NOx, particle-count,
and Wi-Fi sensor entities. D23 authorization is checked before any network I/O.

Local configuration mutation, LED/display control, and CO2 calibration remain
separate work.

```bash
cargo run -p smart-home-airgradient-local-integration -- discover ecda3b1eaaaf
cargo run -p smart-home-airgradient-local-integration -- inspect 192.0.2.50
```
