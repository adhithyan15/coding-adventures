# hue-core

Philips Hue CLIP v2 resource and mapping primitives for the smart-home runtime.

This crate contains no network I/O. It gives later Hue client and integration
packages a typed surface for:

- Hue resource kinds and ids
- CLIP v2 resource paths
- event stream path constants
- structured Hue command intents
- discovery-to-`Bridge` projection
- Hue light/device-to-normalized-model projection
- Hue light state update-to-`StateDelta` projection
- integration descriptor metadata for Chief of Staff discovery

## Dependencies

- `smart-home-core`

## Development

```bash
bash BUILD
```
