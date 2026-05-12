# hue-core

Philips Hue CLIP v2 resource and mapping primitives for the smart-home runtime.

This crate contains no network I/O. It gives later Hue client and integration
packages a typed surface for:

- Hue resource kinds and ids
- CLIP v2 resource paths
- event stream path constants
- structured Hue command intents
- Hue application registration requests and discovered-bridge pairing plans
- typed Hue bridge resources for paired bridge identity/health refresh
- typed Hue device resources and service references
- typed Hue grouped-light resources for room/zone aggregate lights
- typed Hue room, zone, and scene resources
- typed Hue motion and button resources for sensor/input surfaces
- discovery-to-`Bridge` projection
- Hue light/device-to-normalized-model projection
- Hue scene-to-normalized-`Scene` projection
- Hue motion/button-to-normalized-`Entity` projection
- Hue light, grouped-light, motion, and button state update-to-`StateDelta`
  projection
- Hue snapshot and scene desired-state values keyed by canonical D23 capability
  ids such as `light.on_off` and `light.brightness`
- integration descriptor metadata for Chief of Staff discovery

## Dependencies

- `smart-home-core`

## Development

```bash
bash BUILD
```
