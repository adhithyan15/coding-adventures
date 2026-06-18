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
- catalog entry summaries that compact integration package shape, local/cloud
  boundaries, policy tier, and metadata counts, including a Hue trial-run helper
- activation package summaries that join catalog entry metadata with activation
  plan prerequisites and target shape, including a Hue trial-run helper
- readiness package summaries that combine Hue activation-package shape with
  host-specific primitive, capability, and dependency blockers
- activation evidence briefing summaries that combine catalog, activation-plan,
  readiness, policy, and local-boundary lanes into compact release evidence
  briefs, including a Hue trial-run helper
- activation evidence scorecard summaries that roll integration evidence
  briefings into catalog-wide ready, blocked, lane, local/cloud, and
  policy-tier counts for Chief planning
- activation evidence rows that project each integration briefing into a
  compact sorted ready/blocker row with the next blocked evidence lane and
  missing-prerequisite counts
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
- activation runbook entries that join playbook steps to audit, risk,
  dependency, and readiness-gap context
- activation handoff packages, execution packets, and verification checkpoints
  that carry runbook context through execution-transfer and post-execution
  readiness checks
- activation operator tasks that turn playbook steps into actionable
  human/operator queue rows
- activation control-room panels that group operator tasks by recommended
  planning view for compact attention, blocker, review, and activation rollups
- activation command-center sections that group control-room panels into
  blocker, review, activation, actionable, and monitoring operating lanes
- activation watchtower signals that roll command-center sections into
  escalation, review, ready, action, and observation signal lanes
- activation sentinel alerts that combine watchtower, risk, dependency, and
  readiness-gap rollups into blocker, dependency, policy-risk, review, ready,
  and observation lanes
- activation audit records that connect sentinel alerts to watchtower signals,
  decisions, evidence, policy risk, dependency blockers, and readiness gaps
- activation escalation cases that package sentinel alerts, verification
  checkpoints, and audit records into Chief-facing blocker, dependency,
  policy-risk, review, verification, and audit cases
- activation response items that turn escalation cases into owner-lane next
  actions for blocker, dependency, policy, review, verification, and audit
  follow-up work
- activation remediation work orders that assign response items to executable
  owner-lane queues with blocked, owner-action, ready-to-execute, and tracking
  status
- activation closure gates that turn remediation work orders into compact
  blocked, owner-action, verification-ready, ready-to-close, and tracking gates
- activation release packets that turn closure gates into compact go/no-go
  release views with blockers, verification requirements, and release readiness
- activation delivery manifests that turn release packets into compact delivery
  views with channel, blocker, verification, and delivery-readiness rollups
- activation deployment records that turn delivery manifests into compact
  deployment views with ring, blocker, verification, and deploy-readiness
  rollups
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
