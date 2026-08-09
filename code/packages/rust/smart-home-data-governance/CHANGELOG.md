# Changelog

## 0.1.0

- Add a bounded, deny-by-default data-governance policy for coarse-location
  configuration and environmental-telemetry egress.
- Bind explicit consent to one principal, resource, action, destination,
  purpose, and validity window without exposing consent references in debug
  output.
- Permit telemetry shutdown as a privacy-protective action without requiring a
  grant that could prevent an operator from stopping disclosure.
