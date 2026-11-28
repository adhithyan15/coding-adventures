# Changelog

- Reject duplicate decoded JSON object members during raw replay and reject
  unknown top-level report members while retaining the object-based API.
- Exclude only StandardRB Layout/EndOfLine in the Windows build front to match the repository LF policy.

- Reject more than 10,000 pairs in public analysis before mapping pair fields.
- Use the shared `arch` metadata field and omit private checksum metadata so
  native CLI reports replay in the other language implementations.
- Validate the complete optional metadata group, descriptive/profile strings,
  and invocation strings before projecting core report replay.
- Fall through empty ambient revision labels using the existing GIT_REVISION,
  GITHUB_SHA, unknown precedence; validate produced reports before output.

## 0.1.0

- Add paired batch collection, ratio analysis, report replay and the indexed-array CLI.
- Preserve signed-64-bit integer fields in memory and through JSON.
