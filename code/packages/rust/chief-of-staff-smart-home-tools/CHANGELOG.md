# Changelog

All notable changes to this package will be documented in this file.

## Unreleased

- Added D18D handlers for the D23A smart-home integration catalog tools:
  `smart_home.list_integrations`, `smart_home.describe_integration`,
  `smart_home.list_primitives`, and `smart_home.describe_primitive`.
- Added D18D handlers for compact shared-core catalog summaries:
  `smart_home.get_integration_catalog_summary` and
  `smart_home.get_tool_catalog_summary`.
- Added D18D handlers for D23A integration policy-surface planning:
  `smart_home.list_integration_policy_surfaces` and
  `smart_home.get_integration_policy_surface_summary`.
- Added D18D handlers for D23A ecosystem platform coverage planning:
  `smart_home.list_integration_platform_coverage` and
  `smart_home.get_integration_platform_coverage_summary`.
- Added D18D handlers for D23A primitive coverage gap planning:
  `smart_home.list_integration_primitive_coverage` and
  `smart_home.get_integration_primitive_coverage_summary`.
- Added D18D handlers for bulk D23A integration activation-plan planning:
  `smart_home.list_integration_activation_plans` and
  `smart_home.get_integration_activation_plan_summary`.
- Added D18D handlers for ranked D23A integration activation candidate
  planning: `smart_home.list_integration_activation_candidates` and
  `smart_home.get_integration_activation_candidate_summary`.
- Added D18D handlers for concrete D23A integration activation action
  planning: `smart_home.list_integration_activation_actions` and
  `smart_home.get_integration_activation_action_summary`.
- Added D18D handlers for D23A priority-wave activation agendas:
  `smart_home.list_integration_activation_agenda` and
  `smart_home.get_integration_activation_agenda_summary`.
- Added D18D handlers for D23A rollout-priority activation runway planning:
  `smart_home.list_integration_activation_runway` and
  `smart_home.get_integration_activation_runway_summary`.
- Added D18D handlers for D23A activation health planning:
  `smart_home.list_integration_activation_health` and
  `smart_home.get_integration_activation_health_summary`.
- Added D18D handlers for D23A activation maintenance planning:
  `smart_home.list_integration_activation_maintenance` and
  `smart_home.get_integration_activation_maintenance_summary`.
- Added D18D handlers for D23A grouped activation constraint planning:
  `smart_home.list_integration_activation_constraints` and
  `smart_home.get_integration_activation_constraint_summary`.
- Added D18D handlers for D23A activation human-review queue planning:
  `smart_home.list_integration_activation_reviews` and
  `smart_home.get_integration_activation_review_summary`.
- Added D18D handlers for D23A activation approval packet planning:
  `smart_home.list_integration_activation_approvals` and
  `smart_home.get_integration_activation_approval_summary`.
- Added D18D handlers for D23A activation readout planning:
  `smart_home.list_integration_activation_readouts` and
  `smart_home.get_integration_activation_readout_summary`.
- Added D18D handlers for D23A activation briefing item planning:
  `smart_home.list_integration_activation_briefing_items` and
  `smart_home.get_integration_activation_briefing_summary`.
- Added D18D handlers for D23A activation dashboard card planning:
  `smart_home.list_integration_activation_dashboard` and
  `smart_home.get_integration_activation_dashboard_summary`.
- Added D18D handlers for D23A activation timeline milestone planning:
  `smart_home.list_integration_activation_timeline` and
  `smart_home.get_integration_activation_timeline_summary`.
- Added D18D handlers for D23A activation forecast next-action planning:
  `smart_home.list_integration_activation_forecasts` and
  `smart_home.get_integration_activation_forecast_summary`.
- Added D18D handlers for D23A activation playbook planning:
  `smart_home.list_integration_activation_playbook` and
  `smart_home.get_integration_activation_playbook_summary`.
- Added D18D handlers for D23A activation runbook planning:
  `smart_home.list_integration_activation_runbook` and
  `smart_home.get_integration_activation_runbook_summary`.
