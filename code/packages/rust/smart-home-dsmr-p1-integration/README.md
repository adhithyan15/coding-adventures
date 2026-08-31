# Smart Home DSMR P1 Integration

Credential-free, serial-only DSMR 5.0.2 P1 telemetry for D23. The runtime
authorizes before opening one explicit `115200 8N1` serial path, reads one
bounded CRC-verified telegram, records supervision state, and installs typed
electricity and optional gas sensors.

The integration exposes no local TCP gateway, Data Request GPIO control,
arbitrary OBIS selection, raw telegram retention, write operation, discovery,
or long-lived serial session.
