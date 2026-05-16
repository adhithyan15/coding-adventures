# @coding-adventures/forme-manifest

Forme plugin manifest layer: parses `plugin.toml`, validates per FM02 §3.3,
serialises canonically for signing/hashing, computes a content hash that
includes the entry file, signs/verifies via Ed25519, and resolves
`$variable` templates against the runtime environment.

The first FM02 package. Implements the schema; nothing else (loading,
sandboxing, wire protocol) lives here. Consumed by `forme-plugin-host`.

## What's in the box

```typescript
// Types — one interface per [section] in plugin.toml
import type {
  Manifest, PluginIdentity, RuntimeSpec, CapabilityEntry,
  StageContribution, KindContribution, ResourceLimits, SignatureBlock,
} from "@coding-adventures/forme-manifest";

// Parsing — strict TOML subset; rejects unknown keys, malformed values
import { parseManifest } from "@coding-adventures/forme-manifest";
const manifest = parseManifest(tomlText);

// Validation — every FM02 §3.3 rule, surfacing every violation
import { validateManifest } from "@coding-adventures/forme-manifest";
validateManifest(manifest);  // throws ManifestError on the first violation;
                             // .errors[] carries all of them

// Canonical encoding — byte-stable serialisation for hashing/signing
import { canonicalManifestToml } from "@coding-adventures/forme-manifest";
const bytes = new TextEncoder().encode(canonicalManifestToml(manifest));

// Hash — BLAKE2b-256 over (canonical manifest minus [signature]) ++ entry hash
import { computeManifestHash } from "@coding-adventures/forme-manifest";
const hash = computeManifestHash(manifest, entryFileBytes);
// → "blake2b:<64 hex>"

// Signature — Ed25519 over the manifest hash
import { signManifest, verifyManifest } from "@coding-adventures/forme-manifest";
const sig = signManifest(manifest, entryFileBytes, secretKey);
const valid = verifyManifest(manifest, entryFileBytes); // reads [signature] block

// Templating — resolve $storageRoot / $cacheDir / $pluginDir at install time
import { resolveCapabilityTemplate } from "@coding-adventures/forme-manifest";
const resolved = resolveCapabilityTemplate("filesystem:read:$storageRoot", {
  storageRoot: "/abs/path",
  cacheDir:    null,
  pluginDir:   "/path/to/plugin",
});
```

## Design notes

### Why a hand-rolled TOML parser

The monorepo's other parsers (gfm-parser, csv-parser, etc.) are all
from-scratch implementations. Pulling a TOML library would diverge from
that pattern and add an audit surface for crypto-adjacent code. The
parser here implements a strict TOML subset — exactly what `plugin.toml`
needs and nothing more (no datetime literals, no float, no multi-line
strings, no inline tables). Rejecting unsupported syntax with a clear
error message is preferable to silently accepting it and producing
nonsense.

### Why BLAKE2b not BLAKE3

Same reason as `forme-identity` (FM01 §7). The monorepo has a from-scratch
BLAKE2b but no BLAKE3. The `RevisionId` format prefixes the algorithm
(`blake2b:<hex>`) so a future migration is just a prefix change.

### Why a separate manifest hash vs FM01 RevisionId

A `RevisionId` (FM01) hashes a `JsonValue`. The manifest hash needs to
hash a TOML document — which is canonically distinct from JSON — plus
the entry-file bytes. Same algorithm, different inputs, so we keep them
separate to avoid cross-confusion at the call site.

### What about FM02 §3.4 `$variable` rules

Only three variables are recognised: `$storageRoot`, `$cacheDir`,
`$pluginDir`. Unrecognised variables fail validation. `$$` is the
literal-dollar escape. This is intentionally minimal — it's NOT a
template engine; it's enough scoping syntax for a plugin's filesystem
grants to inherit the user's project root without the plugin knowing
that path.

## Status

v0.1.0 — implements FM02 §3 (manifest format), §3.4 (templating),
§3.5 (hash). Sign/verify via the existing `@coding-adventures/ed25519`.

Not yet implemented (will land with `forme-plugin-host`):
- `forme install` flow (FM02 §4.2) — CLI concern, lives in FM07.
- Trust store (`~/.forme/trust.toml`) — also CLI.
- Grants file (`grants.toml`) — host concern; the host reads/writes
  using the types this package exports.

## See also

- `code/specs/FM02-forme-plugin-host.md` — the full spec
- `@coding-adventures/forme-capability` — the `Capability` type and
  parser
- `@coding-adventures/blake2b`, `@coding-adventures/ed25519` — the
  crypto primitives
