# Smart Home Enphase Envoy Integration

This package implements read-only local telemetry for Enphase IQ Gateway and
Envoy devices using Enphase's documented token-authenticated local APIs.

Production configuration requires a credential-free HTTPS origin, a known
gateway serial number, a Vault reference for a pre-generated access token, and
certificate trust configured through the shared TLS platform. Plain HTTP is
accepted only for loopback protocol tests.

The integration authorizes D23 smart-home reads before transport I/O, requests
only `/ivp/meters` and `/ivp/meters/readings`, bounds response sizes and
meter counts, matches native records by EID, and installs confirmed aggregate
meter telemetry. The bearer token is zeroized and materialized only while the
transport encodes the Authorization header.

Cloud login, token generation or renewal, legacy gateway authentication,
inverter-serial inspection, live battery or relay topology, and control APIs
remain outside this package.
