# smart-home-nut-ups-integration

Authorized, read-only Network UPS Tools telemetry for explicitly configured
local UPS and PDU servers.

The runtime opens one bounded TCP exchange to a private, link-local, or
loopback endpoint, sends `LIST VAR` for one exact UPS name, and projects only
profile-selected number, boolean, or text variables into normalized D23 sensor
entities. D23 read authorization runs before socket creation, and every list
record must correlate to the configured UPS before any runtime mutation.

Run one anonymous local inspection:

```bash
cargo run -p smart-home-nut-ups-integration -- \
  inspect 192.168.1.20:3493 ups-1 rack-ups battery-charge battery.charge decimal '%'
```

The package does not expose authentication, `SET VAR`, instant commands,
forced shutdown, UPS enumeration, subscriptions, reconnect loops, public
endpoints, or TLS negotiation. Deployments that require credentials or TLS
need a separately owned session and trust-lifetime boundary.
