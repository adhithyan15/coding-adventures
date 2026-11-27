# Changelog

- Reject more than 10,000 pairs in public analysis before mapping pair fields.
- Reject inherited Object member names as CLI options; only explicitly
  declared flags may update benchmark configuration.
- Validate the complete optional metadata group, descriptive/profile strings,
  and invocation strings before projecting core report replay.
- Fall through empty ambient revision labels using the existing GIT_REVISION,
  GITHUB_SHA, unknown precedence; validate produced reports before output.

## 0.1.0

- Add paired observation analysis, deterministic collection, report replay, and indexed-array driver.
- Preserve signed-64-bit integer values through lossless JSON and BigInt collection.
