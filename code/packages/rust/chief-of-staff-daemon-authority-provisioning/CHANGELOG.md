# Changelog

## Unreleased

### Removed

- Channel-key provisioning (D18S P2.6d-2c). The daemon reads no channel key
  file: `[data_plane] channel_keys` belongs to the per-agent brokers, and the
  broker launcher opens one agent's key files when it launches that agent's
  broker. Gone with it: `ProvisionedAuthorities::channel_keys`, `into_parts`,
  and the `InvalidHome`, `InvalidIdentity`, `SecretFile` and `ChannelKey`
  errors.

### Changed

- `provision_authorities(config)` takes no home directory, since nothing it
  reads is a file; `into_models()` replaces `into_parts()`.

## 0.1.0

- Add exact typed channel-key provisioning through owner-only, no-link,
  exact-length, zeroizing file reads.
- Add exact model-to-Ollama client provisioning with bounded startup validation
  and no implicit endpoint or network probe.
