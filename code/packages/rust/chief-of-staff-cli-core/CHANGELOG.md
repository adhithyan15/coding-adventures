# Changelog

## 0.1.0

- Add a declarative host-lifecycle command surface over an authenticated Chief daemon client.
- Validate host identity and package hashes before dispatch.
- Keep credentials, endpoints, terminal access, and connection setup outside argv parsing.
- Add deterministic pretty-JSON result rendering.
- Add a typed local `install-daemon` action without introducing filesystem or
  process authority into the command parser.
