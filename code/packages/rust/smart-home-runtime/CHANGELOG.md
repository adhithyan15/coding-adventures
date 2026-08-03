# Changelog

All notable changes to this package will be documented in this file.

## Unreleased

### Added

- Authorization and optimistic-state routing for typed D23 media commands.
- `RuntimeDurableSnapshot` plus snapshot and restore helpers for normalized
  registry, state, history, pairing, optimistic, and desired-state data.
- `RuntimeEventDeliverySummary` plus delivery-batch `summary()` helpers for
  compact delivered-event and remaining-backlog counts after subscription polls.
- `RuntimePollEventsToolRequest` / `RuntimePollEventsToolOutput` and
  `RuntimeUnsubscribeToolRequest` / `RuntimeUnsubscribeToolOutput` for
  authorized D18D event delivery polling and subscription teardown.
- `RuntimePendingWorkSummary` helpers for compact pending-work status derived
  from runtime read snapshots.
- `RuntimeEventLogSummary` plus `event_log_summary()` for compact event-kind
  counts over selected runtime replay windows.
- `RuntimeCommandResultQuery`, `RuntimeCommandResultRecord`, and
  `RuntimeCommandResultSummary` plus authorized read-tool variants for typed
  command-result audit views over runtime event history.
- Upper sequence bounds on runtime event-log and command-result queries for
  bounded replay/audit windows.
- `RuntimeSubscriptionInventorySummary` plus `subscription_inventory_summary()`
  for compact event-stream filter coverage and backlog pressure checks.
- `RuntimeEventBusHealthSummary` plus event-bus/runtime helpers for composed
  replay history, stream coverage, and queue pressure checks.
- `RuntimePairingSessionInventorySummary` plus
  `pairing_session_inventory_summary_at()` for compact bridge-pairing status,
  expiry, and VaultRef completion counts.
- `RuntimePairingCompletion` for completing pending pairing sessions with a
  VaultRef plus non-secret completion metadata that is copied into the session
  and bridge-health audit event.
- `RuntimeCompletePairingToolRequest`,
  `RuntimeCompletePairingToolOutput`, and
  `SmartHomeRuntime::execute_complete_pairing_tool` for authorized D18D pairing
  completion through the runtime-owned mutation path.
- `RuntimeReportEventToolRequest`, `RuntimeReportEventToolOutput`, and
  `SmartHomeRuntime::execute_report_event_tool` for authorized D18D event
  ingest through the runtime-owned device-event and bridge-health paths.
- `RuntimeSetDesiredStateToolRequest`,
  `RuntimeSetDesiredStateToolOutput`,
  `RuntimeClearDesiredStateToolRequest`,
  `RuntimeClearDesiredStateToolOutput`, and runtime facade methods for
  authorized desired-state target mutation through runtime-owned validation.
- Runtime discovery catalog recording plus `RuntimeDiscoverToolRequest` and
  `RuntimeDiscoverToolOutput` for authorized discovery reads, freshness
  filtering, and unpaired bridge-candidate projection.
- `SmartHomeRuntime::record_discovery_worker_run` for ingesting reported
  discovery-worker batches with inserted, replaced, and ignored catalog counts.
- `ScheduledDiscoveryWorker`, `RuntimeDiscoveryScheduler`, and scheduled
  discovery run-plan helpers for runtime-owned discovery cadence, selected
  network interfaces, and last-run status.
- `DiscoveryWorkerRunPlan::mdns_scan_plan` and
  `SmartHomeRuntime::discovery_mdns_scan_plan_at` for projecting due mDNS
  schedules into per-interface IPv4/IPv6 scan requests.
- `SmartHomeRuntime::record_scheduled_discovery_worker_run` for ingesting a
  registered worker run, reconciling discovery results, and advancing the next
  scheduled due time.
- `SmartHomeRuntime::run_due_mdns_discovery_workers_with_executor` and
  `MdnsDiscoveryRunAdapter` for supervised mDNS discovery passes that execute
  due scan plans through injectable runners, record scheduled run summaries,
  and convert adapter failures into deterministic failed worker runs.
- `ScheduledDiscoveryWorkerSnapshot` and discovery scheduler details on
  `RuntimeSupervisionObservation` for read-side inspection of due workers, last
  run status, record/failure counts, catalog changes, and consecutive failure
  pressure.
- Scheduled discovery worker retry/backoff policy, including configurable
  initial retry delay, capped retry delay, multiplier, failure-driven cadence,
  and snapshot exposure of the current retry delay.
- `RuntimeSupervisionPlanSummary` plus plan/observation helpers for compact
  due-work counts over non-mutating supervision plans.
- `SupervisionTickSummary` plus tick-report helpers for compact actual-work
  counts after supervision ticks mutate runtime state.
- `RuntimeReadToolRequest::ListScenes` and
  `RuntimeReadToolRequest::DescribeScene` for authorized D18D reads over the
  registry-backed scene inventory.
- `RuntimeReadToolRequest::ListSubscriptions` and
  `RuntimeReadToolRequest::InspectEventLog` for authorized D18D reads over
  event-stream backlog pressure and checkpointed runtime event history.
