# chief-of-staff-smart-home-tools

`chief-of-staff-smart-home-tools` connects the D18D Chief of Staff tool runtime
to the D23 smart-home runtime.

The crate is intentionally a thin adapter:

- it publishes `smart_home.*` D18D tool definitions
- it registers in-process handlers on `InMemoryToolRuntime`
- handlers translate JSON arguments into `SmartHomeRuntime` read, discover,
  event-subscription, pair, command, event-ingest, and supervision execution
  requests, desired-state target mutations, plus read-only D23A catalog queries
- `SmartHomeRuntime` still owns smart-home authorization, command validation,
  event subscriptions, pairing sessions, optimistic state, discovery scheduler
  policy and observability, discovery pairing plans, supervision, and audit
  decisions
- `smart-home-integration-catalog` still owns D23A integration and primitive
  catalog semantics
- D18D still owns tool validation, policy decisions, event streams, terminal
  results, and execution journals

The first slice proves an end-to-end local path with an in-memory Hue-style
fixture:

```text
Chief of Staff job/session/agent
  -> D18D smart_home.discover / smart_home.command tool calls
  -> smart-home runtime authorization
  -> discovery records and unpaired bridge candidates
  -> discovery worker inventory and discovery freshness summaries
  -> discovery pairing-plan previews for required host actions
  -> human-approved pairing completion with VaultRef handles
  -> room topology and aggregate topology summary reads
  -> scene inventory and scene detail reads
  -> discovery worker health and retry state in smart_home.observe_supervision
  -> event-log and subscription-backlog reads
  -> typed command-result audit reads and summaries
  -> authorization decision audit reads and summaries
  -> capability grant ledger reads and summaries
  -> controller-readiness handoff summary over runtime and platform primitives
  -> platform brief over HTTP, dashboard, fixture, state/history/event,
     command, scene, and authorization readiness
  -> platform evidence ledger over platform, closeout-retention, and runtime
     maintenance evidence signals without owning smart-home state
  -> platform access review over command-risk, authorization-gap, authorization
     decision, and capability-grant signals without owning smart-home state
  -> platform event-ops review over event delivery, event log, and pending-work
     signals without draining queues or owning smart-home state
  -> recovery brief over incident, policy, runtime, state, and validation
     handoff signals
  -> escalation brief over morning, blocker, review, and owner-lane signals
  -> continuity brief over escalation, recovery, and operator handoff signals
  -> operator readiness brief over continuity lanes and handoff decisions
  -> shift handoff, closeout, and closeout-receipt briefs over existing
     operator lanes without owning smart-home state
  -> policy-surface inventory and summary reads for review planning
  -> ecosystem platform coverage and summary reads for primitive planning
  -> primitive coverage gap list and summary reads for backlog planning
  -> activation-plan list and summary reads for D23A rollout planning
  -> activation-candidate list and summary reads for ranked rollout planning
  -> activation-action list and summary reads for concrete unblock/activate work
  -> activation-agenda list and summary reads for concrete work by rollout wave
  -> activation-runway list and summary reads for rollout-priority waves
  -> activation-health list and summary reads for priority-wave readiness status
  -> activation-maintenance list and summary reads for combined wave health
  -> activation-constraint list and summary reads for grouped blockers/reviews
  -> activation-review list and summary reads for human-review queue planning
  -> activation-approval list and summary reads for bundled approval packets
  -> activation-decision list and summary reads for approve/block queue planning
  -> activation-evidence list and summary reads for approval/block evidence rows
  -> activation-evidence remediation list and summary reads for D23A lane fixes
  -> activation-evidence lane inventory and scorecard summary reads for blocker
     lane rollups
  -> mesh primitive-readiness, substrate-stage, and substrate-action reads for
     low-level protocol release blockers
  -> mesh substrate preflight check, repair action, repair batch, repair
     schedule, repair slot audit, repair slot execution ticket, and preflight
     repair/batch/schedule/slot readiness reads for release-prep blockers
  -> mesh readiness package, stage-release, action-readiness, and
     release-readiness reads for release go/no-go
  -> mesh readiness handoff list and summary reads for release coordination
  -> mesh release-readiness check list and summary reads for final gate review
  -> activation-dossier list and summary reads for bundled decision evidence
  -> activation-dashboard list and summary reads for priority-wave status cards
  -> activation-timeline list and summary reads for ordered wave milestones
  -> activation-forecast list and summary reads for next-action wave planning
  -> activation-playbook list and summary reads for operator-ready planning steps
  -> activation-runbook list and summary reads for audit-context-rich phases
  -> activation-handoff list and summary reads for execution-transfer packages
  -> activation-execution list and summary reads for executable packet state
  -> activation-verification list and summary reads for post-execution checks
  -> activation-operator queue list and summary reads for actionable work
  -> activation-control-room list and summary reads for grouped operator panels
  -> activation-command-center list and summary reads for operating-lane rollups
  -> activation-watchtower list and summary reads for escalation signal rollups
  -> activation-sentinel list and summary reads for compact alert rollups
  -> activation-audit list and summary reads for source-linked audit trails
  -> activation-escalation list and summary reads for Chief-facing cases
  -> activation-response list and summary reads for owner-lane next actions
  -> activation-remediation list and summary reads for owner-lane work orders
  -> activation-closure list and summary reads for verification/close gates
  -> activation-release list and summary reads for go/no-go release packets
  -> activation-delivery list and summary reads for delivery-channel manifests
  -> activation-deployment list and summary reads for deployment-ring records
  -> activation waiver archive list and summary reads for source-linked waiver
     closeout retention
  -> activation-risk list and summary reads for policy-tier/surface rollout risk
  -> activation dependency graph list and summary reads for prerequisite edges
  -> runtime snapshot, morning-brief, platform-brief, pending-work, attention-overview,
     remediation-plan, operations-brief, safety-brief, readiness-brief,
     maintenance-brief, incident-brief, recovery-brief, escalation-brief,
     continuity-brief, desired-state, and pairing-session inventory reads
  -> desired-state target set/clear through runtime authorization
  -> non-mutating supervision plan previews
  -> supervision remediation and runtime maintenance-window/action/plan/ticket/work-order/guardrail/evidence/review/disposition/action/outcome/readiness/handoff/reconciliation/closeout reads plus closeout-receipt briefs
  -> authorized desired-state reconciliation and supervision ticks
  -> supervised worker inventory and heartbeat schedule reads
  -> device command acceptance
  -> adapter-observed device and bridge-health event ingest
  -> confirmed state update
  -> D18D trace and audit record
```

