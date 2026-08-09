# Changelog

## 0.5.0

- Add operational telemetry as a distinct consent category for bounded local
  health and performance inspection.
- Prove that operational-telemetry grants match exact retention and remain
  deny-by-default for near-match requests.

## 0.4.0

- Add a host-owned presence category that can be bound to explicit ephemeral or
  bounded retention before a local integration performs inspection.

## 0.3.0

- Add exact device-identifier inspection policy with explicit ephemeral or
  bounded retention semantics.
- Bind retention to grants and requests, reject zero-duration or inapplicable
  identifier retention, and keep inspection deny-by-default.
- Mark configuration and telemetry-egress operations explicitly as having no
  policy-level retention so cross-operation grants remain fail-closed.

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