- `RuntimeReadToolRequest::ListAuthorizationDecisions` and
  `RuntimeReadToolRequest::GetAuthorizationSummary` for authorized D18D reads
  over registry-backed smart-home authorization audit decisions.
- `RuntimeReadToolRequest::ListCapabilityGrants` and
  `RuntimeReadToolRequest::GetCapabilityGrantSummary` for authorized D18D
  reads over registry-backed smart-home capability grant governance.
- `RuntimeRoomQuery`, `RuntimeRoomSummary`,
  `RuntimeReadToolRequest::ListRooms`, and
  `RuntimeReadToolRequest::GetTopologySummary` for authorized D18D reads over
  registry-derived room and topology coverage.
- `RuntimeReadToolRequest::GetRuntimeSnapshot`,
  `RuntimeReadToolRequest::ListDesiredStates`, and
  `RuntimeReadToolRequest::ListPairingSessions` for authorized D18D reads over
  runtime pending work, desired-state targets, and pairing-session inventories.
- `RuntimeReadToolRequest::ListWorkers` and
  `RuntimeReadToolRequest::GetWorkerHeartbeatSchedule` for authorized D18D
  reads over supervised bridge-worker inventory and heartbeat deadlines.
- `RuntimeReadToolRequest::ListDiscoveryWorkers` and
  `RuntimeReadToolRequest::GetDiscoverySummary` for authorized D18D reads over
  scheduled discovery worker pressure and discovery freshness summaries.
- `RuntimePairingPlanToolRequest` and
  `RuntimeReadToolRequest::GetPairingPlan` for authorized D18D reads over
  discovery pairing plans derived from recorded discovery signals and the
  first-party integration catalog.
- `RuntimeReadToolRequest::GetSupervisionPlan` for authorized D18D previews of
  non-mutating supervision due work.
- `RuntimeSupervisionToolRequest` / `RuntimeSupervisionToolOutput` and
  `SmartHomeRuntime::execute_supervision_tool` for authorized D18D desired-state
  reconciliation and supervision ticks through runtime-owned mutation paths.

## [0.1.0] - 2026-05-06

### Added

- Runtime event bus with subscription filters for all, bridge, entity, command,
  and supervision events.
- Event replay checkpoints that let new subscribers catch up from a prior
  runtime event-log position before receiving live deliveries.
- Boxed registry-backed runtime errors to keep public `Result` error payloads
  small as the runtime API grows.
- `SmartHomeRuntime` facade over `smart-home-registry` for command validation,
  optimistic state caching, event replay, and bridge health updates.
- Grant-backed command authorization path for checking Chief of Staff agent
  capabilities before command acceptance.
- Registry-backed authorization decision auditing for accepted and rejected
  authorized commands.
- Registry-backed tool authorization decisions for Chief of Staff tool calls.
- D18D-style read tool execution for listing bridges/devices, reading entity
  state, describing entity capabilities, inspecting bridge health, and observing
  supervision status through the registry without dispatching integration work.
- D18D-style subscribe tool execution for authorized, filtered event-stream
  subscriptions with checkpointed replay metadata.
- D18D-style pair-bridge execution with short-lived pairing sessions, VaultRef
  completion, and credential-free bridge registry updates.
- D18D-style command tool execution for authorized `smart_home.command` calls,
  including tool-level audit decisions, command-level audit decisions, and
  deterministic runtime command/correlation ids.
- Supervisor primitives for bridge-worker heartbeat tracking and restart
  signaling.
- Worker heartbeat deadline schedules for deterministic supervisor wakeups.
- Desired-state reconciliation for missing, stale, or drifted entity state,
  producing deterministic corrective commands and supervision events.
- Non-mutating supervision plans that preview state refresh targets,
  pairing expiry, desired-state drift, and overdue worker restarts before a tick
  writes.
- Read-only supervision observations that combine due supervision work with
  worker heartbeat schedules for status tools.
- Deterministic supervision ticks that combine optimistic-state expiry,
  desired-state reconciliation, and worker restart checks into one report.
- Deterministic worker restart plans for inspecting overdue bridge workers
  before mutating supervisor state.
- Worker restart reconciliation marks registered bridges degraded and emits
  deterministic health events.
- Read-side queries for event-log entries, subscription backlogs, pairing
  sessions, desired-state targets, and supervised bridge workers.
- Bounded event-bus delivery peeking and draining for subscription polling.
- Event-bus unsubscribe lifecycle that returns undelivered events and clears
  subscription delivery state.
- Compact read-only runtime snapshots that summarize registry counts,
  event-bus backlog, supervisor restart pressure, pairing expiry, desired
  state, and stale cached state without mutating runtime state.
- Event-bus backlog status helpers that distinguish absent subscribers,
  caught-up streams, and backlogged streams.
- Event-bus aggregate pressure helpers for lagging subscription counts and
  maximum pending delivery depth.
- Event-bus lagging-subscription percentage helpers for read-side pressure
  thresholds.
- Event-bus pressure status helpers for classifying caught-up, partially
  backlogged, and fully backlogged subscriber fan-out.
- Subscription backlog status helpers that identify caught-up versus
  backlogged event-stream subscribers without draining their delivery queues.
