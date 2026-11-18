# Changelog

## 0.1.0

- Bind one exact-message retry transcript and message-bound X25519 secret to
  the repository-owned no-PSK TLS 1.3 base-secret schedule.
- Reject message-proof or cipher-suite substitution before base-secret storage
  can be cleared or written.