- Added D18D handlers for D23A activation handoff packages:
  `smart_home.list_integration_activation_handoff` and
  `smart_home.get_integration_activation_handoff_summary`.
- Added D18D handlers for D23A activation execution packets:
  `smart_home.list_integration_activation_execution` and
  `smart_home.get_integration_activation_execution_summary`.
- Added D18D handlers for D23A activation verification checkpoints:
  `smart_home.list_integration_activation_verification` and
  `smart_home.get_integration_activation_verification_summary`.
- Added D18D handlers for D23A activation operator queue planning:
  `smart_home.list_integration_activation_operator_queue` and
  `smart_home.get_integration_activation_operator_queue_summary`.
- Added D18D handlers for D23A activation control-room panel planning:
  `smart_home.list_integration_activation_control_room` and
  `smart_home.get_integration_activation_control_room_summary`.
- Added D18D handlers for D23A activation command-center operating lanes:
  `smart_home.list_integration_activation_command_center` and
  `smart_home.get_integration_activation_command_center_summary`.
- Added D18D handlers for D23A activation watchtower signals:
  `smart_home.list_integration_activation_watchtower` and
  `smart_home.get_integration_activation_watchtower_summary`.
- Added D18D handlers for D23A activation sentinel alerts:
  `smart_home.list_integration_activation_sentinel` and
  `smart_home.get_integration_activation_sentinel_summary`.
- Added D18D handlers for D23A activation audit trails:
  `smart_home.list_integration_activation_audit` and
  `smart_home.get_integration_activation_audit_summary`.
- Added D18D handlers for D23A activation escalation cases:
  `smart_home.list_integration_activation_escalations` and
  `smart_home.get_integration_activation_escalation_summary`.
- Added D18D handlers for D23A activation response planning:
  `smart_home.list_integration_activation_responses` and
  `smart_home.get_integration_activation_response_summary`.
- Added D18D handlers for D23A activation evidence remediation lanes:
  `smart_home.list_integration_activation_evidence_remediation` and
  `smart_home.get_integration_activation_evidence_remediation_summary`.
- Added D18D handlers for D23A activation evidence lane inventory and
  scorecard rollups:
  `smart_home.list_integration_activation_evidence_lane_inventory`,
  `smart_home.get_integration_activation_evidence_lane_inventory_summary`, and
  `smart_home.get_integration_activation_evidence_scorecard_summary`.
- Added D18D handlers for D23 mesh primitive readiness, substrate-stage
  readiness, substrate-action queues, package readiness, and stage-release
  summaries:
  `smart_home.list_integration_mesh_primitive_readiness`,
  `smart_home.get_integration_mesh_primitive_readiness_summary`,
  `smart_home.list_integration_mesh_substrate_stages`,
  `smart_home.get_integration_mesh_substrate_stage_summary`,
  `smart_home.list_integration_mesh_substrate_actions`,
  `smart_home.get_integration_mesh_substrate_action_summary`,
  `smart_home.get_integration_mesh_readiness_package_summary`, and
  `smart_home.get_integration_mesh_stage_release_summary`.
- Added D18D handlers for D23 mesh substrate action queues and action-readiness
  rollups: `smart_home.list_integration_mesh_substrate_actions` and
  `smart_home.get_integration_mesh_action_readiness_summary`.
