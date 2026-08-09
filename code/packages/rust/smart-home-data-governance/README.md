# smart-home-data-governance

Pure host-owned privacy, consent, and telemetry-egress policy for D23 smart-home
integrations.

The policy is deny-by-default. A trusted host may install a bounded grant that
binds one authenticated principal and resource to a data category, operation,
destination, declared purpose, consent receipt, and expiry. Integrations ask the
policy for a decision before transport I/O; model-facing command arguments
cannot create or widen grants.

The categories cover coarse country configuration, device identifiers, and
environmental telemetry. Destinations are the local device, an exact validated
HTTPS origin, or a credential-free `mqtt://` or `mqtts://` broker URI with an
explicit port. Identifier inspection requires an exact grant whose retention is
either ephemeral or a non-zero bounded duration. Telemetry shutdown is always
allowed as a privacy-protective operation, but the ordinary D23 command
authorization layer may still require human approval for the device mutation.

Consent references, purpose text, HTTPS origins, and MQTT broker identities stay
private to the policy and are redacted from `Debug`. Decision records expose
only inert enums and whether a matching grant existed.

```bash
bash BUILD
```
