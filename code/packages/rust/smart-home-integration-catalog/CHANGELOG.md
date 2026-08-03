# Changelog

- Add first-party Blue Iris local HTTPS challenge-response authentication and
  read-only NVR/camera health inspection.
- Record capability-probed, queue-aware Axis preset recall and bounded PTZ
  movement over the authenticated VAPIX host.
- Add the first-party Axis VAPIX mDNS and authenticated inspection runtime.
- Record capability-probed Reolink preset recall and bounded PTZ movement over
  the authenticated local CGI runtime.

- Upgrade Reolink with capability-probed recording state and authorized,
  readback-verified recording enable/disable over the authenticated CGI host.

- Record AirGradient's typed non-credential configuration entity and validated
  correction-profile surface.

- Upgrade AirGradient runtime coverage with authorized local indicator/display
  controls, CO2 calibration, readback verification, and explicit cloud-control
  conflict handling.

- Add first-party AirGradient local environmental telemetry coverage and the
  environmental telemetry primitive.

## Unreleased

- Upgraded HEOS runtime coverage with D23-authorized local playback, volume,
  grouping, and queue controls over the existing TCP command host.
- Upgraded HEOS CLI runtime coverage from polling-only inspection to local
  push through authorized, bounded change-event subscriptions.
- Added first-party HEOS CLI runtime coverage for SSDP/manual discovery and
  read-only player identity, playback, volume, mute, and media inspection.
- Added a reusable TCP primitive family for bounded local stream protocols.
- Added first-party HomeWizard Energy API v1 local runtime coverage for verified
  mDNS/manual discovery and read-only device and external-meter telemetry.
- Added first-party Fronius Solar API v1 local runtime coverage for mDNS/manual
  discovery and read-only site and inverter power and energy telemetry.
- Added a first-party native Tasmota local HTTP path for mDNS/manual discovery,
  optional authenticated polling, and verified relay and light commands while
  preserving MQTT as the preferred push transport.
- Added first-party Nanoleaf local runtime coverage for mDNS discovery,
  physical-presence token pairing, authenticated polling, and verified light
  commands.
- Marked Sonos as a first-party polling runtime for SSDP discovery and
  read-only UPnP player-state inspection.

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-05-08

### Added

- First-party TP-Link Kasa legacy LAN runtime coverage with UDP broadcast
  discovery, local polling, and verified plug, switch, and light commands.
- First-party Govee LAN runtime coverage with UDP multicast discovery,
  normalized local polling, and bounded UDP command primitives.
- First-party ONVIF runtime catalog coverage with WS-Discovery, normalized
  camera entities, local HTTP, and privacy-gated media primitives.
- Integration catalog enums for category, connectivity class, discovery,
  authentication, and implementation status.
- Primitive-family metadata for discovery, transport, auth/pairing, command
  mapping, capability policy, Vault leases, supervision, camera/media,
  telemetry, and test simulators.
- First-party seed catalog entries for Hue, Zigbee, Z-Wave, Thread, MQTT,
  Matter, HomeKit Controller, ESPHome, Tasmota, Shelly, TP-Link/Tapo, WLED,
  LIFX, cameras/media, energy/climate, and cloud hubs.
- Mesh release ticket handoff disposition-action slot clearance readiness
  summaries that combine slot readiness with clearance-row counts and next
  clearance pointers.
- Mesh release ticket handoff disposition-action slot clearance action rows
  that turn clearance outcomes into repair, review, lineage, and release
  handoff work.
- Mesh release ticket handoff disposition-action slot clearance action
  readiness summaries that combine clearance readiness with next action
  pointers.
- Mesh release ticket handoff disposition-action slot clearance action evidence
  rows that capture clearance-action evidence lineage for repair, review,
  lineage, and release handoff work.
- Mesh release ticket handoff disposition-action slot clearance action evidence
  review rows that classify clearance-action evidence into blocker, operator,
  review, lineage, and release-ready review outcomes.
- Mesh release ticket handoff disposition-action slot clearance action evidence
  readiness summaries that combine action readiness with evidence counts and
  next evidence pointers.
- Mesh release ticket handoff disposition-action slot clearance action evidence
  review readiness summaries that combine evidence readiness with review counts
  and next review pointers.
