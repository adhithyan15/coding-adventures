# smart-home-modbus-tcp-integration

This package polls explicitly configured Modbus TCP holding or input registers
and installs them as normalized D23 sensor entities. An explicit identity mode
also reads the standard mandatory vendor name, product code, and revision and
uses those native values for the normalized device.

The first-party runtime is intentionally read-only. It opens no socket until
the D23 principal is authorized, limits a profile to 64 points, limits each
request to the Modbus read maximum, and checks every MBAP correlation field.
Identity mode follows at most three correlated basic-identification pages over
one bounded connection before reading registers. Configuration accepts only a
private, link-local, or loopback IP literal and non-zero port; DNS and public
endpoints are rejected. The runtime exposes no register-write function.

The CLI can inspect a contiguous range as unsigned 16-bit values:

```text
smart-home-modbus-tcp-integration inspect 192.168.1.20 1 holding 0 8
```

Use `inspect-identity` instead of `inspect` to precede the register read with
the fixed native identity exchange. An optional final argument overrides the
default TCP port `502`.
