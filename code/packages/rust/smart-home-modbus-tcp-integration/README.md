# smart-home-modbus-tcp-integration

This package polls explicitly configured Modbus TCP holding or input registers
and installs them as normalized D23 sensor entities.

The first-party runtime is intentionally read-only. It opens no socket until
the D23 principal is authorized, limits a profile to 64 points, limits each
request to the Modbus read maximum, checks every MBAP correlation field, and
does not expose register-write functions.

The CLI can inspect a contiguous range as unsigned 16-bit values:

```text
smart-home-modbus-tcp-integration inspect 192.0.2.10 1 holding 0 8
```

An optional final argument overrides the default TCP port `502`.