- Mesh release ticket handoff disposition-action slot clearance action evidence
  review clearance rows that project review readiness into repair, review,
  dispatch, execution, lineage, and release handoff outcomes.
- Mesh release ticket handoff disposition-action slot clearance action evidence
  review clearance readiness summaries that combine review readiness with
  clearance-row counts and next clearance pointers.
- Mesh release ticket handoff disposition-action slot clearance action evidence
  review clearance action rows that turn clearance readiness into repair,
  review, dispatch, execution, lineage, and release handoff work.
- Mesh release ticket handoff disposition-action slot clearance action evidence
  review clearance action readiness summaries that combine clearance readiness
  with action-row counts and next clearance-action pointers.
- Mesh release ticket handoff disposition-action slot clearance action evidence
  review clearance action readiness evidence rows that capture repair, review,
  dispatch, execution, lineage, and release handoff evidence.
- Mesh release ticket handoff disposition-action slot clearance action evidence
  review clearance action readiness evidence summaries that combine evidence
  readiness with evidence counts and next readiness evidence pointers.
- Mesh release ticket handoff disposition-action slot clearance action evidence
  review clearance action readiness evidence review rows that classify
  readiness evidence into blocker, operator, review, lineage, and release-ready
  outcomes.
- Mesh release ticket handoff disposition-action slot clearance action evidence
  review clearance action readiness evidence review summaries that combine
  readiness-evidence review rows with review counts and next review pointers.
- Mesh release ticket handoff disposition-action slot clearance action evidence
  review clearance action readiness evidence review disposition rows that turn
  reviewed readiness evidence into repair, operator, review, lineage, and
  release-ready handoff outcomes.
- Mesh release ticket handoff disposition-action slot clearance action evidence
  review clearance action readiness evidence review disposition summaries that
  combine disposition outcomes with next repair, review, lineage, and release
  handoff pointers.
- Mesh release ticket handoff disposition-action slot clearance action evidence
  review clearance action readiness evidence review disposition-action rows
  that turn reviewed readiness-evidence dispositions into repair, review,
  lineage, and release handoff work.
- Mesh release ticket handoff disposition-action slot clearance action evidence
  review clearance action readiness evidence review disposition-action
  summaries that combine action counts with next repair, review, lineage, and
  release handoff pointers.
- Mesh release ticket handoff disposition-action slot clearance action evidence
  review clearance action readiness evidence review disposition-action
  readiness summaries that lift action outcomes into release-ready package
  rollups.
- Mesh release ticket handoff readiness-evidence review disposition-action
  readiness execution slots that sequence action-readiness outcomes into
  repair, review, and release handoff work.
- Mesh release ticket handoff readiness-evidence review disposition-action
  readiness execution-slot summaries that combine slot counts with next
  actionable repair, review, and release handoff pointers.
- Mesh release ticket handoff readiness-evidence review disposition-action
  readiness execution handoff rows that project execution slots into repair,
  review, and release handoff lineage.
- Mesh release ticket handoff readiness-evidence review disposition-action
  readiness execution handoff summaries that combine handoff rows with next
  repair, review, and release handoff pointers.
- Mesh release ticket handoff readiness-evidence review disposition-action
  readiness execution handoff action rows that turn handoff lineage into
  repair, review, operator, dispatch, execution, and release handoff actions.
- Mesh release ticket handoff readiness-evidence review disposition-action
  readiness execution handoff action summaries that combine action-row counts
  with next repair, review, and release handoff pointers.
- Mesh release ticket handoff protocol-evidence package handoff execution
  action rows that turn package execution slots into repair, review, operator,
  dispatch, execution, and release handoff work.
- Mesh release ticket handoff protocol-evidence package handoff execution
  action summaries that combine action-row counts with next repair, review,
  operator, dispatch, execution, and release handoff pointers.
- Mesh release ticket handoff protocol-evidence package handoff execution
  action evidence rows that preserve action, slot, package, packet, blocker,
  and release evidence lineage.
- Mesh release ticket handoff protocol-evidence package handoff execution
  action evidence review rows that classify protocol evidence into blocker,
  operator, review, lineage, and release-ready outcomes.
