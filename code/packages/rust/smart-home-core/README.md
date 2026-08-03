# smart-home-core

Repository-owned normalized smart-home model shared by integrations, tools, and
Chief of Staff agents.

This crate is the D23 common vocabulary. Hue, Zigbee, Z-Wave, Thread, Matter,
MQTT, and future adapters project into these same records:

- `Bridge`
- `Device`
- `Entity`
- `Capability`
- `DeviceEvent`
- `DeviceCommand`
- `CommandResult`
- `Scene`
- `StateSnapshot`
- `IntegrationDescriptor`
- `SmartHomeTool` / `ToolDescriptor`
- `CapabilityGrant`
- `AuthorizationDecision`
- `AuthorizationDecisionSummary`
- `AuthorizationDecisionLogSummary`

Protocol-private identifiers stay in `ProtocolIdentifier` records rather than
becoming repository-owned entity ids.

## Scope

Current scope:

- normalized bridge/device/entity records
- capability and value typing
- capability-surface summaries for describe-capabilities tools
- inventory summaries for bridge/device health and entity state coverage
- canonical capability catalog entries for light, scene, lock, climate, media,
  device indicator/display/configuration, sensor, calibration, and input families
- typed media playback, volume, grouping, and queue operations inside the
  existing authorized device-command envelope
- typed device indicator/display and sensor-calibration operations with
  command-capability and policy-tier mappings
- typed non-credential device configuration operations for display standards,
  learning settings, self-tests, and correction profiles
- canonical integration descriptors for Hue, Zigbee, Z-Wave, Thread, Matter,
  and MQTT bootstrap families
- compact integration catalog summaries for runtime, discovery, pairing, and
  capability coverage
- per-integration surface summaries for adapter capability, discovery, and
  pairing introspection
- immutable device events and command requests
- health and command-result status helpers for supervision/read-side loops
- command risk tier helpers
- state freshness helpers
- D18D-style smart-home tool descriptors
- D18D integration-catalog descriptors for model-facing integration and
  reusable-primitive inspection
- D18D catalog-summary descriptors for compact integration and tool surface
  inspection
- D18D integration policy-surface descriptors for read-only review, cloud,
  local, and privilege-tier planning rollups
- D18D integration activation-plan descriptors for bulk rollout plan listing
  and compact activation-plan rollups
- D18D integration activation-candidate descriptors for ranked ready, review,
  and blocked rollout planning
- D18D integration activation-action descriptors for concrete activate,
  policy-review, primitive, capability, and dependency work planning
- D18D integration activation-agenda descriptors for grouping concrete action
  work by rollout-priority wave
- D18D integration activation-runway descriptors for rollout-priority wave
  grouping and compact actionable/blocker summaries
- D18D integration activation-health descriptors for priority-wave ready,
  review, blocked, and missing-prerequisite status summaries
- D18D integration activation-maintenance descriptors for combined
  priority-wave health, action, constraint, risk, and dependency summaries
- D18D integration activation-constraint descriptors for grouped primitive,
  capability, dependency, and policy-review blocker summaries
- D18D integration activation-review descriptors for read-only human-review
  queue entries and compact review-ready/blocker summaries
- D18D integration activation-approval descriptors for bundled human-decision
  packets that combine review rows with actions, constraints, risk, and
  dependency blockers
- D18D integration activation-decision descriptors for read-only approve/block
  queue planning derived from approval packets
- D18D integration activation-evidence descriptors for read-only evidence rows
  behind approval and blocker decisions
- D18D integration mesh-release descriptors for read-only Zigbee, Z-Wave, and
  Thread primitive, substrate, preflight repair schedule, preflight
  repair slot audit, repair slot execution tickets/work orders,
  batch/schedule/slot/execution readiness, handoff, and release-readiness gate
  planning
- D18D integration activation-dossier descriptors for read-only bundled
  decision and evidence planning
- D18D integration activation-readout descriptors for priority-wave health,
  dossier, evidence, risk, action, and dependency blocker rollups
- D18D integration activation-briefing descriptors for Chief-ready activation,
  approval, review, blocker, risk, and dependency briefing sections
- D18D integration activation-dashboard descriptors for Chief-ready
  priority-wave status cards derived from readouts and briefing rows
- D18D integration activation-timeline descriptors for ordered milestone views
  derived from dashboard cards
- D18D integration activation-forecast descriptors for Chief-ready next-action
  classification derived from activation timeline milestones
- D18D integration activation-playbook descriptors for operator-ready planning
  steps derived from forecast next actions
- D18D integration activation-runbook descriptors for audit-context-rich
  operator phases derived from activation playbook steps
- D18D integration activation-handoff descriptors for execution-transfer
  packages derived from runbooks, risk, dependencies, audit records, and gaps
- D18D integration activation-execution descriptors for executable, approval,
  operator, dependency, and blocker state derived from handoff packages
- D18D integration activation-verification descriptors for post-execution
  verification checkpoints derived from activation execution packets
- D18D integration activation-operator queue descriptors for actionable
  human/operator work derived from playbook steps
- D18D integration activation-control-room descriptors for grouped
  operator-view panels derived from the activation operator queue
- D18D integration activation-command-center descriptors for grouped
  operating-lane sections derived from control-room panels
