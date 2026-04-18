# skill-store

Typed skill store built on storage-core

`skill-store` keeps skill manifests and bundled assets in portable storage so a
future skill runtime can load them from local folders, SQLite, NAS storage, or
other backends without changing the skill API.

## What it owns

- `SkillManifest`
- `SkillAssetRecord`
- installation of manifests plus asset bundles
- active-version switching and uninstall semantics

## Key layout

- `skills/manifests/<skill_id>/<version>.json`
- `skills/assets/<skill_id>/<version>/<asset_path>`

## Current API

- `install_skill()`
- `load_manifest()`
- `list_installed_skills()`
- `read_asset()`
- `activate_version()`
- `deactivate_version()`
- `uninstall_skill()`

## Development

```bash
# Run tests
bash BUILD
```