- Mesh release ticket handoff protocol-evidence package handoff execution
  action evidence summaries that combine evidence-row counts with next
  blocker, protocol, and release handoff pointers.
- Mesh release ticket handoff protocol-evidence package handoff execution
  action evidence review summaries that combine review counts with next
  blocker, protocol, and release handoff pointers.
- Mesh release ticket handoff readiness-evidence review disposition-action
  readiness execution handoff action evidence rows that preserve action
  lineage, release evidence, and handoff readiness flags.
- Mesh release ticket handoff readiness-evidence review disposition-action
  readiness execution handoff action evidence review rows that classify
  handoff-action evidence into blocker, operator, review, lineage, and
  release-ready outcomes.
- Mesh release ticket handoff readiness-evidence review disposition-action
  readiness execution handoff action evidence review disposition rows that
  turn evidence review outcomes into repair, review, lineage, operator, and
  release handoff actions.
- Mesh release ticket handoff readiness-evidence review disposition-action
  readiness execution handoff action evidence review disposition action rows
  that turn reviewed dispositions into concrete repair, review, lineage,
  operator, and release handoff actions.
- Mesh release ticket handoff readiness-evidence review disposition-action
  readiness execution handoff action evidence review disposition action
  readiness rows that mark action readiness for repair, review, lineage,
  operator, and release handoff follow-up.
- Mesh release ticket handoff readiness-evidence review disposition-action
  readiness execution handoff action evidence review disposition action
  readiness summaries that combine readiness rows with next repair, review,
  lineage, operator, and release handoff pointers.
- Mesh release ticket handoff readiness-evidence review disposition-action
  readiness execution handoff action evidence review disposition action
  readiness execution slots that sequence readiness rows into deterministic
  repair, review, operator, and release handoff work.
- Mesh release ticket handoff readiness-evidence review disposition-action
  readiness execution handoff action evidence review disposition action
  readiness execution slot summaries that combine slot counts with next repair,
  review, operator, lineage, and release handoff pointers.
- Mesh release ticket handoff readiness-evidence review disposition-action
  readiness execution handoff action evidence review disposition action
  readiness execution handoff rows that project execution slots into repair,
  review, operator, and release handoff work.
- Mesh release ticket handoff readiness-evidence review disposition-action
  readiness execution handoff action evidence review disposition action
  readiness execution handoff summaries that combine handoff rows with next
  repair, review, operator, lineage, and release handoff pointers.
- Mesh release ticket handoff readiness-evidence review disposition-action
  readiness execution handoff action evidence review disposition action
  readiness execution handoff action rows that turn handoff readiness into
  repair, review, operator, and release actions.
- Mesh release ticket handoff readiness-evidence review disposition-action
  readiness execution handoff action evidence review disposition action
  readiness execution handoff action summaries that combine action rows with
  next repair, review, operator, lineage, and release handoff pointers.
- Mesh release ticket handoff readiness-evidence review disposition-action
  readiness execution handoff action evidence review disposition action
  summaries that combine action rows with next repair, review, lineage, and
  release handoff pointers.
- Mesh release ticket handoff readiness-evidence review disposition-action
  readiness execution handoff action evidence review disposition summaries
  that combine disposition rows with next repair, review, lineage, and release
  handoff pointers.
- Mesh release ticket handoff readiness-evidence review disposition-action
  readiness execution handoff action evidence review summaries that combine
  review rows with next blocker, review, lineage, and release-ready pointers.
- Mesh release ticket handoff readiness-evidence review disposition-action
  readiness execution handoff action evidence summaries that combine evidence
  rows with next repair, review, and release handoff pointers.
- Mesh release ticket handoff readiness-evidence review disposition-action
  readiness execution handoff action protocol-evidence packet rows that join
  Zigbee, Z-Wave, and Thread substrate readiness with handoff-action evidence
  state.
- Mesh release ticket handoff readiness-evidence review disposition-action
  readiness execution handoff action protocol-evidence packet summaries that
  combine packet readiness, substrate blockers, and first actionable evidence
  pointers.
- Mesh release ticket handoff readiness-evidence review disposition-action
  readiness execution handoff action protocol-evidence package readiness
  summaries that lift protocol packet state into package-facing go/no-go
  release coordination.