## Included Tools

- `smart_home.list_integrations`
- `smart_home.describe_integration`
- `smart_home.list_primitives`
- `smart_home.describe_primitive`
- `smart_home.get_integration_catalog_summary`
- `smart_home.get_tool_catalog_summary`
- `smart_home.list_integration_policy_surfaces`
- `smart_home.get_integration_policy_surface_summary`
- `smart_home.list_integration_platform_coverage`
- `smart_home.get_integration_platform_coverage_summary`
- `smart_home.list_integration_primitive_coverage`
- `smart_home.get_integration_primitive_coverage_summary`
- `smart_home.list_integration_activation_plans`
- `smart_home.get_integration_activation_plan_summary`
- `smart_home.list_integration_activation_candidates`
- `smart_home.get_integration_activation_candidate_summary`
- `smart_home.list_integration_activation_actions`
- `smart_home.get_integration_activation_action_summary`
- `smart_home.list_integration_activation_agenda`
- `smart_home.get_integration_activation_agenda_summary`
- `smart_home.list_integration_activation_runway`
- `smart_home.get_integration_activation_runway_summary`
- `smart_home.list_integration_activation_health`
- `smart_home.get_integration_activation_health_summary`
- `smart_home.list_integration_activation_maintenance`
- `smart_home.get_integration_activation_maintenance_summary`
- `smart_home.list_integration_activation_constraints`
- `smart_home.get_integration_activation_constraint_summary`
- `smart_home.list_integration_activation_reviews`
- `smart_home.get_integration_activation_review_summary`
- `smart_home.list_integration_activation_approvals`
- `smart_home.get_integration_activation_approval_summary`
- `smart_home.list_integration_activation_decisions`
- `smart_home.get_integration_activation_decision_summary`
- `smart_home.list_integration_activation_evidence`
- `smart_home.get_integration_activation_evidence_summary`
- `smart_home.list_integration_activation_evidence_remediation`
- `smart_home.get_integration_activation_evidence_remediation_summary`
- `smart_home.list_integration_activation_evidence_lane_inventory`
- `smart_home.get_integration_activation_evidence_lane_inventory_summary`
- `smart_home.get_integration_activation_evidence_scorecard_summary`
- `smart_home.list_integration_mesh_primitive_readiness`
- `smart_home.get_integration_mesh_primitive_readiness_summary`
- `smart_home.list_integration_mesh_substrate_stages`
- `smart_home.get_integration_mesh_substrate_stage_summary`
- `smart_home.list_integration_mesh_substrate_actions`
- `smart_home.get_integration_mesh_substrate_action_summary`
- `smart_home.get_integration_mesh_readiness_package_summary`
- `smart_home.get_integration_mesh_stage_release_summary`
- `smart_home.get_integration_mesh_action_readiness_summary`
- `smart_home.list_integration_mesh_substrate_preflight_checks`
- `smart_home.get_integration_mesh_substrate_preflight_summary`
- `smart_home.list_integration_mesh_preflight_repair_actions`
- `smart_home.get_integration_mesh_preflight_repair_action_summary`
- `smart_home.list_integration_mesh_preflight_repair_batches`
- `smart_home.get_integration_mesh_preflight_repair_batch_summary`
- `smart_home.list_integration_mesh_preflight_repair_schedule`
- `smart_home.get_integration_mesh_preflight_repair_schedule_summary`
- `smart_home.list_integration_mesh_preflight_repair_slot_audits`
- `smart_home.get_integration_mesh_preflight_repair_slot_audit_summary`
- `smart_home.list_integration_mesh_preflight_repair_slot_execution_tickets`
- `smart_home.get_integration_mesh_preflight_repair_slot_execution_ticket_summary`
- `smart_home.list_integration_mesh_preflight_repair_slot_execution_work_orders`
- `smart_home.get_integration_mesh_preflight_repair_slot_execution_work_order_summary`
- `smart_home.get_integration_mesh_preflight_readiness_summary`
- `smart_home.get_integration_mesh_preflight_repair_readiness_summary`
- `smart_home.get_integration_mesh_preflight_batch_readiness_summary`
- `smart_home.get_integration_mesh_preflight_schedule_readiness_summary`
- `smart_home.get_integration_mesh_preflight_execution_readiness_summary`
- `smart_home.get_integration_mesh_release_readiness_summary`
- `smart_home.list_integration_mesh_readiness_handoffs`
- `smart_home.get_integration_mesh_readiness_handoff_summary`
- `smart_home.list_integration_mesh_release_readiness_checks`
- `smart_home.get_integration_mesh_release_readiness_check_summary`
- `smart_home.list_integration_activation_dossiers`
- `smart_home.get_integration_activation_dossier_summary`
- `smart_home.list_integration_activation_readouts`
- `smart_home.get_integration_activation_readout_summary`
- `smart_home.list_integration_activation_briefing_items`
- `smart_home.get_integration_activation_briefing_summary`
- `smart_home.list_integration_activation_dashboard`
- `smart_home.get_integration_activation_dashboard_summary`
- `smart_home.list_integration_activation_timeline`
- `smart_home.get_integration_activation_timeline_summary`
- `smart_home.list_integration_activation_forecasts`
- `smart_home.get_integration_activation_forecast_summary`
- `smart_home.list_integration_activation_playbook`
- `smart_home.get_integration_activation_playbook_summary`
- `smart_home.list_integration_activation_runbook`
- `smart_home.get_integration_activation_runbook_summary`
- `smart_home.list_integration_activation_handoff`
- `smart_home.get_integration_activation_handoff_summary`
- `smart_home.list_integration_activation_execution`
- `smart_home.get_integration_activation_execution_summary`
- `smart_home.list_integration_activation_verification`
- `smart_home.get_integration_activation_verification_summary`
- `smart_home.list_integration_activation_operator_queue`
- `smart_home.get_integration_activation_operator_queue_summary`
- `smart_home.list_integration_activation_control_room`
- `smart_home.get_integration_activation_control_room_summary`
- `smart_home.list_integration_activation_command_center`
- `smart_home.get_integration_activation_command_center_summary`
- `smart_home.list_integration_activation_watchtower`
- `smart_home.get_integration_activation_watchtower_summary`
- `smart_home.list_integration_activation_sentinel`
- `smart_home.get_integration_activation_sentinel_summary`
- `smart_home.list_integration_activation_audit`
- `smart_home.get_integration_activation_audit_summary`
- `smart_home.list_integration_activation_escalations`
- `smart_home.get_integration_activation_escalation_summary`
- `smart_home.list_integration_activation_responses`
- `smart_home.get_integration_activation_response_summary`
- `smart_home.list_integration_activation_remediation`
- `smart_home.get_integration_activation_remediation_summary`
- `smart_home.list_integration_activation_closure`
- `smart_home.get_integration_activation_closure_summary`
- `smart_home.list_integration_activation_release`
- `smart_home.get_integration_activation_release_summary`
- `smart_home.list_integration_activation_delivery`
- `smart_home.get_integration_activation_delivery_summary`
- `smart_home.list_integration_activation_deployment`
- `smart_home.get_integration_activation_deployment_summary`
- `smart_home.list_integration_activation_waiver_closures`
- `smart_home.get_integration_activation_waiver_closure_summary`
- `smart_home.list_integration_activation_waiver_archives`
- `smart_home.get_integration_activation_waiver_archive_summary`
- `smart_home.list_integration_activation_risk`
- `smart_home.get_integration_activation_risk_summary`
- `smart_home.list_integration_activation_dependencies`
- `smart_home.get_integration_activation_dependency_summary`
- `smart_home.list_integration_readiness`
- `smart_home.get_integration_readiness_summary`
- `smart_home.list_integration_readiness_gaps`
- `smart_home.get_integration_readiness_gap_summary`
- `smart_home.discover`
- `smart_home.list_discovery_workers`
- `smart_home.get_discovery_summary`
- `smart_home.get_pairing_plan`
- `smart_home.list_bridges`
- `smart_home.list_devices`
- `smart_home.list_rooms`
- `smart_home.list_scenes`
- `smart_home.describe_scene`
- `smart_home.get_state`
- `smart_home.describe_capabilities`
- `smart_home.get_health`
- `smart_home.command`
- `smart_home.report_event`
- `smart_home.subscribe`
- `smart_home.poll_events`
- `smart_home.unsubscribe`
- `smart_home.list_subscriptions`
- `smart_home.inspect_event_log`
- `smart_home.list_command_results`
- `smart_home.get_command_result_summary`
- `smart_home.list_authorization_decisions`
- `smart_home.get_authorization_summary`
- `smart_home.list_capability_grants`
- `smart_home.get_capability_grant_summary`
- `smart_home.get_controller_handoff_summary`
- `smart_home.get_platform_brief`
- `smart_home.list_platform_evidence_ledger`
- `smart_home.get_platform_evidence_ledger_summary`
- `smart_home.list_platform_access_review`
- `smart_home.get_platform_access_review_summary`
- `smart_home.list_platform_event_ops_review`
- `smart_home.get_platform_event_ops_review_summary`
- `smart_home.get_runtime_snapshot`
- `smart_home.get_pending_work_summary`
- `smart_home.get_attention_overview`
- `smart_home.get_system_health_brief`
- `smart_home.get_remediation_plan`
- `smart_home.get_operations_brief`
- `smart_home.get_safety_brief`
- `smart_home.get_readiness_brief`
- `smart_home.get_maintenance_brief`
- `smart_home.get_incident_brief`
- `smart_home.get_recovery_brief`
- `smart_home.get_morning_brief`
- `smart_home.get_escalation_brief`
- `smart_home.get_continuity_brief`
- `smart_home.get_operator_readiness_brief`
- `smart_home.get_shift_handoff_brief`
- `smart_home.get_closeout_brief`
- `smart_home.get_closeout_receipt`
- `smart_home.get_closeout_audit_trail`
- `smart_home.get_closeout_archive`
- `smart_home.get_closeout_archive_manifest`
- `smart_home.get_closeout_retention_ledger`
- `smart_home.get_topology_summary`
- `smart_home.list_desired_states`
- `smart_home.set_desired_state`
- `smart_home.clear_desired_state`
- `smart_home.list_pairing_sessions`
- `smart_home.list_workers`
- `smart_home.get_worker_heartbeat_schedule`
- `smart_home.get_supervision_plan`
- `smart_home.list_runtime_maintenance_windows`
- `smart_home.get_runtime_maintenance_window_summary`
- `smart_home.list_runtime_maintenance_actions`
- `smart_home.get_runtime_maintenance_action_summary`
- `smart_home.list_runtime_maintenance_plans`
- `smart_home.get_runtime_maintenance_plan_summary`
- `smart_home.list_runtime_maintenance_tickets`
- `smart_home.get_runtime_maintenance_ticket_summary`
- `smart_home.list_runtime_maintenance_work_orders`
- `smart_home.get_runtime_maintenance_work_order_summary`
- `smart_home.list_runtime_maintenance_work_order_guardrails`
- `smart_home.get_runtime_maintenance_work_order_guardrail_summary`
- `smart_home.list_runtime_maintenance_work_order_evidence`
- `smart_home.get_runtime_maintenance_work_order_evidence_summary`
- `smart_home.list_runtime_maintenance_work_order_evidence_reviews`
- `smart_home.get_runtime_maintenance_work_order_evidence_review_summary`
- `smart_home.list_runtime_maintenance_work_order_evidence_review_dispositions`
- `smart_home.get_runtime_maintenance_work_order_evidence_review_disposition_summary`
- `smart_home.list_runtime_maintenance_work_order_evidence_review_disposition_actions`
- `smart_home.get_runtime_maintenance_work_order_evidence_review_disposition_action_summary`
- `smart_home.list_runtime_maintenance_work_order_evidence_review_disposition_action_outcomes`
- `smart_home.get_runtime_maintenance_work_order_evidence_review_disposition_action_outcome_summary`
- `smart_home.list_runtime_maintenance_work_order_evidence_review_disposition_action_outcome_readiness`
- `smart_home.get_runtime_maintenance_work_order_evidence_review_disposition_action_outcome_readiness_summary`
- `smart_home.list_runtime_maintenance_work_order_evidence_review_disposition_action_outcome_readiness_handoffs`
- `smart_home.get_runtime_maintenance_work_order_evidence_review_disposition_action_outcome_readiness_handoff_summary`
- `smart_home.list_runtime_maintenance_work_order_evidence_review_disposition_action_outcome_readiness_handoff_reconciliations`
- `smart_home.get_runtime_maintenance_work_order_evidence_review_disposition_action_outcome_readiness_handoff_reconciliation_summary`
- `smart_home.list_runtime_maintenance_closeout_packets`
- `smart_home.get_runtime_maintenance_closeout_summary`
- `smart_home.reconcile_desired_states`
- `smart_home.run_supervision_tick`
- `smart_home.pair_bridge`
- `smart_home.complete_pairing`
- `smart_home.observe_supervision`

## Development

```bash
bash BUILD
```
