# snmp-protocol

Dependency-free, bounded SNMPv2c framing for read-only local telemetry.

The crate encodes one `GetRequest-PDU` and decodes its exact `Response-PDU`.
It strictly validates canonical BER lengths and integers, message version,
community correlation, request id, error status, variable-binding count, and
OID order. Requests are capped at 32 OIDs and UDP datagrams at 1472 bytes.

The crate does not implement SET, GET-NEXT, GET-BULK, traps, informs, SNMPv1,
SNMPv3, MIB lookup, retries, discovery, or transport ownership.