- Low-level mesh release ticket handoff readiness-evidence review
  disposition-action readiness execution handoff action protocol-evidence
  package handoff rows that project Zigbee, Z-Wave, and Thread packet
  readiness into package handoff work.
- Mesh release ticket handoff readiness-evidence review disposition-action
  readiness execution handoff action protocol-evidence package handoff
  summaries that combine package handoff rows with next actionable protocol
  and evidence pointers.
- Low-level mesh release ticket handoff readiness-evidence review
  disposition-action readiness execution handoff action protocol-evidence
  package handoff execution slots that order Zigbee, Z-Wave, and Thread
  package handoff work into deterministic release lanes.
- Mesh release ticket handoff readiness-evidence review disposition-action
  readiness execution handoff action protocol-evidence package handoff
  execution slot summaries that combine slot lane counts with next actionable
  protocol and release handoff pointers.
- Virtual alias entries for product lines supported by another integration or
  standard.
- Read-only D18D tool descriptors for listing/describing integrations and
  primitive families.
- Typed ecosystem-survey source rows for the cross-platform references used to
  plan primitive families across Home Assistant, Hubitat, Homey Pro,
  SmartThings, openHAB, Homebridge, ioBroker, Domoticz, Jeedom, HomeSeer, Apple
  Home, Google Home, Alexa, Z-Wave Alliance, and Thread Group.
- Ecosystem primitive coverage reports that map surveyed platforms onto rollout
  backlog primitive families.
- Primitive coverage summary helpers for counting uncovered, single-source, and
  multi-platform primitive-backlog rows.
- Ecosystem platform coverage item and summary helpers that show which surveyed
  platform lessons overlap a priority-bounded reusable primitive backlog.
- Query helpers for integration id, category, connectivity, capability,
  primitive family, implementation status, and rollout priority.
- `IntegrationCatalogEntrySummary` and `hue_catalog_entry_summary` for compact
  Hue package/spec metadata, local/cloud, policy-tier, and catalog shape
  inspection.
- `IntegrationActivationPackageSummary` and `hue_activation_package_summary`
  for joining Hue catalog metadata with activation-plan target, prerequisite,
  and policy-review shape.
- `IntegrationReadinessPackageSummary` and `hue_readiness_package_summary` for
  joining Hue activation-package shape with host-specific readiness blockers.
- `IntegrationActivationEvidenceBriefingSummary` and
  `hue_activation_evidence_briefing_summary` for compact catalog,
  activation-plan, readiness, policy, and local-boundary release evidence
  briefs.
- `IntegrationActivationEvidenceScorecardSummary` and catalog-wide activation
  evidence scorecard helpers for rolling briefing readiness, blocker lanes,
  missing prerequisites, local/cloud boundaries, and policy tiers into compact
  Chief planning counts.
- `IntegrationActivationEvidenceRow` and activation evidence row helpers for
  turning each integration briefing into sorted ready/blocker rows with the
  next blocked evidence lane and missing-prerequisite counts.
- `IntegrationActivationEvidenceLaneInventoryItem` and lane inventory helpers
  for grouping blocked evidence rows by their first catalog, activation-plan,
  readiness, policy, or local-boundary blocker lane.
- `IntegrationActivationEvidenceRemediationItem` and remediation helpers for
  turning evidence lane inventories into sorted catalog-owned remediation
  plans before Chief-specific escalation tooling consumes them.
- `IntegrationMeshReadinessPackageSummary` and mesh readiness package helpers
  for combining Zigbee, Z-Wave, and Thread primitive substrate readiness with
  mesh-scoped activation evidence remediation rollups.
- `IntegrationMeshProtocolSubstrateStageRow` and summary helpers for
  classifying low-level mesh primitive blockers into controller, radio,
  discovery, network-security, and supervision stages.
- `IntegrationMeshProtocolSubstrateAction` and summary helpers for ordering
  missing Zigbee, Z-Wave, and Thread substrate primitives into protocol-scoped
  action queues.
- `IntegrationMeshProtocolSubstratePreflightCheck` and summary helpers for
  package-independent Zigbee, Z-Wave, and Thread substrate preflight gates.
- `IntegrationMeshProtocolSubstratePreflightAction` and summary helpers for
  turning failed mesh substrate preflight gates into protocol-scoped repair
  actions.