- Added D18D handlers for D23 mesh substrate preflight checks, repair actions,
  repair batches, repair schedules, and preflight repair readiness rollups:
  `smart_home.list_integration_mesh_substrate_preflight_checks`,
  `smart_home.get_integration_mesh_substrate_preflight_summary`,
  `smart_home.list_integration_mesh_preflight_repair_actions`,
  `smart_home.get_integration_mesh_preflight_repair_action_summary`,
  `smart_home.list_integration_mesh_preflight_repair_batches`,
  `smart_home.get_integration_mesh_preflight_repair_batch_summary`,
  `smart_home.list_integration_mesh_preflight_repair_schedule`,
  `smart_home.get_integration_mesh_preflight_repair_schedule_summary`,
  `smart_home.list_integration_mesh_preflight_repair_slot_audits`,
  `smart_home.get_integration_mesh_preflight_repair_slot_audit_summary`,
  `smart_home.list_integration_mesh_preflight_repair_slot_execution_tickets`,
  `smart_home.get_integration_mesh_preflight_repair_slot_execution_ticket_summary`,
  `smart_home.list_integration_mesh_preflight_repair_slot_execution_work_orders`,
  `smart_home.get_integration_mesh_preflight_repair_slot_execution_work_order_summary`,
  `smart_home.get_integration_mesh_preflight_readiness_summary`,
  `smart_home.get_integration_mesh_preflight_repair_readiness_summary`,
  `smart_home.get_integration_mesh_preflight_batch_readiness_summary`,
  `smart_home.get_integration_mesh_preflight_schedule_readiness_summary`,
  `smart_home.get_integration_mesh_preflight_slot_readiness_summary`, and
  `smart_home.get_integration_mesh_preflight_execution_readiness_summary`.
- Added the D23 mesh release-readiness summary handler:
  `smart_home.get_integration_mesh_release_readiness_summary`.
- Added D18D handlers for D23 mesh readiness handoff packages:
  `smart_home.list_integration_mesh_readiness_handoffs` and
  `smart_home.get_integration_mesh_readiness_handoff_summary`.
- Added D18D handlers for D23 mesh release-readiness gate checks:
  `smart_home.list_integration_mesh_release_readiness_checks` and
  `smart_home.get_integration_mesh_release_readiness_check_summary`.
- Added D18D handlers for D23A activation remediation work orders:
  `smart_home.list_integration_activation_remediation` and
  `smart_home.get_integration_activation_remediation_summary`.
- Added D18D handlers for D23A activation closure gates:
  `smart_home.list_integration_activation_closure` and
  `smart_home.get_integration_activation_closure_summary`.
- Added D18D handlers for D23A activation release packets:
  `smart_home.list_integration_activation_release` and
  `smart_home.get_integration_activation_release_summary`.
- Added D18D handlers for D23A activation delivery manifests:
  `smart_home.list_integration_activation_delivery` and
  `smart_home.get_integration_activation_delivery_summary`.
- Added D18D handlers for D23A activation deployment records:
  `smart_home.list_integration_activation_deployment` and
  `smart_home.get_integration_activation_deployment_summary`.
- Added D18D handlers for D23A activation waiver closure records:
  `smart_home.list_integration_activation_waiver_closures` and
  `smart_home.get_integration_activation_waiver_closure_summary`.
- Added D18D handlers for D23A activation waiver archive records:
  `smart_home.list_integration_activation_waiver_archives` and
  `smart_home.get_integration_activation_waiver_archive_summary`.
- Added D18D handlers for D23A activation risk planning:
  `smart_home.list_integration_activation_risk` and
  `smart_home.get_integration_activation_risk_summary`.
- Added D18D handlers for D23A integration activation dependency graph
  planning: `smart_home.list_integration_activation_dependencies` and
  `smart_home.get_integration_activation_dependency_summary`.
- Added D18D handlers for bulk D23A integration readiness planning:
  `smart_home.list_integration_readiness` and
  `smart_home.get_integration_readiness_summary`.
- Added D18D handlers for grouped D23A integration readiness blocker planning:
  `smart_home.list_integration_readiness_gaps` and
  `smart_home.get_integration_readiness_gap_summary`.
- Added D18D smart-home tool definitions and in-memory handlers over
  `SmartHomeRuntime`.
- Added an end-to-end Hue-style fixture test that lists devices, commands a
  light, reads optimistic state, and records a D18D execution journal entry.
- Added D18D handlers for `smart_home.subscribe`,
  `smart_home.pair_bridge`, `smart_home.describe_capabilities`, and
  `smart_home.get_health` so Chief of Staff jobs can reach the existing D23
  subscription, pairing, capability, and health runtime paths.
