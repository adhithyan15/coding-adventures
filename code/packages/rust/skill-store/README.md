# skill-store

Typed skill store built on storage-core

`skill-store` keeps skill manifests and bundled assets in portable storage so a
future skill runtime can load them from local folders, SQLite, NAS storage, or
other backends without changing the skill API.

## What it owns

- `SkillManifest`
- `SkillAssetRecord`
- installation of manifests plus asset bundles
- bounded manifest listing by active status, entrypoint, required tool, and
  required capability
- active-version switching and uninstall semantics

## Key layout

- `skills/manifests/<skill_id>/<version>.json`
- `skills/assets/<skill_id>/<version>/<asset_path>`

## Current API

- `install_skill()`
- `load_manifest()`
- `list_skills()`
- `list_installed_skills()`
- `read_asset()`
- `activate_version()`
- `deactivate_version()`
- `uninstall_skill()`

`SkillListOptions` gives D18/D18D hosts and tool catalogs a narrow read model
over installed manifests. Callers can list active skills, find skills that
require a specific tool or capability, restrict to an entrypoint, or cap the
number of results without loading asset bodies.

## Development

```bash
# Run tests
bash BUILD
```