- `IntegrationMeshProtocolSubstratePreflightRepairBatch` and summary helpers
  for grouping failed mesh substrate preflight repairs by stage and action kind.
- `IntegrationMeshProtocolSubstratePreflightRepairScheduleSlot` and summary
  helpers for ordering mesh preflight repair batches into execution slots.
- `IntegrationMeshProtocolSubstratePreflightRepairSlotAuditRow` and summary
  helpers for auditing scheduled repair-slot blockers and operator handoffs.
- `IntegrationMeshProtocolSubstratePreflightRepairSlotExecutionTicket` and
  summary helpers for turning audited repair slots into operator-ready
  execution tickets.
- `IntegrationMeshProtocolSubstratePreflightRepairSlotExecutionWorkOrder` and
  summary helpers for projecting execution tickets into deterministic release
  work orders.
- `IntegrationMeshProtocolSubstratePreflightRepairSlotExecutionWorkOrderGuardrail`
  and summary helpers for classifying low-level release work orders into
  release blockers, operator handoffs, and ready-to-execute work.
- `IntegrationMeshStageReleaseSummary` and helpers for combining substrate
  stage blockers, primitive readiness blockers, and mesh-scoped remediation
  into a single release readiness rollup.
- `IntegrationMeshActionReadinessSummary` and helpers for combining mesh
  release readiness with the substrate action queue and next concrete low-level
  action.
- `IntegrationMeshReleaseReadinessSummary` and helpers for package-facing mesh
  release readiness across package, substrate-stage, queued-action, and
  remediation blockers.
- `IntegrationMeshPreflightReadinessSummary` and helpers for combining mesh
  release readiness with substrate preflight gates.
- `IntegrationMeshPreflightRepairReadinessSummary` and helpers for combining
  mesh preflight readiness with protocol-scoped repair actions.
- `IntegrationMeshPreflightBatchReadinessSummary` and helpers for combining
  mesh preflight repair readiness with stage/action-kind repair batches.
- `IntegrationMeshPreflightScheduleReadinessSummary` and helpers for combining
  mesh preflight batch readiness with deterministic repair execution slots.
- `IntegrationMeshPreflightSlotReadinessSummary` and helpers for combining mesh
  preflight schedule readiness with repair-slot audit blockers and operator
  handoff counts.
- `IntegrationMeshPreflightExecutionReadinessSummary` and helpers for combining
  mesh preflight slot readiness with operator-ready execution ticket counts.
- `IntegrationMeshPreflightWorkOrderReadinessSummary` and helpers for combining
  mesh preflight execution readiness with release work-order counts.
- `IntegrationMeshProtocolSubstratePreflightRepairSlotExecutionEvidenceReviewDispositionAction`
  and summary helpers for turning mesh execution evidence review dispositions
  into operator, repair, lineage, and release queues.
- `IntegrationMeshProtocolSubstratePreflightRepairSlotExecutionEvidenceReviewDispositionActionSlot`
  and summary helpers for sequencing disposition-action queues into
  deterministic operator and repair execution slots.
- `IntegrationMeshPreflightGuardrailReadinessSummary` and helpers for combining
  mesh preflight work-order readiness with guardrail and evidence-review
  disposition counts.
- `IntegrationMeshPreflightDispositionActionReadinessSummary` and helpers for
  combining mesh preflight guardrail readiness with evidence disposition action
  queue counts.
- `IntegrationMeshReadinessHandoffPackage`, summary helpers, and D23 mesh
  readiness handoff projections for release coordination across substrate
  actions, evidence remediation, and release-ready state.
- `IntegrationMeshReleaseReadinessCheck`, summary helpers, and D23 mesh
  release-readiness gate projections for substrate actions, evidence
  remediation, human review, operator handoff, and release packet state.
- `IntegrationMeshReleaseReadinessCheckSlot` and summary helpers for
  sequencing mesh release-readiness gates into deterministic low-level
  operator and release execution slots.
- `IntegrationMeshReleasePacketReadinessSummary` and helpers for condensing
  mesh release-readiness checks into package-facing go/no-go state and first
  actionable gates.