- Added the `smart_home.discover` D18D handler over
  `RuntimeDiscoverToolRequest`, including discovery filters, bridge-candidate
  output, and end-to-end journal coverage.
- Added `smart_home.list_discovery_workers` and
  `smart_home.get_discovery_summary` handlers over the D23 runtime read facade
  so Chief of Staff jobs can inspect scheduled discovery work and candidate
  freshness without owning discovery scheduler logic.
- Added `smart_home.get_pairing_plan` over the D23 runtime read facade so Chief
  of Staff jobs can inspect actionable discovery pairing plans before starting
  a pairing session.
- Added `smart_home.complete_pairing` over the D23 runtime mutation facade so
  Chief of Staff jobs can finish pairing ceremonies with VaultRef handles and
  non-secret metadata without owning credential material.
- Added `smart_home.report_event` over the D23 runtime mutation facade so Chief
  of Staff jobs can ingest adapter-observed device events and bridge-health
  reports without owning registry mutation logic.
- Added scheduled discovery worker observability to
  `smart_home.observe_supervision`, including worker status, due time, last run
  counts, and failure pressure from the D23 runtime.
- Added D23 discovery worker retry policy fields to
  `smart_home.observe_supervision`, including configured retry delays,
  multiplier, and the current retry delay during failure pressure.
- Added `smart_home.poll_events` and `smart_home.unsubscribe` handlers so Chief
  of Staff jobs can drain, peek, summarize, and retire runtime event
  subscriptions without bypassing D23 authorization.
- Added `smart_home.list_scenes` and `smart_home.describe_scene` handlers over
  the D23 runtime scene read facade, including Hue-style fixture coverage.
- Added `smart_home.list_subscriptions` and `smart_home.inspect_event_log`
  handlers over the D23 runtime read facade so Chief of Staff jobs can inspect
  event-stream backlog pressure and checkpointed event history without draining
  subscriptions.
- Added `smart_home.list_command_results` and
  `smart_home.get_command_result_summary` handlers over the D23 runtime read
  facade so Chief of Staff jobs can inspect accepted and failed command results
  without owning a parallel command audit log.
- Added `smart_home.list_authorization_decisions` and
  `smart_home.get_authorization_summary` handlers over the D23 runtime read
  facade so Chief of Staff jobs can inspect allow/deny audit history without
  owning smart-home authorization logic.
- Added `smart_home.list_capability_grants` and
  `smart_home.get_capability_grant_summary` handlers over the D23 runtime read
  facade so Chief of Staff jobs can inspect grant governance without owning
  smart-home authorization policy.
- Added `smart_home.list_rooms` and `smart_home.get_topology_summary` handlers
  over the D23 runtime read facade so Chief of Staff jobs can inspect room,
  state, and scene coverage without owning registry topology logic.
- Added `smart_home.get_runtime_snapshot`, `smart_home.list_desired_states`,
  and `smart_home.list_pairing_sessions` handlers over the D23 runtime read
  facade so Chief of Staff jobs can inspect automation backlog, reconciliation
  targets, and pairing ceremonies without bypassing runtime ownership.
- Added `smart_home.get_supervision_plan` over the D23 runtime read facade so
  Chief of Staff jobs can preview due pairing, refresh, reconciliation,
  restart, and discovery work without ticking supervision.
- Added `smart_home.list_runtime_maintenance_windows` and
  `smart_home.get_runtime_maintenance_window_summary` over the D23 runtime read
  facade so Chief of Staff jobs can group supervision remediation work into
  schedulable maintenance windows without owning platform mutation logic.
- Added `smart_home.list_runtime_maintenance_actions` and
  `smart_home.get_runtime_maintenance_action_summary` over the D23 runtime read
  facade so Chief of Staff jobs can inspect execution-ordered maintenance
  actions without owning platform mutation logic.
