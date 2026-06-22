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
- activation evidence lane inventories that group blocked integration rows by
  first-blocked catalog, activation-plan, readiness, policy, or local-boundary
  lane for reusable release planning
- activation evidence remediation items that turn lane inventories into a
  sorted catalog-owned work plan before Chief-specific escalation or response
  tooling
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
- low-level Zigbee, Z-Wave, and Thread primitive-readiness rows and summaries
  for checking radio substrate, controller, network-key, and supervision gaps
- low-level mesh substrate stage rows that classify required primitives into
  controller, radio, discovery, network-security, and supervision blockers
- low-level mesh substrate action queues that order missing stage primitives
  into concrete protocol-scoped substrate work
- low-level mesh preflight repair actions that translate failed substrate
  gates into concrete protocol-scoped remediation work
- low-level mesh preflight repair batches that group failed substrate gates by
  stage and action kind
- low-level mesh preflight repair schedules that order repair batches into
  deterministic execution slots
- low-level mesh preflight repair slot audits that expose per-slot blocker and
  operator handoff risk
- low-level mesh preflight repair slot execution tickets that turn audited
  slots into operator-ready execution work
- low-level mesh preflight repair slot execution work orders that project
  execution tickets into deterministic release work
- low-level mesh preflight repair slot execution work-order guardrails that
  classify release blockers, operator handoffs, and ready-to-execute work
- mesh readiness package summaries that combine Zigbee, Z-Wave, and Thread
  primitive substrate readiness with mesh-scoped activation evidence
  remediation rollups
- mesh stage release summaries that combine substrate-stage blockers,
  primitive-readiness blockers, and mesh-scoped remediation into one release
  readiness rollup
- mesh action readiness summaries that combine release readiness with the
  substrate action queue and next concrete low-level action
- mesh preflight repair readiness summaries that combine release readiness,
  substrate preflight gates, and protocol-scoped repair queues
- mesh preflight batch readiness summaries that combine repair readiness with
  stage/action-kind repair batches
- mesh preflight schedule readiness summaries that combine repair-batch
  readiness with deterministic repair execution slots
- mesh preflight slot readiness summaries that combine schedule readiness with
  slot-audit blocker and operator handoff counts
- mesh preflight execution readiness summaries that combine slot readiness
  with operator-ready execution ticket counts
- mesh preflight work-order readiness summaries that combine execution
  readiness with release work-order counts
- mesh repair-slot execution evidence review disposition actions that turn
  review dispositions into operator, repair, lineage, and release queues
- low-level mesh disposition-action execution slots that sequence those
  queues into deterministic operator and repair work
- mesh preflight guardrail readiness summaries that combine work-order
  readiness with guardrail and evidence-review disposition counts
- mesh preflight disposition-action readiness summaries that combine guardrail
  readiness with evidence disposition action queues
- mesh readiness handoff packages that project substrate actions, evidence
  remediation, and release-ready state for reusable release coordination
- mesh release-readiness checks that summarize substrate action, evidence,
  human-review, operator handoff, and release packet gates
- low-level mesh release-readiness check slots that sequence those gates into
  deterministic operator and release execution work
- mesh release packet readiness summaries that condense release-readiness
  checks into package-facing go/no-go state and first actionable gates
- mesh release execution readiness summaries that combine release packet
  go/no-go state with release-readiness check slot execution state
- low-level mesh release execution tasks that turn release-readiness check
  slots into deterministic operator work
- mesh release task readiness summaries that combine execution readiness with
  deterministic task state
- low-level mesh release task dispatch slots that turn execution tasks into
  deterministic dispatch queues
- mesh release dispatch readiness summaries that combine task readiness with
  dispatch queue state
- low-level mesh release dispatch tickets that wrap dispatch queues in
  deterministic ticket keys
- low-level mesh release dispatch ticket handoff packets that classify ticket
  dispatch into release, operator, repair, and review lanes
- mesh release ticket readiness summaries that combine release dispatch
  readiness with deterministic ticket state
- mesh release ticket handoff readiness summaries that combine ticket
  readiness with dispatch handoff lane state
- low-level mesh release ticket handoff execution slots that sequence handoff
  packets into deterministic lane execution work
- mesh release ticket handoff execution readiness summaries that combine
  handoff readiness with deterministic execution slot state
- low-level mesh release ticket handoff execution work orders that turn
  execution slots into lane-scoped operator work
- mesh release ticket handoff work-order readiness summaries that combine
  handoff execution readiness with deterministic work-order lane state
- low-level mesh release ticket handoff execution work-order guardrails that
  classify lane work into release blockers, operator handoffs, review gates,
  and ready-to-execute checks
- low-level mesh release ticket handoff execution work-order guardrail audit
  rows that expose release blockers, operator handoffs, review gates, and
  ready-to-execute checks for audit/release coordination
- low-level mesh release ticket handoff execution work-order guardrail audit
  clearance rows that turn audit findings into deterministic clear/block,
  review, and operator handoff decisions
- low-level mesh release ticket handoff execution work-order guardrail audit
  clearance action rows that turn clearance decisions into repair, review,
  handoff, or release-clearance work
- low-level mesh release ticket handoff execution work-order guardrail audit
  clearance action evidence rows that preserve clearance action lineage for
  blocker, operator, review, and release evidence
- low-level mesh release ticket handoff execution work-order guardrail audit
  clearance action evidence review rows that classify evidence into blocker,
  operator, review, lineage, and release-ready outcomes
- low-level mesh release ticket handoff execution work-order guardrail audit
  clearance action evidence review disposition rows that route reviewed
  evidence into repair, operator, review, lineage, and release-ready outcomes
- low-level mesh release ticket handoff execution work-order guardrail audit
  clearance action evidence review disposition action rows that turn review
  dispositions into repair, operator, review, lineage, and release handoff work
- mesh release ticket handoff guardrail audit clearance action evidence
  readiness summaries that combine clearance action readiness with evidence
  lineage state and first actionable evidence pointers
- mesh release ticket handoff guardrail audit clearance action evidence review
  readiness summaries that combine action-evidence readiness with evidence
  review outcomes and next review pointers
- mesh release ticket handoff guardrail audit clearance action evidence review
  disposition readiness summaries that combine evidence-review readiness with
  disposition outcomes and next disposition pointers
- mesh release ticket handoff work-order guardrail readiness summaries that
  combine work-order readiness with guardrail counts and first actionable
  handoff work
- mesh release ticket handoff guardrail audit readiness summaries that combine
  work-order guardrail readiness with audit-row counts and first actionable
  audit lineage
- mesh release ticket handoff guardrail audit clearance readiness summaries
  that combine audit readiness with deterministic clearance rows and first
  clearance lineage
- mesh release ticket handoff guardrail audit clearance action readiness
  summaries that combine clearance readiness with action rows and next
  actionable release handoff pointers
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
