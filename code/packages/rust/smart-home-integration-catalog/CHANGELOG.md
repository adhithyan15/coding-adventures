# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-05-08

### Added

- Integration catalog enums for category, connectivity class, discovery,
  authentication, and implementation status.
- Primitive-family metadata for discovery, transport, auth/pairing, command
  mapping, capability policy, Vault leases, supervision, camera/media,
  telemetry, and test simulators.
- First-party seed catalog entries for Hue, Zigbee, Z-Wave, Thread, MQTT,
  Matter, HomeKit Controller, ESPHome, Tasmota, Shelly, TP-Link/Tapo, WLED,
  LIFX, cameras/media, energy/climate, and cloud hubs.
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