- Added `smart_home.list_runtime_maintenance_plans` and
  `smart_home.get_runtime_maintenance_plan_summary` over the D23 runtime read
  facade so Chief of Staff jobs can group execution actions into schedulable
  maintenance plans without owning platform mutation logic.
- Added `smart_home.list_runtime_maintenance_tickets` and
  `smart_home.get_runtime_maintenance_ticket_summary` over the D23 runtime read
  facade so Chief of Staff jobs can inspect ticket-ready maintenance queues
  without owning platform mutation logic.
- Added `smart_home.list_runtime_maintenance_work_orders` and
  `smart_home.get_runtime_maintenance_work_order_summary` over the D23 runtime
  read facade so Chief of Staff jobs can inspect execution work-order handoffs
  without owning platform mutation logic.
- Added `smart_home.list_runtime_maintenance_work_order_guardrails` and
  `smart_home.get_runtime_maintenance_work_order_guardrail_summary` over the
  D23 runtime read facade so Chief of Staff jobs can inspect release blockers,
  operator handoffs, and ready execution lanes without owning mutation logic.
- Added `smart_home.list_runtime_maintenance_work_order_evidence` and
  `smart_home.get_runtime_maintenance_work_order_evidence_summary` over the D23
  runtime read facade so Chief of Staff jobs can inspect work-order evidence
  packets without owning platform mutation logic.
- Added `smart_home.list_runtime_maintenance_work_order_evidence_reviews` and
  `smart_home.get_runtime_maintenance_work_order_evidence_review_summary` over
  the D23 runtime read facade so Chief of Staff jobs can inspect work-order
  evidence review rows without owning platform mutation logic.
- Added
  `smart_home.list_runtime_maintenance_work_order_evidence_review_dispositions`
  and
  `smart_home.get_runtime_maintenance_work_order_evidence_review_disposition_summary`
  over the D23 runtime read facade so Chief of Staff jobs can inspect
  release-blocker, operator-handoff, and accepted evidence-review dispositions
  without owning platform mutation logic.
- Added
  `smart_home.list_runtime_maintenance_work_order_evidence_review_disposition_actions`
  and
  `smart_home.get_runtime_maintenance_work_order_evidence_review_disposition_action_summary`
  over the D23 runtime read facade so Chief of Staff jobs can inspect
  release-hold, operator-handoff, and acceptance actions derived from
  evidence-review dispositions without owning platform mutation logic.
- Added
  `smart_home.list_runtime_maintenance_work_order_evidence_review_disposition_action_outcomes`
  and
  `smart_home.get_runtime_maintenance_work_order_evidence_review_disposition_action_outcome_summary`
  over the D23 runtime read facade so Chief of Staff jobs can inspect
  release-hold, operator-handoff, and accepted outcomes derived from
  disposition actions without owning platform mutation logic.
- Added
  `smart_home.list_runtime_maintenance_work_order_evidence_review_disposition_action_outcome_readiness`
  and
  `smart_home.get_runtime_maintenance_work_order_evidence_review_disposition_action_outcome_readiness_summary`
  over the D23 runtime read facade so Chief of Staff jobs can inspect
  release-hold, operator-handoff, lineage-gap, and close-ready outcome
  readiness without owning platform mutation logic.
- Added `smart_home.list_workers` and
  `smart_home.get_worker_heartbeat_schedule` over the D23 runtime read facade
  so Chief of Staff jobs can inspect supervised bridge workers and heartbeat
  deadlines without mutating supervisor state.
- Added `smart_home.reconcile_desired_states` and
  `smart_home.run_supervision_tick` handlers over the D23 runtime supervision
  facade so Chief of Staff jobs can run authorized reconciliation and
  supervisor ticks without owning D23 mutation logic.
- Added `smart_home.set_desired_state` and
  `smart_home.clear_desired_state` handlers over the D23 runtime mutation
  facade so Chief of Staff jobs can manage desired-state targets without
  owning validation, storage, or reconciliation semantics.
