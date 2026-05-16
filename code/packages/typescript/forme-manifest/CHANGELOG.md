# Changelog — @coding-adventures/forme-manifest

## 0.1.0 — 2026-05-16

Initial release. First FM02 package — the manifest layer that
`forme-plugin-host` will consume. Implements FM02 §3 (`plugin.toml`
format), §3.4 (`$variable` templating), §3.5 (manifest hash).

### Added

- **`Manifest` and supporting interfaces** — one TypeScript interface
  per `[section]` of `plugin.toml`: `PluginIdentity`, `RuntimeSpec`,
  `CapabilityEntry`, `StageContribution`, `KindContribution`,
  `ResourceLimits`, `SignatureBlock`. All fields `readonly` end-to-end.
- **`parseManifest(text)`** — hand-rolled TOML subset parser. Supports
  the surface FM02 §3.2 names: string values (single/double-quoted),
  integers, booleans, dotted keys, `[section]` / `[sub.section]`
  headers, `[[array.of.tables]]`, inline arrays of strings, `#`
  comments. Explicitly rejects multi-line strings, floats, datetime
  literals, inline tables — none of which appear in plugin manifests.
- **`validateManifest(manifest)`** — implements every numbered rule
  from FM02 §3.3. Aggregates every violation into a single
  `ManifestError` with structured `errors[]` rather than throwing on
  the first one (the FM03 §2.4 `ConfigError` precedent).
- **`canonicalManifestToml(manifest)`** — byte-stable TOML serialiser.
  Keys sorted at every level; one section per line; values
  canonically formatted. Excludes the `[signature]` section — that's
  the document being signed, not part of it.
- **`computeManifestHash(manifest, entryBytes)`** — BLAKE2b-256 over
  `canonicalManifestToml(manifest) ++ "\0" ++ entryBytes`. Returns
  `"blake2b:<64 hex>"`. The null-byte separator prevents
  manifest-truncation attacks (a forged manifest cannot inherit a
  legitimate signature by extending into the entry).
- **`signManifest(manifest, entryBytes, secretSeed)`** — Ed25519 sign
  over the manifest hash. Returns a `SignatureBlock` ready to drop
  into the manifest.
- **`verifyManifest(manifest, entryBytes)`** — reads `manifest.signature`,
  recomputes the hash, verifies the signature. Returns `true`/`false`
  rather than throwing — tamper detection is a routine outcome the
  host loop handles, not an exception.
- **`resolveCapabilityTemplate(capString, env)`** — substitutes
  `$storageRoot`, `$cacheDir`, `$pluginDir`. `$$` is the literal-dollar
  escape. Unknown variables throw `ManifestError` rather than
  passing through — silent passthrough would defeat the whole point
  of validating at install time.
- **`ManifestError`** — structured error type with `code`, `path`
  (JSON-path-like field locator), `message`, `errors[]` (when used as
  an aggregate by the validator).
- **`MANIFEST_ERROR_CODES`** — frozen vocabulary of every code the
  validator and parser emit.

### Spec adherence

No deliberate divergences from FM02 §3. v0 simplifications:

- **TOML subset**: only the syntax `plugin.toml` actually uses is
  accepted. A future revision can broaden if needed; for now, narrow
  is safer.
- **Hash separator**: §3.5 reads "concatenated with BLAKE2b-256 of
  the entry file"; this implementation interposes a single `\0` byte
  to defeat extension attacks. The byte is documented and stable.
- **Signature scheme**: only Ed25519 is implemented; the manifest's
  `algorithm` field is checked but no other algorithm is accepted.
  Future revisions may add Ed448 / ECDSA-P256 / Sigstore-style
  keyless signing.

### Notes

- `parseManifest` produces a structurally valid `Manifest` value even
  for inputs that fail `validateManifest`. The split exists so the
  CLI can surface "this manifest parses but is invalid" diagnostics
  with field-level locations.
- The `$variable` resolver intentionally rejects unknown variables
  rather than treating them as literal — it's an opt-in template,
  not a string-formatter.
