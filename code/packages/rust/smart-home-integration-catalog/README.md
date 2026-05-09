# smart-home-integration-catalog

First-party smart-home integration and primitive catalog model with seed
entries.

This crate is the executable companion to D23A. It has no network, filesystem,
Vault, radio, serial, or worker-management behavior. It gives the smart-home
runtime and Chief of Staff tools a typed catalog for:

- Home Assistant-style connectivity classes
- integration categories
- implementation status
- discovery and auth metadata
- required primitive-family hints
- required capability hints
- target entity kind hints
- computed D21/D18D policy surfaces for privacy, credentials, cloud accounts,
  local actuation, entry access, radio networks, and infrastructure control
- read-only D18D tool descriptors for listing/describing integrations and
  primitive families
- typed ecosystem-survey source rows that map Home Assistant, Hubitat, Homey
  Pro, SmartThings, openHAB, Homebridge, ioBroker, Domoticz, Jeedom, HomeSeer,
  Apple Home, Google Home, Alexa, Z-Wave Alliance, and Thread Group references
  to reusable primitive-family hints
- primitive backlog planning for prioritizing the shared families needed by a
  rollout wave
- activation plans that resolve direct integrations, virtual aliases, and
  standard-backed products into primitive/capability/auth/policy requirements
- readiness reports that compare activation plans against available primitives,
  allowed capabilities, and already-enabled dependency integrations
- composable bounded catalog queries for D18D read tools that need to combine
  priority, primitive, capability, policy, protocol, local/cloud, and virtual
  alias selectors
- first-party rollout seed entries
- virtual product aliases that point to real implementations or standards

Hue is treated as the trial run for the primitive shape: local discovery,
physical pairing, local token storage, local HTTP reads, event-stream updates,
normalized entity projection, command mapping, health, audit, and tests.

## Dependencies

- `smart-home-core`

## Development

```bash
bash BUILD
```