- `IntegrationMeshReleaseExecutionReadinessSummary` and helpers for combining
  mesh release packet readiness with release-readiness check slot execution
  state.
- `IntegrationMeshReleaseExecutionTask` and summary helpers for turning mesh
  release-readiness check slots into deterministic low-level operator work.
- `IntegrationMeshReleaseTaskReadinessSummary` and helpers for combining mesh
  release execution readiness with deterministic task state.
- `IntegrationMeshReleaseExecutionTaskDispatchSlot` and summary helpers for
  turning release execution tasks into deterministic dispatch queues.
- `IntegrationMeshReleaseDispatchReadinessSummary` and helpers for combining
  release task readiness with dispatch queue state.
- `IntegrationMeshReleaseDispatchTicket` and summary helpers for wrapping
  release dispatch queues in deterministic ticket keys.
- `IntegrationMeshReleaseDispatchTicketHandoffPacket` and summary helpers for
  classifying release dispatch tickets into release, operator, repair, and
  review lanes.
- `IntegrationMeshReleaseTicketReadinessSummary` and helpers for combining
  release dispatch readiness with deterministic ticket state.
- `IntegrationMeshReleaseTicketHandoffReadinessSummary` and helpers for
  combining release ticket readiness with dispatch handoff lane state.
- `IntegrationMeshReleaseTicketHandoffExecutionSlot` and summary helpers for
  sequencing dispatch handoff packets into deterministic lane execution slots.
- `IntegrationMeshReleaseTicketHandoffExecutionReadinessSummary` and helpers
  for combining handoff readiness with execution slot state.
- `IntegrationMeshReleaseTicketHandoffExecutionWorkOrder` and summary helpers
  for turning release handoff execution slots into lane-scoped work orders.
- `IntegrationMeshReleaseTicketHandoffWorkOrderReadinessSummary` and helpers
  for combining handoff execution readiness with deterministic work-order lane
  state.
- `IntegrationMeshReleaseTicketHandoffExecutionWorkOrderGuardrail` and summary
  helpers for classifying handoff work-order lanes into release blockers,
  operator handoffs, review gates, and ready-to-execute checks.
- `IntegrationMeshReleaseTicketHandoffExecutionWorkOrderGuardrailAuditRow` and
  summary helpers for exposing handoff work-order guardrail audit rows for
  release coordination.
- `IntegrationMeshReleaseTicketHandoffExecutionWorkOrderGuardrailAuditClearanceRow`
  and summary helpers for turning guardrail audit rows into deterministic
  clear/block, review, and operator handoff decisions.
- `IntegrationMeshReleaseTicketHandoffExecutionWorkOrderGuardrailAuditClearanceAction`
  and summary helpers for turning clearance decisions into deterministic
  repair, review, handoff, or release-clearance work.
- `IntegrationMeshReleaseTicketHandoffExecutionWorkOrderGuardrailAuditClearanceActionEvidence`
  and summary helpers for preserving clearance action evidence lineage across
  blocker, operator, review, and release lanes.
- `IntegrationMeshReleaseTicketHandoffExecutionWorkOrderGuardrailAuditClearanceActionEvidenceReview`
  and summary helpers for classifying clearance action evidence into blocker,
  operator, review, lineage, and release-ready outcomes.
- `IntegrationMeshReleaseTicketHandoffExecutionWorkOrderGuardrailAuditClearanceActionEvidenceReviewDisposition`
  and summary helpers for routing clearance action evidence reviews into
  repair, operator, review, lineage, and release-ready disposition rows.
- `IntegrationMeshReleaseTicketHandoffExecutionWorkOrderGuardrailAuditClearanceActionEvidenceReviewDispositionAction`
  and summary helpers for turning clearance evidence-review dispositions into
  repair, operator, review, lineage, and release handoff actions.
- `IntegrationMeshReleaseTicketHandoffExecutionWorkOrderGuardrailAuditClearanceActionEvidenceReviewDispositionActionSlot`
  and summary helpers for sequencing disposition actions into release handoff
  execution slots.
- `IntegrationMeshReleaseTicketHandoffExecutionWorkOrderGuardrailAuditClearanceActionEvidenceReviewDispositionActionSlotClearanceRow`
  and summary helpers for classifying sequenced disposition-action slots into
  repair, review, lineage, and release handoff outcomes.
