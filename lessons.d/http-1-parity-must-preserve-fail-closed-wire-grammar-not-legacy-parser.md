---
category: Workspace & package metadata
---

# HTTP/1 parity must preserve fail-closed wire grammar, not legacy parser permissiveness

Reject TE+CL ambiguity, conflicting duplicate lengths, non-final request chunking, whitespace before a field colon, variable start-line delimiters, and unbounded heads; response framing must receive HEAD/CONNECT request context, and parse errors must not retain raw targets or field values.
