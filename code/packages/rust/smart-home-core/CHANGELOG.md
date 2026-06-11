# Changelog

All notable changes to this package will be documented in this file.

## Unreleased

### Added

- `IntegrationDescriptor` builder/query helpers plus a canonical integration
  descriptor catalog for Hue, Zigbee, Z-Wave, Thread, Matter, and MQTT
  bootstrap families.
- `IntegrationCatalogSummary` and `canonical_integration_catalog_summary()` for
  compact read-side inspection of integration coverage.
- `SmartHomeToolCatalogSummary` and `smart_home_tool_catalog_summary()` for
  compact read-side inspection of the smart-home tool surface.
- `smart_home.list_integrations`, `smart_home.describe_integration`,
  `smart_home.list_primitives`, and `smart_home.describe_primitive` tool
  descriptors for read-only integration-catalog and reusable-primitive
  inspection.
- `smart_home.get_integration_catalog_summary` and
  `smart_home.get_tool_catalog_summary` tool descriptors for compact read-only
  catalog rollups.
- `smart_home.list_integration_policy_surfaces` and
  `smart_home.get_integration_policy_surface_summary` tool descriptors for
  read-only policy-surface planning rollups.
- `smart_home.list_integration_activation_plans` and
  `smart_home.get_integration_activation_plan_summary` tool descriptors for
  read-only activation-plan listing and compact rollout plan summaries.
- `smart_home.list_integration_activation_candidates` and
  `smart_home.get_integration_activation_candidate_summary` tool descriptors
  for read-only ranked activation candidate planning.
- `smart_home.list_integration_activation_actions` and
  `smart_home.get_integration_activation_action_summary` tool descriptors for
  read-only concrete activation action planning.
- `smart_home.list_integration_activation_agenda` and
  `smart_home.get_integration_activation_agenda_summary` tool descriptors for
  read-only priority-wave activation action planning.
- `smart_home.list_integration_activation_runway` and
  `smart_home.get_integration_activation_runway_summary` tool descriptors for
  read-only rollout-priority activation wave planning.
- `smart_home.list_integration_activation_health` and
  `smart_home.get_integration_activation_health_summary` tool descriptors for
  read-only rollout-priority activation health planning.
- `smart_home.list_integration_activation_maintenance` and
  `smart_home.get_integration_activation_maintenance_summary` tool descriptors
  for read-only combined activation maintenance planning.
- `smart_home.list_integration_activation_constraints` and
  `smart_home.get_integration_activation_constraint_summary` tool descriptors
  for read-only grouped activation constraint planning.
- `smart_home.list_integration_activation_reviews` and
  `smart_home.get_integration_activation_review_summary` tool descriptors for
  read-only human-review queue planning.
- `smart_home.list_integration_activation_approvals` and
  `smart_home.get_integration_activation_approval_summary` tool descriptors
  for read-only human-decision packet planning across review rows, actions,
  constraints, risk, and dependency blockers.
- `smart_home.list_integration_activation_decisions` and
  `smart_home.get_integration_activation_decision_summary` tool descriptors
  for read-only approve/block queue planning derived from activation approval
  packets.
- `smart_home.list_integration_activation_evidence` and
  `smart_home.get_integration_activation_evidence_summary` tool descriptors
  for read-only approval/block evidence row planning.
- `smart_home.list_integration_activation_risk` and
  `smart_home.get_integration_activation_risk_summary` tool descriptors for
  read-only policy-tier and policy-surface activation risk planning.
- `smart_home.list_integration_activation_dependencies` and
  `smart_home.get_integration_activation_dependency_summary` tool descriptors
  for read-only activation dependency graph planning.
- `smart_home.list_integration_readiness` and
  `smart_home.get_integration_readiness_summary` tool descriptors for read-only
  integration activation blocker planning.
