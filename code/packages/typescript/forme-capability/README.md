# @coding-adventures/forme-capability

Forme kernel capability layer — capability strings, parser, and host-aware matcher.

See [code/specs/FM01-forme-kernel.md](../../../specs/FM01-forme-kernel.md) §5 for the design.

## Capability format

A capability is a colon-separated string with **two or three segments**:

```
<realm>:<scope>[:<detail>]
```

Examples:

| Capability                   | Meaning                                         |
| ---------------------------- | ----------------------------------------------- |
| `storage:read`               | Read from the pipeline's storage root.          |
| `storage:write`              | Write to the pipeline's storage root.           |
| `network:*`                  | Talk to any host (sensitive — warn on install). |
| `network:api.github.com`     | Talk to api.github.com and its subdomains.      |
| `network:*.google.com`       | Talk to any subdomain of google.com.            |
| `network:https:foo.com`      | Scheme-restricted: only HTTPS to foo.com.       |
| `env:GITHUB_TOKEN`           | Read one named env var.                         |
| `env:*`                      | Read any env var (sensitive).                   |
| `filesystem:user`            | Reach into arbitrary user paths (sensitive).    |
| `system:shell`               | Run shell commands (first-party only).          |

## API

```typescript
import {
  parseCapability, tryParseCapability,
  matchesCapability,
  KERNEL_REALMS, isFirstPartyOnly, isSensitive,
} from "@coding-adventures/forme-capability";
```

| Function                              | Purpose                                                  |
| ------------------------------------- | -------------------------------------------------------- |
| `parseCapability(s)`                  | Throws `RangeError` on malformed input.                  |
| `tryParseCapability(s)`               | Returns `null` on malformed input.                       |
| `matchesCapability(declared, requested)` | Returns `true` if the declared cap covers the requested. |
| `isKernelRealm(realm)`                | Predicate: is this a kernel-blessed realm name?          |
| `isFirstPartyOnly(cap)`               | Predicate: is this cap reserved for first-party stages?  |
| `isSensitive(cap)`                    | Predicate: should the install UX warn loudly?            |

## Matching rules

1. Realms must match exactly. No cross-realm coverage.
2. Scope `*` covers any scope/detail in the same realm.
3. Detail `*` covers any non-null detail at the same scope.
4. **Network host hierarchy**: `network:foo.com` covers `foo.com` and any subdomain. `network:*.foo.com` covers subdomains only (not the bare host). DNS comparisons are case-insensitive.
5. Other realms use exact-scope matching only — no hierarchy.
6. 2-segment and 3-segment forms are not interchangeable (a scheme-restricted request is not covered by a non-scheme-restricted declaration, and vice versa).
7. Malformed inputs return `false` from `matchesCapability` rather than throwing — enforcement-time call sites stay clean.

## Coverage

```bash
npm install
npx vitest run --coverage
```

Targets 100% line + branch.