- `IntegrationMeshReleaseTicketHandoffGuardrailAuditClearanceActionEvidenceReviewDispositionActionReadinessSummary`
  and helpers for combining disposition readiness with disposition-action
  counts and next action pointers.
- `IntegrationMeshReleaseTicketHandoffGuardrailAuditClearanceActionEvidenceReviewDispositionActionSlotReadinessSummary`
  and helpers for combining disposition-action readiness with slot counts and
  next slot pointers.
- `IntegrationMeshReleaseTicketHandoffGuardrailAuditClearanceActionEvidenceReviewDispositionReadinessSummary`
  and helpers for combining clearance evidence-review readiness with
  disposition outcomes.
- `IntegrationMeshReleaseTicketHandoffGuardrailAuditClearanceActionEvidenceReadinessSummary`
  and helpers for combining clearance action readiness with evidence lineage
  state.
- `IntegrationMeshReleaseTicketHandoffGuardrailAuditClearanceActionEvidenceReviewReadinessSummary`
  and helpers for combining clearance action evidence readiness with
  evidence-review outcomes.
- `IntegrationMeshReleaseTicketHandoffWorkOrderGuardrailReadinessSummary` and
  helpers for combining work-order readiness with handoff guardrail state.
- `IntegrationMeshReleaseTicketHandoffGuardrailAuditReadinessSummary` and
  helpers for combining work-order guardrail readiness with audit-row state.
- `IntegrationMeshReleaseTicketHandoffGuardrailAuditClearanceReadinessSummary`
  and helpers for combining audit readiness with guardrail audit clearance
  state.
- `IntegrationMeshReleaseTicketHandoffGuardrailAuditClearanceActionReadinessSummary`
  and helpers for combining clearance readiness with guardrail audit clearance
  action state.
- Primitive backlog planning helpers for ranking the shared primitive families
  needed by priority-bounded rollout waves.
- Integration activation planning helpers for resolving virtual aliases,
  standard-backed products, required primitives, capabilities, auth modes,
  dependencies, and review tiers before enabling an integration.
- `IntegrationActivationPlanSummary` for compact direct/delegated target,
  review, local/cloud, dependency, primitive, and capability rollups over
  activation-plan sets.
- `IntegrationActivationCandidate` and `IntegrationActivationCandidateSummary`
  for ranking ready, human-review, and blocked activation work after applying
  host-specific readiness context.
- `IntegrationActivationAction` and `IntegrationActivationActionSummary` for
  converting activation candidates into concrete activate, policy-review,
  primitive, capability, and dependency work items.
- `IntegrationActivationAgendaStage` and `IntegrationActivationAgendaSummary`
  for grouping activation candidates and concrete action work by rollout
  priority wave.
- `IntegrationActivationRunwayStage` and `IntegrationActivationRunwaySummary`
  for grouping activation candidates by rollout priority wave and identifying
  actionable, review, and blocked stages.
- `IntegrationActivationHealthStage` and `IntegrationActivationHealthSummary`
  for compact ready, review, blocked, and missing-prerequisite priority-wave
  status rollups.
- `IntegrationActivationMaintenanceWindow` and
  `IntegrationActivationMaintenanceSummary` for combining priority-wave
  health, actions, constraints, risk, and dependency blockers into compact
  Chief planning windows.
- `IntegrationActivationConstraint` and
  `IntegrationActivationConstraintSummary` for grouping unresolved primitive,
  capability, dependency, and policy-review surface work by affected
  integrations.
- `IntegrationActivationReviewItem` and `IntegrationActivationReviewSummary`
  for exposing human-review queue entries with review-ready and blocked
  rollups.
- `IntegrationActivationApprovalPacket` and
  `IntegrationActivationApprovalSummary` for bundling review rows with concrete
  actions, grouped constraints, policy risk, and dependency blockers before a
  human approval request.
- `IntegrationActivationForecastItem` and
  `IntegrationActivationForecastSummary` for classifying activation timeline
  milestones into Chief-ready next actions across blockers, dependencies,
  approvals, reviews, risks, activation, and monitoring.
- `IntegrationActivationPlaybookStep` and
  `IntegrationActivationPlaybookSummary` for pairing forecast next actions with
  recommended planning views and operator-readiness flags.
