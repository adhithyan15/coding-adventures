# smart-home-zoneminder-integration

Production-facing ZoneMinder NVR and camera health inspection for D23.

The integration accepts a manually configured local HTTPS origin or path prefix and a Vault
credential reference. After D23 authorizes `smart_home.read`, the production
transport logs in through ZoneMinder's documented API 2.0
`/api/host/login.json` endpoint, keeps the short-lived access JWT inside the
bounded transport, and reads `/api/host/getVersion.json` plus
`/api/monitors.json`. Credentials, login bodies, refresh tokens, access tokens,
and token-bearing request targets never
enter request plans, normalized state, or debug output.

Camera entities expose confirmed enablement, capture, analysis, recording,
native status, capture/analysis FPS, and capture bandwidth. The health mapping
preserves ZoneMinder's native monitor status and marks disabled, stopped, or
no-signal monitors offline. Plain HTTP is accepted only for loopback protocol
tests.

This slice intentionally does not read ZoneMinder configuration or expose event,
snapshot, recording, export, playback, or mutation endpoints. Those operations
need concrete event or media hosts and operation-specific D23 contracts.

Protocol references:

- [ZoneMinder API](https://zoneminder.readthedocs.io/en/stable/api.html)
- [ZoneMinder API options](https://zoneminder.readthedocs.io/en/stable/userguide/options/options_api.html)
