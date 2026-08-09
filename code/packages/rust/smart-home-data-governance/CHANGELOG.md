# Changelog

## 0.2.0

- Add exact credential-free MQTT broker destinations for telemetry start/stop
  policy, requiring an explicit port and rejecting userinfo, paths, queries,
  fragments, and unsupported schemes.
- Redact MQTT broker identities from policy debug output and preserve
  privacy-protective shutdown semantics.

## 0.1.0

- Add a bounded, deny-by-default data-governance policy for coarse-location
  configuration and environmental-telemetry egress.
- Bind explicit consent to one principal, resource, action, destination,
  purpose, and validity window without exposing consent references in debug
  output.
- Permit telemetry shutdown as a privacy-protective action without requiring a
  grant that could prevent an operator from stopping disclosure.
