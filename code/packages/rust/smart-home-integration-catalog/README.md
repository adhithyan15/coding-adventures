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
- policy-surface inventory and summary rollups for review, cloud, local, and
  privilege-tier planning
- read-only D18D tool descriptors for listing/describing integrations and
  primitive families
- typed ecosystem-survey source rows that map Home Assistant, Hubitat, Homey
  Pro, SmartThings, openHAB, Homebridge, ioBroker, Domoticz, Jeedom, HomeSeer,
  Apple Home, Google Home, Alexa, Z-Wave Alliance, and Thread Group references
  to reusable primitive-family hints
- ecosystem primitive coverage reports that connect those survey sources to
  rollout backlog primitives
- primitive coverage summaries that identify uncovered, single-source, and
  multi-platform primitive-backlog rows
- ecosystem platform coverage rollups that show which surveyed platform
  lessons overlap a priority-bounded reusable primitive backlog
- primitive backlog planning for prioritizing the shared families needed by a
  rollout wave
- activation plans that resolve direct integrations, virtual aliases, and
  standard-backed products into primitive/capability/auth/policy requirements
- activation plan summaries that count direct/delegated targets, review work,
  local/cloud boundaries, dependencies, and unique primitive/capability needs
- activation candidates that rank ready, human-review, and blocked rollout work
  after applying host-specific primitive, capability, and dependency context
- activation actions that turn ready/review/blocker candidates into concrete
  activate, policy-review, primitive, capability, and dependency work items
- activation agenda stages that combine candidates and concrete actions by
  rollout priority wave for bounded Chief-of-Staff planning
- activation runway stages that group those candidates by rollout priority wave
  with compact ready, review, and blocker rollups
- activation health stages that add priority-wave ready, review, blocked, and
  missing-prerequisite gap rollups for compact platform status inspection
- activation maintenance windows that combine priority-wave health, concrete
  actions, constraints, policy risk, and dependency blockers for Chief planning
- activation constraints that group unresolved primitive, capability,
  dependency, and policy-review surface work by affected integrations
- activation review queue entries that separate review-ready integrations from
  human-review integrations still blocked by primitives, capabilities, or
  dependencies
- activation approval packets that bundle each human-review row with its
  concrete actions, grouped constraints, policy risk, and dependency blockers
  for approval preparation
- activation decision rows that project approval packets into ready-to-approve
  and prerequisite-blocked queues for Chief planning
- activation evidence rows that explain approval decisions with blocker,
  policy-review, risk, and dependency evidence
- activation dossier rows that bundle approval decisions with their evidence
  rows and compact evidence rollups for Chief planning
- activation readout rows that combine priority-wave health, maintenance,
  dossier, evidence, risk, action, and dependency blocker rollups
- activation briefing rows that split readouts into Chief-ready activation,
  approval, review, blocker, risk, and dependency briefing sections
- activation dashboard cards that condense readouts and briefing rows into
  priority-wave Chief status cards
- activation timeline milestones that order dashboard cards into a Chief-ready
  wave sequence
- activation forecast rows that classify timeline milestones into Chief-ready
  next actions for blockers, dependencies, approvals, reviews, risks,
  activation, and monitoring
- activation playbook steps that pair forecast next actions with recommended
  planning views and operator-readiness flags
- activation operator tasks that turn playbook steps into actionable
  human/operator queue rows
- activation control-room panels that group operator tasks by recommended
  planning view for compact attention, blocker, review, and activation rollups
- activation risk rows that group rollout candidates by policy tier and policy
  surface after applying host-specific readiness context
- activation dependency graphs that expose prerequisite nodes, satisfied edges,
  and blocking edges after applying host-specific enabled-integration context
- readiness reports that compare activation plans against available primitives,
  allowed capabilities, and already-enabled dependency integrations
- readiness summaries that roll activation blockers, review requirements, and
  delegated targets into compact planner counts
- readiness gap inventories that group missing primitives, capability grants,
  and delegated integration dependencies by affected integrations
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
