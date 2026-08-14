# smart-home-snmp-integration

Authorized, read-only SNMPv2c telemetry for explicitly configured local
devices such as UPSes, PDUs, network equipment, environmental monitors, and
building controllers.

The runtime sends one bounded GET containing at most 32 exact OIDs to one
private, link-local, or loopback unicast endpoint. It requires exact response
community, request-id, OID-order, BER, and value-syntax correlation before it
atomically installs normalized D23 sensor state. The community remains in a
redacted, zeroized live-host value and is never written to runtime metadata,
errors, CLI arguments, or output.

Run one inspection with the community supplied out of band:

```bash
SMART_HOME_SNMP_VAULT_REF='vault:snmp/ups-1' \
SMART_HOME_SNMP_COMMUNITY='monitor-only' \
cargo run -p smart-home-snmp-integration -- \
  inspect 192.168.1.20:161 ups-1 uptime 1.3.6.1.2.1.1.3.0 timeticks-seconds s
```

This package does not expose SNMP SET, GET-NEXT, GET-BULK, traps, informs,
multicast, broadcast, public endpoints, retries, MIB interpretation, SNMPv1,
or SNMPv3. SNMPv2c has no confidentiality or integrity, so this runtime is
intentionally limited to local read-only polling; authenticated SNMPv3 requires
a separately owned USM/session boundary.
