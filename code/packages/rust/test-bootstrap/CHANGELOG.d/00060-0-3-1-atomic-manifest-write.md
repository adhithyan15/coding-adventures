## 0.3.1

- `manifest::write` (security review A4) now writes to a `create_new` temp file in the target's directory and renames it over the target, and refuses a target that is a symlink (`symlink_metadata`), so a manifest path cannot be redirected onto another file and readers never see a partial manifest.