- D18D integration activation-watchtower descriptors for escalation, review,
  ready, action, and observation signal rollups
- D18D integration activation-sentinel descriptors for compact blocker,
  dependency, policy-risk, review, ready, and observation alert rollups
- D18D integration activation-audit descriptors for read-only audit trails
  connecting sentinel alerts to watchtower, decision, evidence, risk,
  dependency, and readiness-gap sources
- D18D integration activation-escalation descriptors for Chief-facing
  blocker, dependency, policy-risk, review, verification, and audit cases
  derived from sentinel, verification, and audit rollups
- D18D integration activation-response descriptors for owner-lane next actions
  derived from Chief-facing escalation cases
- D18D integration activation-remediation descriptors for owner-lane work
  orders derived from activation response items
- D18D integration activation-closure descriptors for release-gate views that
  combine remediation status, owner lanes, blockers, and verification readiness
- D18D integration activation-release descriptors for compact go/no-go packet
  views derived from activation closure gates
- D18D integration activation-delivery descriptors for delivery-channel
  manifests derived from activation release packets
- D18D integration activation-deployment descriptors for deployment-ring
  records derived from activation delivery manifests
- D18D integration activation-waiver closure descriptors for read-only waiver
  closeout records derived from waiver remediation posture
- D18D integration activation-waiver archive descriptors for read-only waiver
  retention records derived from waiver closure posture
- D18D integration activation-risk descriptors for policy-tier and
  policy-surface rollout risk summaries
- D18D integration activation-dependency descriptors for prerequisite node and
  edge rollups
- D18D integration-readiness descriptors for bulk activation blocker reports
  and compact readiness rollups
- D18D integration-readiness gap descriptors for grouped primitive,
  capability, and delegated-dependency blocker planning
- D18D scene inventory/read tool descriptors for model-facing scene lookup
- D18D event lifecycle tool descriptors for subscribing, polling, and
  unsubscribing from runtime event streams
- D18D event observability tool descriptors for listing active subscriptions and
  inspecting checkpointed runtime event-log entries
- D18D command-result observability descriptors for listing and summarizing
  command results from checkpointed runtime event history
- D18D platform event-ops review descriptors for Chief-visible rollups over
  event delivery, event-log, and pending-work primitives
- D18D recovery-readiness descriptor for Chief-visible return-to-normal gates
  over recovery, service-execution safety, activation, and evidence primitives
- D18D authorization-audit tool descriptors for listing decisions and compact
  allow/deny summaries
- D18D capability-grant tool descriptors for listing grant rows and compact
  grant inventory summaries
- D18D topology tool descriptors for room summaries and aggregate bridge,
  device, entity, state, and scene coverage
- D18D runtime automation inventory tool descriptors for read-only snapshots,
  desired-state targets, and pairing-session status
- D18D bridge-worker inventory descriptors for read-only supervisor and
  heartbeat deadline inspection
- D18D discovery-governance descriptors for read-only scheduled discovery
  worker inventory and compact discovery freshness summaries
- D18D discovery pairing-plan descriptor for read-only host-action previews
  before pairing-session creation
- D18D pairing-completion descriptor for human-approved VaultRef completion
  without exposing raw credentials
- D18D event-ingest descriptor for authorized adapter-observed device and
  bridge-health events
- D18D desired-state mutation descriptors for authorized runtime target
  set/clear operations
- D18D supervision planning tool descriptor for non-mutating due-work previews
- D18D runtime maintenance-window descriptors for Chief-visible grouped
  supervision work
- D18D runtime maintenance-action descriptors for Chief-visible execution
  planning over supervision work
- D18D runtime maintenance-plan descriptors for Chief-visible grouped execution
  plans over supervision work
- D18D runtime maintenance-ticket descriptors for Chief-visible ticket queues
  over grouped supervision work
- D18D runtime maintenance-work-order descriptors for Chief-visible execution
  handoff over maintenance tickets
- D18D runtime maintenance-work-order guardrail descriptors for Chief-visible
  release-blocker and operator-handoff checks
- D18D runtime maintenance-work-order evidence descriptors for Chief-visible
  release-blocking evidence packets
- D18D runtime maintenance-work-order evidence-review descriptors for
  Chief-visible release-blocker review rows
- D18D runtime maintenance-work-order evidence-review disposition descriptors
  for Chief-visible release, handoff, and acceptance decisions
- D18D runtime maintenance-work-order evidence-review disposition-action
  descriptors for Chief-visible release-hold and handoff actions
- D18D supervision execution tool descriptors for authorized desired-state
  reconciliation and runtime supervision ticks
- compact smart-home tool catalog summaries for read-side inspection
- read-only supervision observation tool descriptor for Chief of Staff status
  loops
- agent capability grants for checking tool access before dispatch
- authorization decisions that can be logged by runtimes and agents
- authorization-decision summaries for allow/deny, grant, and
  missing-capability inspection
- capability-grant inventory summaries for grant status, scope, tier, expiry,
  and principal review counts
- MQTT topic names, topic filters, QoS levels, topic roles, and topic bindings
  for MQTT-backed integrations

Out of scope:

- persistent registry storage
- actor supervision
- HTTP/serial/radio I/O
- Vault leases
- policy execution

## Development

```bash
bash BUILD
```
