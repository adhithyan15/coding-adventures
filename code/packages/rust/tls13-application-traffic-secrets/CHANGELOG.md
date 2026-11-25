# Changelog

## 0.1.0

- Derive client and server TLS 1.3 application traffic secrets from one
  suite-bound base schedule and transcript snapshot.
- Retain both secrets in caller-owned fixed wipe-on-drop storage and expose
  them only through fresh success evidence.
