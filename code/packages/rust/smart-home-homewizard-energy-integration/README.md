# smart-home-homewizard-energy-integration

First-party, read-only HomeWizard Energy API v1 integration for D23.

It discovers API-enabled P1 Meters, Energy Sockets, kWh Meters, and Watermeters
over mDNS or accepts a manual HTTP origin. The integration verifies device
identity through `/api`, fetches current measurements from `/api/v1/data`, and
installs normalized device and external-meter sensors into the smart-home
runtime. Authorization is checked before either LAN request is made.

Energy API v2 authentication, WebSocket telemetry, and Energy Socket control
are intentionally separate production slices.

```bash
cargo run -p smart-home-homewizard-energy-integration -- discover
cargo run -p smart-home-homewizard-energy-integration -- inspect 192.0.2.20
```
