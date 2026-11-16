---
category: Security boundaries
---

# An audit label cannot supply missing credential provenance

A request encoder accepted raw credential-bearing HTTP fields while the audit
provider and trace were independent caller inputs. Exact field-name policy and
audit-before-copy ordering did not prevent provider B's credential bytes from
being released under provider A's success event. A credential release must
consume a non-forgeable proof that already retains the exact provider, trace,
request, and value from custody or signing. If that proof does not belong in the
current dependency layer, keep credential-bearing and custom fields blocked and
release only closed public metadata whose exact name and value are retained by
provider policy.