- `smart_home.list_integration_readiness_gaps` and
  `smart_home.get_integration_readiness_gap_summary` tool descriptors for
  read-only grouped blocker planning across primitives, capabilities, and
  delegated integration dependencies.
- `smart_home.poll_events` and `smart_home.unsubscribe` tool descriptors for
  model-facing event subscription lifecycle control.
- `smart_home.list_subscriptions` and `smart_home.inspect_event_log` tool
  descriptors for read-only event-stream backlog and replay-log inspection.
- `smart_home.list_command_results` and
  `smart_home.get_command_result_summary` tool descriptors for read-only
  command-result audit views over runtime event history.
- `smart_home.list_authorization_decisions` and
  `smart_home.get_authorization_summary` tool descriptors for read-only
  authorization-audit inspection.
- `smart_home.list_capability_grants` and
  `smart_home.get_capability_grant_summary` tool descriptors plus
  `CapabilityGrantInventorySummary` for read-only grant-governance inspection.
- `smart_home.list_rooms` and `smart_home.get_topology_summary` tool
  descriptors for read-only room and topology coverage inspection.
- `smart_home.get_runtime_snapshot`, `smart_home.list_desired_states`, and
  `smart_home.list_pairing_sessions` tool descriptors for read-only runtime
  automation inventory inspection.
- `smart_home.list_workers` and `smart_home.get_worker_heartbeat_schedule`
  tool descriptors for read-only supervised bridge-worker inventory and
  heartbeat deadline inspection.
- `smart_home.list_discovery_workers` and
  `smart_home.get_discovery_summary` tool descriptors for read-only scheduled
  discovery governance and freshness inspection.
- `smart_home.get_pairing_plan` tool descriptor for read-only discovery
  pairing-plan inspection before pairing-session creation.
- `smart_home.complete_pairing` tool descriptor for human-approved completion
  of pairing sessions with VaultRef handles and non-secret metadata.
- `smart_home.report_event` tool descriptor for authorized adapter-observed
  device and bridge-health event ingest.
- `smart_home.set_desired_state` and `smart_home.clear_desired_state` tool
  descriptors for authorized runtime desired-state target mutation.
- `smart_home.get_supervision_plan` tool descriptor for read-only runtime
  supervision due-work previews.
- `smart_home.reconcile_desired_states` and `smart_home.run_supervision_tick`
  tool descriptors for authorized runtime supervision execution.
- Health and command-result status helpers for shared supervision/read-side
  classification of pairing, attention, acceptance, rejection, and timeout
  states.
- `AuthorizationDecisionSummary` and `AuthorizationDecisionLogSummary` for
  compact allow/deny, grant, and missing-capability inspection.
- `SmartHomeInventorySummary` for compact bridge/device health and entity state
  coverage checks.
- `CapabilitySurfaceSummary` and `Entity::capability_summary()` for compact
  describe-capabilities views over entity capability surfaces.
- `IntegrationSurfaceSummary` and `IntegrationDescriptor::surface_summary()`
  for payload-free adapter capability, discovery, and pairing introspection.
- `smart_home.list_scenes` and `smart_home.describe_scene` tool descriptors for
  read-only scene inventory and detail surfaces.

## [0.1.0] - 2026-05-06

### Added

- Normalized bridge, device, entity, capability, event, command, scene, and
  state snapshot types for D23.
- Protocol identifier records for Hue, Zigbee, Z-Wave, Thread, Matter, MQTT,
  and vendor adapters.
- D18D-style smart-home tool descriptors and command risk-tier helpers.
- Read-only `smart_home.observe_supervision` tool descriptor for status loops.
- Agent capability grant primitives for checking smart-home tool access before
  runtime dispatch.
- Authorization-decision records for capturing allowed or denied tool/command
  checks with matched and missing grants.
- Canonical capability catalog helpers for light, scene, lock, climate, sensor,
  and input integration families.
- MQTT topic names, filters, QoS levels, roles, and bindings for MQTT-backed
  device integrations.
