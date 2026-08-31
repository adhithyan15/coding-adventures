# DSMR P1 Protocol

Strict, transport-free DSMR 5.0.2 P1 telegram framing, CRC-16 validation,
and typed decoding for a fixed electricity and gas telemetry allowlist.

The crate does not open serial ports, control the Data Request line, retain raw
telegrams, or expose arbitrary OBIS selection.
