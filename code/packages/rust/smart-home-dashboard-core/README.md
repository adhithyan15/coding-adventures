# smart-home-dashboard-core

Shared native smart-home dashboard manifest contracts.

The package owns the versioned manifest shape used by Home Assistant dashboard
migration and the operational local controller. It accepts either an applied
migration artifact or a raw native manifest, rejects dry-run artifacts, and
validates identifiers before the dashboard is served.

## Development

```bash
bash BUILD
```
