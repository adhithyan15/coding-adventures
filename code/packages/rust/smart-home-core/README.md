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
- canonical capability catalog entries for light, scene, lock, climate, sensor,
  and input families
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