- `IntegrationActivationRunbookEntry` and
  `IntegrationActivationRunbookSummary` for joining playbook steps to audit,
  risk, dependency, and readiness-gap context.
- `IntegrationActivationHandoffPackage`,
  `IntegrationActivationExecutionPacket`, and
  `IntegrationActivationVerificationCheckpoint` rollups for turning activation
  runbooks into execution-transfer, execution-readiness, and post-execution
  verification views.
- `IntegrationActivationOperatorTask` and
  `IntegrationActivationOperatorTaskSummary` for turning playbook steps into
  actionable human/operator queue rows.
- `IntegrationActivationControlRoomPanel` and
  `IntegrationActivationControlRoomSummary` for grouping operator queue work by
  recommended planning view.
- `IntegrationActivationCommandCenterSection` and
  `IntegrationActivationCommandCenterSummary` for grouping control-room panels
  into blocker, review, activation, actionable, and monitoring operating lanes.
- `IntegrationActivationWatchtowerSignal` and
  `IntegrationActivationWatchtowerSummary` for rolling command-center sections
  into escalation, review, ready, action, and observation signal lanes.
- `IntegrationActivationSentinelAlert` and
  `IntegrationActivationSentinelSummary` for combining watchtower, risk,
  dependency, and readiness-gap rollups into blocker, dependency, policy-risk,
  review, ready, and observation alert lanes.
- `IntegrationActivationAuditRecord` and `IntegrationActivationAuditSummary`
  for connecting sentinel alerts to watchtower signals, decisions, evidence,
  policy risk, dependency blockers, and readiness gaps in one audit trail.
- `IntegrationActivationEscalationCase` and
  `IntegrationActivationEscalationSummary` for packaging sentinel alerts,
  verification checkpoints, and audit records into Chief-facing escalation
  queues.
- `IntegrationActivationResponseItem` and
  `IntegrationActivationResponseSummary` for turning escalation cases into
  owner-lane next actions for response planning.
- `IntegrationActivationRemediationItem` and
  `IntegrationActivationRemediationSummary` for turning response items into
  executable owner-lane remediation queues.
- `IntegrationActivationClosureGate` and `IntegrationActivationClosureSummary`
  for turning remediation work orders into release closure gates.
- `IntegrationActivationReleasePacket` and
  `IntegrationActivationReleaseSummary` for turning closure gates into compact
  activation release go/no-go packets.
- `IntegrationActivationDeliveryManifest` and
  `IntegrationActivationDeliverySummary` for turning release packets into
  delivery-channel manifests and readiness rollups.
- `IntegrationActivationDeploymentRecord` and
  `IntegrationActivationDeploymentSummary` for turning delivery manifests into
  deployment-ring records and deploy-readiness rollups.
- `IntegrationActivationRiskItem` and `IntegrationActivationRiskSummary` for
  grouping rollout candidates by policy tier and policy surface after applying
  host-specific readiness context.
- `IntegrationActivationDependencyGraph` plus node, edge, and summary types for
  exposing satisfied and blocking integration prerequisites in rollout plans.
- Integration readiness reports that expose missing primitive families, missing
  capability grants, and missing delegated integrations before activation.
- Integration readiness summaries for compact activation-ready, blocker,
  review, cloud, local, and delegated-target rollups.
- Integration readiness gap inventories that group missing primitive families,
  capability grants, and delegated integration dependencies by affected
  integrations.
- Mesh protocol primitive readiness rows and summaries for checking Zigbee,
  Z-Wave, and Thread controller, radio substrate, network-key, and supervision
  gaps before low-level activation work.
- Computed policy-surface helpers so Chief of Staff tools can identify camera,
  entry-access, climate, energy, cloud, credential, radio-network, and local
  actuation review boundaries before activating integrations.
- `IntegrationPolicySurfaceInventoryItem`, `IntegrationPolicySurfaceSummary`,
  and policy-surface inventory helpers for compact review, cloud, local, and
  privilege-tier planning rollups.
- Composable bounded integration catalog queries for combining priority,
  primitive, capability, policy, protocol, local/cloud, and virtual alias
  selectors in read-only D18D tools.
