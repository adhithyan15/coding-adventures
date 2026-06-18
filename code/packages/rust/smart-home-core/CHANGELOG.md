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
- `smart_home.list_integration_activation_evidence_remediation` and
  `smart_home.get_integration_activation_evidence_remediation_summary` tool
  descriptors for read-only D23A evidence-lane remediation planning.
- `smart_home.list_integration_activation_evidence_lane_inventory`,
  `smart_home.get_integration_activation_evidence_lane_inventory_summary`, and
  `smart_home.get_integration_activation_evidence_scorecard_summary` tool
  descriptors for read-only D23A evidence-lane inventory and scorecard
  planning.
- `smart_home.list_integration_mesh_primitive_readiness`,
  `smart_home.get_integration_mesh_primitive_readiness_summary`,
  `smart_home.list_integration_mesh_substrate_stages`,
  `smart_home.get_integration_mesh_substrate_stage_summary`,
  `smart_home.get_integration_mesh_readiness_package_summary`, and
  `smart_home.get_integration_mesh_stage_release_summary` descriptors for
  read-only D23 mesh readiness release planning.
- `smart_home.list_integration_mesh_substrate_actions` and
  `smart_home.get_integration_mesh_action_readiness_summary` descriptors for
  read-only D23 mesh substrate action and action-readiness planning.
- `smart_home.list_integration_activation_dossiers` and
  `smart_home.get_integration_activation_dossier_summary` tool descriptors
  for read-only bundled activation decision and evidence planning.
- `smart_home.list_integration_activation_readouts` and
  `smart_home.get_integration_activation_readout_summary` tool descriptors
  for read-only priority-wave activation readouts across health, dossiers,
  evidence, risk, action, and dependency blockers.
- `smart_home.list_integration_activation_briefing_items` and
  `smart_home.get_integration_activation_briefing_summary` tool descriptors
  for read-only Chief activation briefing sections derived from priority-wave
  readouts.
- `smart_home.list_integration_activation_dashboard` and
  `smart_home.get_integration_activation_dashboard_summary` tool descriptors
  for read-only Chief activation dashboard cards derived from readouts and
  briefing rows.
- `smart_home.list_integration_activation_timeline` and
  `smart_home.get_integration_activation_timeline_summary` tool descriptors
  for read-only Chief activation milestone views derived from dashboard cards.
- `smart_home.list_integration_activation_forecasts` and
  `smart_home.get_integration_activation_forecast_summary` tool descriptors
  for read-only Chief next-action classification derived from activation
  timeline milestones.
- `smart_home.list_integration_activation_playbook` and
  `smart_home.get_integration_activation_playbook_summary` tool descriptors
  for read-only operator-ready planning steps derived from activation
  forecasts.
- `smart_home.list_integration_activation_runbook` and
  `smart_home.get_integration_activation_runbook_summary` tool descriptors for
  read-only audit-context-rich operator phases derived from activation
  playbook steps.
- `smart_home.list_integration_activation_handoff` and
  `smart_home.get_integration_activation_handoff_summary` tool descriptors for
  read-only execution-transfer handoff packages derived from activation
  runbooks, risk, dependencies, audit records, and readiness gaps.
- `smart_home.list_integration_activation_execution` and
  `smart_home.get_integration_activation_execution_summary` tool descriptors
  for read-only executable, approval, operator, dependency, and blocker state
  derived from activation handoff packages.
- `smart_home.list_integration_activation_verification` and
  `smart_home.get_integration_activation_verification_summary` tool
  descriptors for read-only post-execution verification checkpoints derived
  from activation execution packets.
- `smart_home.list_integration_activation_operator_queue` and
  `smart_home.get_integration_activation_operator_queue_summary` tool
  descriptors for read-only actionable human/operator work derived from
  playbook steps.
- `smart_home.list_integration_activation_control_room` and
  `smart_home.get_integration_activation_control_room_summary` tool
  descriptors for read-only grouped operator-view panels derived from the
  activation operator queue.
- `smart_home.list_integration_activation_command_center` and
  `smart_home.get_integration_activation_command_center_summary` tool
  descriptors for read-only grouped operating-lane planning derived from
  activation control-room panels.
- `smart_home.list_integration_activation_watchtower` and
  `smart_home.get_integration_activation_watchtower_summary` tool descriptors
  for read-only escalation, review, ready, action, and observation signal
  rollups derived from activation command-center sections.
- `smart_home.list_integration_activation_sentinel` and
  `smart_home.get_integration_activation_sentinel_summary` tool descriptors
  for read-only blocker, dependency, policy-risk, review, ready, and
  observation alert rollups derived from activation watchtower, risk,
  dependency, and readiness-gap rollups.
- `smart_home.list_integration_activation_audit` and
  `smart_home.get_integration_activation_audit_summary` tool descriptors for
  read-only activation audit trails that connect sentinel alerts to
  watchtower, decision, evidence, risk, dependency, and readiness-gap sources.
- `smart_home.list_integration_activation_escalations` and
  `smart_home.get_integration_activation_escalation_summary` tool descriptors
  for read-only Chief-facing activation escalation cases derived from sentinel
  alerts, verification checkpoints, and audit records.
- `smart_home.list_integration_activation_responses` and
  `smart_home.get_integration_activation_response_summary` tool descriptors for
  read-only owner-lane activation response planning derived from escalation
  cases.
- `smart_home.list_integration_activation_remediation` and
  `smart_home.get_integration_activation_remediation_summary` tool descriptors
  for read-only owner-lane activation remediation work orders derived from
  response items.
- `smart_home.list_integration_activation_closure` and
  `smart_home.get_integration_activation_closure_summary` tool descriptors for
  read-only activation closure gates derived from remediation status.
- `smart_home.list_integration_activation_release` and
  `smart_home.get_integration_activation_release_summary` tool descriptors for
  read-only activation release packets derived from closure gates.
- `smart_home.list_integration_activation_delivery` and
  `smart_home.get_integration_activation_delivery_summary` tool descriptors for
  read-only activation delivery manifests derived from release packets.
- `smart_home.list_integration_activation_deployment` and
  `smart_home.get_integration_activation_deployment_summary` tool descriptors
  for read-only activation deployment records derived from delivery manifests.
- `smart_home.list_integration_activation_waiver_closures` and
  `smart_home.get_integration_activation_waiver_closure_summary` tool
  descriptors for read-only waiver closure records derived from waiver
  remediation posture.
- `smart_home.list_integration_activation_waiver_archives` and
  `smart_home.get_integration_activation_waiver_archive_summary` tool
  descriptors for read-only waiver archive records derived from waiver closure
  posture.
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
