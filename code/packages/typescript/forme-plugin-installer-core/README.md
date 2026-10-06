# @coding-adventures/forme-plugin-installer-core

Registry-independent validation and atomic filesystem installation for complete
Forme plugin snapshots. This package implements the FM-B049 core boundary: a
registry, local archive reader, or test fixture supplies bounded path-and-byte
rows; the core validates the full snapshot, resolves reviewed capabilities,
generates the manifest-bound `grants.toml`, and publishes one complete plugin
directory beneath an explicit host-owned install root.

## Public API

```ts
const prepared = preparePluginInstallSnapshot({
  installRoot: "/srv/forme/plugins",
  files: packageSnapshot,
  trustStore,
  reviewedGrants,
  capabilityEnvironment: {
    storageRoot: "/srv/site/content",
    cacheDir: "/srv/site/.cache",
  },
});

const result = await installPreparedPlugin({
  prepared,
  mode: "replace", // or "immutable"
  signal,
});
```

`preparePluginInstallSnapshot` accepts at most 4,096 files, 4,096 distinct
directories, 256 directory levels, 16 MiB per file, 128 MiB in aggregate, and
4,096 reviewed grants. It defensively copies every byte row, rejects
absolute/traversing/control/non-NFC/non-portable paths, case-fold and prefix
collisions, the case-insensitive `grants.toml` namespace, missing selected runtime entries, and
missing declared schemas. Required reviewed grants must be present; every
reviewed grant must be declared by the manifest after template resolution.
Templated paths use reversible URI-path encoding, so Windows drive paths,
backslashes, whitespace, and controls stay inside one capability detail segment
without aliasing distinct POSIX names. Destination and transaction basenames are
bounded below common 255-byte filesystem limits.

The current FM02 signature authenticates canonical manifest semantics plus the
one selected runtime entry. Consequently, only a trusted signed package whose
distribution contains exactly `plugin.toml` and that selected entry receives
the `verified-third-party` tier. A package with any auxiliary or alternate
platform file remains `unverified-third-party`; an invalid declared signature
fails installation. Signed manifests that reference external schemas remain
unsupported until a later package-signature version binds every file.

`installPreparedPlugin` requires an existing canonical, real host-owned root.
Callers must resolve platform aliases such as macOS `/tmp` before preparation so
`$pluginDir` grants bind the same canonical path discovery will observe. It acquires
an exclusive per-plugin directory lock, writes restrictive files into a private
same-parent stage, revalidates the staged tree, and swaps it into place with a
private backup for rollback. Existing symlinks, hardlinks, special files,
unsafe names, identity changes, and concurrent locks fail closed. An exact
reinstall is `unchanged`; immutable mode rejects a different existing target;
replace mode restores the old target if commit fails or cancellation arrives
after backup. Existing-tree inspection has independent directory, file, depth,
and aggregate-entry ceilings, including for directory-only trees.

POSIX roots and every accepted existing target directory/file must not be
group- or world-writable. Windows callers must supply `verifyWindowsAcl`, which
independently verifies both the canonical install root and, when present, the
complete existing target tree exclude untrusted writers; installation fails
closed without it. Every created lock, stage, and backup is rechecked against
the root owner and device.

## Development

```bash
sh BUILD
```

The suite covers validation, trust classification, nested publication,
idempotence, replacement, rollback, cancellation, unsafe filesystem entries,
and resource ceilings with at least 95% branch coverage.

This package implements the atomic installer contract in
[FM02 §4.2](../../../specs/FM02-forme-plugin-host.md).
