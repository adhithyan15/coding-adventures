## 0.2.3 - portable fuzz frame basenames (VER06i)

### Fixed
- Frame basenames now split on both slash styles independently of the host OS,
  preserving colons within POSIX leaf names before the documented sanitization.
- Trailing separators and POSIX or drive roots have deterministic frame names.
