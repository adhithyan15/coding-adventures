# smart-home-frigate-integration

Production-facing Frigate NVR and camera health inspection for D23.

The integration accepts a manually configured local HTTPS origin and a Vault
credential reference. After D23 authorizes `smart_home.read`, the production
transport logs in through Frigate's documented `/api/login` endpoint, keeps the
JWT cookie inside the bounded transport, reads `/api/version` and `/api/stats`,
and explicitly logs out. Credentials, login bodies, cookies, and JWTs never
enter request plans, normalized state, or debug output.

Camera entities expose confirmed processing FPS, detection state, connection
quality, expected FPS, recent reconnect/stall counters, and the documented
snapshot capability. The reusable snapshot transaction performs a fresh login,
fetches one bounded JPEG from `/api/{camera}/latest.jpg` through a reviewed
pinned address, and explicitly logs out after every authenticated outcome. The
health mapping preserves Frigate's native connection-quality signal and marks
stopped camera processing offline. Plain HTTP is accepted only for loopback
protocol tests.

This slice intentionally does not read Frigate configuration or expose event,
recording, export, playback, or mutation endpoints. Those operations need
concrete event or resource hosts and operation-specific D23 contracts.

Protocol references:

- [Frigate authentication](https://docs.frigate.video/configuration/authentication/)
- [Frigate login API](https://docs.frigate.video/integrations/api/login-login-post/)
- [Frigate stats API](https://docs.frigate.video/integrations/api/stats-stats-get/)
- [Frigate camera API](https://docs.frigate.video/integrations/api/camera/)
