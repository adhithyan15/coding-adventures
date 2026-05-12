# skill-store

Typed skill store built on storage-core

`skill-store` keeps skill manifests and bundled assets in portable storage so a
future skill runtime can load them from local folders, SQLite, NAS storage, or
other backends without changing the skill API.

## What it owns

- `SkillManifest`
- `SkillManifestSummary`
- `SkillAssetRecord`
- `SkillAssetSummary`
- `SkillCatalogSummary`
- `SkillRequirementSummary`
- `SkillAssetInventorySummary`
- `SkillSourceSummary`
- installation of manifests plus asset bundles
- bounded manifest listing by active status, entrypoint, required tool, and
  required capability
- metadata-only summary reads for catalog and `skill.list` surfaces
- metadata-only asset summary reads for catalog and preload planning surfaces
- compact catalog summaries for installed versions, active state, and stored
  asset material
- compact requirement summaries for entrypoint, tool, and capability coverage
- compact asset inventory summaries for content-type, path-depth, and byte
  coverage over one skill version
- compact source summaries for manifest `source.kind` provenance coverage
- active-version switching and uninstall semantics

## Key layout

- `skills/manifests/<skill_id>/<version>.json`
- `skills/assets/<skill_id>/<version>/<asset_path>`

## Current API

- `install_skill()`
- `load_manifest()`
- `load_manifest_summary()`
- `list_skills()`
- `list_skill_summaries()`
- `catalog_summary()`
- `requirement_summary()`
- `asset_inventory_summary()`
- `source_summary()`
- `list_asset_summaries()`
- `list_installed_skills()`
- `read_asset()`
- `activate_version()`
- `deactivate_version()`
- `uninstall_skill()`

`SkillListOptions` gives D18/D18D hosts and tool catalogs a narrow read model
over installed manifests. Callers can list active skills, find skills that
require a specific tool or capability, restrict to an entrypoint, or cap the
number of results without loading asset bodies.

`SkillManifestSummary` is the model-facing catalog projection for listing and
selection. It keeps names, descriptions, entrypoints, tool/capability
requirements, asset counts, version, and active status while leaving raw
manifest `source` and asset bytes behind the explicit manifest/asset reads.

`SkillAssetListOptions` gives D18D catalog and preload tools a bounded asset
read model. Callers can list asset metadata for one skill version, narrow by
logical path prefix or content type, and cap the number of returned summaries
without materializing asset bodies.

`SkillCatalogSummary` gives hosts and catalog tools a compact rollup across
filtered skill versions, active state, manifest asset references, and stored
asset material without returning raw manifests or asset bodies.

`SkillRequirementSummary` gives D18D tool catalogs and capability cages a
compact rollup across filtered skill versions, entrypoints, required tools,
required capabilities, and requirement gaps without returning raw manifests or
asset bodies.

`SkillAssetInventorySummary` gives catalog and preload planners a compact
rollup over one skill version's asset metadata, including content-type classes,
root versus nested paths, and total bytes without returning asset bodies.

`SkillSourceSummary` gives D18/D18D hosts a compact provenance rollup across
filtered skill versions, including specified versus unspecified `source.kind`
coverage and active/inactive counts per source kind without returning raw
manifest source payloads.

## Development

```bash
# Run tests
bash BUILD
```
