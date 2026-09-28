# x509-extension (Swift)

Decodes the generic RFC 5280 `Extension` container with shared `DerAsn1`
budgets, omitted-default critical handling, opaque value bytes, and stable
payload-free errors. It owns no registry, policy, trust, or crypto.

The native suite consumes all 48 language-neutral v1 cases.
