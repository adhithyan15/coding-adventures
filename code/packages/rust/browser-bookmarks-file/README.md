# browser-bookmarks-file

Crash-safe native file adapter for `browser-bookmarks`.

The adapter stores a bounded, explicitly versioned JSON document. Saves write
an owner-only temporary file in the destination directory, synchronize it,
atomically replace the destination, and synchronize the directory where the
platform supports it. A failed candidate write leaves the previous catalog
untouched. Unknown schema versions, duplicate canonical URLs, symlinks, and
oversized documents are rejected.

`default_bookmark_path` follows native profile conventions and honors
`VENTURE_BOOKMARKS_PATH` for portable applications and deterministic tests:

- macOS: `~/Library/Application Support/Venture/bookmarks.json`
- Windows: `%LOCALAPPDATA%/Venture/bookmarks.json`
- other desktop Unix: `$XDG_DATA_HOME/venture/bookmarks.json`, falling back to
  `~/.local/share/venture/bookmarks.json`
