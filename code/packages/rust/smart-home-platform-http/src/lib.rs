//! Home Assistant-compatible local HTTP API routes for the smart-home platform.
//!
//! The crate builds `web-core::WebApp` routes over runtime-owned smart-home
//! registry snapshots. It deliberately uses the repo's own HTTP server stack;
//! service calls are wired through runtime command authorization instead of a
//! parallel mutation path.

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use smart_home_automation_runtime::{
    AutomationDefinition, AutomationEvaluationReport, AutomationTriggerInput,
    SmartHomeAutomationRuntime,
};
use smart_home_core::{
    AgentId, AuthorizationDecision, AuthorizationOutcome, AuthorizationSubject, Bridge, BridgeId,
    BridgeTransport, Capability, CapabilityGrant, CapabilityGrantId,
    CapabilityGrantInventorySummary, CapabilityGrantScope, CapabilityGrantStatus, CapabilityId,
    CapabilityMode, CommandId, CommandResult, CommandStatus, CommandType, CorrelationId, Device,
    DeviceCommand, DeviceControlCommandType, DeviceEvent, DeviceEventType, Entity, EntityId,
    EntityKind, EventId, Health,
    IntegrationId, MediaCommandType, PrivilegeTier, Scene, SceneScope, SmartHomeTool,
    StateConfidence, StateDelta, StateSource, Value, ValueKind,
};
use smart_home_dashboard_core::NativeDashboardManifest;
use smart_home_runtime::{
    DesiredEntityState, DesiredStateQuery, PairingSessionStatus, RuntimeAuthorizationDecisionQuery,
    RuntimeAuthorizationDecisionSort, RuntimeCapabilityGrantQuery, RuntimeCapabilityGrantScopeKind,
    RuntimeCapabilityGrantSort, RuntimeClearDesiredStateToolOutput,
    RuntimeClearDesiredStateToolRequest, RuntimeCommandResultQuery, RuntimeCommandResultRecord,
    RuntimeCommandResultSort, RuntimeCommandToolRequest, RuntimeError, RuntimeEvent,
    RuntimeEventCheckpoint, RuntimeEventFilter, RuntimeEventLogEntry, RuntimeEventQuery,
    RuntimeEventSort, RuntimePairingSession, RuntimePairingSessionId, RuntimePairingSessionQuery,
    RuntimePairingSessionSort, RuntimeReadSnapshot, RuntimeRoomQuery, RuntimeRoomSort,
    RuntimeRoomSummary, RuntimeSetDesiredStateToolOutput, RuntimeSetDesiredStateToolRequest,
    SmartHomeRuntime,
};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use web_core::{WebApp, WebRequest, WebResponse};

pub const VERSION: &str = "0.1.0";

const CONTROLLER_HANDOFF_PATH: &str = "/api/smart_home/controller_handoff";

type RuntimeClock = Arc<dyn Fn() -> u64 + Send + Sync>;
type RuntimeMutationPersistence =
    Arc<dyn Fn(&SmartHomeRuntime, u64) -> Result<(), String> + Send + Sync>;
type AutomationMutationPersistence = Arc<
    dyn Fn(&SmartHomeRuntime, &SmartHomeAutomationRuntime, u64) -> Result<(), String> + Send + Sync,
>;

const DASHBOARD_HTML: &str = r##"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>Codex Home</title>
  <style>
    :root {
      color-scheme: light;
      --ink: #1b2428;
      --muted: #5d6b70;
      --line: #d8e0df;
      --panel: #ffffff;
      --page: #eef3f1;
      --good: #28724f;
      --warn: #a46614;
      --bad: #a43939;
      --blue: #276d9a;
      --teal: #19706d;
    }

    * {
      box-sizing: border-box;
    }

    body {
      margin: 0;
      color: var(--ink);
      background: var(--page);
      font: 14px/1.45 -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
    }

    header {
      display: flex;
      align-items: center;
      justify-content: space-between;
      gap: 16px;
      padding: 18px 24px;
      border-bottom: 1px solid var(--line);
      background: #f9fbfa;
    }

    .section-tabs {
      display: flex;
      gap: 4px;
      overflow-x: auto;
      padding: 8px 16px;
      border-bottom: 1px solid var(--line);
      background: #ffffff;
    }

    .section-tabs a {
      flex: 0 0 auto;
      padding: 7px 10px;
      color: var(--ink);
      text-decoration: none;
      border-bottom: 2px solid transparent;
      font-weight: 700;
    }

    .section-tabs a:hover,
    .section-tabs a:focus-visible {
      border-bottom-color: var(--teal);
    }

    h1, h2, h3, p {
      margin: 0;
    }

    h1 {
      font-size: 24px;
      font-weight: 700;
    }

    h2 {
      font-size: 16px;
      font-weight: 700;
    }

    h3 {
      font-size: 14px;
      font-weight: 700;
    }

    main {
      display: grid;
      grid-template-columns: minmax(260px, 340px) minmax(0, 1fr);
      gap: 16px;
      padding: 16px;
    }

    section, aside {
      display: grid;
      gap: 12px;
      align-content: start;
      min-width: 0;
    }

    .panel {
      min-width: 0;
      overflow-x: auto;
      background: var(--panel);
      border: 1px solid var(--line);
      border-radius: 8px;
      padding: 14px;
    }

    .panel > * {
      min-width: 0;
    }

    .toolbar, .row, .metric-grid {
      display: flex;
      gap: 8px;
      align-items: center;
      flex-wrap: wrap;
    }

    .row > div {
      min-width: 0;
    }

    #checks .row > div {
      flex: 1 1 180px;
      min-width: 0;
    }

    h3, p, td {
      overflow-wrap: anywhere;
    }

    .metric-grid {
      display: grid;
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }

    .metric {
      min-height: 66px;
      border: 1px solid var(--line);
      border-radius: 8px;
      padding: 10px;
      background: #fbfcfb;
    }

    .metric strong {
      display: block;
      font-size: 24px;
      line-height: 1.1;
    }

    .muted {
      color: var(--muted);
    }

    .status {
      display: inline-flex;
      align-items: center;
      min-height: 28px;
      padding: 4px 9px;
      border-radius: 999px;
      color: #fff;
      background: var(--blue);
      font-weight: 700;
      text-transform: uppercase;
      font-size: 12px;
      letter-spacing: 0;
    }

    .status.ready, .status.ok {
      background: var(--good);
    }

    .status.attention {
      background: var(--warn);
    }

    .status.blocked, .status.degraded {
      background: var(--bad);
    }

    button {
      min-height: 36px;
      padding: 0 12px;
      border: 1px solid #8aa0a0;
      border-radius: 8px;
      color: var(--ink);
      background: #ffffff;
      font: inherit;
      font-weight: 700;
      cursor: pointer;
    }

    button.primary {
      color: #fff;
      border-color: var(--teal);
      background: var(--teal);
    }

    button:disabled {
      color: #7b878a;
      background: #edf1f0;
      cursor: wait;
    }

    input, select {
      min-height: 36px;
      width: 100%;
      padding: 0 10px;
      border: 1px solid #8aa0a0;
      border-radius: 8px;
      color: var(--ink);
      background: #ffffff;
      font: inherit;
    }

    .filter-grid {
      display: grid;
      grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
      gap: 10px;
    }

    .filter-grid label {
      display: grid;
      gap: 5px;
      color: var(--muted);
      font-size: 12px;
      font-weight: 700;
      text-transform: uppercase;
    }

    table {
      width: 100%;
      border-collapse: collapse;
    }

    th, td {
      padding: 9px 8px;
      border-bottom: 1px solid var(--line);
      text-align: left;
      vertical-align: top;
    }

    th {
      color: var(--muted);
      font-size: 12px;
      text-transform: uppercase;
      letter-spacing: 0;
    }

    .cards {
      display: grid;
      grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
      gap: 12px;
    }

    .entity-card {
      min-height: 132px;
      border: 1px solid var(--line);
      border-radius: 8px;
      padding: 12px;
      background: #fbfcfb;
    }

    .manifest-dashboard + .manifest-dashboard {
      margin-top: 14px;
      padding-top: 14px;
      border-top: 1px solid var(--line);
    }

    .manifest-view {
      display: grid;
      grid-template-columns: minmax(140px, 1fr) minmax(0, 3fr) auto;
      gap: 10px;
      align-items: center;
      padding: 9px 0;
      border-bottom: 1px solid var(--line);
    }

    .manifest-view:last-child {
      border-bottom: 0;
    }

    .entity-card .actions {
      margin-top: 10px;
    }

    .range-control {
      display: grid;
      gap: 6px;
      margin-top: 10px;
    }

    .range-control input {
      width: 100%;
    }

    .log {
      max-height: 170px;
      overflow: auto;
      white-space: pre-wrap;
      color: #263235;
      background: #f5f7f6;
      border: 1px solid var(--line);
      border-radius: 8px;
      padding: 10px;
    }

    .detail-meta {
      justify-content: space-between;
      margin-bottom: 8px;
    }

    .detail-body {
      min-height: 180px;
      max-height: 360px;
      overflow: auto;
      white-space: pre-wrap;
      color: #263235;
      background: #f5f7f6;
      border: 1px solid var(--line);
      border-radius: 8px;
      padding: 10px;
      font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", monospace;
      font-size: 12px;
      line-height: 1.45;
    }

    @media (max-width: 800px) {
      header {
        align-items: flex-start;
        flex-direction: column;
      }

      main {
        grid-template-columns: 1fr;
      }

      .manifest-view {
        grid-template-columns: 1fr;
      }
    }
  </style>
</head>
<body>
  <header>
    <div>
      <h1>Codex Home</h1>
      <p id="location" class="muted"></p>
    </div>
    <div class="toolbar">
      <span id="status" class="status">Loading</span>
      <button id="refresh" class="primary" type="button">Refresh</button>
    </div>
  </header>
  <nav class="section-tabs" aria-label="Dashboard sections">
    <a href="#overview-panel">Overview</a>
    <a href="#dashboards-panel">Dashboards</a>
    <a href="#rooms-panel">Rooms</a>
    <a href="#devices-panel">Devices</a>
    <a href="#entities-panel">State</a>
    <a href="#automations-panel">Automations</a>
    <a href="#pairing-panel">Pairing</a>
    <a href="#history-panel">History</a>
    <a href="#commands-panel">Audit</a>
  </nav>
  <main>
    <aside>
      <section class="panel">
        <h2>Readiness</h2>
        <div id="checks"></div>
      </section>
      <section class="panel">
        <h2>Activity</h2>
        <div id="activity" class="metric-grid"></div>
      </section>
      <section class="panel">
        <h2>Services</h2>
        <div id="services"></div>
      </section>
      <section class="panel">
        <h2>API Surface</h2>
        <div id="routes"></div>
      </section>
    </aside>
    <section>
      <div id="overview-panel" class="panel">
        <h2>Home</h2>
        <div id="summary" class="metric-grid"></div>
      </div>
      <div id="dashboards-panel" class="panel">
        <div class="row" style="justify-content: space-between;">
          <div>
            <h2>Native Dashboards</h2>
            <span id="dashboard-manifest-summary" class="muted"></span>
          </div>
          <button id="dashboard-view-all" type="button">All entities</button>
        </div>
        <div id="dashboard-manifests"></div>
      </div>
      <div id="filters-panel" class="panel">
        <div class="row" style="justify-content: space-between;">
          <h2>Filters</h2>
          <button id="reset-filters" type="button">Reset</button>
        </div>
        <div class="filter-grid">
          <label>Search
            <input id="filter-search" data-dashboard-filter="search" type="search" autocomplete="off">
          </label>
          <label>Room
            <select id="filter-room" data-dashboard-filter="room">
              <option value="">All rooms</option>
            </select>
          </label>
          <label>Entities
            <select id="filter-domain" data-dashboard-filter="domain">
              <option value="">All domains</option>
              <option value="light">Lights</option>
              <option value="sensor">Sensors</option>
              <option value="switch">Switches</option>
              <option value="climate">Climate</option>
              <option value="lock">Locks</option>
            </select>
          </label>
          <label>State
            <select id="filter-state" data-dashboard-filter="state">
              <option value="">All states</option>
              <option value="stale">Needs refresh</option>
              <option value="fresh">Fresh</option>
            </select>
          </label>
          <label>Control
            <select id="filter-control" data-dashboard-filter="control">
              <option value="">All entities</option>
              <option value="commandable">Commandable</option>
              <option value="readonly">Read-only</option>
            </select>
          </label>
          <label>Capability ID
            <input id="filter-capability-id" data-dashboard-filter="capability-id" type="search" autocomplete="off">
          </label>
          <label>Capability Command
            <select id="filter-capability-commandable" data-dashboard-filter="capability-commandable">
              <option value="">All capabilities</option>
              <option value="true">Commandable</option>
              <option value="false">Read-only</option>
            </select>
          </label>
          <label>Capability Observation
            <select id="filter-capability-observable" data-dashboard-filter="capability-observable">
              <option value="">All observation</option>
              <option value="true">Observable</option>
              <option value="false">Command-only</option>
            </select>
          </label>
          <label>Desired Entity
            <input id="filter-desired-entity" data-dashboard-filter="desired-entity" type="search" autocomplete="off">
          </label>
          <label>Desired Requester
            <input id="filter-desired-requested-by" data-dashboard-filter="desired-requested-by" type="search" autocomplete="off">
          </label>
          <label>Device Bridge
            <input id="filter-device-bridge" data-dashboard-filter="device-bridge" type="search" autocomplete="off">
          </label>
          <label>Device Manufacturer
            <input id="filter-device-manufacturer" data-dashboard-filter="device-manufacturer" type="search" autocomplete="off">
          </label>
          <label>Device Health
            <select id="filter-device-health" data-dashboard-filter="device-health">
              <option value="">All device health</option>
              <option value="online">Online</option>
              <option value="degraded">Degraded</option>
              <option value="offline">Offline</option>
              <option value="discoverable">Discoverable</option>
              <option value="unpaired">Unpaired</option>
              <option value="auth_failed">Auth failed</option>
              <option value="unsupported">Unsupported</option>
              <option value="removed">Removed</option>
              <option value="unknown">Unknown</option>
            </select>
          </label>
          <label>Bridge Integration
            <input id="filter-bridge-integration" data-dashboard-filter="bridge-integration" type="search" autocomplete="off">
          </label>
          <label>Bridge Transport
            <select id="filter-bridge-transport" data-dashboard-filter="bridge-transport">
              <option value="">All transports</option>
              <option value="lan_http">LAN HTTP</option>
              <option value="mdns">mDNS</option>
              <option value="serial">Serial</option>
              <option value="ble">BLE</option>
              <option value="cloud">Cloud</option>
              <option value="local_process">Local process</option>
            </select>
          </label>
          <label>Bridge Health
            <select id="filter-bridge-health" data-dashboard-filter="bridge-health">
              <option value="">All bridge health</option>
              <option value="online">Online</option>
              <option value="degraded">Degraded</option>
              <option value="offline">Offline</option>
              <option value="discoverable">Discoverable</option>
              <option value="unpaired">Unpaired</option>
              <option value="auth_failed">Auth failed</option>
              <option value="unsupported">Unsupported</option>
              <option value="removed">Removed</option>
              <option value="unknown">Unknown</option>
            </select>
          </label>
          <label>Scene Scope
            <select id="filter-scene-scope" data-dashboard-filter="scene-scope">
              <option value="">All scene scopes</option>
              <option value="room">Room</option>
              <option value="zone">Zone</option>
              <option value="home">Home</option>
              <option value="bridge">Bridge</option>
              <option value="custom">Custom</option>
            </select>
          </label>
          <label>Scene Entity
            <input id="filter-scene-entity" data-dashboard-filter="scene-entity" type="search" autocomplete="off">
          </label>
          <label>Service Name
            <input id="filter-service-name" data-dashboard-filter="service-name" type="search" autocomplete="off">
          </label>
          <label>Service Capability
            <input id="filter-service-capability" data-dashboard-filter="service-capability" type="search" autocomplete="off">
          </label>
          <label>Service Entity
            <input id="filter-service-entity" data-dashboard-filter="service-entity" type="search" autocomplete="off">
          </label>
          <label>Service Scene
            <input id="filter-service-scene" data-dashboard-filter="service-scene" type="search" autocomplete="off">
          </label>
          <label>API Surface
            <select id="filter-api-surface" data-dashboard-filter="api-surface">
              <option value="">All surfaces</option>
              <option value="smart_home">Smart home</option>
              <option value="home_assistant">Home Assistant</option>
              <option value="browser">Browser</option>
            </select>
          </label>
          <label>API Method
            <select id="filter-api-method" data-dashboard-filter="api-method">
              <option value="">All methods</option>
              <option value="GET">GET</option>
              <option value="POST">POST</option>
              <option value="DELETE">DELETE</option>
            </select>
          </label>
          <label>API Category
            <input id="filter-api-category" data-dashboard-filter="api-category" type="search" autocomplete="off">
          </label>
          <label>API Mutation
            <select id="filter-api-mutating" data-dashboard-filter="api-mutating">
              <option value="">All routes</option>
              <option value="false">Read-only</option>
              <option value="true">Mutating</option>
            </select>
          </label>
          <label>API Authorization
            <select id="filter-api-authorized" data-dashboard-filter="api-authorized">
              <option value="">All authorization</option>
              <option value="true">Runtime authorized</option>
              <option value="false">Not runtime authorized</option>
            </select>
          </label>
          <label>Events
            <select id="filter-event-kind" data-dashboard-filter="event-kind">
              <option value="">All events</option>
              <option value="commands">Commands</option>
              <option value="supervision">Supervision</option>
            </select>
          </label>
          <label>Event From
            <input id="filter-event-from-sequence" data-dashboard-filter="event-from-sequence" type="number" min="0" step="1" inputmode="numeric">
          </label>
          <label>Event To
            <input id="filter-event-to-sequence" data-dashboard-filter="event-to-sequence" type="number" min="0" step="1" inputmode="numeric">
          </label>
          <label>Activity Entity
            <input id="filter-activity-entity" data-dashboard-filter="activity-entity" type="search" autocomplete="off">
          </label>
          <label>History Type
            <select id="filter-history-type" data-dashboard-filter="history-type">
              <option value="">All history</option>
              <option value="discovered">Discovered</option>
              <option value="updated">Updated</option>
              <option value="removed">Removed</option>
              <option value="unavailable">Unavailable</option>
              <option value="error">Error</option>
              <option value="health">Health</option>
            </select>
          </label>
          <label>History Bridge
            <input id="filter-history-bridge" data-dashboard-filter="history-bridge" type="search" autocomplete="off">
          </label>
          <label>Observed From
            <input id="filter-history-from-ms" data-dashboard-filter="history-from-ms" type="number" min="0" step="1" inputmode="numeric">
          </label>
          <label>Observed To
            <input id="filter-history-to-ms" data-dashboard-filter="history-to-ms" type="number" min="0" step="1" inputmode="numeric">
          </label>
          <label>Received From
            <input id="filter-history-received-from-ms" data-dashboard-filter="history-received-from-ms" type="number" min="0" step="1" inputmode="numeric">
          </label>
          <label>Received To
            <input id="filter-history-received-to-ms" data-dashboard-filter="history-received-to-ms" type="number" min="0" step="1" inputmode="numeric">
          </label>
          <label>Commands
            <select id="filter-command-status" data-dashboard-filter="command-status">
              <option value="">All commands</option>
              <option value="accepted">Accepted</option>
              <option value="rejected">Rejected</option>
              <option value="timed_out">Timed out</option>
              <option value="failed">Failed</option>
            </select>
          </label>
          <label>Command ID
            <input id="filter-command-id" data-dashboard-filter="command-id" type="search" autocomplete="off">
          </label>
          <label>Command Bridge
            <input id="filter-command-bridge" data-dashboard-filter="command-bridge" type="search" autocomplete="off">
          </label>
          <label>Correlation
            <input id="filter-command-correlation" data-dashboard-filter="command-correlation" type="search" autocomplete="off">
          </label>
          <label>Command From
            <input id="filter-command-from-sequence" data-dashboard-filter="command-from-sequence" type="number" min="0" step="1" inputmode="numeric">
          </label>
          <label>Command To
            <input id="filter-command-to-sequence" data-dashboard-filter="command-to-sequence" type="number" min="0" step="1" inputmode="numeric">
          </label>
          <label>Authorization
            <select id="filter-authorization-outcome" data-dashboard-filter="authorization-outcome">
              <option value="">All decisions</option>
              <option value="allowed">Allowed</option>
              <option value="denied">Denied</option>
            </select>
          </label>
          <label>Auth Principal
            <input id="filter-authorization-principal" data-dashboard-filter="authorization-principal" type="search" autocomplete="off">
          </label>
          <label>Grant Status
            <select id="filter-grant-status" data-dashboard-filter="grant-status">
              <option value="">All grants</option>
              <option value="active">Active</option>
              <option value="pending">Pending</option>
              <option value="revoked">Revoked</option>
              <option value="expired">Expired</option>
            </select>
          </label>
          <label>Grant Scope
            <select id="filter-grant-scope" data-dashboard-filter="grant-scope">
              <option value="">All scopes</option>
              <option value="all_smart_home">All smart home</option>
              <option value="tool">Tool</option>
              <option value="capability">Capability</option>
              <option value="entity_capability">Entity capability</option>
            </select>
          </label>
          <label>Grant Principal
            <input id="filter-grant-principal" data-dashboard-filter="grant-principal" type="search" autocomplete="off">
          </label>
        </div>
      </div>
      <div id="rooms-panel" class="panel">
        <div class="row">
          <h2>Rooms</h2>
          <span class="muted">Topology and coverage</span>
        </div>
        <div id="rooms" class="cards"></div>
      </div>
      <div id="devices-panel" class="panel">
        <div class="row">
          <h2>Devices</h2>
          <span class="muted">Bridge inventory</span>
        </div>
        <div id="devices" class="cards"></div>
      </div>
      <div class="panel">
        <div class="row">
          <h2>Bridges</h2>
          <span class="muted">Integration health</span>
        </div>
        <div id="bridges" class="cards"></div>
      </div>
      <div class="panel">
        <div class="row">
          <h2>Scenes</h2>
          <span class="muted">Run saved room states</span>
        </div>
        <div id="scenes" class="cards"></div>
      </div>
      <div id="entities-panel" class="panel">
        <div class="row">
          <h2>Entities</h2>
          <span id="state-count" class="muted"></span>
        </div>
        <div id="entities" class="cards"></div>
      </div>
      <div class="panel">
        <div class="row">
          <h2>Capabilities</h2>
          <span id="capability-count" class="muted"></span>
        </div>
        <div id="capabilities" class="cards"></div>
      </div>
      <div class="panel">
        <h2>Desired State</h2>
        <table>
          <thead>
            <tr><th>Entity</th><th>Targets</th><th>Requested By</th><th></th></tr>
          </thead>
          <tbody id="desired"></tbody>
        </table>
      </div>
      <div class="panel">
        <h2>State Gaps</h2>
        <table>
          <thead>
            <tr><th>Entity</th><th>Domain</th><th>Status</th><th></th></tr>
          </thead>
          <tbody id="gaps"></tbody>
        </table>
      </div>
      <div id="automations-panel" class="panel">
        <div class="row">
          <h2>Automations</h2>
          <span id="automation-count" class="muted"></span>
        </div>
        <table>
          <thead>
            <tr><th>Automation</th><th>Trigger</th><th>Actions</th><th>Status</th><th></th></tr>
          </thead>
          <tbody id="automations"></tbody>
        </table>
        <h3 style="margin-top: 14px;">Automation Audit</h3>
        <table>
          <thead>
            <tr><th>Automation</th><th>Outcome</th><th>Occurrence</th><th>Time</th></tr>
          </thead>
          <tbody id="automation-audit"></tbody>
        </table>
      </div>
      <div id="pairing-panel" class="panel">
        <div class="row">
          <h2>Pairing</h2>
          <span id="pairing-count" class="muted"></span>
        </div>
        <table>
          <thead>
            <tr><th>Session</th><th>Bridge</th><th>Status</th><th>Expires</th><th></th></tr>
          </thead>
          <tbody id="pairing-sessions"></tbody>
        </table>
      </div>
      <div id="history-panel" class="panel">
        <h2>History</h2>
        <table>
          <thead>
            <tr><th>Entity</th><th>Event</th><th>State</th><th>Observed</th><th></th></tr>
          </thead>
          <tbody id="history"></tbody>
        </table>
      </div>
      <div class="panel">
        <h2>Events</h2>
        <table>
          <thead>
            <tr><th>Sequence</th><th>Kind</th><th>Subject</th><th>Status</th><th></th></tr>
          </thead>
          <tbody id="events"></tbody>
        </table>
      </div>
      <div id="commands-panel" class="panel">
        <h2>Commands</h2>
        <table>
          <thead>
            <tr><th>Command</th><th>Status</th><th>Bridge</th><th>Sequence</th><th></th></tr>
          </thead>
          <tbody id="command-results"></tbody>
        </table>
      </div>
      <div class="panel">
        <h2>Authorization</h2>
        <table>
          <thead>
            <tr><th>Principal</th><th>Subject</th><th>Outcome</th><th>Tier</th><th></th></tr>
          </thead>
          <tbody id="authorization-decisions"></tbody>
        </table>
      </div>
      <div class="panel">
        <h2>Capability Grants</h2>
        <table>
          <thead>
            <tr><th>Principal</th><th>Scope</th><th>Status</th><th>Tier</th><th></th></tr>
          </thead>
          <tbody id="capability-grants"></tbody>
        </table>
      </div>
      <div class="panel">
        <div class="row detail-meta">
          <div>
            <h2 id="detail-title">Detail</h2>
            <p id="detail-endpoint" class="muted">Select any View action</p>
          </div>
          <span id="detail-status" class="status">Idle</span>
        </div>
        <pre id="detail-body" class="detail-body">No detail selected</pre>
      </div>
      <div class="panel">
        <h2>Log</h2>
        <div id="log" class="log"></div>
      </div>
    </section>
  </main>
  <script>
    const els = {
      activity: document.querySelector("#activity"),
      automationAudit: document.querySelector("#automation-audit"),
      automationCount: document.querySelector("#automation-count"),
      automations: document.querySelector("#automations"),
      authorizationDecisions: document.querySelector("#authorization-decisions"),
      bridges: document.querySelector("#bridges"),
      capabilities: document.querySelector("#capabilities"),
      capabilityCount: document.querySelector("#capability-count"),
      capabilityGrants: document.querySelector("#capability-grants"),
      checks: document.querySelector("#checks"),
      commandResults: document.querySelector("#command-results"),
      detailBody: document.querySelector("#detail-body"),
      detailEndpoint: document.querySelector("#detail-endpoint"),
      detailStatus: document.querySelector("#detail-status"),
      detailTitle: document.querySelector("#detail-title"),
      desired: document.querySelector("#desired"),
      devices: document.querySelector("#devices"),
      dashboardManifestSummary: document.querySelector("#dashboard-manifest-summary"),
      dashboardManifests: document.querySelector("#dashboard-manifests"),
      dashboardViewAll: document.querySelector("#dashboard-view-all"),
      entities: document.querySelector("#entities"),
      events: document.querySelector("#events"),
      filterActivityEntity: document.querySelector("#filter-activity-entity"),
      filterApiAuthorized: document.querySelector("#filter-api-authorized"),
      filterApiCategory: document.querySelector("#filter-api-category"),
      filterApiMethod: document.querySelector("#filter-api-method"),
      filterApiMutating: document.querySelector("#filter-api-mutating"),
      filterApiSurface: document.querySelector("#filter-api-surface"),
      filterAuthorizationOutcome: document.querySelector("#filter-authorization-outcome"),
      filterAuthorizationPrincipal: document.querySelector("#filter-authorization-principal"),
      filterBridgeHealth: document.querySelector("#filter-bridge-health"),
      filterBridgeIntegration: document.querySelector("#filter-bridge-integration"),
      filterBridgeTransport: document.querySelector("#filter-bridge-transport"),
      filterCommandBridge: document.querySelector("#filter-command-bridge"),
      filterCommandCorrelation: document.querySelector("#filter-command-correlation"),
      filterCommandFromSequence: document.querySelector("#filter-command-from-sequence"),
      filterCommandId: document.querySelector("#filter-command-id"),
      filterCommandStatus: document.querySelector("#filter-command-status"),
      filterCommandToSequence: document.querySelector("#filter-command-to-sequence"),
      filterCapabilityCommandable: document.querySelector("#filter-capability-commandable"),
      filterCapabilityId: document.querySelector("#filter-capability-id"),
      filterCapabilityObservable: document.querySelector("#filter-capability-observable"),
      filterControl: document.querySelector("#filter-control"),
      filterDesiredEntity: document.querySelector("#filter-desired-entity"),
      filterDesiredRequestedBy: document.querySelector("#filter-desired-requested-by"),
      filterDeviceBridge: document.querySelector("#filter-device-bridge"),
      filterDeviceHealth: document.querySelector("#filter-device-health"),
      filterDeviceManufacturer: document.querySelector("#filter-device-manufacturer"),
      filterDomain: document.querySelector("#filter-domain"),
      filterEventFromSequence: document.querySelector("#filter-event-from-sequence"),
      filterEventKind: document.querySelector("#filter-event-kind"),
      filterEventToSequence: document.querySelector("#filter-event-to-sequence"),
      filterGrantPrincipal: document.querySelector("#filter-grant-principal"),
      filterGrantScope: document.querySelector("#filter-grant-scope"),
      filterGrantStatus: document.querySelector("#filter-grant-status"),
      filterHistoryBridge: document.querySelector("#filter-history-bridge"),
      filterHistoryFromMs: document.querySelector("#filter-history-from-ms"),
      filterHistoryReceivedFromMs: document.querySelector("#filter-history-received-from-ms"),
      filterHistoryReceivedToMs: document.querySelector("#filter-history-received-to-ms"),
      filterHistoryToMs: document.querySelector("#filter-history-to-ms"),
      filterHistoryType: document.querySelector("#filter-history-type"),
      filterRoom: document.querySelector("#filter-room"),
      filterSceneEntity: document.querySelector("#filter-scene-entity"),
      filterSceneScope: document.querySelector("#filter-scene-scope"),
      filterSearch: document.querySelector("#filter-search"),
      filterServiceCapability: document.querySelector("#filter-service-capability"),
      filterServiceEntity: document.querySelector("#filter-service-entity"),
      filterServiceName: document.querySelector("#filter-service-name"),
      filterServiceScene: document.querySelector("#filter-service-scene"),
      filterState: document.querySelector("#filter-state"),
      gaps: document.querySelector("#gaps"),
      history: document.querySelector("#history"),
      location: document.querySelector("#location"),
      log: document.querySelector("#log"),
      pairingCount: document.querySelector("#pairing-count"),
      pairingSessions: document.querySelector("#pairing-sessions"),
      refresh: document.querySelector("#refresh"),
      resetFilters: document.querySelector("#reset-filters"),
      routes: document.querySelector("#routes"),
      rooms: document.querySelector("#rooms"),
      scenes: document.querySelector("#scenes"),
      services: document.querySelector("#services"),
      stateCount: document.querySelector("#state-count"),
      status: document.querySelector("#status"),
      summary: document.querySelector("#summary")
    };

    const json = async (url, options) => {
      const response = await fetch(url, options);
      const body = await response.json();
      if (!response.ok) {
        throw new Error(body.error || response.statusText);
      }
      return body;
    };

    const FILTER_QUERY_PARAMS = [
      ["search", els.filterSearch],
      ["room", els.filterRoom],
      ["domain", els.filterDomain],
      ["state", els.filterState],
      ["control", els.filterControl],
      ["capability_id", els.filterCapabilityId],
      ["capability_commandable", els.filterCapabilityCommandable],
      ["capability_observable", els.filterCapabilityObservable],
      ["desired_entity", els.filterDesiredEntity],
      ["desired_requested_by", els.filterDesiredRequestedBy],
      ["device_bridge", els.filterDeviceBridge],
      ["device_manufacturer", els.filterDeviceManufacturer],
      ["device_health", els.filterDeviceHealth],
      ["bridge_integration", els.filterBridgeIntegration],
      ["bridge_transport", els.filterBridgeTransport],
      ["bridge_health", els.filterBridgeHealth],
      ["scene_scope", els.filterSceneScope],
      ["scene_entity", els.filterSceneEntity],
      ["service_name", els.filterServiceName],
      ["service_capability", els.filterServiceCapability],
      ["service_entity", els.filterServiceEntity],
      ["service_scene", els.filterServiceScene],
      ["api_surface", els.filterApiSurface],
      ["api_method", els.filterApiMethod],
      ["api_category", els.filterApiCategory],
      ["api_mutating", els.filterApiMutating],
      ["api_authorized", els.filterApiAuthorized],
      ["event_kind", els.filterEventKind],
      ["event_from_sequence", els.filterEventFromSequence],
      ["event_to_sequence", els.filterEventToSequence],
      ["activity_entity", els.filterActivityEntity],
      ["history_type", els.filterHistoryType],
      ["history_bridge", els.filterHistoryBridge],
      ["history_from_ms", els.filterHistoryFromMs],
      ["history_to_ms", els.filterHistoryToMs],
      ["history_received_from_ms", els.filterHistoryReceivedFromMs],
      ["history_received_to_ms", els.filterHistoryReceivedToMs],
      ["command_status", els.filterCommandStatus],
      ["command_id", els.filterCommandId],
      ["command_bridge", els.filterCommandBridge],
      ["command_correlation", els.filterCommandCorrelation],
      ["command_from_sequence", els.filterCommandFromSequence],
      ["command_to_sequence", els.filterCommandToSequence],
      ["authorization_outcome", els.filterAuthorizationOutcome],
      ["authorization_principal", els.filterAuthorizationPrincipal],
      ["grant_status", els.filterGrantStatus],
      ["grant_scope", els.filterGrantScope],
      ["grant_principal", els.filterGrantPrincipal]
    ];

    let selectedManifestView = new URLSearchParams(window.location.search).get("dashboard_view") || "";
    let selectedManifestEntityIds = new Set();

    const ensureSelectOption = (control, value) => {
      if (!value || control.tagName !== "SELECT") {
        return;
      }
      const hasOption = Array.from(control.options).some((option) => option.value === value);
      if (!hasOption) {
        control.add(new Option(value, value));
      }
    };

    const restoreFiltersFromUrl = () => {
      const params = new URLSearchParams(window.location.search);
      FILTER_QUERY_PARAMS.forEach(([queryParam, control]) => {
        const value = params.get(queryParam) || "";
        ensureSelectOption(control, value);
        control.value = value;
      });
    };

    const syncFiltersToUrl = () => {
      const params = new URLSearchParams(window.location.search);
      FILTER_QUERY_PARAMS.forEach(([queryParam, control]) => {
        const value = control.value.trim ? control.value.trim() : control.value;
        if (value) {
          params.set(queryParam, value);
        } else {
          params.delete(queryParam);
        }
      });
      const query = params.toString();
      const nextUrl = `${window.location.pathname}${query ? `?${query}` : ""}${window.location.hash}`;
      const currentUrl = `${window.location.pathname}${window.location.search}${window.location.hash}`;
      if (nextUrl !== currentUrl) {
        window.history.replaceState(null, "", nextUrl);
      }
    };

    const readFilters = () => ({
      search: els.filterSearch.value.trim().toLowerCase(),
      room: els.filterRoom.value,
      domain: els.filterDomain.value,
      state: els.filterState.value,
      control: els.filterControl.value,
      capabilityId: els.filterCapabilityId.value.trim(),
      capabilityCommandable: els.filterCapabilityCommandable.value,
      capabilityObservable: els.filterCapabilityObservable.value,
      desiredEntity: els.filterDesiredEntity.value.trim(),
      desiredRequestedBy: els.filterDesiredRequestedBy.value.trim(),
      deviceBridge: els.filterDeviceBridge.value.trim(),
      deviceManufacturer: els.filterDeviceManufacturer.value.trim(),
      deviceHealth: els.filterDeviceHealth.value,
      bridgeIntegration: els.filterBridgeIntegration.value.trim(),
      bridgeTransport: els.filterBridgeTransport.value,
      bridgeHealth: els.filterBridgeHealth.value,
      sceneScope: els.filterSceneScope.value,
      sceneEntity: els.filterSceneEntity.value.trim(),
      serviceName: els.filterServiceName.value.trim(),
      serviceCapability: els.filterServiceCapability.value.trim(),
      serviceEntity: els.filterServiceEntity.value.trim(),
      serviceScene: els.filterServiceScene.value.trim(),
      apiSurface: els.filterApiSurface.value,
      apiMethod: els.filterApiMethod.value,
      apiCategory: els.filterApiCategory.value.trim(),
      apiMutating: els.filterApiMutating.value,
      apiAuthorized: els.filterApiAuthorized.value,
      eventKind: els.filterEventKind.value,
      eventFromSequence: els.filterEventFromSequence.value.trim(),
      eventToSequence: els.filterEventToSequence.value.trim(),
      activityEntity: els.filterActivityEntity.value.trim(),
      historyType: els.filterHistoryType.value,
      historyBridge: els.filterHistoryBridge.value.trim(),
      historyFromMs: els.filterHistoryFromMs.value.trim(),
      historyToMs: els.filterHistoryToMs.value.trim(),
      historyReceivedFromMs: els.filterHistoryReceivedFromMs.value.trim(),
      historyReceivedToMs: els.filterHistoryReceivedToMs.value.trim(),
      commandStatus: els.filterCommandStatus.value,
      commandId: els.filterCommandId.value.trim(),
      commandBridge: els.filterCommandBridge.value.trim(),
      commandCorrelation: els.filterCommandCorrelation.value.trim(),
      commandFromSequence: els.filterCommandFromSequence.value.trim(),
      commandToSequence: els.filterCommandToSequence.value.trim(),
      authorizationOutcome: els.filterAuthorizationOutcome.value,
      authorizationPrincipal: els.filterAuthorizationPrincipal.value.trim(),
      grantStatus: els.filterGrantStatus.value,
      grantScope: els.filterGrantScope.value,
      grantPrincipal: els.filterGrantPrincipal.value.trim()
    });

    const queryUrl = (path, params) => {
      const query = new URLSearchParams();
      Object.entries(params).forEach(([key, value]) => {
        if (value !== undefined && value !== null && value !== "") {
          query.set(key, String(value));
        }
      });
      const suffix = query.toString();
      return suffix ? `${path}?${suffix}` : path;
    };

    const matchesSearch = (filters, item) =>
      !filters.search || JSON.stringify(item).toLowerCase().includes(filters.search);
    const filterRows = (items, filters) => items.filter((item) => matchesSearch(filters, item));
    const roomMatchesFilters = (filters, room) => !filters.room || room.room_id === filters.room;
    const isCommandable = (entity) => (entity.capabilities || []).some((item) => item.commandable);
    const entityMatchesFilters = (filters, entity) => {
      if (selectedManifestEntityIds.size > 0) {
        const ids = [entity.entity_id, entity.home_assistant_entity_id].filter(Boolean);
        if (!ids.some((id) => selectedManifestEntityIds.has(id))) {
          return false;
        }
      }
      if (!matchesSearch(filters, entity)) {
        return false;
      }
      if (filters.control === "commandable") {
        return isCommandable(entity);
      }
      if (filters.control === "readonly") {
        return !isCommandable(entity);
      }
      return true;
    };
    const countLabel = (shown, total, noun) =>
      shown === total ? `${total} ${noun}` : `${shown} of ${total} ${noun}`;

    const metric = (label, value) =>
      `<div class="metric"><strong>${value}</strong><span class="muted">${label}</span></div>`;

    const escapeText = (value) => String(value ?? "")
      .replaceAll("&", "&amp;")
      .replaceAll("<", "&lt;")
      .replaceAll(">", "&gt;")
      .replaceAll('"', "&quot;")
      .replaceAll("'", "&#39;");

    const statusClass = (status) => `status ${String(status || "ok").toLowerCase()}`;
    const valueText = (value) => value === null || value === undefined ? "null" : JSON.stringify(value);
    const deltasText = (deltas) => (deltas || [])
      .map((delta) => `${delta.capability_id}: ${valueText(delta.value)}`)
      .join(", ") || "No targets";
    const capability = (entity, capabilityId) => (entity.capabilities || [])
      .find((item) => item.capability_id === capabilityId);
    const clamp = (value, min, max) => Math.max(min, Math.min(max, value));
    const brightnessValue = (entity, min, max) => {
      const value = entity.value;
      const raw = value && typeof value === "object" ? value["light.brightness"] : undefined;
      return clamp(typeof raw === "number" ? raw : max, min, max);
    };
    const brightnessInputFor = (entityId) => Array.from(document.querySelectorAll("[data-brightness-input]"))
      .find((input) => input.dataset.brightnessInput === entityId);
    const observedText = (ms) => {
      if (typeof ms !== "number") {
        return "";
      }
      return ms > 1000000000000 ? new Date(ms).toLocaleString() : `${ms} ms`;
    };
    const subjectText = (subject) => {
      if (!subject) {
        return "unknown";
      }
      if (subject.kind === "tool") {
        return subject.tool_id || "tool";
      }
      return [subject.command_type, subject.entity_id].filter(Boolean).join(" ") || subject.kind || "subject";
    };
    const eventSubject = (event) => {
      if (!event) {
        return "unknown";
      }
      if (event.kind === "command_result") {
        return event.result?.command_id || event.result?.bridge_id || "command";
      }
      return event.entity_id || event.device_id || event.bridge_id || event.integration_id || event.event_id || "runtime";
    };
    const eventStatus = (event) => {
      if (!event) {
        return "unknown";
      }
      if (event.kind === "command_result") {
        return event.result?.status || "command";
      }
      return event.health || event.reason || event.event_type || event.kind || "event";
    };
    const grantScopeText = (scope) => {
      if (!scope) {
        return "unknown";
      }
      if (scope.kind === "tool") {
        return scope.tool_id || "tool";
      }
      if (scope.kind === "capability") {
        return scope.capability_id || "capability";
      }
      if (scope.kind === "entity_capability") {
        return [scope.entity_id, scope.capability_id].filter(Boolean).join(" / ") || "entity capability";
      }
      return scope.kind || "scope";
    };
    const inspectButton = (url, label, text = "View") =>
      `<button type="button" data-inspect-url="${url}" data-inspect-label="${label}">${text}</button>`;
    const entityIdentity = (entity) => entity.home_assistant_entity_id || entity.entity_id;
    const stateDetailUrl = (entity) =>
      `/api/smart_home/states/${encodeURIComponent(entityIdentity(entity))}`;
    const entityDetailUrl = (entity) =>
      `/api/smart_home/entities/${encodeURIComponent(entityIdentity(entity))}`;
    const entityDesiredStateUrl = (entity) =>
      `/api/smart_home/desired_states?entity_id=${encodeURIComponent(entityIdentity(entity))}`;
    const entityHistoryUrl = (entity) =>
      `/api/smart_home/state_history?entity_id=${encodeURIComponent(entityIdentity(entity))}`;
    const entityEventsUrl = (entity) =>
      `/api/smart_home/events?entity_id=${encodeURIComponent(entityIdentity(entity))}&limit=12&sort=desc`;
    const entityBridgeCommandsUrl = (entity) => entity.bridge_id
      ? `/api/smart_home/command_results?bridge_id=${encodeURIComponent(entity.bridge_id)}&limit=8&sort=status_then_newest`
      : "/api/smart_home/command_results?limit=8&sort=status_then_newest";
    const commandAuthorizationUrl = (entity, commandType) =>
      `/api/smart_home/command_authorization?entity_id=${encodeURIComponent(entityIdentity(entity))}&command_type=${encodeURIComponent(commandType)}`;
    const desiredStateAuthorizationUrl = (entity, operation) =>
      `/api/smart_home/desired_state_authorization?entity_id=${encodeURIComponent(entityIdentity(entity))}&operation=${encodeURIComponent(operation)}`;
    const sceneDetailUrl = (scene) =>
      `/api/smart_home/scenes/${encodeURIComponent(scene.home_assistant_scene_id || scene.scene_id)}`;
    const sceneAuthorizationUrl = (scene) =>
      `/api/smart_home/scene_authorization?scene_id=${encodeURIComponent(scene.home_assistant_scene_id || scene.scene_id)}`;
    const serviceDetailUrl = (service) => {
      const [domain, serviceName] = String(service.service_id || "").split(".");
      return domain && serviceName
        ? `/api/smart_home/services/${encodeURIComponent(domain)}/${encodeURIComponent(serviceName)}`
        : "/api/smart_home/services";
    };
    const serviceAuthorizationUrl = (service) => {
      const [domain, serviceName] = String(service.service_id || "").split(".");
      if (!domain || !serviceName) {
        return "/api/smart_home/services";
      }
      const params = new URLSearchParams();
      const entity = (service.home_assistant_entity_ids || [])[0];
      const scene = (service.home_assistant_scene_ids || [])[0];
      if (scene) {
        params.set("scene_id", scene);
      } else if (entity) {
        params.set("entity_id", entity);
      }
      if (domain === "light" && serviceName === "set_brightness") {
        params.set("brightness_pct", "75");
      } else if (domain === "light" && serviceName === "set_color_temperature") {
        params.set("kelvin", "2700");
      } else if (domain === "light" && serviceName === "set_color") {
        params.set("rgb_color", "255,244,229");
      } else if (domain === "climate" && serviceName === "set_temperature") {
        params.set("temperature", "70");
      }
      const query = params.toString();
      return `/api/smart_home/service_authorization/${encodeURIComponent(domain)}/${encodeURIComponent(serviceName)}${query ? `?${query}` : ""}`;
    };
    const capabilityDetailUrl = (capability) =>
      `/api/smart_home/capabilities?capability_id=${encodeURIComponent(capability.capability_id)}`;
    const capabilityServicesUrl = (capability) =>
      `/api/smart_home/services?capability_id=${encodeURIComponent(capability.capability_id)}`;
    const capabilityEntitiesUrl = (capability) =>
      `/api/smart_home/entities?capability_id=${encodeURIComponent(capability.capability_id)}`;
    const roomDetailUrl = (room) =>
      `/api/smart_home/rooms/${encodeURIComponent(room.room_id)}`;
    const capabilityGrantDetailUrl = (grant) =>
      `/api/smart_home/capability_grants/${encodeURIComponent(grant.grant_id)}`;
    const principalCapabilityGrantsUrl = (principalId) =>
      `/api/smart_home/capability_grants?principal_id=${encodeURIComponent(principalId)}&status=active&sort=principal_id`;

    const log = (message) => {
      const at = new Date().toLocaleTimeString();
      els.log.textContent = `[${at}] ${message}\n${els.log.textContent}`.slice(0, 2000);
    };

    const renderDetail = (label, url, status, ok, body) => {
      els.detailTitle.textContent = label || "Detail";
      els.detailEndpoint.textContent = url;
      els.detailStatus.className = statusClass(ok ? "ready" : "blocked");
      els.detailStatus.textContent = String(status);
      els.detailBody.textContent = typeof body === "string" ? body : JSON.stringify(body, null, 2);
    };

    const inspectDetail = async (button) => {
      const url = button.dataset.inspectUrl;
      const label = button.dataset.inspectLabel || "detail";
      const response = await fetch(url);
      const text = await response.text();
      let body = text;
      try {
        body = JSON.parse(text);
      } catch (_) {
        body = text || response.statusText;
      }
      renderDetail(label, url, response.status, response.ok, body);
      log(`${label}: ${url}`);
      if (!response.ok) {
        throw new Error(body.error || response.statusText);
      }
    };

    const uniqueValues = (items) => Array.from(new Set(items.filter(Boolean)));
    const commandActionFollowUp = (body, entityId) => {
      const results = body && Array.isArray(body.results) ? body.results : [];
      const commandIds = uniqueValues(results.map((result) => result.command_id));
      const correlationIds = uniqueValues(results.map((result) => result.correlation_id));
      return {
        state: entityId ? stateDetailUrl({home_assistant_entity_id: entityId}) : undefined,
        command_results: commandIds.map((commandId) =>
          `/api/smart_home/command_results/${encodeURIComponent(commandId)}`
        ),
        correlation_results: correlationIds.map((correlationId) =>
          `/api/smart_home/command_results?correlation_id=${encodeURIComponent(correlationId)}`
        ),
        latest_command_results: "/api/smart_home/command_results?limit=8&sort=status_then_newest"
      };
    };
    const desiredStateFollowUp = (entityId) => ({
      desired_state: `/api/smart_home/desired_states?entity_id=${encodeURIComponent(entityId)}`,
      state: stateDetailUrl({home_assistant_entity_id: entityId}),
      history: `/api/smart_home/state_history?entity_id=${encodeURIComponent(entityId)}`
    });
    const renderActionDetail = (label, url, status, ok, body, followUp) => {
      renderDetail(label, url, status, ok, {
        accepted: ok,
        response: body,
        follow_up: followUp
      });
    };
    const actionJson = async (url, options, label, followUpFactory = () => ({})) => {
      const response = await fetch(url, options);
      const text = await response.text();
      let body = text;
      try {
        body = JSON.parse(text);
      } catch (_) {
        body = text || response.statusText;
      }
      renderActionDetail(label, url, response.status, response.ok, body, followUpFactory(body));
      if (!response.ok) {
        throw new Error(body.error || response.statusText);
      }
      return body;
    };

    const renderChecks = (readiness) => {
      els.checks.innerHTML = readiness.checks.map((check) => `
        <div class="row" style="justify-content: space-between; margin-top: 8px;">
          <div>
            <h3>${check.label}</h3>
            <p class="muted">${check.message}</p>
          </div>
          <span class="${statusClass(check.status)}">${check.status}</span>
        </div>
      `).join("");
    };

    const renderScenes = (sceneData) => {
      const scenes = sceneData.scenes || [];
      els.scenes.innerHTML = scenes.map((scene) => `
        <article class="entity-card">
          <div class="row" style="justify-content: space-between;">
            <h3>${scene.home_assistant_scene_id}</h3>
            <span class="${statusClass("ready")}">${scene.scope}</span>
          </div>
          <p class="muted">${scene.scene_id}</p>
          <p>${scene.action_count} actions${scene.room_ids.length ? ` | ${scene.room_ids.join(", ")}` : ""}</p>
          <div class="actions row">
            <button type="button" data-scene="${scene.home_assistant_scene_id}">Run</button>
            ${inspectButton(sceneAuthorizationUrl(scene), "Auth scene")}
            ${inspectButton(sceneDetailUrl(scene), "scene detail")}
          </div>
        </article>
      `).join("") || `<p class="muted">No scenes</p>`;
    };

    const renderServices = (catalog) => {
      const services = catalog.services || [];
      els.services.innerHTML = services.map((service) => `
        <div class="row" style="justify-content: space-between; margin-top: 8px;">
          <div>
            <h3>${service.service_id}</h3>
            <p class="muted">${service.home_assistant_path}</p>
          </div>
          <div class="row">
            <span class="${statusClass(service.runtime_authorized ? "ready" : "attention")}">
              ${service.home_assistant_entity_ids.length + service.home_assistant_scene_ids.length}
            </span>
            ${inspectButton(serviceAuthorizationUrl(service), "Auth service")}
            ${inspectButton(serviceDetailUrl(service), "service detail")}
          </div>
        </div>
      `).join("") || `<p class="muted">No services</p>`;
    };

    const renderRoutes = (catalog) => {
      const routes = catalog.routes || [];
      els.routes.innerHTML = routes.map((route) => `
        <div class="row" style="justify-content: space-between; margin-top: 8px;">
          <div>
            <h3>${route.method} ${route.path}</h3>
            <p class="muted">${route.category} | ${route.surface}</p>
          </div>
          <span class="${statusClass(route.runtime_authorized ? "ready" : "attention")}">
            ${route.mutates_runtime ? "mutates" : "read"}
          </span>
        </div>
      `).join("") || `<p class="muted">No routes</p>`;
    };

    const activateManifestView = (manifestData) => {
      selectedManifestEntityIds.clear();
      if (!selectedManifestView || !manifestData.manifest) {
        return;
      }
      for (const dashboard of manifestData.manifest.dashboards || []) {
        for (const view of dashboard.views || []) {
          if (`${dashboard.dashboard_id}/${view.view_id}` !== selectedManifestView) {
            continue;
          }
          for (const card of view.cards || []) {
            for (const entityId of card.entity_ids || []) {
              selectedManifestEntityIds.add(entityId);
            }
          }
        }
      }
    };

    const renderDashboardManifests = (manifestData) => {
      const summary = manifestData.summary || {};
      els.dashboardManifestSummary.textContent = manifestData.configured
        ? `${summary.dashboards || 0} dashboards, ${summary.views || 0} views, ${summary.cards || 0} cards`
        : "No migrated manifest configured";
      els.dashboardViewAll.disabled = !selectedManifestView;
      const manifest = manifestData.manifest;
      if (!manifest) {
        els.dashboardManifests.innerHTML = `<p class="muted">The controller is using the native operational overview.</p>`;
        return;
      }
      els.dashboardManifests.innerHTML = (manifest.dashboards || []).map((dashboard) => `
        <div class="manifest-dashboard">
          <div class="row" style="justify-content: space-between;">
            <h3>${escapeText(dashboard.title)}</h3>
            <span class="muted">${escapeText(dashboard.url_path)}</span>
          </div>
          ${(dashboard.views || []).map((view) => {
            const viewKey = `${dashboard.dashboard_id}/${view.view_id}`;
            const cards = view.cards || [];
            const entityCount = new Set(cards.flatMap((card) => card.entity_ids || [])).size;
            return `
              <div class="manifest-view">
                <strong>${escapeText(view.title)}</strong>
                <span class="muted">${cards.length} cards, ${entityCount} entities</span>
                <button type="button" data-manifest-view="${escapeText(viewKey)}"${selectedManifestView === viewKey ? " disabled" : ""}>${selectedManifestView === viewKey ? "Active" : "Open"}</button>
              </div>
            `;
          }).join("") || `<p class="muted">No migrated views</p>`}
        </div>
      `).join("") || `<p class="muted">No migrated dashboards</p>`;
    };

    const renderAutomations = (automationData, auditData) => {
      const definitions = automationData.definitions || [];
      els.automationCount.textContent = `${definitions.length} definitions`;
      els.automations.innerHTML = definitions.map((definition) => {
        const trigger = definition.trigger || {};
        const triggerText = trigger.kind === "schedule"
          ? `Every ${trigger.every_ms} ms`
          : `${trigger.event_type || "event"}${trigger.entity_id ? ` / ${trigger.entity_id}` : ""}`;
        return `
          <tr>
            <td>${escapeText(definition.automation_id)}</td>
            <td>${escapeText(triggerText)}</td>
            <td>${(definition.actions || []).length}</td>
            <td><span class="${statusClass(definition.enabled ? "ready" : "attention")}">${definition.enabled ? "enabled" : "disabled"}</span></td>
            <td>${inspectButton("/api/smart_home/automations", "automation definitions")}</td>
          </tr>
        `;
      }).join("") || `<tr><td colspan="5" class="muted">No automation definitions</td></tr>`;

      const records = [...(auditData.records || [])].reverse().slice(0, 12);
      els.automationAudit.innerHTML = records.map((record) => `
        <tr>
          <td>${escapeText(record.automation_id)}</td>
          <td><span class="${statusClass(record.outcome)}">${escapeText(record.outcome)}</span></td>
          <td>${escapeText(record.trigger_key)}</td>
          <td>${escapeText(observedText(record.evaluated_at_ms))}</td>
        </tr>
      `).join("") || `<tr><td colspan="4" class="muted">No automation audit records</td></tr>`;
    };

    const renderPairingSessions = (pairingData) => {
      const sessions = pairingData.sessions || [];
      els.pairingCount.textContent = `${pairingData.summary?.total_sessions || 0} sessions`;
      els.pairingSessions.innerHTML = sessions.map((session) => `
        <tr>
          <td>${escapeText(session.session_id)}<br><span class="muted">${escapeText(session.requested_by)}</span></td>
          <td>${escapeText(session.bridge_id)}<br><span class="muted">${escapeText(session.integration_id)}</span></td>
          <td><span class="${statusClass(session.status === "completed" ? "ready" : session.status === "pending_user_presence" ? "attention" : session.status)}">${escapeText(session.status)}</span></td>
          <td>${escapeText(observedText(session.expires_at_ms))}</td>
          <td>${inspectButton(`/api/smart_home/pairing_sessions/${encodeURIComponent(session.session_id)}`, "pairing session")}</td>
        </tr>
      `).join("") || `<tr><td colspan="5" class="muted">No pairing sessions</td></tr>`;
    };

    const renderRoomOptions = (inventory, selectedRoom) => {
      const rooms = inventory.rooms || [];
      const options = [`<option value="">All rooms</option>`].concat(
        rooms.map((room) => `<option value="${room.room_id}">${room.room_id}</option>`)
      );
      els.filterRoom.innerHTML = options.join("");
      els.filterRoom.value = rooms.some((room) => room.room_id === selectedRoom) ? selectedRoom : "";
      if (selectedRoom && !els.filterRoom.value) {
        syncFiltersToUrl();
      }
    };

    const renderRooms = (inventory, filters) => {
      const rooms = (inventory.rooms || []).filter((room) => roomMatchesFilters(filters, room));
      els.rooms.innerHTML = rooms.map((room) => `
        <article class="entity-card">
          <div class="row" style="justify-content: space-between;">
            <h3>${room.room_id}</h3>
            <span class="${statusClass(room.has_attention || room.has_state_gaps ? "attention" : "ready")}">
              ${room.has_state_gaps ? "state gaps" : "ready"}
            </span>
          </div>
          <p>${room.device_count} devices | ${room.entity_count} entities | ${room.scene_count} scenes</p>
          <p class="muted">${room.online_devices} online, ${room.scene_action_count} scene actions</p>
          <div class="actions row">${inspectButton(roomDetailUrl(room), "room detail")}</div>
        </article>
      `).join("") || `<p class="muted">No rooms</p>`;
    };

    const renderDevices = (inventory) => {
      const devices = inventory.devices || [];
      els.devices.innerHTML = devices.map((device) => `
        <article class="entity-card">
          <div class="row" style="justify-content: space-between;">
            <h3>${device.name}</h3>
            <span class="${statusClass(device.health)}">${device.health}</span>
          </div>
          <p class="muted">${device.device_id} | ${device.bridge_id}</p>
          <p>${device.entity_count} entities | ${device.capability_count} capabilities</p>
          <p class="muted">${device.room_id || "unassigned"} | ${device.manufacturer} ${device.model}</p>
          <div class="actions row">${inspectButton(`/api/smart_home/devices/${encodeURIComponent(device.device_id)}`, "device detail")}</div>
        </article>
      `).join("") || `<p class="muted">No devices</p>`;
    };

    const renderBridges = (inventory) => {
      const bridges = inventory.bridges || [];
      els.bridges.innerHTML = bridges.map((bridge) => `
        <article class="entity-card">
          <div class="row" style="justify-content: space-between;">
            <h3>${bridge.bridge_id}</h3>
            <span class="${statusClass(bridge.health)}">${bridge.health}</span>
          </div>
          <p>${bridge.integration_id} | ${bridge.transport}</p>
          <p class="muted">${bridge.device_count} devices | ${bridge.entity_count} entities | ${bridge.room_count} rooms</p>
          <div class="actions row">${inspectButton(`/api/smart_home/bridges/${encodeURIComponent(bridge.bridge_id)}`, "bridge detail")}</div>
        </article>
      `).join("") || `<p class="muted">No bridges</p>`;
    };

    const renderEntities = (states, filters) => {
      const entities = (states.states || []).filter((entity) => entityMatchesFilters(filters, entity));
      els.stateCount.textContent = countLabel(
        entities.length,
        states.summary.total_entities,
        "tracked"
      );
      els.entities.innerHTML = entities.map((entity) => {
        const value = entity.value === null ? "No state" : JSON.stringify(entity.value);
        const canToggle = entity.domain === "light";
        const brightness = capability(entity, "light.brightness");
        const canSetBrightness = canToggle && brightness && brightness.commandable;
        const brightnessMin = Number.isFinite(brightness?.min) ? brightness.min : 0;
        const brightnessMax = Number.isFinite(brightness?.max) ? brightness.max : 100;
        const brightnessStep = Number.isFinite(brightness?.step) && brightness.step > 0 ? brightness.step : 1;
        const brightnessCurrent = brightnessValue(entity, brightnessMin, brightnessMax);
        const commandableCount = (entity.capabilities || []).filter((item) => item.commandable).length;
        return `
          <article class="entity-card">
            <div class="row" style="justify-content: space-between;">
              <h3>${entity.name}</h3>
              <span class="${statusClass(entity.stale ? "attention" : entity.confidence || "ok")}">
                ${entity.stale ? "stale" : entity.confidence || "ready"}
              </span>
            </div>
            <p class="muted">${entity.home_assistant_entity_id}</p>
            <p>${value}</p>
            <p class="muted">${commandableCount} commandable capabilities</p>
            <div class="actions row">
              ${inspectButton(stateDetailUrl(entity), "state detail")}
              ${inspectButton(entityDetailUrl(entity), "entity detail")}
              ${inspectButton(entityHistoryUrl(entity), "entity history", "History")}
              ${inspectButton(entityEventsUrl(entity), "entity events", "Events")}
              ${inspectButton(entityDesiredStateUrl(entity), "desired state", "Desired")}
              ${inspectButton(entityBridgeCommandsUrl(entity), "bridge command results", "Commands")}
              ${canToggle ? `${inspectButton(commandAuthorizationUrl(entity, "turn_on"), "turn on authorization", "Auth on")} ${inspectButton(commandAuthorizationUrl(entity, "turn_off"), "turn off authorization", "Auth off")} <button type="button" data-service="turn_on" data-entity="${entity.home_assistant_entity_id}">Turn on</button><button type="button" data-service="turn_off" data-entity="${entity.home_assistant_entity_id}">Turn off</button>` : ""}
              ${canToggle ? `${inspectButton(desiredStateAuthorizationUrl(entity, "set"), "desired-state set authorization", "Auth target")} ${inspectButton(desiredStateAuthorizationUrl(entity, "clear"), "desired-state clear authorization", "Auth clear")} <button type="button" data-desired-action="on" data-entity="${entity.home_assistant_entity_id}">Target on</button><button type="button" data-desired-action="off" data-entity="${entity.home_assistant_entity_id}">Target off</button>` : ""}
            </div>
            ${canSetBrightness ? `
              <label class="range-control">
                <span class="muted">Brightness <strong data-brightness-value="${entity.home_assistant_entity_id}">${brightnessCurrent}%</strong></span>
                <input type="range" min="${brightnessMin}" max="${brightnessMax}" step="${brightnessStep}" value="${brightnessCurrent}" data-brightness-input="${entity.home_assistant_entity_id}">
                ${inspectButton(commandAuthorizationUrl(entity, "set_brightness"), "brightness authorization", "Auth brightness")}
                <button type="button" data-service="set_brightness" data-entity="${entity.home_assistant_entity_id}" data-brightness-for="${entity.home_assistant_entity_id}">Set brightness</button>
                <button type="button" data-desired-action="brightness" data-entity="${entity.home_assistant_entity_id}" data-brightness-for="${entity.home_assistant_entity_id}">Target brightness</button>
              </label>
            ` : ""}
          </article>
        `;
      }).join("") || `<p class="muted">No matching entities</p>`;
    };

    const renderCapabilities = (catalog) => {
      const capabilities = catalog.capabilities || [];
      const total = catalog.summary?.total_capabilities || capabilities.length;
      els.capabilityCount.textContent = countLabel(capabilities.length, total, "capabilities");
      els.capabilities.innerHTML = capabilities.map((capability) => {
        const range = [capability.min, capability.max]
          .filter((value) => value !== null && value !== undefined)
          .join("..");
        const meta = [
          capability.value_kind,
          capability.unit,
          range ? `range ${range}` : ""
        ].filter(Boolean).join(" | ");
        const domains = (capability.domains || []).join(", ") || "no domains";
        const kinds = (capability.entity_kinds || []).join(", ") || "no entity kinds";
        const serviceCount = capability.service_count || (capability.service_ids || []).length;
        const status = capability.commandable ? "ready" : capability.observable ? "ok" : "attention";
        const mode = capability.commandable ? "command" : capability.observable ? "observe" : "catalog";
        return `
          <article class="entity-card">
            <div class="row" style="justify-content: space-between;">
              <h3>${capability.capability_id}</h3>
              <span class="${statusClass(status)}">${mode}</span>
            </div>
            <p class="muted">${meta || capability.mode}</p>
            <p>${capability.entity_count} entities | ${capability.device_count} devices | ${serviceCount} services</p>
            <p class="muted">${domains} | ${kinds}</p>
            <div class="actions row">
              ${inspectButton(capabilityDetailUrl(capability), "capability catalog")}
              ${inspectButton(capabilityServicesUrl(capability), "capability services", "Services")}
              ${inspectButton(capabilityEntitiesUrl(capability), "capability entities", "Entities")}
            </div>
          </article>
        `;
      }).join("") || `<p class="muted">No matching capabilities</p>`;
    };

    const renderDesiredStates = (desiredStates, filters) => {
      const targets = filterRows(desiredStates.desired_states || [], filters);
      els.desired.innerHTML = targets.map((target) => `
        <tr>
          <td>${target.home_assistant_entity_id}<br><span class="muted">${target.entity_id}</span></td>
          <td>${deltasText(target.desired)}</td>
          <td>${target.requested_by}<br><span class="muted">${target.command_timeout_ms} ms</span></td>
          <td>${inspectButton(desiredStateAuthorizationUrl(target, "clear"), "desired-state clear authorization", "Auth clear")} <button type="button" data-clear-desired="${target.home_assistant_entity_id}">Clear</button></td>
        </tr>
      `).join("") || `<tr><td colspan="4" class="muted">No desired-state targets</td></tr>`;
    };

    const renderGaps = (stateGaps, filters) => {
      const states = filterRows(stateGaps.states || [], filters);
      els.gaps.innerHTML = states.map((entity) => `
        <tr>
          <td>${entity.name}<br><span class="muted">${entity.home_assistant_entity_id}</span></td>
          <td>${entity.domain}</td>
          <td><span class="${statusClass(entity.stale ? "attention" : "ok")}">${entity.stale ? "needs refresh" : "ok"}</span></td>
          <td>${inspectButton(stateDetailUrl(entity), "state gap detail", "State")} ${inspectButton(entityHistoryUrl(entity), "state gap history", "History")} ${inspectButton(entityEventsUrl(entity), "state gap events", "Events")}</td>
        </tr>
      `).join("") || `<tr><td colspan="4" class="muted">Clear</td></tr>`;
    };

    const renderHistory = (history, filters) => {
      const events = filterRows(history.events || [], filters);
      els.history.innerHTML = events.map((row) => {
        const event = row.event || {};
        const detailUrl = `/api/smart_home/state_history/${encodeURIComponent(event.event_id || "")}`;
        return `
          <tr>
            <td>${row.home_assistant_entity_id || event.entity_id || "unknown"}</td>
            <td>${event.event_type || event.kind || "event"}<br><span class="muted">${event.event_id || ""}</span></td>
            <td>${event.state_delta ? deltasText([event.state_delta]) : "No state delta"}</td>
            <td>${observedText(event.observed_at_ms)}</td>
            <td>${event.event_id ? inspectButton(detailUrl, "history event") : ""}</td>
          </tr>
        `;
      }).join("") || `<tr><td colspan="5" class="muted">No state history</td></tr>`;
    };

    const renderEvents = (eventLog, filters) => {
      const entries = filterRows(eventLog.events || [], filters);
      els.events.innerHTML = entries.map((entry) => {
        const event = entry.event || {};
        const eventLinks = entry.links || {};
        const eventDetailUrl = eventLinks.self || `/api/smart_home/events/${entry.sequence}`;
        return `
          <tr>
            <td>${entry.sequence}<br><span class="muted">next ${entry.next_sequence}</span></td>
            <td>${event.kind || "event"}</td>
            <td>${eventSubject(event)}</td>
            <td><span class="${statusClass(eventStatus(event))}">${eventStatus(event)}</span></td>
            <td>${inspectButton(eventDetailUrl, "runtime event")}</td>
          </tr>
        `;
      }).join("") || `<tr><td colspan="5" class="muted">No runtime events</td></tr>`;
    };

    const renderCommandResults = (audit, filters) => {
      const results = filterRows(audit.results || [], filters);
      els.commandResults.innerHTML = results.map((record) => {
        const result = record.result || {};
        const resultLinks = result.links || {};
        const recordLinks = record.links || {};
        const detailUrl = recordLinks.self || resultLinks.self || `/api/smart_home/command_results/${encodeURIComponent(result.command_id || "")}`;
        const eventUrl = recordLinks.event;
        return `
          <tr>
            <td>${result.command_id || "unknown"}<br><span class="muted">${result.correlation_id || ""}</span></td>
            <td><span class="${statusClass(result.status || "ok")}">${result.status || "unknown"}</span></td>
            <td>${result.bridge_id || ""}</td>
            <td>${record.sequence}</td>
            <td>${result.command_id ? inspectButton(detailUrl, "command result") : ""} ${eventUrl ? inspectButton(eventUrl, "command result event", "Event") : ""}</td>
          </tr>
        `;
      }).join("") || `<tr><td colspan="5" class="muted">No command results</td></tr>`;
    };

    const renderAuthorizationDecisions = (audit, filters) => {
      const decisions = filterRows(audit.decisions || [], filters);
      els.authorizationDecisions.innerHTML = decisions.map((record) => {
        const links = record.links || {};
        const detailUrl = links.self || `/api/smart_home/authorization_decisions/${record.decision_index}`;
        const grantsUrl = links.principal_grants || principalCapabilityGrantsUrl(record.principal_id);
        return `
          <tr>
            <td>${record.principal_id}<br><span class="muted">${observedText(record.decided_at_ms)}</span></td>
            <td>${subjectText(record.subject)}</td>
            <td><span class="${statusClass(record.outcome || "ok")}">${record.outcome}</span></td>
            <td>${record.required_tier}</td>
            <td>${inspectButton(detailUrl, "authorization decision")} ${inspectButton(grantsUrl, "principal grants", "Grants")} ${links.subject_command_result ? inspectButton(links.subject_command_result, "subject command result", "Command") : ""}</td>
          </tr>
        `;
      }).join("") || `<tr><td colspan="5" class="muted">No authorization decisions</td></tr>`;
    };

    const renderCapabilityGrants = (audit, filters) => {
      const grants = filterRows(audit.grants || [], filters);
      els.capabilityGrants.innerHTML = grants.map((grant) => `
        <tr>
          <td>${grant.principal_id}<br><span class="muted">${grant.grant_id}</span></td>
          <td>${grantScopeText(grant.scope)}</td>
          <td><span class="${statusClass(grant.active ? "ready" : "attention")}">${grant.effective_status}</span><br><span class="muted">configured ${grant.configured_status}</span></td>
          <td>${grant.max_tier}</td>
          <td>${inspectButton(capabilityGrantDetailUrl(grant), "capability grant")}</td>
        </tr>
      `).join("") || `<tr><td colspan="5" class="muted">No capability grants</td></tr>`;
    };

    let renderTimer = 0;
    const scheduleRender = () => {
      window.clearTimeout(renderTimer);
      renderTimer = window.setTimeout(render, 150);
    };

    const applyFilterChange = () => {
      syncFiltersToUrl();
      scheduleRender();
    };

    const render = async () => {
      els.refresh.disabled = true;
      const filters = readFilters();
      const stale = filters.state === "stale"
        ? true
        : filters.state === "fresh"
          ? false
          : undefined;
      const roomId = filters.room || undefined;
      const activityEntity = filters.activityEntity || undefined;
      const historyType = filters.historyType || undefined;
      const capabilityId = filters.capabilityId || undefined;
      try {
        const [
          bootstrap,
          readiness,
          states,
          stateGaps,
          scenes,
          desiredStates,
          history,
          services,
          capabilities,
          routes,
          rooms,
          devices,
          bridges,
          events,
          commandResults,
          authorizationDecisions,
          capabilityGrants,
          dashboardManifest,
          automations,
          automationAudit,
          pairingSessions
        ] = await Promise.all([
          json("/api/smart_home/bootstrap"),
          json("/api/smart_home/readiness"),
          json(queryUrl("/api/smart_home/states", {limit: 24, domain: filters.domain, room_id: roomId, stale, capability_id: capabilityId})),
          json(queryUrl("/api/smart_home/states", {limit: 24, room_id: roomId, stale: true, capability_id: capabilityId})),
          json(queryUrl("/api/smart_home/scenes", {
            limit: 12,
            room_id: roomId,
            scope: filters.sceneScope,
            entity_id: filters.sceneEntity
          })),
          json(queryUrl("/api/smart_home/desired_states", {
            limit: 12,
            entity_id: filters.desiredEntity,
            capability_id: capabilityId,
            requested_by: filters.desiredRequestedBy
          })),
          json(queryUrl("/api/smart_home/state_history", {
            limit: 12,
            room_id: roomId,
            entity_id: activityEntity,
            event_type: historyType,
            bridge_id: filters.historyBridge,
            from_ms: filters.historyFromMs,
            to_ms: filters.historyToMs,
            received_at_or_after_ms: filters.historyReceivedFromMs,
            received_at_or_before_ms: filters.historyReceivedToMs
          })),
          json(queryUrl("/api/smart_home/services", {
            limit: 8,
            domain: filters.domain,
            service: filters.serviceName,
            capability_id: filters.serviceCapability,
            entity_id: filters.serviceEntity,
            scene_id: filters.serviceScene
          })),
          json(queryUrl("/api/smart_home/capabilities", {
            limit: 12,
            domain: filters.domain,
            capability_id: filters.capabilityId,
            commandable: filters.capabilityCommandable,
            observable: filters.capabilityObservable
          })),
          json(queryUrl("/api/smart_home/api", {
            surface: filters.apiSurface,
            method: filters.apiMethod,
            category: filters.apiCategory,
            mutating: filters.apiMutating,
            authorized: filters.apiAuthorized
          })),
          json("/api/smart_home/rooms?sort=scene_count"),
          json(queryUrl("/api/smart_home/devices", {
            limit: 8,
            room_id: roomId,
            bridge_id: filters.deviceBridge,
            manufacturer: filters.deviceManufacturer,
            health: filters.deviceHealth
          })),
          json(queryUrl("/api/smart_home/bridges", {
            limit: 8,
            integration_id: filters.bridgeIntegration,
            transport: filters.bridgeTransport,
            health: filters.bridgeHealth
          })),
          json(queryUrl("/api/smart_home/events", {
            limit: 12,
            kind: filters.eventKind,
            room_id: roomId,
            entity_id: activityEntity,
            from_sequence: filters.eventFromSequence,
            to_sequence: filters.eventToSequence
          })),
          json(queryUrl("/api/smart_home/command_results", {
            limit: 8,
            room_id: roomId,
            status: filters.commandStatus,
            command_id: filters.commandId,
            bridge_id: filters.commandBridge,
            correlation_id: filters.commandCorrelation,
            from_sequence: filters.commandFromSequence,
            to_sequence: filters.commandToSequence
          })),
          json(queryUrl("/api/smart_home/authorization_decisions", {
            limit: 8,
            outcome: filters.authorizationOutcome,
            principal_id: filters.authorizationPrincipal
          })),
          json(queryUrl("/api/smart_home/capability_grants", {
            limit: 8,
            principal_id: filters.grantPrincipal,
            status: filters.grantStatus,
            scope: filters.grantScope,
            sort: "principal_id"
          })),
          json("/api/smart_home/dashboard_manifest"),
          json("/api/smart_home/automations"),
          json("/api/smart_home/automation_audit"),
          json("/api/smart_home/pairing_sessions?limit=12&sort=status_then_expires_at")
        ]);
        const summary = bootstrap.dashboard.summary;
        els.location.textContent = bootstrap.dashboard.config.location_name;
        els.status.className = statusClass(readiness.status);
        els.status.textContent = readiness.status;
        els.summary.innerHTML = [
          metric("Entities", summary.entity_count),
          metric("Devices", summary.device_count),
          metric("Rooms", summary.room_count),
          metric("Scenes", summary.scene_count),
          metric("Automations", automations.summary.definition_count),
          metric("Pairing", pairingSessions.summary.total_sessions),
          metric("Active grants", capabilityGrants.summary.active_grants)
        ].join("");
        els.activity.innerHTML = [
          metric("Events", bootstrap.recent_activity.events.summary.total_events),
          metric("Commands", bootstrap.recent_activity.command_results.summary.total_results),
          metric("Decisions", bootstrap.recent_activity.authorization_decisions.summary.total_decisions),
          metric("Grants", capabilityGrants.summary.total_grants),
          metric("State gaps", bootstrap.state_gaps.summary.total_entities)
        ].join("");
        renderChecks(readiness);
        activateManifestView(dashboardManifest);
        renderDashboardManifests(dashboardManifest);
        renderServices(services);
        renderRoutes(routes);
        renderRoomOptions(rooms, filters.room);
        renderRooms(rooms, filters);
        renderDevices(devices);
        renderBridges(bridges);
        renderScenes(scenes);
        renderEntities(states, filters);
        renderCapabilities(capabilities);
        renderDesiredStates(desiredStates, filters);
        renderGaps(stateGaps, filters);
        renderHistory(history, filters);
        renderEvents(events, filters);
        renderCommandResults(commandResults, filters);
        renderAuthorizationDecisions(authorizationDecisions, filters);
        renderCapabilityGrants(capabilityGrants, filters);
        renderAutomations(automations, automationAudit);
        renderPairingSessions(pairingSessions);
        log("Dashboard refreshed");
      } catch (error) {
        els.status.className = statusClass("blocked");
        els.status.textContent = "blocked";
        log(error.message);
      } finally {
        els.refresh.disabled = false;
      }
    };

    document.addEventListener("input", (event) => {
      if (event.target.closest("[data-dashboard-filter]")) {
        applyFilterChange();
        return;
      }
      const input = event.target.closest("input[data-brightness-input]");
      if (!input) {
        return;
      }
      const value = document.querySelector(`[data-brightness-value="${input.dataset.brightnessInput}"]`);
      if (value) {
        value.textContent = `${input.value}%`;
      }
    });

    document.addEventListener("change", (event) => {
      if (event.target.closest("[data-dashboard-filter]")) {
        applyFilterChange();
      }
    });

    document.addEventListener("click", async (event) => {
      const serviceButton = event.target.closest("button[data-service]");
      const sceneButton = event.target.closest("button[data-scene]");
      const clearDesiredButton = event.target.closest("button[data-clear-desired]");
      const desiredButton = event.target.closest("button[data-desired-action]");
      const inspectDetailButton = event.target.closest("button[data-inspect-url]");
      const manifestViewButton = event.target.closest("button[data-manifest-view]");
      const manifestAllButton = event.target.closest("#dashboard-view-all");
      const button = serviceButton || sceneButton || clearDesiredButton || desiredButton || inspectDetailButton || manifestViewButton || manifestAllButton;
      if (!button) {
        return;
      }
      if (manifestViewButton || manifestAllButton) {
        selectedManifestView = manifestViewButton?.dataset.manifestView || "";
        const params = new URLSearchParams(window.location.search);
        if (selectedManifestView) {
          params.set("dashboard_view", selectedManifestView);
        } else {
          params.delete("dashboard_view");
        }
        const query = params.toString();
        window.history.replaceState(null, "", `${window.location.pathname}${query ? `?${query}` : ""}${window.location.hash}`);
        await render();
        return;
      }
      button.disabled = true;
      try {
        if (inspectDetailButton) {
          await inspectDetail(inspectDetailButton);
        } else if (serviceButton) {
          const body = {entity_id: serviceButton.dataset.entity};
          if (serviceButton.dataset.service === "set_brightness") {
            const input = brightnessInputFor(serviceButton.dataset.brightnessFor);
            body.brightness_pct = input ? Number(input.value) : 100;
          }
          const url = `/api/services/light/${serviceButton.dataset.service}`;
          await actionJson(url, {
            method: "POST",
            headers: {"content-type": "application/json"},
            body: JSON.stringify(body)
          }, `light.${serviceButton.dataset.service} response`, (responseBody) =>
            commandActionFollowUp(responseBody, serviceButton.dataset.entity)
          );
          log(`${serviceButton.dataset.service} accepted for ${serviceButton.dataset.entity}`);
        } else if (sceneButton) {
          await actionJson("/api/services/scene/turn_on", {
            method: "POST",
            headers: {"content-type": "application/json"},
            body: JSON.stringify({entity_id: sceneButton.dataset.scene})
          }, "scene.turn_on response", commandActionFollowUp);
          log(`scene.turn_on accepted for ${sceneButton.dataset.scene}`);
        } else if (clearDesiredButton) {
          const entityId = clearDesiredButton.dataset.clearDesired;
          const url = `/api/smart_home/desired_states/${encodeURIComponent(entityId)}`;
          await actionJson(url, {
            method: "DELETE"
          }, "clear desired state response", () => desiredStateFollowUp(entityId));
          log(`desired state cleared for ${entityId}`);
        } else {
          const desiredState = {};
          if (desiredButton.dataset.desiredAction === "brightness") {
            const input = brightnessInputFor(desiredButton.dataset.brightnessFor);
            desiredState["light.brightness"] = input ? Number(input.value) : 100;
          } else {
            desiredState["light.on_off"] = desiredButton.dataset.desiredAction === "on";
          }
          const entityId = desiredButton.dataset.entity;
          const url = `/api/smart_home/desired_states/${encodeURIComponent(entityId)}`;
          await actionJson(url, {
            method: "POST",
            headers: {"content-type": "application/json"},
            body: JSON.stringify({
              desired_state: desiredState,
              requested_by: "agent:dashboard",
              command_timeout_ms: 3000
            })
          }, `desired ${desiredButton.dataset.desiredAction} response`, () =>
            desiredStateFollowUp(entityId)
          );
          log(`desired ${desiredButton.dataset.desiredAction} target accepted for ${entityId}`);
        }
        if (!inspectDetailButton) {
          await render();
        }
      } catch (error) {
        log(error.message);
      } finally {
        button.disabled = false;
      }
    });

    els.resetFilters.addEventListener("click", () => {
      document.querySelectorAll("[data-dashboard-filter]").forEach((control) => {
        control.value = "";
      });
      syncFiltersToUrl();
      render();
    });
    els.refresh.addEventListener("click", render);
    window.addEventListener("popstate", () => {
      restoreFiltersFromUrl();
      render();
    });
    restoreFiltersFromUrl();
    render();
  </script>
</body>
</html>
"##;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmartHomePlatformHttpConfig {
    pub location_name: String,
    pub unit_system: String,
    pub time_zone: String,
    pub version: String,
}

impl SmartHomePlatformHttpConfig {
    pub fn new(location_name: impl Into<String>) -> Self {
        Self {
            location_name: location_name.into(),
            unit_system: "metric".to_string(),
            time_zone: "UTC".to_string(),
            version: VERSION.to_string(),
        }
    }

    pub fn with_unit_system(mut self, unit_system: impl Into<String>) -> Self {
        self.unit_system = unit_system.into();
        self
    }

    pub fn with_time_zone(mut self, time_zone: impl Into<String>) -> Self {
        self.time_zone = time_zone.into();
        self
    }

    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.version = version.into();
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SmartHomePlatformHttpState {
    pub config: SmartHomePlatformHttpConfig,
    pub entities: Vec<Entity>,
    pub scenes: Vec<Scene>,
    pub event_types: Vec<String>,
    pub generated_at_ms: u64,
}

impl SmartHomePlatformHttpState {
    pub fn from_runtime(
        runtime: &SmartHomeRuntime,
        config: SmartHomePlatformHttpConfig,
        event_types: impl IntoIterator<Item = impl Into<String>>,
        generated_at_ms: u64,
    ) -> Self {
        let mut event_types = event_types.into_iter().map(Into::into).collect::<Vec<_>>();
        event_types.sort();
        event_types.dedup();

        Self {
            config,
            entities: runtime.registry().entities().cloned().collect(),
            scenes: runtime.registry().scenes().cloned().collect(),
            event_types,
            generated_at_ms,
        }
    }

    pub fn summary(&self) -> SmartHomePlatformHttpSummary {
        SmartHomePlatformHttpSummary::from_state(self)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SmartHomePlatformHttpSummary {
    pub state_count: usize,
    pub known_state_count: usize,
    pub unknown_state_count: usize,
    pub stale_state_count: usize,
    pub optimistic_state_count: usize,
    pub service_count: usize,
    pub event_type_count: usize,
    pub scene_count: usize,
}

impl SmartHomePlatformHttpSummary {
    pub fn from_state(state: &SmartHomePlatformHttpState) -> Self {
        let mut summary = Self {
            state_count: state.entities.len(),
            event_type_count: state.event_types.len(),
            scene_count: state.scenes.len(),
            service_count: platform_services(state).len(),
            ..Self::default()
        };

        for entity in &state.entities {
            match &entity.state {
                Some(snapshot) if snapshot.confidence == StateConfidence::Stale => {
                    summary.stale_state_count += 1;
                }
                Some(snapshot) if snapshot.confidence == StateConfidence::Optimistic => {
                    summary.optimistic_state_count += 1;
                    summary.known_state_count += 1;
                }
                Some(_) => summary.known_state_count += 1,
                None => summary.unknown_state_count += 1,
            }
        }

        summary
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmartHomePlatformService {
    pub domain: String,
    pub service: String,
    pub description: String,
    pub target_entity_ids: Vec<String>,
    pub target_scene_ids: Vec<String>,
    pub capability_ids: Vec<String>,
}

#[derive(Clone)]
pub struct SmartHomePlatformHttpRuntime {
    runtime: Arc<Mutex<SmartHomeRuntime>>,
    automation_runtime: Option<Arc<Mutex<SmartHomeAutomationRuntime>>>,
    dashboard_manifest: Option<Arc<NativeDashboardManifest>>,
    config: SmartHomePlatformHttpConfig,
    event_types: Vec<String>,
    principal_id: AgentId,
    clock: RuntimeClock,
    mutation_persistence: Option<RuntimeMutationPersistence>,
    automation_persistence: Option<AutomationMutationPersistence>,
}

impl SmartHomePlatformHttpRuntime {
    pub fn new(runtime: SmartHomeRuntime, config: SmartHomePlatformHttpConfig) -> Self {
        Self::from_shared_runtime(Arc::new(Mutex::new(runtime)), config)
    }

    pub fn from_shared_runtime(
        runtime: Arc<Mutex<SmartHomeRuntime>>,
        config: SmartHomePlatformHttpConfig,
    ) -> Self {
        Self {
            runtime,
            automation_runtime: None,
            dashboard_manifest: None,
            config,
            event_types: default_event_types(),
            principal_id: AgentId::trusted("agent:home-assistant-local-api"),
            clock: Arc::new(|| 0),
            mutation_persistence: None,
            automation_persistence: None,
        }
    }

    pub fn with_event_types(
        mut self,
        event_types: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.event_types = sorted_unique_strings(event_types);
        self
    }

    pub fn with_principal_id(mut self, principal_id: AgentId) -> Self {
        self.principal_id = principal_id;
        self
    }

    pub fn with_now_ms(mut self, now_ms: u64) -> Self {
        self.clock = Arc::new(move || now_ms);
        self
    }

    /// Use a live clock for request timestamps and freshness projections.
    pub fn with_clock(mut self, clock: impl Fn() -> u64 + Send + Sync + 'static) -> Self {
        self.clock = Arc::new(clock);
        self
    }

    /// Persist each accepted mutation before exposing it as successful.
    ///
    /// A failed persistence call restores the in-memory runtime to its
    /// pre-request state and returns HTTP 503 to the caller.
    pub fn with_mutation_persistence(
        mut self,
        persistence: impl Fn(&SmartHomeRuntime, u64) -> Result<(), String> + Send + Sync + 'static,
    ) -> Self {
        self.mutation_persistence = Some(Arc::new(persistence));
        self
    }

    pub fn with_automation_runtime(
        mut self,
        automation_runtime: Arc<Mutex<SmartHomeAutomationRuntime>>,
    ) -> Self {
        self.automation_runtime = Some(automation_runtime);
        self
    }

    pub fn with_dashboard_manifest(mut self, manifest: NativeDashboardManifest) -> Self {
        self.dashboard_manifest = Some(Arc::new(manifest));
        self
    }

    pub fn with_automation_persistence(
        mut self,
        persistence: impl Fn(&SmartHomeRuntime, &SmartHomeAutomationRuntime, u64) -> Result<(), String>
            + Send
            + Sync
            + 'static,
    ) -> Self {
        self.automation_persistence = Some(Arc::new(persistence));
        self
    }

    pub fn automation_runtime(&self) -> Option<Arc<Mutex<SmartHomeAutomationRuntime>>> {
        self.automation_runtime.clone()
    }

    pub fn evaluate_automations(
        &self,
        input: AutomationTriggerInput,
        dry_run: bool,
    ) -> Result<AutomationEvaluationReport, String> {
        let automation_runtime = self
            .automation_runtime
            .as_ref()
            .ok_or_else(|| "automation runtime is not configured".to_string())?;
        let mut runtime = self
            .runtime
            .lock()
            .map_err(|_| "smart-home runtime mutex was poisoned".to_string())?;
        let mut automations = automation_runtime
            .lock()
            .map_err(|_| "automation runtime mutex was poisoned".to_string())?;
        let previous_runtime = runtime.clone();
        let previous_automations = automations.clone();
        let now_ms = self.now_ms();
        let report = automations
            .evaluate(
                &mut runtime,
                self.principal_id.clone(),
                input,
                dry_run,
                now_ms,
            )
            .map_err(|error| error.to_string())?;
        if !dry_run && !report.records.is_empty() {
            if let Err(error) = self.persist_automation_mutation(&runtime, &automations, now_ms) {
                *runtime = previous_runtime;
                *automations = previous_automations;
                return Err(error);
            }
        }
        Ok(report)
    }

    pub fn upsert_automation_definition(
        &self,
        definition: AutomationDefinition,
    ) -> Result<Option<AutomationDefinition>, String> {
        let automation_runtime = self
            .automation_runtime
            .as_ref()
            .ok_or_else(|| "automation runtime is not configured".to_string())?;
        let runtime = self
            .runtime
            .lock()
            .map_err(|_| "smart-home runtime mutex was poisoned".to_string())?;
        let mut automations = automation_runtime
            .lock()
            .map_err(|_| "automation runtime mutex was poisoned".to_string())?;
        let previous = automations.clone();
        let replaced = automations
            .upsert_definition(definition)
            .map_err(|error| error.to_string())?;
        if let Err(error) = self.persist_automation_mutation(&runtime, &automations, self.now_ms())
        {
            *automations = previous;
            return Err(error);
        }
        Ok(replaced)
    }

    pub fn grant_local_full_access(
        self,
        granted_by: impl Into<String>,
        granted_at_ms: u64,
    ) -> Self {
        let grant = CapabilityGrant::for_all_smart_home(
            CapabilityGrantId::trusted(format!(
                "grant:{}:local-api-full-access",
                self.principal_id.as_str()
            )),
            self.principal_id.clone(),
            PrivilegeTier::HighRisk,
            granted_by,
            granted_at_ms,
        );
        self.runtime
            .lock()
            .expect("smart-home runtime mutex should not be poisoned")
            .registry_mut()
            .upsert_capability_grant(grant);
        self
    }

    pub fn snapshot(&self) -> SmartHomePlatformHttpState {
        let runtime = self
            .runtime
            .lock()
            .expect("smart-home runtime mutex should not be poisoned");
        SmartHomePlatformHttpState::from_runtime(
            &runtime,
            self.config.clone(),
            self.event_types.clone(),
            self.now_ms(),
        )
    }

    fn now_ms(&self) -> u64 {
        (self.clock)()
    }

    fn persist_mutation_or_restore(
        &self,
        runtime: &mut SmartHomeRuntime,
        previous: SmartHomeRuntime,
        saved_at_ms: u64,
    ) -> Result<(), ApiError> {
        let Some(persistence) = &self.mutation_persistence else {
            return Ok(());
        };
        if let Err(error) = persistence(runtime, saved_at_ms) {
            *runtime = previous;
            return Err(ApiError::new(
                503,
                format!("could not persist smart-home runtime mutation: {error}"),
            ));
        }
        Ok(())
    }

    fn persist_automation_mutation(
        &self,
        runtime: &SmartHomeRuntime,
        automations: &SmartHomeAutomationRuntime,
        saved_at_ms: u64,
    ) -> Result<(), String> {
        let Some(persistence) = &self.automation_persistence else {
            return Ok(());
        };
        persistence(runtime, automations, saved_at_ms)
    }
}

pub fn home_assistant_web_app(state: SmartHomePlatformHttpState) -> WebApp {
    let state = Arc::new(state);
    let mut app = WebApp::new();

    app.get("/api/", move |_| {
        WebResponse::json(api_root_json().into_bytes())
    });

    {
        let state = Arc::clone(&state);
        app.get("/api/config", move |_| {
            WebResponse::json(config_json(&state).into_bytes())
        });
    }

    {
        let state = Arc::clone(&state);
        app.get("/api/states", move |_| {
            WebResponse::json(states_json(&state.entities, state.generated_at_ms).into_bytes())
        });
    }

    {
        let state = Arc::clone(&state);
        app.get("/api/states/:entity_id", move |request| {
            let Some(entity_id) = request.route_params.get("entity_id") else {
                return WebResponse::new(400, br#"{"error":"missing entity_id"}"#.to_vec())
                    .with_content_type("application/json");
            };
            match state
                .entities
                .iter()
                .find(|entity| entity.entity_id.as_str() == entity_id)
            {
                Some(entity) => {
                    WebResponse::json(state_json(entity, state.generated_at_ms).into_bytes())
                }
                None => WebResponse::new(404, br#"{"error":"entity not found"}"#.to_vec())
                    .with_content_type("application/json"),
            }
        });
    }

    {
        let state = Arc::clone(&state);
        app.get("/api/services", move |_| {
            WebResponse::json(services_json(&platform_services(&state)).into_bytes())
        });
    }

    {
        let state = Arc::clone(&state);
        app.get("/api/events", move |_| {
            WebResponse::json(events_json(&state.event_types).into_bytes())
        });
    }

    app
}

pub fn home_assistant_runtime_web_app(runtime: SmartHomePlatformHttpRuntime) -> WebApp {
    let mut app = WebApp::new();

    app.get("/", |_| dashboard_ui_response());
    app.get("/dashboard", |_| dashboard_ui_response());
    app.get("/smart-home", |_| dashboard_ui_response());

    app.get("/api/", move |_| {
        WebResponse::json(api_root_json().into_bytes())
    });

    {
        let runtime = runtime.clone();
        app.get("/api/config", move |_| {
            let state = runtime.snapshot();
            WebResponse::json(config_json(&state).into_bytes())
        });
    }

    {
        let runtime = runtime.clone();
        app.get("/api/states", move |_| {
            let state = runtime.snapshot();
            WebResponse::json(states_json(&state.entities, state.generated_at_ms).into_bytes())
        });
    }

    {
        let runtime = runtime.clone();
        app.get("/api/states/:entity_id", move |request| {
            let Some(entity_id) = request.route_params.get("entity_id") else {
                return json_error(400, "missing entity_id");
            };
            let state = runtime.snapshot();
            match state
                .entities
                .iter()
                .find(|entity| entity_matches_external_id(entity, entity_id))
            {
                Some(entity) => {
                    WebResponse::json(state_json(entity, state.generated_at_ms).into_bytes())
                }
                None => json_error(404, "entity not found"),
            }
        });
    }

    {
        let runtime = runtime.clone();
        app.post("/api/states/:entity_id", move |request| {
            set_desired_state_response(&runtime, request, true)
        });
    }

    {
        let runtime = runtime.clone();
        app.get("/api/services", move |_| {
            let state = runtime.snapshot();
            WebResponse::json(services_json(&platform_services(&state)).into_bytes())
        });
    }

    {
        let runtime = runtime.clone();
        app.get("/api/events", move |_| {
            let state = runtime.snapshot();
            WebResponse::json(events_json(&state.event_types).into_bytes())
        });
    }

    {
        let runtime = runtime.clone();
        app.get("/api/history/period", move |request| {
            home_assistant_history_response(&runtime, request)
        });
    }

    {
        let runtime = runtime.clone();
        app.get("/api/history/period/:start_time", move |request| {
            home_assistant_history_response(&runtime, request)
        });
    }

    {
        let runtime = runtime.clone();
        app.get("/api/smart_home/runtime", move |_| {
            runtime_snapshot_response(&runtime)
        });
    }

    {
        let runtime = runtime.clone();
        app.get("/api/smart_home/health", move |_| {
            runtime_health_response(&runtime)
        });
    }

    {
        let runtime = runtime.clone();
        app.get("/api/smart_home/readiness", move |_| {
            runtime_readiness_response(&runtime)
        });
    }

    {
        let runtime = runtime.clone();
        app.get(CONTROLLER_HANDOFF_PATH, move |_| {
            runtime_controller_handoff_response(&runtime)
        });
    }

    {
        let runtime = runtime.clone();
        app.get("/api/smart_home/dashboard", move |_| {
            runtime_dashboard_response(&runtime)
        });
    }

    {
        let runtime = runtime.clone();
        app.get("/api/smart_home/dashboard_manifest", move |_| {
            dashboard_manifest_response(&runtime)
        });
    }

    {
        let runtime = runtime.clone();
        app.get("/api/smart_home/bootstrap", move |_| {
            runtime_bootstrap_response(&runtime)
        });
    }

    {
        let runtime = runtime.clone();
        app.get("/api/smart_home/smoke", move |_| {
            runtime_smoke_response(&runtime)
        });
    }

    {
        let runtime = runtime.clone();
        app.get("/api/smart_home/smoke_script", move |request| {
            runtime_smoke_script_response(&runtime, request)
        });
    }

    app.get("/api/smart_home/api", api_catalog_response);

    {
        let runtime = runtime.clone();
        app.get("/api/smart_home/states", move |request| {
            runtime_states_response(&runtime, request)
        });
    }

    {
        let runtime = runtime.clone();
        app.get("/api/smart_home/states/:entity_id", move |request| {
            runtime_state_response(&runtime, request)
        });
    }

    {
        let runtime = runtime.clone();
        app.get("/api/smart_home/services", move |request| {
            runtime_services_response(&runtime, request)
        });
    }

    {
        let runtime = runtime.clone();
        app.get(
            "/api/smart_home/services/:domain/:service",
            move |request| runtime_service_response(&runtime, request),
        );
    }

    {
        let runtime = runtime.clone();
        app.get("/api/smart_home/entities", move |request| {
            runtime_entities_response(&runtime, request)
        });
    }

    {
        let runtime = runtime.clone();
        app.get("/api/smart_home/entities/:entity_id", move |request| {
            runtime_entity_response(&runtime, request)
        });
    }

    {
        let runtime = runtime.clone();
        app.get("/api/smart_home/capabilities", move |request| {
            runtime_capabilities_response(&runtime, request)
        });
    }

    {
        let runtime = runtime.clone();
        app.get("/api/smart_home/capability_grants", move |request| {
            runtime_capability_grants_response(&runtime, request)
        });
    }

    {
        let runtime = runtime.clone();
        app.get(
            "/api/smart_home/capability_grants/:grant_id",
            move |request| runtime_capability_grant_response(&runtime, request),
        );
    }

    {
        let runtime = runtime.clone();
        app.get("/api/smart_home/devices", move |request| {
            runtime_devices_response(&runtime, request)
        });
    }

    {
        let runtime = runtime.clone();
        app.get("/api/smart_home/devices/:device_id", move |request| {
            runtime_device_response(&runtime, request)
        });
    }

    {
        let runtime = runtime.clone();
        app.get("/api/smart_home/bridges", move |request| {
            runtime_bridges_response(&runtime, request)
        });
    }

    {
        let runtime = runtime.clone();
        app.get("/api/smart_home/bridges/:bridge_id", move |request| {
            runtime_bridge_response(&runtime, request)
        });
    }

    {
        let runtime = runtime.clone();
        app.get("/api/smart_home/pairing_sessions", move |request| {
            runtime_pairing_sessions_response(&runtime, request)
        });
    }

    {
        let runtime = runtime.clone();
        app.get(
            "/api/smart_home/pairing_sessions/:session_id",
            move |request| runtime_pairing_session_response(&runtime, request),
        );
    }

    {
        let runtime = runtime.clone();
        app.get("/api/smart_home/rooms", move |request| {
            runtime_rooms_response(&runtime, request)
        });
    }

    {
        let runtime = runtime.clone();
        app.get("/api/smart_home/rooms/:room_id", move |request| {
            runtime_room_response(&runtime, request)
        });
    }

    {
        let runtime = runtime.clone();
        app.get("/api/smart_home/scenes", move |request| {
            runtime_scenes_response(&runtime, request)
        });
    }

    {
        let runtime = runtime.clone();
        app.get("/api/smart_home/scenes/:scene_id", move |request| {
            runtime_scene_response(&runtime, request)
        });
    }

    {
        let runtime = runtime.clone();
        app.get("/api/smart_home/events", move |request| {
            runtime_events_response(&runtime, request)
        });
    }

    {
        let runtime = runtime.clone();
        app.get("/api/smart_home/events/:sequence", move |request| {
            runtime_event_response(&runtime, request)
        });
    }

    {
        let runtime = runtime.clone();
        app.get("/api/smart_home/command_results", move |request| {
            runtime_command_results_response(&runtime, request)
        });
    }

    {
        let runtime = runtime.clone();
        app.get(
            "/api/smart_home/command_results/:command_id",
            move |request| runtime_command_result_response(&runtime, request),
        );
    }

    {
        let runtime = runtime.clone();
        app.get("/api/smart_home/authorization_decisions", move |request| {
            runtime_authorization_decisions_response(&runtime, request)
        });
    }

    {
        let runtime = runtime.clone();
        app.get("/api/smart_home/command_authorization", move |request| {
            runtime_command_authorization_response(&runtime, request)
        });
    }

    {
        let runtime = runtime.clone();
        app.get(
            "/api/smart_home/desired_state_authorization",
            move |request| runtime_desired_state_authorization_response(&runtime, request),
        );
    }

    {
        let runtime = runtime.clone();
        app.get("/api/smart_home/scene_authorization", move |request| {
            runtime_scene_authorization_response(&runtime, request)
        });
    }

    {
        let runtime = runtime.clone();
        app.get(
            "/api/smart_home/service_authorization/:domain/:service",
            move |request| runtime_service_authorization_response(&runtime, request),
        );
    }

    {
        let runtime = runtime.clone();
        app.get(
            "/api/smart_home/authorization_decisions/:decision_index",
            move |request| runtime_authorization_decision_response(&runtime, request),
        );
    }

    {
        let runtime = runtime.clone();
        app.get("/api/smart_home/desired_states", move |request| {
            runtime_desired_states_response(&runtime, request)
        });
    }

    {
        let runtime = runtime.clone();
        app.post(
            "/api/smart_home/desired_states/:entity_id",
            move |request| set_desired_state_response(&runtime, request, false),
        );
    }

    {
        let runtime = runtime.clone();
        app.delete(
            "/api/smart_home/desired_states/:entity_id",
            move |request| clear_desired_state_response(&runtime, request),
        );
    }

    {
        let runtime = runtime.clone();
        app.get("/api/smart_home/state_history", move |request| {
            runtime_state_history_response(&runtime, request)
        });
    }

    {
        let runtime = runtime.clone();
        app.get("/api/smart_home/state_history/:event_id", move |request| {
            runtime_state_history_event_response(&runtime, request)
        });
    }

    {
        let runtime = runtime.clone();
        app.get("/api/smart_home/automations", move |_| {
            automation_definitions_response(&runtime)
        });
    }

    {
        let runtime = runtime.clone();
        app.post("/api/smart_home/automations", move |request| {
            upsert_automation_response(&runtime, request)
        });
    }

    {
        let runtime = runtime.clone();
        app.get("/api/smart_home/automation_audit", move |_| {
            automation_audit_response(&runtime)
        });
    }

    {
        let runtime = runtime.clone();
        app.post("/api/smart_home/automations/evaluate", move |request| {
            evaluate_automations_response(&runtime, request)
        });
    }

    {
        let runtime = runtime.clone();
        app.post("/api/services/:domain/:service", move |request| {
            service_call_response(&runtime, request)
        });
    }

    app
}

#[derive(Debug, Deserialize)]
struct AutomationEvaluationRequest {
    #[serde(default)]
    dry_run: bool,
    #[serde(default)]
    event: Option<DeviceEvent>,
}

fn automation_definitions_response(runtime: &SmartHomePlatformHttpRuntime) -> WebResponse {
    let Some(automations) = runtime.automation_runtime.as_ref() else {
        return json_error(503, "automation runtime is not configured");
    };
    let automations = match automations.lock() {
        Ok(automations) => automations,
        Err(_) => return json_error(503, "automation runtime mutex was poisoned"),
    };
    let definitions = automations.definitions().collect::<Vec<_>>();
    serialized_json_response(&serde_json::json!({
        "summary": {
            "definition_count": definitions.len(),
            "enabled_count": definitions.iter().filter(|definition| definition.enabled).count(),
            "audit_record_count": automations.audit_records().len(),
        },
        "definitions": definitions,
    }))
}

fn automation_audit_response(runtime: &SmartHomePlatformHttpRuntime) -> WebResponse {
    let Some(automations) = runtime.automation_runtime.as_ref() else {
        return json_error(503, "automation runtime is not configured");
    };
    let automations = match automations.lock() {
        Ok(automations) => automations,
        Err(_) => return json_error(503, "automation runtime mutex was poisoned"),
    };
    serialized_json_response(&serde_json::json!({
        "summary": {
            "record_count": automations.audit_records().len(),
        },
        "records": automations.audit_records(),
    }))
}

fn upsert_automation_response(
    runtime: &SmartHomePlatformHttpRuntime,
    request: &WebRequest,
) -> WebResponse {
    let definition: AutomationDefinition = match serde_json::from_slice(request.body()) {
        Ok(definition) => definition,
        Err(error) => return json_error(400, format!("invalid automation JSON: {error}")),
    };
    let automation_id = definition.automation_id.clone();
    match runtime.upsert_automation_definition(definition) {
        Ok(replaced) => serialized_json_response(&serde_json::json!({
            "automation_id": automation_id,
            "replaced": replaced.is_some(),
            "definitions": runtime
                .automation_runtime
                .as_ref()
                .and_then(|automations| automations.lock().ok())
                .map(|automations| automations.definitions().cloned().collect::<Vec<_>>())
                .unwrap_or_default(),
        })),
        Err(error) if error.starts_with("invalid automation:") => json_error(400, error),
        Err(error) => json_error(503, error),
    }
}

fn evaluate_automations_response(
    runtime: &SmartHomePlatformHttpRuntime,
    request: &WebRequest,
) -> WebResponse {
    let request = if request.body().is_empty() {
        AutomationEvaluationRequest {
            dry_run: false,
            event: None,
        }
    } else {
        match serde_json::from_slice(request.body()) {
            Ok(request) => request,
            Err(error) => {
                return json_error(400, format!("invalid automation evaluation JSON: {error}"));
            }
        }
    };
    let input = request
        .event
        .map(Box::new)
        .map(AutomationTriggerInput::Event)
        .unwrap_or(AutomationTriggerInput::Schedule);
    match runtime.evaluate_automations(input, request.dry_run) {
        Ok(report) => serialized_json_response(&report),
        Err(error) => json_error(503, error),
    }
}

fn serialized_json_response(value: &impl Serialize) -> WebResponse {
    match serde_json::to_vec(value) {
        Ok(body) => WebResponse::json(body),
        Err(error) => json_error(500, format!("could not encode JSON response: {error}")),
    }
}

pub fn platform_services(state: &SmartHomePlatformHttpState) -> Vec<SmartHomePlatformService> {
    let mut services = BTreeMap::<(String, String), SmartHomePlatformService>::new();

    for entity in &state.entities {
        let domain = entity_domain(entity.kind).to_string();
        for capability in entity
            .capabilities
            .iter()
            .filter(|capability| capability_allows_command(capability))
        {
            for service in services_for_capability(&domain, capability) {
                let key = (domain.clone(), service.to_string());
                let entry = services
                    .entry(key)
                    .or_insert_with(|| SmartHomePlatformService {
                        domain: domain.clone(),
                        service: service.to_string(),
                        description: format!("{} {}", service.replace('_', " "), domain),
                        target_entity_ids: Vec::new(),
                        target_scene_ids: Vec::new(),
                        capability_ids: Vec::new(),
                    });
                push_unique_string(&mut entry.target_entity_ids, entity.entity_id.as_str());
                push_unique_string(&mut entry.capability_ids, capability.capability_id.as_str());
            }
        }
    }

    if !state.scenes.is_empty() {
        let entry = services
            .entry(("scene".to_string(), "turn_on".to_string()))
            .or_insert_with(|| SmartHomePlatformService {
                domain: "scene".to_string(),
                service: "turn_on".to_string(),
                description: "activate scene".to_string(),
                target_entity_ids: Vec::new(),
                target_scene_ids: Vec::new(),
                capability_ids: vec!["scene.recall".to_string()],
            });
        for scene in &state.scenes {
            push_unique_string(&mut entry.target_scene_ids, scene.scene_id.as_str());
        }
    }

    services.into_values().collect()
}

fn config_json(state: &SmartHomePlatformHttpState) -> String {
    let summary = state.summary();
    format!(
        "{{\"location_name\":{},\"unit_system\":{},\"time_zone\":{},\"version\":{},\"components\":[\"smart_home\"],\"state_count\":{},\"service_count\":{},\"event_type_count\":{},\"generated_at_ms\":{}}}",
        json_string(&state.config.location_name),
        json_string(&state.config.unit_system),
        json_string(&state.config.time_zone),
        json_string(&state.config.version),
        summary.state_count,
        summary.service_count,
        summary.event_type_count,
        state.generated_at_ms,
    )
}

fn api_root_json() -> String {
    "{\"message\":\"API running.\"}".to_string()
}

fn states_json(entities: &[Entity], now_ms: u64) -> String {
    format!(
        "[{}]",
        entities
            .iter()
            .map(|entity| state_json(entity, now_ms))
            .collect::<Vec<_>>()
            .join(",")
    )
}

fn state_json(entity: &Entity, now_ms: u64) -> String {
    let (state_value, last_changed_ms, last_updated_ms, source, confidence, stale) =
        match &entity.state {
            Some(snapshot) => (
                value_json(&snapshot.value),
                snapshot.observed_at_ms,
                snapshot.received_at_ms,
                state_source_label(snapshot.source),
                state_confidence_label(snapshot.confidence),
                snapshot.is_stale_at(now_ms),
            ),
            None => (json_string("unknown"), 0, 0, "unknown", "unknown", true),
        };

    let capability_ids = entity
        .capabilities
        .iter()
        .map(|capability| json_string(capability.capability_id.as_str()))
        .collect::<Vec<_>>()
        .join(",");

    format!(
        "{{\"entity_id\":{},\"state\":{},\"attributes\":{{\"friendly_name\":{},\"device_id\":{},\"domain\":{},\"entity_kind\":{},\"home_assistant_entity_id\":{},\"capability_count\":{},\"capabilities\":[{}],\"stale\":{}}},\"last_changed_ms\":{},\"last_updated_ms\":{},\"context\":{{\"source\":{},\"confidence\":{}}}}}",
        json_string(entity.entity_id.as_str()),
        state_value,
        json_string(&entity.name),
        json_string(entity.device_id.as_str()),
        json_string(entity_domain(entity.kind)),
        json_string(entity_kind_label(entity.kind)),
        json_string(home_assistant_entity_id(entity)),
        entity.capabilities.len(),
        capability_ids,
        stale,
        last_changed_ms,
        last_updated_ms,
        json_string(source),
        json_string(confidence),
    )
}

fn services_json(services: &[SmartHomePlatformService]) -> String {
    let mut domains = BTreeMap::<&str, Vec<&SmartHomePlatformService>>::new();
    for service in services {
        domains.entry(&service.domain).or_default().push(service);
    }

    format!(
        "[{}]",
        domains
            .into_iter()
            .map(|(domain, services)| {
                format!(
                    "{{\"domain\":{},\"services\":[{}]}}",
                    json_string(domain),
                    services
                        .into_iter()
                        .map(service_json)
                        .collect::<Vec<_>>()
                        .join(",")
                )
            })
            .collect::<Vec<_>>()
            .join(",")
    )
}

fn service_json(service: &SmartHomePlatformService) -> String {
    format!(
        "{{\"service\":{},\"description\":{},\"target_entity_ids\":[{}],\"target_scene_ids\":[{}],\"capability_ids\":[{}]}}",
        json_string(&service.service),
        json_string(&service.description),
        json_string_array(&service.target_entity_ids),
        json_string_array(&service.target_scene_ids),
        json_string_array(&service.capability_ids),
    )
}

fn service_catalog_items(
    state: &SmartHomePlatformHttpState,
    request: &WebRequest,
) -> Result<Vec<SmartHomePlatformService>, ApiError> {
    let mut services = platform_services(state);

    if let Some(domain) = query_string(request, "domain") {
        services.retain(|service| service.domain == domain);
    }
    if let Some(service_name) = query_string(request, "service") {
        services.retain(|service| service.service == service_name);
    }
    if let Some(capability_id) = query_string(request, "capability_id") {
        services.retain(|service| {
            service
                .capability_ids
                .iter()
                .any(|candidate| candidate == capability_id)
        });
    }
    if let Some(entity_id) = query_string(request, "entity_id") {
        services.retain(|service| {
            service.target_entity_ids.iter().any(|candidate| {
                candidate == entity_id
                    || state
                        .entities
                        .iter()
                        .find(|entity| entity.entity_id.as_str() == candidate)
                        .is_some_and(|entity| home_assistant_entity_id(entity) == entity_id)
            })
        });
    }
    if let Some(scene_id) = query_string(request, "scene_id") {
        services.retain(|service| {
            service.target_scene_ids.iter().any(|candidate| {
                candidate == scene_id
                    || state
                        .scenes
                        .iter()
                        .find(|scene| scene.scene_id.as_str() == candidate)
                        .is_some_and(|scene| home_assistant_scene_id(scene) == scene_id)
            })
        });
    }

    let limit = query_limit(request, 100, 500)?;
    services.truncate(limit);
    Ok(services)
}

fn service_catalog_json(
    services: &[SmartHomePlatformService],
    state: &SmartHomePlatformHttpState,
) -> String {
    let mut domains = Vec::<String>::new();
    let mut entity_ids = Vec::<String>::new();
    let mut scene_ids = Vec::<String>::new();
    for service in services {
        push_unique_string(&mut domains, &service.domain);
        for entity_id in &service.target_entity_ids {
            push_unique_string(&mut entity_ids, entity_id);
        }
        for scene_id in &service.target_scene_ids {
            push_unique_string(&mut scene_ids, scene_id);
        }
    }
    format!(
        "{{\"summary\":{{\"total_services\":{},\"domain_count\":{},\"target_entity_count\":{},\"target_scene_count\":{},\"runtime_authorized_services\":{}}},\"services\":[{}]}}",
        services.len(),
        domains.len(),
        entity_ids.len(),
        scene_ids.len(),
        services.len(),
        services
            .iter()
            .map(|service| service_catalog_item_json(service, state))
            .collect::<Vec<_>>()
            .join(",")
    )
}

fn service_catalog_item_json(
    service: &SmartHomePlatformService,
    state: &SmartHomePlatformHttpState,
) -> String {
    let home_assistant_entity_ids = service
        .target_entity_ids
        .iter()
        .map(|entity_id| {
            state
                .entities
                .iter()
                .find(|entity| entity.entity_id.as_str() == entity_id)
                .map(home_assistant_entity_id)
                .unwrap_or_else(|| {
                    home_assistant_entity_id_for(&EntityId::trusted(entity_id.as_str()))
                })
        })
        .collect::<Vec<_>>();
    let home_assistant_scene_ids = service
        .target_scene_ids
        .iter()
        .map(|scene_id| {
            state
                .scenes
                .iter()
                .find(|scene| scene.scene_id.as_str() == scene_id)
                .map(home_assistant_scene_id)
                .unwrap_or_else(|| format!("scene.{}", object_id(scene_id)))
        })
        .collect::<Vec<_>>();
    format!(
        "{{\"service_id\":{},\"domain\":{},\"service\":{},\"description\":{},\"home_assistant_path\":{},\"mutates_runtime\":true,\"runtime_authorized\":true,\"target_entity_ids\":[{}],\"home_assistant_entity_ids\":[{}],\"target_scene_ids\":[{}],\"home_assistant_scene_ids\":[{}],\"capability_ids\":[{}]}}",
        json_string(format!("{}.{}", service.domain, service.service)),
        json_string(&service.domain),
        json_string(&service.service),
        json_string(&service.description),
        json_string(format!("/api/services/{}/{}", service.domain, service.service)),
        json_string_array(&service.target_entity_ids),
        json_string_array(&home_assistant_entity_ids),
        json_string_array(&service.target_scene_ids),
        json_string_array(&home_assistant_scene_ids),
        json_string_array(&service.capability_ids),
    )
}

fn events_json(event_types: &[String]) -> String {
    format!(
        "[{}]",
        event_types
            .iter()
            .map(|event_type| {
                format!(
                    "{{\"event\":{},\"description\":{}}}",
                    json_string(event_type),
                    json_string(format!("{event_type} platform event")),
                )
            })
            .collect::<Vec<_>>()
            .join(",")
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ApiRouteDescriptor {
    method: &'static str,
    path: &'static str,
    category: &'static str,
    surface: &'static str,
    mutates_runtime: bool,
    runtime_authorized: bool,
    query_params: &'static [&'static str],
}

const API_ROUTE_CATALOG: &[ApiRouteDescriptor] = &[
    ApiRouteDescriptor {
        method: "GET",
        path: "/",
        category: "dashboard",
        surface: "browser",
        mutates_runtime: false,
        runtime_authorized: false,
        query_params: &[],
    },
    ApiRouteDescriptor {
        method: "GET",
        path: "/dashboard",
        category: "dashboard",
        surface: "browser",
        mutates_runtime: false,
        runtime_authorized: false,
        query_params: &[],
    },
    ApiRouteDescriptor {
        method: "GET",
        path: "/smart-home",
        category: "dashboard",
        surface: "browser",
        mutates_runtime: false,
        runtime_authorized: false,
        query_params: &[],
    },
    ApiRouteDescriptor {
        method: "GET",
        path: "/api/",
        category: "home_assistant",
        surface: "home_assistant",
        mutates_runtime: false,
        runtime_authorized: false,
        query_params: &[],
    },
    ApiRouteDescriptor {
        method: "GET",
        path: "/api/config",
        category: "home_assistant",
        surface: "home_assistant",
        mutates_runtime: false,
        runtime_authorized: false,
        query_params: &[],
    },
    ApiRouteDescriptor {
        method: "GET",
        path: "/api/states",
        category: "home_assistant",
        surface: "home_assistant",
        mutates_runtime: false,
        runtime_authorized: false,
        query_params: &[],
    },
    ApiRouteDescriptor {
        method: "GET",
        path: "/api/states/:entity_id",
        category: "home_assistant",
        surface: "home_assistant",
        mutates_runtime: false,
        runtime_authorized: false,
        query_params: &[],
    },
    ApiRouteDescriptor {
        method: "POST",
        path: "/api/states/:entity_id",
        category: "desired_state",
        surface: "home_assistant",
        mutates_runtime: true,
        runtime_authorized: true,
        query_params: &[],
    },
    ApiRouteDescriptor {
        method: "GET",
        path: "/api/services",
        category: "home_assistant",
        surface: "home_assistant",
        mutates_runtime: false,
        runtime_authorized: false,
        query_params: &[],
    },
    ApiRouteDescriptor {
        method: "POST",
        path: "/api/services/:domain/:service",
        category: "commands",
        surface: "home_assistant",
        mutates_runtime: true,
        runtime_authorized: true,
        query_params: &[],
    },
    ApiRouteDescriptor {
        method: "GET",
        path: "/api/events",
        category: "home_assistant",
        surface: "home_assistant",
        mutates_runtime: false,
        runtime_authorized: false,
        query_params: &[],
    },
    ApiRouteDescriptor {
        method: "GET",
        path: "/api/history/period",
        category: "state_history",
        surface: "home_assistant",
        mutates_runtime: false,
        runtime_authorized: false,
        query_params: &[
            "end_time",
            "filter_entity_id",
            "minimal_response",
            "room_id",
        ],
    },
    ApiRouteDescriptor {
        method: "GET",
        path: "/api/history/period/:start_time",
        category: "state_history",
        surface: "home_assistant",
        mutates_runtime: false,
        runtime_authorized: false,
        query_params: &[
            "end_time",
            "filter_entity_id",
            "minimal_response",
            "room_id",
        ],
    },
    ApiRouteDescriptor {
        method: "GET",
        path: "/api/smart_home/runtime",
        category: "runtime",
        surface: "smart_home",
        mutates_runtime: false,
        runtime_authorized: false,
        query_params: &[],
    },
    ApiRouteDescriptor {
        method: "GET",
        path: "/api/smart_home/health",
        category: "health",
        surface: "smart_home",
        mutates_runtime: false,
        runtime_authorized: false,
        query_params: &[],
    },
    ApiRouteDescriptor {
        method: "GET",
        path: "/api/smart_home/readiness",
        category: "health",
        surface: "smart_home",
        mutates_runtime: false,
        runtime_authorized: false,
        query_params: &[],
    },
    ApiRouteDescriptor {
        method: "GET",
        path: CONTROLLER_HANDOFF_PATH,
        category: "handoff",
        surface: "smart_home",
        mutates_runtime: false,
        runtime_authorized: false,
        query_params: &[],
    },
    ApiRouteDescriptor {
        method: "GET",
        path: "/api/smart_home/dashboard",
        category: "dashboard",
        surface: "smart_home",
        mutates_runtime: false,
        runtime_authorized: false,
        query_params: &[],
    },
    ApiRouteDescriptor {
        method: "GET",
        path: "/api/smart_home/dashboard_manifest",
        category: "dashboard",
        surface: "smart_home",
        mutates_runtime: false,
        runtime_authorized: false,
        query_params: &[],
    },
    ApiRouteDescriptor {
        method: "GET",
        path: "/api/smart_home/bootstrap",
        category: "dashboard",
        surface: "smart_home",
        mutates_runtime: false,
        runtime_authorized: false,
        query_params: &[],
    },
    ApiRouteDescriptor {
        method: "GET",
        path: "/api/smart_home/smoke",
        category: "smoke",
        surface: "smart_home",
        mutates_runtime: false,
        runtime_authorized: false,
        query_params: &[],
    },
    ApiRouteDescriptor {
        method: "GET",
        path: "/api/smart_home/smoke_script",
        category: "smoke",
        surface: "smart_home",
        mutates_runtime: false,
        runtime_authorized: false,
        query_params: &[],
    },
    ApiRouteDescriptor {
        method: "GET",
        path: "/api/smart_home/api",
        category: "api_catalog",
        surface: "smart_home",
        mutates_runtime: false,
        runtime_authorized: false,
        query_params: &["authorized", "category", "method", "mutating", "surface"],
    },
    ApiRouteDescriptor {
        method: "GET",
        path: "/api/smart_home/states",
        category: "states",
        surface: "smart_home",
        mutates_runtime: false,
        runtime_authorized: false,
        query_params: &[
            "capability_id",
            "confidence",
            "domain",
            "has_state",
            "kind",
            "limit",
            "room_id",
            "source",
            "stale",
        ],
    },
    ApiRouteDescriptor {
        method: "GET",
        path: "/api/smart_home/states/:entity_id",
        category: "states",
        surface: "smart_home",
        mutates_runtime: false,
        runtime_authorized: false,
        query_params: &[],
    },
    ApiRouteDescriptor {
        method: "GET",
        path: "/api/smart_home/services",
        category: "services",
        surface: "smart_home",
        mutates_runtime: false,
        runtime_authorized: false,
        query_params: &[
            "capability_id",
            "domain",
            "entity_id",
            "limit",
            "scene_id",
            "service",
        ],
    },
    ApiRouteDescriptor {
        method: "GET",
        path: "/api/smart_home/services/:domain/:service",
        category: "services",
        surface: "smart_home",
        mutates_runtime: false,
        runtime_authorized: false,
        query_params: &[],
    },
    ApiRouteDescriptor {
        method: "GET",
        path: "/api/smart_home/entities",
        category: "entities",
        surface: "smart_home",
        mutates_runtime: false,
        runtime_authorized: false,
        query_params: &[
            "capability_id",
            "commandable",
            "domain",
            "kind",
            "limit",
            "room_id",
        ],
    },
    ApiRouteDescriptor {
        method: "GET",
        path: "/api/smart_home/entities/:entity_id",
        category: "entities",
        surface: "smart_home",
        mutates_runtime: false,
        runtime_authorized: false,
        query_params: &[],
    },
    ApiRouteDescriptor {
        method: "GET",
        path: "/api/smart_home/capabilities",
        category: "capabilities",
        surface: "smart_home",
        mutates_runtime: false,
        runtime_authorized: false,
        query_params: &[
            "capability_id",
            "commandable",
            "domain",
            "limit",
            "observable",
        ],
    },
    ApiRouteDescriptor {
        method: "GET",
        path: "/api/smart_home/capability_grants",
        category: "authorization",
        surface: "smart_home",
        mutates_runtime: false,
        runtime_authorized: false,
        query_params: &[
            "capability_id",
            "entity_id",
            "limit",
            "principal_id",
            "scope",
            "sort",
            "status",
        ],
    },
    ApiRouteDescriptor {
        method: "GET",
        path: "/api/smart_home/capability_grants/:grant_id",
        category: "authorization",
        surface: "smart_home",
        mutates_runtime: false,
        runtime_authorized: false,
        query_params: &[],
    },
    ApiRouteDescriptor {
        method: "GET",
        path: "/api/smart_home/devices",
        category: "devices",
        surface: "smart_home",
        mutates_runtime: false,
        runtime_authorized: false,
        query_params: &["bridge_id", "health", "limit", "manufacturer", "room_id"],
    },
    ApiRouteDescriptor {
        method: "GET",
        path: "/api/smart_home/devices/:device_id",
        category: "devices",
        surface: "smart_home",
        mutates_runtime: false,
        runtime_authorized: false,
        query_params: &[],
    },
    ApiRouteDescriptor {
        method: "GET",
        path: "/api/smart_home/bridges",
        category: "bridges",
        surface: "smart_home",
        mutates_runtime: false,
        runtime_authorized: false,
        query_params: &["health", "integration_id", "limit", "transport"],
    },
    ApiRouteDescriptor {
        method: "GET",
        path: "/api/smart_home/bridges/:bridge_id",
        category: "bridges",
        surface: "smart_home",
        mutates_runtime: false,
        runtime_authorized: false,
        query_params: &[],
    },
    ApiRouteDescriptor {
        method: "GET",
        path: "/api/smart_home/pairing_sessions",
        category: "pairing",
        surface: "smart_home",
        mutates_runtime: false,
        runtime_authorized: false,
        query_params: &[
            "bridge_id",
            "expires_before_ms",
            "expiring_at_ms",
            "integration_id",
            "limit",
            "requested_by",
            "session_id",
            "sort",
            "status",
        ],
    },
    ApiRouteDescriptor {
        method: "GET",
        path: "/api/smart_home/pairing_sessions/:session_id",
        category: "pairing",
        surface: "smart_home",
        mutates_runtime: false,
        runtime_authorized: false,
        query_params: &[],
    },
    ApiRouteDescriptor {
        method: "GET",
        path: "/api/smart_home/rooms",
        category: "rooms",
        surface: "smart_home",
        mutates_runtime: false,
        runtime_authorized: false,
        query_params: &[
            "attention_only",
            "limit",
            "room_id",
            "sort",
            "state_gaps_only",
        ],
    },
    ApiRouteDescriptor {
        method: "GET",
        path: "/api/smart_home/rooms/:room_id",
        category: "rooms",
        surface: "smart_home",
        mutates_runtime: false,
        runtime_authorized: false,
        query_params: &[],
    },
    ApiRouteDescriptor {
        method: "GET",
        path: "/api/smart_home/scenes",
        category: "scenes",
        surface: "smart_home",
        mutates_runtime: false,
        runtime_authorized: false,
        query_params: &["entity_id", "limit", "room_id", "scope"],
    },
    ApiRouteDescriptor {
        method: "GET",
        path: "/api/smart_home/scenes/:scene_id",
        category: "scenes",
        surface: "smart_home",
        mutates_runtime: false,
        runtime_authorized: false,
        query_params: &[],
    },
    ApiRouteDescriptor {
        method: "GET",
        path: "/api/smart_home/events",
        category: "events",
        surface: "smart_home",
        mutates_runtime: false,
        runtime_authorized: false,
        query_params: &[
            "entity_id",
            "from_sequence",
            "kind",
            "limit",
            "room_id",
            "sort",
            "to_sequence",
        ],
    },
    ApiRouteDescriptor {
        method: "GET",
        path: "/api/smart_home/events/:sequence",
        category: "events",
        surface: "smart_home",
        mutates_runtime: false,
        runtime_authorized: false,
        query_params: &[],
    },
    ApiRouteDescriptor {
        method: "GET",
        path: "/api/smart_home/command_results",
        category: "command_results",
        surface: "smart_home",
        mutates_runtime: false,
        runtime_authorized: false,
        query_params: &[
            "bridge_id",
            "command_id",
            "correlation_id",
            "from_sequence",
            "limit",
            "room_id",
            "sort",
            "status",
            "to_sequence",
        ],
    },
    ApiRouteDescriptor {
        method: "GET",
        path: "/api/smart_home/command_results/:command_id",
        category: "command_results",
        surface: "smart_home",
        mutates_runtime: false,
        runtime_authorized: false,
        query_params: &[],
    },
    ApiRouteDescriptor {
        method: "GET",
        path: "/api/smart_home/authorization_decisions",
        category: "authorization",
        surface: "smart_home",
        mutates_runtime: false,
        runtime_authorized: false,
        query_params: &["limit", "outcome", "principal_id", "sort"],
    },
    ApiRouteDescriptor {
        method: "GET",
        path: "/api/smart_home/command_authorization",
        category: "authorization",
        surface: "smart_home",
        mutates_runtime: false,
        runtime_authorized: false,
        query_params: &["command_type", "entity_id", "principal_id"],
    },
    ApiRouteDescriptor {
        method: "GET",
        path: "/api/smart_home/desired_state_authorization",
        category: "authorization",
        surface: "smart_home",
        mutates_runtime: false,
        runtime_authorized: false,
        query_params: &["entity_id", "operation", "principal_id"],
    },
    ApiRouteDescriptor {
        method: "GET",
        path: "/api/smart_home/scene_authorization",
        category: "authorization",
        surface: "smart_home",
        mutates_runtime: false,
        runtime_authorized: false,
        query_params: &["scene_id", "principal_id"],
    },
    ApiRouteDescriptor {
        method: "GET",
        path: "/api/smart_home/service_authorization/:domain/:service",
        category: "authorization",
        surface: "smart_home",
        mutates_runtime: false,
        runtime_authorized: false,
        query_params: &[
            "brightness",
            "brightness_pct",
            "color_temp",
            "color_temp_kelvin",
            "entity_id",
            "entity_ids",
            "idempotency_key",
            "kelvin",
            "principal_id",
            "rgb_color",
            "scene_id",
            "scene_ids",
            "temperature",
            "timeout_ms",
        ],
    },
    ApiRouteDescriptor {
        method: "GET",
        path: "/api/smart_home/authorization_decisions/:decision_index",
        category: "authorization",
        surface: "smart_home",
        mutates_runtime: false,
        runtime_authorized: false,
        query_params: &[],
    },
    ApiRouteDescriptor {
        method: "GET",
        path: "/api/smart_home/desired_states",
        category: "desired_state",
        surface: "smart_home",
        mutates_runtime: false,
        runtime_authorized: false,
        query_params: &["capability_id", "entity_id", "limit", "requested_by"],
    },
    ApiRouteDescriptor {
        method: "POST",
        path: "/api/smart_home/desired_states/:entity_id",
        category: "desired_state",
        surface: "smart_home",
        mutates_runtime: true,
        runtime_authorized: true,
        query_params: &[],
    },
    ApiRouteDescriptor {
        method: "DELETE",
        path: "/api/smart_home/desired_states/:entity_id",
        category: "desired_state",
        surface: "smart_home",
        mutates_runtime: true,
        runtime_authorized: true,
        query_params: &[],
    },
    ApiRouteDescriptor {
        method: "GET",
        path: "/api/smart_home/state_history",
        category: "state_history",
        surface: "smart_home",
        mutates_runtime: false,
        runtime_authorized: false,
        query_params: &[
            "bridge_id",
            "entity_id",
            "event_type",
            "from_ms",
            "limit",
            "observed_at_or_after_ms",
            "observed_at_or_before_ms",
            "received_at_or_after_ms",
            "received_at_or_before_ms",
            "room_id",
            "sort",
            "to_ms",
        ],
    },
    ApiRouteDescriptor {
        method: "GET",
        path: "/api/smart_home/state_history/:event_id",
        category: "state_history",
        surface: "smart_home",
        mutates_runtime: false,
        runtime_authorized: false,
        query_params: &[],
    },
    ApiRouteDescriptor {
        method: "GET",
        path: "/api/smart_home/automations",
        category: "automations",
        surface: "smart_home",
        mutates_runtime: false,
        runtime_authorized: false,
        query_params: &[],
    },
    ApiRouteDescriptor {
        method: "POST",
        path: "/api/smart_home/automations",
        category: "automations",
        surface: "smart_home",
        mutates_runtime: true,
        runtime_authorized: true,
        query_params: &[],
    },
    ApiRouteDescriptor {
        method: "GET",
        path: "/api/smart_home/automation_audit",
        category: "automations",
        surface: "smart_home",
        mutates_runtime: false,
        runtime_authorized: false,
        query_params: &[],
    },
    ApiRouteDescriptor {
        method: "POST",
        path: "/api/smart_home/automations/evaluate",
        category: "automations",
        surface: "smart_home",
        mutates_runtime: true,
        runtime_authorized: true,
        query_params: &[],
    },
];

fn api_catalog_routes(request: &WebRequest) -> Result<Vec<&'static ApiRouteDescriptor>, ApiError> {
    let method = query_string(request, "method").map(str::to_ascii_uppercase);
    let category = query_string(request, "category");
    let surface = query_string(request, "surface");
    let mutating = query_bool(request, "mutating")?;
    let authorized = query_bool(request, "authorized")?;

    if let Some(surface) = surface {
        if !matches!(surface, "all" | "browser" | "home_assistant" | "smart_home") {
            return Err(ApiError::bad_request(format!(
                "unsupported API surface `{surface}`"
            )));
        }
    }

    Ok(API_ROUTE_CATALOG
        .iter()
        .filter(|route| {
            method
                .as_deref()
                .is_none_or(|method| route.method == method)
        })
        .filter(|route| category.is_none_or(|category| route.category == category))
        .filter(|route| surface.is_none_or(|surface| surface == "all" || route.surface == surface))
        .filter(|route| mutating.is_none_or(|mutating| route.mutates_runtime == mutating))
        .filter(|route| authorized.is_none_or(|authorized| route.runtime_authorized == authorized))
        .collect())
}

fn api_catalog_json(routes: &[&ApiRouteDescriptor]) -> String {
    format!(
        "{{\"version\":{},\"route_count\":{},\"routes\":[{}]}}",
        json_string(VERSION),
        routes.len(),
        routes
            .iter()
            .map(|route| api_route_json(route))
            .collect::<Vec<_>>()
            .join(",")
    )
}

fn api_route_json(route: &ApiRouteDescriptor) -> String {
    format!(
        "{{\"method\":{},\"path\":{},\"category\":{},\"surface\":{},\"mutates_runtime\":{},\"runtime_authorized\":{},\"query_params\":[{}]}}",
        json_string(route.method),
        json_string(route.path),
        json_string(route.category),
        json_string(route.surface),
        route.mutates_runtime,
        route.runtime_authorized,
        json_id_array(route.query_params.iter().copied()),
    )
}

fn dashboard_ui_response() -> WebResponse {
    WebResponse::ok(DASHBOARD_HTML.as_bytes().to_vec())
        .with_content_type("text/html; charset=utf-8")
}

fn runtime_snapshot_response(runtime: &SmartHomePlatformHttpRuntime) -> WebResponse {
    let runtime_guard = runtime
        .runtime
        .lock()
        .expect("smart-home runtime mutex should not be poisoned");
    WebResponse::json(
        runtime_snapshot_json(&runtime_guard.read_snapshot_at(runtime.now_ms())).into_bytes(),
    )
}

fn runtime_health_response(runtime: &SmartHomePlatformHttpRuntime) -> WebResponse {
    let runtime_guard = runtime
        .runtime
        .lock()
        .expect("smart-home runtime mutex should not be poisoned");
    WebResponse::json(runtime_health_json(runtime, &runtime_guard).into_bytes())
}

fn runtime_readiness_response(runtime: &SmartHomePlatformHttpRuntime) -> WebResponse {
    let runtime_guard = runtime
        .runtime
        .lock()
        .expect("smart-home runtime mutex should not be poisoned");
    WebResponse::json(runtime_readiness_json(runtime, &runtime_guard).into_bytes())
}

fn runtime_controller_handoff_response(runtime: &SmartHomePlatformHttpRuntime) -> WebResponse {
    let runtime_guard = runtime
        .runtime
        .lock()
        .expect("smart-home runtime mutex should not be poisoned");
    WebResponse::json(runtime_controller_handoff_json(runtime, &runtime_guard).into_bytes())
}

fn runtime_dashboard_response(runtime: &SmartHomePlatformHttpRuntime) -> WebResponse {
    let runtime_guard = runtime
        .runtime
        .lock()
        .expect("smart-home runtime mutex should not be poisoned");
    WebResponse::json(runtime_dashboard_json(runtime, &runtime_guard).into_bytes())
}

fn dashboard_manifest_response(runtime: &SmartHomePlatformHttpRuntime) -> WebResponse {
    match runtime.dashboard_manifest.as_deref() {
        Some(manifest) => serialized_json_response(&serde_json::json!({
            "configured": true,
            "summary": manifest.summary(),
            "manifest": manifest,
        })),
        None => serialized_json_response(&serde_json::json!({
            "configured": false,
            "summary": {
                "dashboards": 0,
                "views": 0,
                "cards": 0,
                "entity_references": 0,
                "source_resources": 0,
            },
            "manifest": null,
        })),
    }
}

fn runtime_bootstrap_response(runtime: &SmartHomePlatformHttpRuntime) -> WebResponse {
    let runtime_guard = runtime
        .runtime
        .lock()
        .expect("smart-home runtime mutex should not be poisoned");
    WebResponse::json(runtime_bootstrap_json(runtime, &runtime_guard).into_bytes())
}

fn runtime_smoke_response(runtime: &SmartHomePlatformHttpRuntime) -> WebResponse {
    let runtime_guard = runtime
        .runtime
        .lock()
        .expect("smart-home runtime mutex should not be poisoned");
    WebResponse::json(runtime_smoke_json(runtime, &runtime_guard).into_bytes())
}

fn runtime_smoke_script_response(
    runtime: &SmartHomePlatformHttpRuntime,
    request: &WebRequest,
) -> WebResponse {
    let runtime_guard = runtime
        .runtime
        .lock()
        .expect("smart-home runtime mutex should not be poisoned");
    WebResponse::text(runtime_smoke_script(runtime, &runtime_guard, request))
}

fn api_catalog_response(request: &WebRequest) -> WebResponse {
    let routes = match api_catalog_routes(request) {
        Ok(routes) => routes,
        Err(error) => return api_error_response(error),
    };
    WebResponse::json(api_catalog_json(&routes).into_bytes())
}

fn runtime_states_response(
    runtime: &SmartHomePlatformHttpRuntime,
    request: &WebRequest,
) -> WebResponse {
    let runtime_guard = runtime
        .runtime
        .lock()
        .expect("smart-home runtime mutex should not be poisoned");
    let entities = match runtime_state_entities(&runtime_guard, request, runtime.now_ms()) {
        Ok(entities) => entities,
        Err(error) => return api_error_response(error),
    };
    WebResponse::json(
        states_registry_json(&entities, &runtime_guard, runtime.now_ms()).into_bytes(),
    )
}

fn runtime_state_response(
    runtime: &SmartHomePlatformHttpRuntime,
    request: &WebRequest,
) -> WebResponse {
    let Some(target) = request.route_params.get("entity_id") else {
        return json_error(400, "missing entity_id");
    };
    let runtime_guard = runtime
        .runtime
        .lock()
        .expect("smart-home runtime mutex should not be poisoned");
    let entity = match runtime_guard
        .registry()
        .entities()
        .find(|entity| entity_matches_external_id(entity, target))
    {
        Some(entity) => entity,
        None => {
            return api_error_response(ApiError::not_found(format!("state `{target}` not found")));
        }
    };
    WebResponse::json(state_registry_json(entity, &runtime_guard, runtime.now_ms()).into_bytes())
}

fn runtime_services_response(
    runtime: &SmartHomePlatformHttpRuntime,
    request: &WebRequest,
) -> WebResponse {
    let state = runtime.snapshot();
    let services = match service_catalog_items(&state, request) {
        Ok(services) => services,
        Err(error) => return api_error_response(error),
    };
    WebResponse::json(service_catalog_json(&services, &state).into_bytes())
}

fn runtime_service_response(
    runtime: &SmartHomePlatformHttpRuntime,
    request: &WebRequest,
) -> WebResponse {
    let Some(domain) = request.route_params.get("domain") else {
        return api_error_response(ApiError::bad_request("missing domain"));
    };
    let Some(service) = request.route_params.get("service") else {
        return api_error_response(ApiError::bad_request("missing service"));
    };
    let state = runtime.snapshot();
    let services = platform_services(&state);
    let Some(service_record) = services
        .iter()
        .find(|record| record.domain == *domain && record.service == *service)
    else {
        return api_error_response(ApiError::not_found(format!(
            "service `{domain}.{service}` not found"
        )));
    };
    WebResponse::json(service_catalog_item_json(service_record, &state).into_bytes())
}

fn runtime_entities_response(
    runtime: &SmartHomePlatformHttpRuntime,
    request: &WebRequest,
) -> WebResponse {
    let runtime_guard = runtime
        .runtime
        .lock()
        .expect("smart-home runtime mutex should not be poisoned");
    let entities = match runtime_entities(&runtime_guard, request) {
        Ok(entities) => entities,
        Err(error) => return api_error_response(error),
    };
    WebResponse::json(
        entities_registry_json(&entities, &runtime_guard, runtime.now_ms()).into_bytes(),
    )
}

fn runtime_entity_response(
    runtime: &SmartHomePlatformHttpRuntime,
    request: &WebRequest,
) -> WebResponse {
    let Some(target) = request.route_params.get("entity_id") else {
        return json_error(400, "missing entity_id");
    };
    let runtime_guard = runtime
        .runtime
        .lock()
        .expect("smart-home runtime mutex should not be poisoned");
    let entity = match runtime_guard
        .registry()
        .entities()
        .find(|entity| entity_matches_external_id(entity, target))
    {
        Some(entity) => entity,
        None => {
            return api_error_response(ApiError::not_found(format!("entity `{target}` not found")));
        }
    };
    WebResponse::json(entity_registry_json(entity, &runtime_guard, runtime.now_ms()).into_bytes())
}

fn runtime_capabilities_response(
    runtime: &SmartHomePlatformHttpRuntime,
    request: &WebRequest,
) -> WebResponse {
    let query = match capability_catalog_query(request) {
        Ok(query) => query,
        Err(error) => return api_error_response(error),
    };
    let runtime_guard = runtime
        .runtime
        .lock()
        .expect("smart-home runtime mutex should not be poisoned");
    let capabilities = runtime_capability_catalog(&runtime_guard, &query);
    WebResponse::json(capabilities_catalog_json(&capabilities).into_bytes())
}

fn runtime_capability_grants_response(
    runtime: &SmartHomePlatformHttpRuntime,
    request: &WebRequest,
) -> WebResponse {
    let runtime_guard = runtime
        .runtime
        .lock()
        .expect("smart-home runtime mutex should not be poisoned");
    let query = match runtime_capability_grant_query(&runtime_guard, request) {
        Ok(query) => query,
        Err(error) => return api_error_response(error),
    };
    let grants = runtime_guard.query_capability_grants_at(&query, runtime.now_ms());
    let summary = runtime_guard.capability_grant_summary_at(&query, runtime.now_ms());
    WebResponse::json(capability_grants_json(&grants, &summary, runtime.now_ms()).into_bytes())
}

fn runtime_capability_grant_response(
    runtime: &SmartHomePlatformHttpRuntime,
    request: &WebRequest,
) -> WebResponse {
    let Some(grant_id) = request.route_params.get("grant_id") else {
        return api_error_response(ApiError::bad_request("missing grant_id"));
    };
    let runtime_guard = runtime
        .runtime
        .lock()
        .expect("smart-home runtime mutex should not be poisoned");
    let Some(grant) = runtime_guard
        .registry()
        .capability_grants()
        .find(|grant| grant.grant_id.as_str() == grant_id)
    else {
        return api_error_response(ApiError::not_found(format!(
            "capability grant `{grant_id}` not found"
        )));
    };
    WebResponse::json(capability_grant_json(grant, runtime.now_ms()).into_bytes())
}

fn runtime_devices_response(
    runtime: &SmartHomePlatformHttpRuntime,
    request: &WebRequest,
) -> WebResponse {
    let runtime_guard = runtime
        .runtime
        .lock()
        .expect("smart-home runtime mutex should not be poisoned");
    let devices = match runtime_devices(&runtime_guard, request) {
        Ok(devices) => devices,
        Err(error) => return api_error_response(error),
    };
    WebResponse::json(
        devices_registry_json(&devices, &runtime_guard, runtime.now_ms()).into_bytes(),
    )
}

fn runtime_device_response(
    runtime: &SmartHomePlatformHttpRuntime,
    request: &WebRequest,
) -> WebResponse {
    let Some(target) = request.route_params.get("device_id") else {
        return json_error(400, "missing device_id");
    };
    let runtime_guard = runtime
        .runtime
        .lock()
        .expect("smart-home runtime mutex should not be poisoned");
    let device = match runtime_guard
        .registry()
        .devices()
        .find(|device| device.device_id.as_str() == target)
    {
        Some(device) => device,
        None => {
            return api_error_response(ApiError::not_found(format!("device `{target}` not found")));
        }
    };
    WebResponse::json(device_registry_json(device, &runtime_guard, runtime.now_ms()).into_bytes())
}

fn runtime_bridges_response(
    runtime: &SmartHomePlatformHttpRuntime,
    request: &WebRequest,
) -> WebResponse {
    let runtime_guard = runtime
        .runtime
        .lock()
        .expect("smart-home runtime mutex should not be poisoned");
    let bridges = match runtime_bridges(&runtime_guard, request) {
        Ok(bridges) => bridges,
        Err(error) => return api_error_response(error),
    };
    WebResponse::json(
        bridges_registry_json(&bridges, &runtime_guard, runtime.now_ms()).into_bytes(),
    )
}

fn runtime_bridge_response(
    runtime: &SmartHomePlatformHttpRuntime,
    request: &WebRequest,
) -> WebResponse {
    let Some(target) = request.route_params.get("bridge_id") else {
        return json_error(400, "missing bridge_id");
    };
    let runtime_guard = runtime
        .runtime
        .lock()
        .expect("smart-home runtime mutex should not be poisoned");
    let bridge = match runtime_guard
        .registry()
        .bridges()
        .find(|bridge| bridge.bridge_id.as_str() == target)
    {
        Some(bridge) => bridge,
        None => {
            return api_error_response(ApiError::not_found(format!("bridge `{target}` not found")));
        }
    };
    WebResponse::json(bridge_registry_json(bridge, &runtime_guard, runtime.now_ms()).into_bytes())
}

fn runtime_pairing_sessions_response(
    runtime: &SmartHomePlatformHttpRuntime,
    request: &WebRequest,
) -> WebResponse {
    let query = match runtime_pairing_session_query(request) {
        Ok(query) => query,
        Err(error) => return api_error_response(error),
    };
    let runtime_guard = runtime
        .runtime
        .lock()
        .expect("smart-home runtime mutex should not be poisoned");
    let sessions = runtime_guard.query_pairing_sessions(&query);
    let summary = runtime_guard.pairing_session_inventory_summary_at(&query, runtime.now_ms());
    serialized_json_response(&serde_json::json!({
        "generated_at_ms": runtime.now_ms(),
        "summary": {
            "total_sessions": summary.total_sessions,
            "pending_user_presence_sessions": summary.pending_user_presence_sessions,
            "completed_sessions": summary.completed_sessions,
            "expired_sessions": summary.expired_sessions,
            "cancelled_sessions": summary.cancelled_sessions,
            "expiring_sessions": summary.expiring_sessions,
            "sessions_with_vault_ref": summary.sessions_with_vault_ref,
        },
        "sessions": sessions
            .iter()
            .map(|session| pairing_session_json(session))
            .collect::<Vec<_>>(),
    }))
}

fn runtime_pairing_session_response(
    runtime: &SmartHomePlatformHttpRuntime,
    request: &WebRequest,
) -> WebResponse {
    let Some(session_id) = request.route_params.get("session_id") else {
        return api_error_response(ApiError::bad_request("missing session_id"));
    };
    let runtime_guard = runtime
        .runtime
        .lock()
        .expect("smart-home runtime mutex should not be poisoned");
    let session_id = RuntimePairingSessionId::trusted(session_id);
    let Some(session) = runtime_guard.pairing_session(&session_id) else {
        return api_error_response(ApiError::not_found(format!(
            "pairing session `{session_id}` not found"
        )));
    };
    serialized_json_response(&pairing_session_json(session))
}

fn pairing_session_json(session: &RuntimePairingSession) -> JsonValue {
    serde_json::json!({
        "session_id": session.session_id,
        "bridge_id": session.bridge_id,
        "integration_id": session.integration_id,
        "requested_by": session.requested_by,
        "started_at_ms": session.started_at_ms,
        "expires_at_ms": session.expires_at_ms,
        "status": pairing_session_status_label(session.status),
        "vault_ref": session.vault_ref,
        "metadata": session.metadata,
    })
}

fn runtime_rooms_response(
    runtime: &SmartHomePlatformHttpRuntime,
    request: &WebRequest,
) -> WebResponse {
    let query = match runtime_room_query(request) {
        Ok(query) => query,
        Err(error) => return api_error_response(error),
    };
    let runtime_guard = runtime
        .runtime
        .lock()
        .expect("smart-home runtime mutex should not be poisoned");
    let rooms = runtime_guard.query_room_summaries_at(&query, runtime.now_ms());
    WebResponse::json(rooms_json(&rooms, &runtime_guard).into_bytes())
}

fn runtime_room_response(
    runtime: &SmartHomePlatformHttpRuntime,
    request: &WebRequest,
) -> WebResponse {
    let Some(room_id) = request.route_params.get("room_id") else {
        return json_error(400, "missing room_id");
    };
    let runtime_guard = runtime
        .runtime
        .lock()
        .expect("smart-home runtime mutex should not be poisoned");
    let query = RuntimeRoomQuery::new()
        .for_room(room_id.as_str())
        .with_limit(1);
    let rooms = runtime_guard.query_room_summaries_at(&query, runtime.now_ms());
    let Some(room) = rooms.first() else {
        return api_error_response(ApiError::not_found(format!("room `{room_id}` not found")));
    };
    WebResponse::json(room_detail_json(room, &runtime_guard, runtime.now_ms()).into_bytes())
}

fn runtime_scenes_response(
    runtime: &SmartHomePlatformHttpRuntime,
    request: &WebRequest,
) -> WebResponse {
    let runtime_guard = runtime
        .runtime
        .lock()
        .expect("smart-home runtime mutex should not be poisoned");
    let scenes = match runtime_scenes(&runtime_guard, request) {
        Ok(scenes) => scenes,
        Err(error) => return api_error_response(error),
    };
    WebResponse::json(scenes_json(&scenes, &runtime_guard).into_bytes())
}

fn runtime_scene_response(
    runtime: &SmartHomePlatformHttpRuntime,
    request: &WebRequest,
) -> WebResponse {
    let Some(target) = request.route_params.get("scene_id") else {
        return json_error(400, "missing scene_id");
    };
    let runtime_guard = runtime
        .runtime
        .lock()
        .expect("smart-home runtime mutex should not be poisoned");
    let scene = match runtime_guard
        .registry()
        .scenes()
        .find(|scene| scene_matches_external_id(scene, target))
    {
        Some(scene) => scene,
        None => {
            return api_error_response(ApiError::not_found(format!("scene `{target}` not found")));
        }
    };
    WebResponse::json(scene_json(scene, &runtime_guard).into_bytes())
}

fn runtime_events_response(
    runtime: &SmartHomePlatformHttpRuntime,
    request: &WebRequest,
) -> WebResponse {
    let query = match runtime_event_query(request) {
        Ok(query) => query,
        Err(error) => return api_error_response(error),
    };
    let runtime_guard = runtime
        .runtime
        .lock()
        .expect("smart-home runtime mutex should not be poisoned");
    let limit = query.limit;
    let mut entries = runtime_guard.event_bus().query_events(&RuntimeEventQuery {
        limit: None,
        ..query
    });
    let entity_id = match query_string(request, "entity_id")
        .map(|value| runtime_entity_id(&runtime_guard, value))
        .transpose()
    {
        Ok(entity_id) => entity_id,
        Err(error) => return api_error_response(error),
    };
    if let Some(entity_id) = entity_id {
        let entity_filter = RuntimeEventFilter::Entity(entity_id);
        entries.retain(|entry| entity_filter.matches(entry.event));
    }
    if let Some(room_id) = query_string(request, "room_id") {
        entries.retain(|entry| runtime_event_matches_room(&runtime_guard, entry.event, room_id));
    }
    if let Some(limit) = limit {
        entries.truncate(limit);
    }
    let summary = smart_home_runtime::RuntimeEventLogSummary::from_entries(entries.iter().copied());
    WebResponse::json(runtime_event_log_json(&entries, &summary).into_bytes())
}

fn runtime_event_response(
    runtime: &SmartHomePlatformHttpRuntime,
    request: &WebRequest,
) -> WebResponse {
    let sequence = match route_u64(request, "sequence") {
        Ok(sequence) => sequence,
        Err(error) => return api_error_response(error),
    };
    let runtime_guard = runtime
        .runtime
        .lock()
        .expect("smart-home runtime mutex should not be poisoned");
    let query = RuntimeEventQuery::new()
        .from_checkpoint(RuntimeEventCheckpoint::from_next_sequence(sequence))
        .with_limit(1);
    let entries = runtime_guard.event_bus().query_events(&query);
    let Some(entry) = entries.first().filter(|entry| entry.sequence == sequence) else {
        return api_error_response(ApiError::not_found(format!(
            "event sequence `{sequence}` not found"
        )));
    };
    WebResponse::json(runtime_event_entry_json(entry).into_bytes())
}

fn runtime_command_results_response(
    runtime: &SmartHomePlatformHttpRuntime,
    request: &WebRequest,
) -> WebResponse {
    let query = match runtime_command_result_query(request) {
        Ok(query) => query,
        Err(error) => return api_error_response(error),
    };
    let runtime_guard = runtime
        .runtime
        .lock()
        .expect("smart-home runtime mutex should not be poisoned");
    let limit = query.limit;
    let mut records = runtime_guard.query_command_results(&RuntimeCommandResultQuery {
        limit: None,
        ..query
    });
    if let Some(room_id) = query_string(request, "room_id") {
        records.retain(|record| bridge_has_room(&runtime_guard, &record.result.bridge_id, room_id));
    }
    if let Some(limit) = limit {
        records.truncate(limit);
    }
    let summary = smart_home_runtime::RuntimeCommandResultSummary::from_records(records.iter());
    WebResponse::json(command_results_audit_json(&records, &summary).into_bytes())
}

fn runtime_command_result_response(
    runtime: &SmartHomePlatformHttpRuntime,
    request: &WebRequest,
) -> WebResponse {
    let Some(command_id) = request.route_params.get("command_id") else {
        return api_error_response(ApiError::bad_request("missing command_id"));
    };
    let runtime_guard = runtime
        .runtime
        .lock()
        .expect("smart-home runtime mutex should not be poisoned");
    let query = RuntimeCommandResultQuery::new()
        .for_command(CommandId::trusted(command_id.clone()))
        .with_limit(1);
    let records = runtime_guard.query_command_results(&query);
    let Some(record) = records.first() else {
        return api_error_response(ApiError::not_found(format!(
            "command result `{command_id}` not found"
        )));
    };
    WebResponse::json(command_result_record_json(record).into_bytes())
}

fn runtime_authorization_decisions_response(
    runtime: &SmartHomePlatformHttpRuntime,
    request: &WebRequest,
) -> WebResponse {
    let query = match runtime_authorization_decision_query(request) {
        Ok(query) => query,
        Err(error) => return api_error_response(error),
    };
    let runtime_guard = runtime
        .runtime
        .lock()
        .expect("smart-home runtime mutex should not be poisoned");
    let decisions = runtime_guard.query_authorization_decisions(&query);
    let records = authorization_decision_records(&runtime_guard, decisions);
    let summary = runtime_guard.authorization_decision_summary(&query);
    WebResponse::json(authorization_decisions_json(&records, &summary).into_bytes())
}

fn runtime_command_authorization_response(
    runtime: &SmartHomePlatformHttpRuntime,
    request: &WebRequest,
) -> WebResponse {
    let Some(target) = query_string(request, "entity_id") else {
        return api_error_response(ApiError::bad_request("missing entity_id"));
    };
    let Some(command_type) = query_string(request, "command_type") else {
        return api_error_response(ApiError::bad_request("missing command_type"));
    };
    let command_type = match command_type_from_label(command_type) {
        Ok(command_type) => command_type,
        Err(error) => return api_error_response(error),
    };
    let principal_id = query_string(request, "principal_id")
        .map(AgentId::trusted)
        .unwrap_or_else(|| runtime.principal_id.clone());

    let runtime_guard = runtime
        .runtime
        .lock()
        .expect("smart-home runtime mutex should not be poisoned");
    let entity = match runtime_entity(&runtime_guard, target) {
        Ok(entity) => entity,
        Err(error) => return api_error_response(error),
    };
    let command = match preview_command(&entity, command_type, &principal_id, runtime.now_ms()) {
        Ok(command) => command,
        Err(error) => return api_error_response(error),
    };
    let grants = runtime_guard
        .registry()
        .capability_grants_for_principal(&principal_id);
    let tool_decision = AuthorizationDecision::for_tool(
        principal_id.clone(),
        SmartHomeTool::Command,
        grants.iter().copied(),
        runtime.now_ms(),
    );
    let command_decision = AuthorizationDecision::for_command(
        principal_id.clone(),
        &command,
        grants.iter().copied(),
        runtime.now_ms(),
    );
    WebResponse::json(
        command_authorization_preview_json(&entity, &command, &tool_decision, &command_decision)
            .into_bytes(),
    )
}

fn runtime_desired_state_authorization_response(
    runtime: &SmartHomePlatformHttpRuntime,
    request: &WebRequest,
) -> WebResponse {
    let Some(target) = query_string(request, "entity_id") else {
        return api_error_response(ApiError::bad_request("missing entity_id"));
    };
    let operation = match query_string(request, "operation")
        .map(desired_state_authorization_operation_from_label)
        .transpose()
    {
        Ok(operation) => operation.unwrap_or(DesiredStateAuthorizationOperation::Set),
        Err(error) => return api_error_response(error),
    };
    let principal_id = query_string(request, "principal_id")
        .map(AgentId::trusted)
        .unwrap_or_else(|| runtime.principal_id.clone());

    let runtime_guard = runtime
        .runtime
        .lock()
        .expect("smart-home runtime mutex should not be poisoned");
    let entity = match runtime_entity(&runtime_guard, target) {
        Ok(entity) => entity,
        Err(error) => return api_error_response(error),
    };
    let grants = runtime_guard
        .registry()
        .capability_grants_for_principal(&principal_id);
    let tool_decision = AuthorizationDecision::for_tool(
        principal_id,
        operation.tool(),
        grants.iter().copied(),
        runtime.now_ms(),
    );
    WebResponse::json(
        desired_state_authorization_preview_json(&entity, operation, &tool_decision).into_bytes(),
    )
}

fn runtime_scene_authorization_response(
    runtime: &SmartHomePlatformHttpRuntime,
    request: &WebRequest,
) -> WebResponse {
    let Some(target) = query_string(request, "scene_id") else {
        return api_error_response(ApiError::bad_request("missing scene_id"));
    };
    let principal_id = query_string(request, "principal_id")
        .map(AgentId::trusted)
        .unwrap_or_else(|| runtime.principal_id.clone());

    let runtime_guard = runtime
        .runtime
        .lock()
        .expect("smart-home runtime mutex should not be poisoned");
    let state = SmartHomePlatformHttpState::from_runtime(
        &runtime_guard,
        runtime.config.clone(),
        runtime.event_types.clone(),
        runtime.now_ms(),
    );
    let scene = match state
        .scenes
        .iter()
        .find(|scene| scene_matches_external_id(scene, target))
        .cloned()
    {
        Some(scene) => scene,
        None => {
            return api_error_response(ApiError::not_found(format!("scene `{target}` not found")))
        }
    };
    let call = ServiceCall {
        target_entity_ids: Vec::new(),
        target_scene_ids: vec![scene.scene_id.as_str().to_string()],
        body: JsonValue::Object(Default::default()),
        idempotency_key: None,
        timeout_ms: None,
    };
    let service_commands = match scene_service_commands(&state, &call) {
        Ok(commands) => commands,
        Err(error) => return api_error_response(error),
    };
    let grants = runtime_guard
        .registry()
        .capability_grants_for_principal(&principal_id);
    let tool_decision = AuthorizationDecision::for_tool(
        principal_id.clone(),
        SmartHomeTool::Command,
        grants.iter().copied(),
        runtime.now_ms(),
    );

    let command_previews = match authorization_command_previews(
        &runtime_guard,
        &service_commands,
        &principal_id,
        &grants,
        runtime.now_ms(),
    ) {
        Ok(command_previews) => command_previews,
        Err(error) => return api_error_response(error),
    };

    WebResponse::json(
        scene_authorization_preview_json(&scene, &tool_decision, &command_previews).into_bytes(),
    )
}

fn runtime_service_authorization_response(
    runtime: &SmartHomePlatformHttpRuntime,
    request: &WebRequest,
) -> WebResponse {
    let domain = match request.route_params.get("domain") {
        Some(domain) => domain.as_str(),
        None => return json_error(400, "missing domain"),
    };
    let service = match request.route_params.get("service") {
        Some(service) => service.as_str(),
        None => return json_error(400, "missing service"),
    };
    let principal_id = query_string(request, "principal_id")
        .map(AgentId::trusted)
        .unwrap_or_else(|| runtime.principal_id.clone());
    let call = match parse_service_call_query(request) {
        Ok(call) => call,
        Err(error) => return api_error_response(error),
    };

    let runtime_guard = runtime
        .runtime
        .lock()
        .expect("smart-home runtime mutex should not be poisoned");
    let state = SmartHomePlatformHttpState::from_runtime(
        &runtime_guard,
        runtime.config.clone(),
        runtime.event_types.clone(),
        runtime.now_ms(),
    );
    let service_commands = match service_commands(&state, domain, service, &call) {
        Ok(commands) => commands,
        Err(error) => return api_error_response(error),
    };
    let grants = runtime_guard
        .registry()
        .capability_grants_for_principal(&principal_id);
    let tool_decision = AuthorizationDecision::for_tool(
        principal_id.clone(),
        SmartHomeTool::Command,
        grants.iter().copied(),
        runtime.now_ms(),
    );
    let command_previews = match authorization_command_previews(
        &runtime_guard,
        &service_commands,
        &principal_id,
        &grants,
        runtime.now_ms(),
    ) {
        Ok(command_previews) => command_previews,
        Err(error) => return api_error_response(error),
    };

    WebResponse::json(
        service_authorization_preview_json(
            domain,
            service,
            &call,
            &tool_decision,
            &command_previews,
        )
        .into_bytes(),
    )
}

fn runtime_authorization_decision_response(
    runtime: &SmartHomePlatformHttpRuntime,
    request: &WebRequest,
) -> WebResponse {
    let decision_index = match route_usize(request, "decision_index") {
        Ok(decision_index) => decision_index,
        Err(error) => return api_error_response(error),
    };
    let runtime_guard = runtime
        .runtime
        .lock()
        .expect("smart-home runtime mutex should not be poisoned");
    let Some(decision) = runtime_guard
        .registry()
        .authorization_decisions()
        .nth(decision_index)
    else {
        return api_error_response(ApiError::not_found(format!(
            "authorization decision `{decision_index}` not found"
        )));
    };
    WebResponse::json(
        authorization_decision_record_json(&AuthorizationDecisionRecord {
            decision_index,
            decision,
        })
        .into_bytes(),
    )
}

fn runtime_desired_states_response(
    runtime: &SmartHomePlatformHttpRuntime,
    request: &WebRequest,
) -> WebResponse {
    let runtime_guard = runtime
        .runtime
        .lock()
        .expect("smart-home runtime mutex should not be poisoned");
    let query = match desired_state_query(&runtime_guard, request) {
        Ok(query) => query,
        Err(error) => return api_error_response(error),
    };
    let desired_states = runtime_guard.query_desired_states(&query);
    WebResponse::json(desired_states_json(&desired_states, &runtime_guard).into_bytes())
}

fn runtime_state_history_response(
    runtime: &SmartHomePlatformHttpRuntime,
    request: &WebRequest,
) -> WebResponse {
    let runtime_guard = runtime
        .runtime
        .lock()
        .expect("smart-home runtime mutex should not be poisoned");
    let events = match state_history_events(&runtime_guard, request) {
        Ok(events) => events,
        Err(error) => return api_error_response(error),
    };
    WebResponse::json(state_history_json(&events, &runtime_guard).into_bytes())
}

fn runtime_state_history_event_response(
    runtime: &SmartHomePlatformHttpRuntime,
    request: &WebRequest,
) -> WebResponse {
    let Some(event_id) = request.route_params.get("event_id") else {
        return api_error_response(ApiError::bad_request("missing event_id"));
    };
    let runtime_guard = runtime
        .runtime
        .lock()
        .expect("smart-home runtime mutex should not be poisoned");
    let Some(event) = runtime_guard
        .registry()
        .event(&EventId::trusted(event_id.clone()))
    else {
        return api_error_response(ApiError::not_found(format!(
            "state history event `{event_id}` not found"
        )));
    };
    WebResponse::json(state_history_event_json(event, &runtime_guard).into_bytes())
}

fn home_assistant_history_response(
    runtime: &SmartHomePlatformHttpRuntime,
    request: &WebRequest,
) -> WebResponse {
    let runtime_guard = runtime
        .runtime
        .lock()
        .expect("smart-home runtime mutex should not be poisoned");
    let events = match state_history_events(&runtime_guard, request) {
        Ok(events) => events,
        Err(error) => return api_error_response(error),
    };
    WebResponse::json(home_assistant_history_json(&events, &runtime_guard).into_bytes())
}

fn runtime_snapshot_json(snapshot: &RuntimeReadSnapshot) -> String {
    let pending = snapshot.pending_work_summary();
    format!(
        "{{\"generated_at_ms\":{},\"registry\":{{\"bridges\":{},\"devices\":{},\"entities\":{},\"scenes\":{},\"states\":{},\"events\":{},\"protocol_identifiers\":{},\"capability_grants\":{},\"authorization_decisions\":{}}},\"event_bus\":{{\"subscription_count\":{},\"pending_delivery_count\":{},\"published_event_count\":{},\"backlogged_subscription_count\":{},\"max_pending_delivery_count\":{}}},\"discovery\":{{\"record_count\":{},\"worker_count\":{},\"due_worker_count\":{},\"unhealthy_worker_count\":{},\"workers_with_failures\":{}}},\"supervisor\":{{\"worker_count\":{},\"restart_due_count\":{},\"unhealthy_count\":{},\"running_count\":{}}},\"desired_state\":{{\"target_count\":{},\"capability_count\":{}}},\"pairing\":{{\"session_count\":{},\"expiring_session_count\":{}}},\"optimistic_state\":{{\"target_count\":{},\"stale_target_count\":{}}},\"pending_work\":{{\"total\":{},\"event_backlog_count\":{},\"backlogged_subscription_count\":{},\"discovery_worker_due_count\":{},\"unhealthy_discovery_worker_count\":{},\"restart_due_count\":{},\"unhealthy_worker_count\":{},\"expiring_pairing_session_count\":{},\"stale_optimistic_state_count\":{},\"state_refresh_target_count\":{}}}}}",
        snapshot.generated_at_ms,
        snapshot.registry_counts.bridges,
        snapshot.registry_counts.devices,
        snapshot.registry_counts.entities,
        snapshot.registry_counts.scenes,
        snapshot.registry_counts.states,
        snapshot.registry_counts.events,
        snapshot.registry_counts.protocol_identifiers,
        snapshot.registry_counts.capability_grants,
        snapshot.registry_counts.authorization_decisions,
        snapshot.event_bus.subscription_count,
        snapshot.event_bus.pending_delivery_count,
        snapshot.event_bus.published_event_count,
        snapshot.event_bus.backlogged_subscription_count,
        snapshot.event_bus.max_pending_delivery_count,
        snapshot.discovery_record_count,
        snapshot.discovery_scheduler.worker_count,
        snapshot.discovery_scheduler.due_worker_count,
        snapshot.discovery_scheduler.unhealthy_count,
        snapshot.discovery_scheduler.workers_with_failures,
        snapshot.supervisor.worker_count,
        snapshot.supervisor.restart_due_count,
        snapshot.supervisor.unhealthy_count,
        snapshot.supervisor.running_count,
        snapshot.desired_state_count,
        snapshot.desired_capability_count,
        snapshot.pairing_session_count,
        snapshot.expiring_pairing_session_count,
        snapshot.optimistic_state_count,
        snapshot.stale_optimistic_state_count,
        pending.total_pending_work_count(),
        pending.event_backlog_count,
        pending.backlogged_subscription_count,
        pending.discovery_worker_due_count,
        pending.unhealthy_discovery_worker_count,
        pending.restart_due_count,
        pending.unhealthy_worker_count,
        pending.expiring_pairing_session_count,
        pending.stale_optimistic_state_count,
        pending.state_refresh_target_count,
    )
}

fn runtime_health_json(
    runtime: &SmartHomePlatformHttpRuntime,
    runtime_guard: &SmartHomeRuntime,
) -> String {
    let snapshot = runtime_guard.read_snapshot_at(runtime.now_ms());
    let topology = runtime_guard.topology_summary();
    let pending = snapshot.pending_work_summary();
    let stale_entities = runtime_guard
        .registry()
        .entities()
        .filter(|entity| {
            entity
                .state
                .as_ref()
                .is_some_and(|snapshot| snapshot.is_stale_at(runtime.now_ms()))
        })
        .count();
    let status = if pending.unhealthy_worker_count > 0
        || pending.unhealthy_discovery_worker_count > 0
        || topology.has_attention_items()
    {
        "degraded"
    } else if snapshot.has_pending_work()
        || topology.has_state_gaps()
        || topology.has_pairing_candidates()
    {
        "attention"
    } else {
        "ok"
    };
    let ready = snapshot.registry_counts.bridges > 0
        && snapshot.registry_counts.devices > 0
        && snapshot.registry_counts.entities > 0
        && pending.unhealthy_worker_count == 0
        && pending.unhealthy_discovery_worker_count == 0;

    format!(
        "{{\"generated_at_ms\":{},\"status\":{},\"live\":true,\"ready\":{},\"has_pending_work\":{},\"has_attention\":{},\"has_state_gaps\":{},\"has_pairing_candidates\":{},\"summary\":{{\"bridge_count\":{},\"online_bridges\":{},\"attention_bridges\":{},\"device_count\":{},\"online_devices\":{},\"attention_devices\":{},\"entity_count\":{},\"entities_with_state\":{},\"entities_without_state\":{},\"stale_entities\":{},\"desired_state_count\":{},\"pending_work_total\":{},\"event_backlog_count\":{},\"discovery_worker_due_count\":{},\"unhealthy_discovery_worker_count\":{},\"restart_due_count\":{},\"unhealthy_worker_count\":{},\"state_refresh_target_count\":{}}},\"checks\":{{\"registry\":{},\"event_bus\":{},\"discovery\":{},\"supervisor\":{},\"state\":{}}}}}",
        runtime.now_ms(),
        json_string(status),
        ready,
        snapshot.has_pending_work(),
        topology.has_attention_items(),
        topology.has_state_gaps(),
        topology.has_pairing_candidates(),
        topology.bridges,
        topology.online_bridges,
        topology.attention_bridges,
        topology.devices,
        topology.online_devices,
        topology.attention_devices,
        topology.entities,
        topology.entities_with_state,
        topology.entities_without_state,
        stale_entities,
        snapshot.desired_state_count,
        pending.total_pending_work_count(),
        pending.event_backlog_count,
        pending.discovery_worker_due_count,
        pending.unhealthy_discovery_worker_count,
        pending.restart_due_count,
        pending.unhealthy_worker_count,
        pending.state_refresh_target_count,
        json_string(if snapshot.registry_counts.entities == 0 {
            "empty"
        } else {
            "ok"
        }),
        json_string(if pending.has_event_backlog() {
            "backlogged"
        } else {
            "ok"
        }),
        json_string(if pending.unhealthy_discovery_worker_count > 0 {
            "degraded"
        } else if pending.discovery_worker_due_count > 0 {
            "attention"
        } else {
            "ok"
        }),
        json_string(if pending.unhealthy_worker_count > 0 {
            "degraded"
        } else if pending.restart_due_count > 0 {
            "attention"
        } else {
            "ok"
        }),
        json_string(if topology.has_state_gaps() {
            "attention"
        } else {
            "ok"
        }),
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RuntimeReadinessCheck {
    check_id: &'static str,
    label: &'static str,
    status: &'static str,
    route: &'static str,
    message: String,
}

fn runtime_readiness_json(
    runtime: &SmartHomePlatformHttpRuntime,
    runtime_guard: &SmartHomeRuntime,
) -> String {
    let checks = runtime_readiness_checks(runtime, runtime_guard);
    let blocking_checks = checks
        .iter()
        .filter(|check| check.status == "blocked")
        .count();
    let attention_checks = checks
        .iter()
        .filter(|check| check.status == "attention")
        .count();
    let passing_checks = checks.iter().filter(|check| check.status == "ok").count();
    let status = if blocking_checks > 0 {
        "blocked"
    } else if attention_checks > 0 {
        "attention"
    } else {
        "ready"
    };

    format!(
        "{{\"generated_at_ms\":{},\"status\":{},\"ready\":{},\"summary\":{{\"total_checks\":{},\"passing_checks\":{},\"attention_checks\":{},\"blocking_checks\":{}}},\"links\":{{\"health\":{},\"dashboard\":{},\"bootstrap\":{},\"controller_handoff\":{},\"smoke\":{},\"api\":{},\"state_gaps\":{},\"command_results\":{},\"authorization_decisions\":{},\"capability_grants\":{}}},\"checks\":[{}]}}",
        runtime.now_ms(),
        json_string(status),
        blocking_checks == 0,
        checks.len(),
        passing_checks,
        attention_checks,
        blocking_checks,
        json_string("/api/smart_home/health"),
        json_string("/api/smart_home/dashboard"),
        json_string("/api/smart_home/bootstrap"),
        json_string(CONTROLLER_HANDOFF_PATH),
        json_string("/api/smart_home/smoke"),
        json_string("/api/smart_home/api"),
        json_string("/api/smart_home/states?stale=true"),
        json_string("/api/smart_home/command_results"),
        json_string("/api/smart_home/authorization_decisions"),
        json_string("/api/smart_home/capability_grants"),
        checks
            .iter()
            .map(readiness_check_json)
            .collect::<Vec<_>>()
            .join(","),
    )
}

fn runtime_readiness_checks(
    runtime: &SmartHomePlatformHttpRuntime,
    runtime_guard: &SmartHomeRuntime,
) -> Vec<RuntimeReadinessCheck> {
    let snapshot = runtime_guard.read_snapshot_at(runtime.now_ms());
    let topology = runtime_guard.topology_summary();
    let pending = snapshot.pending_work_summary();

    vec![
        readiness_check(
            "registry",
            "Registry",
            if snapshot.registry_counts.entities == 0 {
                "blocked"
            } else {
                "ok"
            },
            "/api/smart_home/runtime",
            format!(
                "{} bridges, {} devices, and {} entities are registered",
                snapshot.registry_counts.bridges,
                snapshot.registry_counts.devices,
                snapshot.registry_counts.entities
            ),
        ),
        readiness_check(
            "topology",
            "Topology",
            if topology.bridges == 0 || topology.devices == 0 {
                "blocked"
            } else {
                "ok"
            },
            "/api/smart_home/dashboard",
            format!(
                "{} devices are assigned to rooms and {} devices are missing rooms",
                topology.devices_with_room, topology.devices_without_room
            ),
        ),
        readiness_check(
            "state_coverage",
            "State Coverage",
            if topology.has_state_gaps() {
                "attention"
            } else {
                "ok"
            },
            "/api/smart_home/states?stale=true",
            format!(
                "{} entities have state and {} entities need state refresh",
                topology.entities_with_state, topology.entities_without_state
            ),
        ),
        readiness_check(
            "event_bus",
            "Event Bus",
            if pending.has_event_backlog() {
                "attention"
            } else {
                "ok"
            },
            "/api/smart_home/events",
            format!(
                "{} events are pending delivery across {} subscriptions",
                pending.event_backlog_count, snapshot.event_bus.subscription_count
            ),
        ),
        readiness_check(
            "discovery",
            "Discovery",
            if pending.unhealthy_discovery_worker_count > 0 {
                "blocked"
            } else if pending.discovery_worker_due_count > 0 {
                "attention"
            } else {
                "ok"
            },
            "/api/smart_home/bridges",
            format!(
                "{} discovery workers are due and {} are unhealthy",
                pending.discovery_worker_due_count, pending.unhealthy_discovery_worker_count
            ),
        ),
        readiness_check(
            "supervisor",
            "Supervisor",
            if pending.unhealthy_worker_count > 0 {
                "blocked"
            } else if pending.restart_due_count > 0 {
                "attention"
            } else {
                "ok"
            },
            "/api/smart_home/health",
            format!(
                "{} workers need restart and {} workers are unhealthy",
                pending.restart_due_count, pending.unhealthy_worker_count
            ),
        ),
        readiness_check(
            "authorization",
            "Authorization",
            if snapshot.registry_counts.capability_grants == 0 {
                "attention"
            } else {
                "ok"
            },
            "/api/smart_home/authorization_decisions",
            format!(
                "{} capability grants are available for runtime-authorized commands",
                snapshot.registry_counts.capability_grants
            ),
        ),
        readiness_check(
            "desired_state",
            "Desired State",
            if snapshot.desired_state_count > 0 {
                "attention"
            } else {
                "ok"
            },
            "/api/smart_home/desired_states",
            format!(
                "{} desired-state targets are active across {} capabilities",
                snapshot.desired_state_count, snapshot.desired_capability_count
            ),
        ),
    ]
}

fn readiness_check(
    check_id: &'static str,
    label: &'static str,
    status: &'static str,
    route: &'static str,
    message: impl Into<String>,
) -> RuntimeReadinessCheck {
    RuntimeReadinessCheck {
        check_id,
        label,
        status,
        route,
        message: message.into(),
    }
}

fn readiness_check_json(check: &RuntimeReadinessCheck) -> String {
    format!(
        "{{\"check_id\":{},\"label\":{},\"status\":{},\"route\":{},\"message\":{}}}",
        json_string(check.check_id),
        json_string(check.label),
        json_string(check.status),
        json_string(check.route),
        json_string(&check.message),
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RuntimeControllerHandoffCategory {
    category_id: &'static str,
    label: &'static str,
    status: &'static str,
    ready: bool,
    evidence: Vec<&'static str>,
    message: String,
}

fn runtime_controller_handoff_json(
    runtime: &SmartHomePlatformHttpRuntime,
    runtime_guard: &SmartHomeRuntime,
) -> String {
    let categories = runtime_controller_handoff_categories(runtime, runtime_guard);
    let blocked_categories = categories
        .iter()
        .filter(|category| category.status == "blocked")
        .count();
    let attention_categories = categories
        .iter()
        .filter(|category| category.status == "attention")
        .count();
    let ready_categories = categories.iter().filter(|category| category.ready).count();
    let status = if blocked_categories > 0 {
        "blocked"
    } else if attention_categories > 0 {
        "attention"
    } else {
        "ready"
    };
    let readiness_checks = runtime_readiness_checks(runtime, runtime_guard);
    let blocking_readiness_checks = readiness_checks
        .iter()
        .filter(|check| check.status == "blocked")
        .count();
    let attention_readiness_checks = readiness_checks
        .iter()
        .filter(|check| check.status == "attention")
        .count();
    let smoke_checks = runtime_smoke_checks(runtime, runtime_guard);
    let smart_home_routes = route_catalog_surface_count("smart_home");
    let home_assistant_routes = route_catalog_surface_count("home_assistant");
    let browser_routes = route_catalog_surface_count("browser");
    let runtime_authorized_routes = API_ROUTE_CATALOG
        .iter()
        .filter(|route| route.runtime_authorized)
        .count();

    format!(
        "{{\"generated_at_ms\":{},\"version\":{},\"status\":{},\"ready\":{},\"principal_id\":{},\"summary\":{{\"total_categories\":{},\"ready_categories\":{},\"attention_categories\":{},\"blocked_categories\":{},\"route_count\":{},\"smart_home_routes\":{},\"home_assistant_routes\":{},\"browser_routes\":{},\"runtime_authorized_routes\":{},\"readiness_checks\":{},\"blocking_readiness_checks\":{},\"attention_readiness_checks\":{},\"smoke_checks\":{}}},\"links\":{{\"self\":{},\"readiness\":{},\"bootstrap\":{},\"smoke\":{},\"smoke_script\":{},\"api\":{},\"dashboard\":{}}},\"handoff\":[{}]}}",
        runtime.now_ms(),
        json_string(VERSION),
        json_string(status),
        status == "ready",
        json_string(runtime.principal_id.as_str()),
        categories.len(),
        ready_categories,
        attention_categories,
        blocked_categories,
        API_ROUTE_CATALOG.len(),
        smart_home_routes,
        home_assistant_routes,
        browser_routes,
        runtime_authorized_routes,
        readiness_checks.len(),
        blocking_readiness_checks,
        attention_readiness_checks,
        smoke_checks.len(),
        json_string(CONTROLLER_HANDOFF_PATH),
        json_string("/api/smart_home/readiness"),
        json_string("/api/smart_home/bootstrap"),
        json_string("/api/smart_home/smoke"),
        json_string("/api/smart_home/smoke_script"),
        json_string("/api/smart_home/api"),
        json_string("/"),
        categories
            .iter()
            .map(controller_handoff_category_json)
            .collect::<Vec<_>>()
            .join(","),
    )
}

fn runtime_controller_handoff_categories(
    runtime: &SmartHomePlatformHttpRuntime,
    runtime_guard: &SmartHomeRuntime,
) -> Vec<RuntimeControllerHandoffCategory> {
    let snapshot = runtime_guard.read_snapshot_at(runtime.now_ms());
    let topology = runtime_guard.topology_summary();
    let state = SmartHomePlatformHttpState::from_runtime(
        runtime_guard,
        runtime.config.clone(),
        runtime.event_types.clone(),
        runtime.now_ms(),
    );
    let services = platform_services(&state);
    let smoke_checks = runtime_smoke_checks(runtime, runtime_guard);
    let safe_get_smoke_checks = smoke_checks
        .iter()
        .filter(|check| check.method == "GET" && !check.mutates_runtime)
        .count();
    let mutating_smoke_checks = smoke_checks
        .iter()
        .filter(|check| check.mutates_runtime)
        .count();
    let runtime_authorized_mutations = API_ROUTE_CATALOG
        .iter()
        .filter(|route| route.mutates_runtime && route.runtime_authorized)
        .count();
    let repo_http_routes = vec![
        "/",
        "/api/",
        "/api/smart_home/api",
        "/api/smart_home/smoke_script",
    ];
    let dashboard_routes = vec![
        "/",
        "/dashboard",
        "/smart-home",
        "/api/smart_home/dashboard",
        "/api/smart_home/bootstrap",
    ];
    let fixture_routes = vec![
        "/api/smart_home/smoke",
        "/api/smart_home/smoke_script",
        "/api/services/:domain/:service",
    ];
    let state_routes = vec![
        "/api/smart_home/states",
        "/api/smart_home/state_history",
        "/api/smart_home/events",
    ];
    let command_routes = vec![
        "/api/smart_home/services",
        "/api/smart_home/scenes",
        "/api/services/:domain/:service",
        "/api/smart_home/command_results",
    ];
    let authorization_routes = vec![
        "/api/smart_home/capability_grants",
        "/api/smart_home/authorization_decisions",
        "/api/smart_home/command_authorization",
        "/api/smart_home/desired_state_authorization",
        "/api/smart_home/scene_authorization",
        "/api/smart_home/service_authorization/:domain/:service",
    ];
    let repo_http_ready = route_catalog_has_all(&repo_http_routes);
    let dashboard_ready = route_catalog_has_all(&dashboard_routes);
    let fixture_ready = route_catalog_has_all(&fixture_routes)
        && smoke_checks
            .iter()
            .any(|check| check.check_id == "command_probe");
    let state_ready = route_catalog_has_all(&state_routes) && snapshot.registry_counts.entities > 0;
    let command_routes_ready = route_catalog_has_all(&command_routes);
    let commands_ready = command_routes_ready && !services.is_empty();
    let authorization_routes_ready = route_catalog_has_all(&authorization_routes);
    let authorization_ready =
        authorization_routes_ready && snapshot.registry_counts.capability_grants > 0;

    vec![
        controller_handoff_category(
            "repo_http_stack",
            "Repo HTTP stack",
            if repo_http_ready { "ready" } else { "blocked" },
            repo_http_routes,
            format!(
                "{} catalog routes are available across browser, Home Assistant, and smart-home surfaces",
                API_ROUTE_CATALOG.len()
            ),
        ),
        controller_handoff_category(
            "browser_dashboard",
            "Browser dashboard",
            if dashboard_ready { "ready" } else { "blocked" },
            dashboard_routes,
            format!(
                "{} browser routes compose dashboard and bootstrap JSON over native smart-home APIs",
                route_catalog_surface_count("browser")
            ),
        ),
        controller_handoff_category(
            "fixture_controller",
            "Fixture controller",
            if fixture_ready { "ready" } else { "blocked" },
            fixture_routes,
            format!(
                "Smoke plan exposes {} checks, including {} safe GET probes and {} mutating command probe",
                smoke_checks.len(),
                safe_get_smoke_checks,
                mutating_smoke_checks
            ),
        ),
        controller_handoff_category(
            "state_history_events",
            "State, history, and events",
            if state_ready { "ready" } else { "blocked" },
            state_routes,
            format!(
                "{} entities, {} current states, {} state-history records, and {} runtime events are exposed",
                topology.entities,
                snapshot.registry_counts.states,
                snapshot.registry_counts.events,
                snapshot.event_bus.published_event_count
            ),
        ),
        controller_handoff_category(
            "commands_services_scenes",
            "Commands, services, and scenes",
            if commands_ready {
                "ready"
            } else if command_routes_ready {
                "attention"
            } else {
                "blocked"
            },
            command_routes,
            format!(
                "{} native services, {} scenes, and {} runtime-authorized mutating routes are available",
                services.len(),
                snapshot.registry_counts.scenes,
                runtime_authorized_mutations
            ),
        ),
        controller_handoff_category(
            "authorization_boundaries",
            "Runtime authorization boundaries",
            if authorization_ready {
                "ready"
            } else if authorization_routes_ready {
                "attention"
            } else {
                "blocked"
            },
            authorization_routes,
            format!(
                "{} capability grants and {} authorization decisions are inspectable for the local API principal",
                snapshot.registry_counts.capability_grants,
                snapshot.registry_counts.authorization_decisions
            ),
        ),
    ]
}

fn controller_handoff_category(
    category_id: &'static str,
    label: &'static str,
    status: &'static str,
    evidence: Vec<&'static str>,
    message: impl Into<String>,
) -> RuntimeControllerHandoffCategory {
    RuntimeControllerHandoffCategory {
        category_id,
        label,
        status,
        ready: status == "ready",
        evidence,
        message: message.into(),
    }
}

fn controller_handoff_category_json(category: &RuntimeControllerHandoffCategory) -> String {
    format!(
        "{{\"category_id\":{},\"label\":{},\"status\":{},\"ready\":{},\"evidence\":[{}],\"message\":{}}}",
        json_string(category.category_id),
        json_string(category.label),
        json_string(category.status),
        category.ready,
        json_id_array(category.evidence.iter().copied()),
        json_string(&category.message),
    )
}

fn route_catalog_surface_count(surface: &str) -> usize {
    API_ROUTE_CATALOG
        .iter()
        .filter(|route| route.surface == surface)
        .count()
}

fn route_catalog_has_all(paths: &[&str]) -> bool {
    paths.iter().all(|path| route_catalog_has_path(path))
}

fn route_catalog_has_path(path: &str) -> bool {
    API_ROUTE_CATALOG.iter().any(|route| route.path == path)
}

fn runtime_dashboard_json(
    runtime: &SmartHomePlatformHttpRuntime,
    runtime_guard: &SmartHomeRuntime,
) -> String {
    let state = SmartHomePlatformHttpState::from_runtime(
        runtime_guard,
        runtime.config.clone(),
        runtime.event_types.clone(),
        runtime.now_ms(),
    );
    let state_summary = state.summary();
    let snapshot = runtime_guard.read_snapshot_at(runtime.now_ms());
    let topology = runtime_guard.topology_summary();
    let pending = snapshot.pending_work_summary();
    let rooms = runtime_guard.query_room_summaries_at(
        &RuntimeRoomQuery::new()
            .sorted_by(RuntimeRoomSort::AttentionDesc)
            .with_limit(50),
        runtime.now_ms(),
    );
    let mut bridges = runtime_guard.registry().bridges().collect::<Vec<_>>();
    let mut devices = runtime_guard.registry().devices().collect::<Vec<_>>();
    let mut entities = runtime_guard.registry().entities().collect::<Vec<_>>();
    let capabilities = runtime_capability_catalog(runtime_guard, &CapabilityCatalogQuery::new(100));
    let desired_query = DesiredStateQuery::new().with_limit(50);
    let desired_states = runtime_guard.query_desired_states(&desired_query);
    let event_query = RuntimeEventQuery::new();
    let event_summary = runtime_guard.event_bus().event_log_summary(&event_query);
    let command_query = RuntimeCommandResultQuery::new()
        .sorted_by(RuntimeCommandResultSort::SequenceDesc)
        .with_limit(50);
    let command_summary = runtime_guard.command_result_summary(&command_query);
    let authorization_query = RuntimeAuthorizationDecisionQuery::new().with_limit(50);
    let authorization_summary = runtime_guard.authorization_decision_summary(&authorization_query);

    bridges.sort_by(|left, right| left.bridge_id.as_str().cmp(right.bridge_id.as_str()));
    devices.sort_by(|left, right| left.device_id.as_str().cmp(right.device_id.as_str()));
    entities.sort_by(|left, right| left.entity_id.as_str().cmp(right.entity_id.as_str()));

    format!(
        "{{\"generated_at_ms\":{},\"config\":{},\"summary\":{{\"state_count\":{},\"known_state_count\":{},\"unknown_state_count\":{},\"stale_state_count\":{},\"optimistic_state_count\":{},\"service_count\":{},\"event_type_count\":{},\"bridge_count\":{},\"device_count\":{},\"entity_count\":{},\"room_count\":{},\"scene_count\":{},\"desired_state_count\":{},\"pending_work_total\":{},\"has_attention\":{},\"has_state_gaps\":{},\"has_pairing_candidates\":{}}},\"health\":{},\"runtime\":{},\"topology\":{{\"bridges\":{},\"devices\":{},\"entities\":{},\"scenes\":{},\"online_bridges\":{},\"attention_bridges\":{},\"online_devices\":{},\"attention_devices\":{},\"devices_with_room\":{},\"devices_without_room\":{},\"unique_rooms\":{},\"entities_with_state\":{},\"entities_without_state\":{},\"total_capabilities\":{},\"scene_actions\":{}}},\"bridges\":{},\"devices\":{},\"entities\":{},\"capabilities\":{},\"rooms\":{},\"desired_states\":{},\"events\":{{\"summary\":{}}},\"command_results\":{{\"summary\":{}}},\"authorization_decisions\":{{\"summary\":{}}}}}",
        runtime.now_ms(),
        config_json(&state),
        state_summary.state_count,
        state_summary.known_state_count,
        state_summary.unknown_state_count,
        state_summary.stale_state_count,
        state_summary.optimistic_state_count,
        state_summary.service_count,
        state_summary.event_type_count,
        topology.bridges,
        topology.devices,
        topology.entities,
        topology.unique_rooms,
        topology.scenes,
        snapshot.desired_state_count,
        pending.total_pending_work_count(),
        topology.has_attention_items(),
        pending.state_refresh_target_count > 0,
        topology.has_pairing_candidates(),
        runtime_health_json(runtime, runtime_guard),
        runtime_snapshot_json(&snapshot),
        topology.bridges,
        topology.devices,
        topology.entities,
        topology.scenes,
        topology.online_bridges,
        topology.attention_bridges,
        topology.online_devices,
        topology.attention_devices,
        topology.devices_with_room,
        topology.devices_without_room,
        topology.unique_rooms,
        topology.entities_with_state,
        topology.entities_without_state,
        topology.total_capabilities,
        topology.scene_actions,
        bridges_registry_json(&bridges, runtime_guard, runtime.now_ms()),
        devices_registry_json(&devices, runtime_guard, runtime.now_ms()),
        entities_registry_json(&entities, runtime_guard, runtime.now_ms()),
        capabilities_catalog_json(&capabilities),
        rooms_json(&rooms, runtime_guard),
        desired_states_json(&desired_states, runtime_guard),
        runtime_event_summary_json(&event_summary),
        command_result_summary_json(&command_summary),
        authorization_decision_summary_json(&authorization_summary),
    )
}

fn runtime_bootstrap_json(
    runtime: &SmartHomePlatformHttpRuntime,
    runtime_guard: &SmartHomeRuntime,
) -> String {
    let routes = API_ROUTE_CATALOG.iter().collect::<Vec<_>>();
    let state_gaps = runtime_state_gap_entities(runtime_guard, runtime.now_ms(), 25);
    let event_query = RuntimeEventQuery::new()
        .sorted_by(RuntimeEventSort::SequenceDesc)
        .with_limit(25);
    let event_summary = runtime_guard.event_bus().event_log_summary(&event_query);
    let command_query = RuntimeCommandResultQuery::new()
        .sorted_by(RuntimeCommandResultSort::SequenceDesc)
        .with_limit(25);
    let command_summary = runtime_guard.command_result_summary(&command_query);
    let authorization_query = RuntimeAuthorizationDecisionQuery::new().with_limit(25);
    let authorization_summary = runtime_guard.authorization_decision_summary(&authorization_query);

    format!(
        "{{\"generated_at_ms\":{},\"version\":{},\"links\":{{\"readiness\":{},\"controller_handoff\":{},\"dashboard\":{},\"smoke\":{},\"smoke_script\":{},\"api\":{},\"states\":{},\"state_history\":{},\"command_results\":{},\"authorization_decisions\":{},\"command_authorization\":{},\"desired_state_authorization\":{},\"scene_authorization\":{},\"service_authorization\":{},\"capability_grants\":{}}},\"health\":{},\"dashboard\":{},\"api\":{},\"state_gaps\":{},\"recent_activity\":{{\"events\":{{\"summary\":{}}},\"command_results\":{{\"summary\":{}}},\"authorization_decisions\":{{\"summary\":{}}}}}}}",
        runtime.now_ms(),
        json_string(VERSION),
        json_string("/api/smart_home/readiness"),
        json_string(CONTROLLER_HANDOFF_PATH),
        json_string("/api/smart_home/dashboard"),
        json_string("/api/smart_home/smoke"),
        json_string("/api/smart_home/smoke_script"),
        json_string("/api/smart_home/api"),
        json_string("/api/smart_home/states"),
        json_string("/api/smart_home/state_history"),
        json_string("/api/smart_home/command_results"),
        json_string("/api/smart_home/authorization_decisions"),
        json_string("/api/smart_home/command_authorization"),
        json_string("/api/smart_home/desired_state_authorization"),
        json_string("/api/smart_home/scene_authorization"),
        json_string("/api/smart_home/service_authorization"),
        json_string("/api/smart_home/capability_grants"),
        runtime_health_json(runtime, runtime_guard),
        runtime_dashboard_json(runtime, runtime_guard),
        api_catalog_json(&routes),
        states_registry_json(&state_gaps, runtime_guard, runtime.now_ms()),
        runtime_event_summary_json(&event_summary),
        command_result_summary_json(&command_summary),
        authorization_decision_summary_json(&authorization_summary),
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RuntimeSmokeCheck {
    check_id: &'static str,
    label: &'static str,
    method: &'static str,
    path: String,
    category: &'static str,
    mutates_runtime: bool,
    runtime_authorized: bool,
    expected_status: u16,
    request_body: Option<String>,
    expected: String,
}

fn runtime_smoke_json(
    runtime: &SmartHomePlatformHttpRuntime,
    runtime_guard: &SmartHomeRuntime,
) -> String {
    let readiness_checks = runtime_readiness_checks(runtime, runtime_guard);
    let blocking_checks = readiness_checks
        .iter()
        .filter(|check| check.status == "blocked")
        .count();
    let attention_checks = readiness_checks
        .iter()
        .filter(|check| check.status == "attention")
        .count();
    let status = if blocking_checks > 0 {
        "blocked"
    } else if attention_checks > 0 {
        "attention"
    } else {
        "ready"
    };
    let checks = runtime_smoke_checks(runtime, runtime_guard);
    let mutating_checks = checks.iter().filter(|check| check.mutates_runtime).count();
    let runtime_authorized_checks = checks
        .iter()
        .filter(|check| check.runtime_authorized)
        .count();
    let safe_get_checks = checks
        .iter()
        .filter(|check| check.method == "GET" && !check.mutates_runtime)
        .count();

    format!(
        "{{\"generated_at_ms\":{},\"version\":{},\"status\":{},\"ready\":{},\"principal_id\":{},\"summary\":{{\"total_checks\":{},\"safe_get_checks\":{},\"mutating_checks\":{},\"runtime_authorized_checks\":{},\"blocking_readiness_checks\":{},\"attention_readiness_checks\":{}}},\"links\":{{\"self\":{},\"script\":{},\"dashboard\":{},\"readiness\":{},\"controller_handoff\":{},\"bootstrap\":{},\"api\":{},\"command_results\":{},\"authorization_decisions\":{},\"command_authorization\":{},\"desired_state_authorization\":{},\"scene_authorization\":{},\"service_authorization\":{},\"capability_grants\":{}}},\"checks\":[{}]}}",
        runtime.now_ms(),
        json_string(VERSION),
        json_string(status),
        blocking_checks == 0,
        json_string(runtime.principal_id.as_str()),
        checks.len(),
        safe_get_checks,
        mutating_checks,
        runtime_authorized_checks,
        blocking_checks,
        attention_checks,
        json_string("/api/smart_home/smoke"),
        json_string("/api/smart_home/smoke_script"),
        json_string("/"),
        json_string("/api/smart_home/readiness"),
        json_string(CONTROLLER_HANDOFF_PATH),
        json_string("/api/smart_home/bootstrap"),
        json_string("/api/smart_home/api"),
        json_string("/api/smart_home/command_results"),
        json_string("/api/smart_home/authorization_decisions"),
        json_string("/api/smart_home/command_authorization"),
        json_string("/api/smart_home/desired_state_authorization"),
        json_string("/api/smart_home/scene_authorization"),
        json_string("/api/smart_home/service_authorization"),
        json_string("/api/smart_home/capability_grants"),
        checks
            .iter()
            .map(runtime_smoke_check_json)
            .collect::<Vec<_>>()
            .join(","),
    )
}

fn runtime_smoke_script(
    runtime: &SmartHomePlatformHttpRuntime,
    runtime_guard: &SmartHomeRuntime,
    request: &WebRequest,
) -> String {
    let base_url = runtime_smoke_base_url(request);
    let checks = runtime_smoke_checks(runtime, runtime_guard);
    let mut script = String::new();

    script.push_str("#!/usr/bin/env sh\n");
    script.push_str("set -eu\n\n");
    script.push_str("# Generated by GET /api/smart_home/smoke_script from the live smoke plan.\n");
    script.push_str(&format!(
        "BASE_URL=${{SMART_HOME_BASE_URL:-{}}}\n",
        shell_single_quote(&base_url)
    ));
    script.push_str("CURL=${CURL:-curl}\n\n");
    script.push_str("run_check() {\n");
    script.push_str("  label=$1\n");
    script.push_str("  method=$2\n");
    script.push_str("  path=$3\n");
    script.push_str("  expected_status=$4\n");
    script.push_str("  body=${5-}\n");
    script.push_str("  url=\"${BASE_URL}${path}\"\n");
    script.push_str("  printf '%s %s ... ' \"$method\" \"$path\"\n");
    script.push_str("  if [ -n \"$body\" ]; then\n");
    script.push_str(
        "    status=$(\"$CURL\" -sS -o /dev/null -w '%{http_code}' -X \"$method\" -H 'Content-Type: application/json' -d \"$body\" \"$url\")\n",
    );
    script.push_str("  else\n");
    script.push_str(
        "    status=$(\"$CURL\" -sS -o /dev/null -w '%{http_code}' -X \"$method\" \"$url\")\n",
    );
    script.push_str("  fi\n");
    script.push_str("  if [ \"$status\" != \"$expected_status\" ]; then\n");
    script.push_str(
        "    printf 'expected %s, got %s for %s\\n' \"$expected_status\" \"$status\" \"$label\" >&2\n",
    );
    script.push_str("    exit 1\n");
    script.push_str("  fi\n");
    script.push_str("  printf 'ok\\n'\n");
    script.push_str("}\n");

    for check in &checks {
        script.push('\n');
        script.push_str(&format!("# {}\n", check.expected));
        script.push_str(&format!(
            "run_check {} {} {} {}",
            shell_single_quote(check.label),
            shell_single_quote(check.method),
            shell_single_quote(&check.path),
            shell_single_quote(&check.expected_status.to_string())
        ));
        if let Some(request_body) = &check.request_body {
            script.push(' ');
            script.push_str(&shell_single_quote(request_body));
        }
        script.push('\n');
    }

    script.push_str(&format!(
        "\nprintf 'All smart-home smoke checks passed ({} checks)\\n'\n",
        checks.len()
    ));
    script
}

fn runtime_smoke_base_url(request: &WebRequest) -> String {
    let host_with_port = request
        .header("host")
        .map(str::trim)
        .filter(|host| !host.is_empty())
        .filter(|host| host.contains(':') || host.starts_with('['));
    let authority = host_with_port
        .map(str::to_string)
        .unwrap_or_else(|| request.http.connection.local_addr.to_string());

    if authority.starts_with("http://") || authority.starts_with("https://") {
        authority
    } else {
        format!("http://{authority}")
    }
}

fn runtime_smoke_checks(
    runtime: &SmartHomePlatformHttpRuntime,
    runtime_guard: &SmartHomeRuntime,
) -> Vec<RuntimeSmokeCheck> {
    let state = SmartHomePlatformHttpState::from_runtime(
        runtime_guard,
        runtime.config.clone(),
        runtime.event_types.clone(),
        runtime.now_ms(),
    );
    let mut checks = vec![
        runtime_smoke_check(
            "dashboard_shell",
            "Dashboard shell",
            "GET",
            "/",
            "browser",
            false,
            false,
            200,
            None,
            "Embedded dashboard HTML is served by the repo HTTP stack.",
        ),
        runtime_smoke_check(
            "bootstrap",
            "Startup bundle",
            "GET",
            "/api/smart_home/bootstrap",
            "dashboard",
            false,
            false,
            200,
            None,
            "Bootstrap JSON includes startup links, API discovery, state gaps, and audit summaries.",
        ),
        runtime_smoke_check(
            "readiness",
            "Readiness",
            "GET",
            "/api/smart_home/readiness",
            "health",
            false,
            false,
            200,
            None,
            "Readiness JSON reports no blocking checks before local-controller control probes run.",
        ),
        runtime_smoke_check(
            "state_registry",
            "State registry",
            "GET",
            "/api/smart_home/states?limit=24",
            "states",
            false,
            false,
            200,
            None,
            "Native state registry returns dashboard-ready entity records and Home Assistant aliases.",
        ),
        runtime_smoke_check(
            "service_catalog",
            "Service catalog",
            "GET",
            "/api/smart_home/services?domain=light",
            "services",
            false,
            false,
            200,
            None,
            "Native service catalog exposes commandable light services and target aliases.",
        ),
        runtime_smoke_check(
            "authorized_routes",
            "Authorized routes",
            "GET",
            "/api/smart_home/api?mutating=true&authorized=true",
            "api_catalog",
            false,
            false,
            200,
            None,
            "API catalog lists mutating routes that still dispatch through runtime authorization.",
        ),
    ];
    checks.push(runtime_smoke_command_authorization_probe(&state));
    checks.push(runtime_smoke_desired_state_authorization_probe(&state));
    checks.push(runtime_smoke_scene_authorization_probe(&state));
    checks.push(runtime_smoke_service_authorization_probe(&state));
    checks.push(runtime_smoke_command_probe(&state));
    checks.extend([
        runtime_smoke_check(
            "controller_handoff",
            "Controller handoff",
            "GET",
            CONTROLLER_HANDOFF_PATH,
            "handoff",
            false,
            false,
            200,
            None,
            "Controller handoff manifest summarizes platform readiness for local-controller and Chief adapter work.",
        ),
        runtime_smoke_check(
            "command_audit",
            "Command audit",
            "GET",
            "/api/smart_home/command_results?limit=10",
            "command_results",
            false,
            false,
            200,
            None,
            "Command-result audit route can confirm accepted command outcomes after the POST probe.",
        ),
        runtime_smoke_check(
            "authorization_audit",
            "Authorization audit",
            "GET",
            "/api/smart_home/authorization_decisions?limit=10",
            "authorization",
            false,
            false,
            200,
            None,
            "Authorization audit route can confirm the local API principal's runtime decision.",
        ),
        runtime_smoke_check(
            "state_history",
            "State history",
            "GET",
            "/api/smart_home/state_history?limit=12",
            "state_history",
            false,
            false,
            200,
            None,
            "State-history route exposes registry-backed events for dashboard drill-downs.",
        ),
    ]);
    checks
}

fn runtime_smoke_command_authorization_probe(
    state: &SmartHomePlatformHttpState,
) -> RuntimeSmokeCheck {
    let Some(target) = smoke_light_target(state) else {
        return runtime_smoke_check(
            "command_authorization_preview",
            "Command authorization preview",
            "GET",
            "/api/smart_home/services?domain=light",
            "authorization",
            false,
            false,
            200,
            None,
            "No commandable light target is available; inspect the service catalog before previewing command authorization.",
        );
    };

    runtime_smoke_check(
        "command_authorization_preview",
        "Command authorization preview",
        "GET",
        format!(
            "/api/smart_home/command_authorization?entity_id={}&command_type=turn_on",
            url_component(&target)
        ),
        "authorization",
        false,
        false,
        200,
        None,
        "Previews the local API principal's runtime grants for a light command without dispatching it.",
    )
}

fn runtime_smoke_desired_state_authorization_probe(
    state: &SmartHomePlatformHttpState,
) -> RuntimeSmokeCheck {
    let Some(target) = smoke_light_target(state) else {
        return runtime_smoke_check(
            "desired_state_authorization_preview",
            "Desired-state authorization preview",
            "GET",
            "/api/smart_home/services?domain=light",
            "authorization",
            false,
            false,
            200,
            None,
            "No commandable light target is available; inspect the service catalog before previewing desired-state authorization.",
        );
    };

    runtime_smoke_check(
        "desired_state_authorization_preview",
        "Desired-state authorization preview",
        "GET",
        format!(
            "/api/smart_home/desired_state_authorization?entity_id={}&operation=set",
            url_component(&target)
        ),
        "authorization",
        false,
        false,
        200,
        None,
        "Previews the local API principal's runtime grants for desired-state set/clear tools without mutating targets.",
    )
}

fn runtime_smoke_scene_authorization_probe(
    state: &SmartHomePlatformHttpState,
) -> RuntimeSmokeCheck {
    let Some(target) = smoke_scene_target(state) else {
        return runtime_smoke_check(
            "scene_authorization_preview",
            "Scene authorization preview",
            "GET",
            "/api/smart_home/scenes",
            "authorization",
            false,
            false,
            200,
            None,
            "No scene target is available; inspect the scene catalog before previewing scene authorization.",
        );
    };

    runtime_smoke_check(
        "scene_authorization_preview",
        "Scene authorization preview",
        "GET",
        format!(
            "/api/smart_home/scene_authorization?scene_id={}",
            url_component(&target)
        ),
        "authorization",
        false,
        false,
        200,
        None,
        "Previews the local API principal's runtime grants for every command a scene activation would dispatch.",
    )
}

fn runtime_smoke_service_authorization_probe(
    state: &SmartHomePlatformHttpState,
) -> RuntimeSmokeCheck {
    let Some(target) = smoke_light_target(state) else {
        return runtime_smoke_check(
            "service_authorization_preview",
            "Service authorization preview",
            "GET",
            "/api/smart_home/services?domain=light",
            "authorization",
            false,
            false,
            200,
            None,
            "No commandable light target is available; inspect the service catalog before previewing service authorization.",
        );
    };

    runtime_smoke_check(
        "service_authorization_preview",
        "Service authorization preview",
        "GET",
        format!(
            "/api/smart_home/service_authorization/light/turn_on?entity_id={}&brightness_pct=75",
            url_component(&target)
        ),
        "authorization",
        false,
        false,
        200,
        None,
        "Previews the local API principal's runtime grants for the exact commands a Home Assistant service call would dispatch.",
    )
}

fn runtime_smoke_command_probe(state: &SmartHomePlatformHttpState) -> RuntimeSmokeCheck {
    let Some(target) = smoke_light_target(state) else {
        return runtime_smoke_check(
            "command_probe",
            "Command probe",
            "GET",
            "/api/smart_home/services?domain=light",
            "services",
            false,
            false,
            200,
            None,
            "No commandable light target is available; inspect the service catalog before running a mutating smoke probe.",
        );
    };

    runtime_smoke_check(
        "command_probe",
        "Command probe",
        "POST",
        "/api/services/light/turn_on",
        "commands",
        true,
        true,
        200,
        Some(format!(
            "{{\"entity_id\":{},\"brightness_pct\":75}}",
            json_string(target)
        )),
        "Runs the Home Assistant-compatible light command through runtime authorization and records command/authorization audit rows.",
    )
}

fn smoke_light_target(state: &SmartHomePlatformHttpState) -> Option<String> {
    platform_services(state)
        .into_iter()
        .find(|service| service.domain == "light" && service.service == "turn_on")
        .and_then(|service| service.target_entity_ids.into_iter().next())
        .map(|entity_id| {
            state
                .entities
                .iter()
                .find(|entity| entity.entity_id.as_str() == entity_id)
                .map(home_assistant_entity_id)
                .unwrap_or(entity_id)
        })
}

fn smoke_scene_target(state: &SmartHomePlatformHttpState) -> Option<String> {
    state.scenes.first().map(home_assistant_scene_id)
}

// Each argument is a distinct field of a runtime smoke-check descriptor (id, label,
// HTTP method/path, category, mutation/authorization flags, expected status); grouping
// them into a struct would not improve clarity for this internal helper.
#[allow(clippy::too_many_arguments)]
fn runtime_smoke_check(
    check_id: &'static str,
    label: &'static str,
    method: &'static str,
    path: impl Into<String>,
    category: &'static str,
    mutates_runtime: bool,
    runtime_authorized: bool,
    expected_status: u16,
    request_body: Option<String>,
    expected: impl Into<String>,
) -> RuntimeSmokeCheck {
    RuntimeSmokeCheck {
        check_id,
        label,
        method,
        path: path.into(),
        category,
        mutates_runtime,
        runtime_authorized,
        expected_status,
        request_body,
        expected: expected.into(),
    }
}

fn runtime_smoke_check_json(check: &RuntimeSmokeCheck) -> String {
    format!(
        "{{\"check_id\":{},\"label\":{},\"method\":{},\"path\":{},\"category\":{},\"mutates_runtime\":{},\"runtime_authorized\":{},\"expected_status\":{},\"request_body\":{},\"expected\":{}}}",
        json_string(check.check_id),
        json_string(check.label),
        json_string(check.method),
        json_string(&check.path),
        json_string(check.category),
        check.mutates_runtime,
        check.runtime_authorized,
        check.expected_status,
        check
            .request_body
            .as_deref()
            .unwrap_or("null"),
        json_string(&check.expected),
    )
}

fn runtime_event_log_json(
    entries: &[RuntimeEventLogEntry<'_>],
    summary: &smart_home_runtime::RuntimeEventLogSummary,
) -> String {
    format!(
        "{{\"summary\":{},\"events\":[{}]}}",
        runtime_event_summary_json(summary),
        entries
            .iter()
            .map(|entry| runtime_event_entry_json(entry))
            .collect::<Vec<_>>()
            .join(","),
    )
}

fn runtime_event_summary_json(summary: &smart_home_runtime::RuntimeEventLogSummary) -> String {
    format!(
        "{{\"total_events\":{},\"device_events\":{},\"command_results\":{},\"bridge_health_events\":{},\"state_expired_events\":{},\"desired_state_drift_events\":{},\"worker_restart_events\":{},\"first_sequence\":{},\"latest_sequence\":{},\"next_sequence\":{}}}",
        summary.total_events,
        summary.device_events,
        summary.command_results,
        summary.bridge_health_events,
        summary.state_expired_events,
        summary.desired_state_drift_events,
        summary.worker_restart_events,
        optional_u64_json(summary.first_sequence),
        optional_u64_json(summary.latest_sequence),
        summary.next_checkpoint.next_sequence(),
    )
}

fn runtime_event_entry_json(entry: &RuntimeEventLogEntry<'_>) -> String {
    format!(
        "{{\"sequence\":{},\"next_sequence\":{},\"links\":{},\"event\":{}}}",
        entry.sequence,
        entry.next_checkpoint.next_sequence(),
        runtime_event_links_json(entry),
        runtime_event_json(entry.event),
    )
}

fn runtime_event_links_json(entry: &RuntimeEventLogEntry<'_>) -> String {
    let event = entry.event;
    let (entity, bridge, state_history_event, command_result, correlation_commands) = match event {
        RuntimeEvent::Device(event) => (
            event.entity_id.as_ref().map(audit_entity_links_json),
            Some(format!(
                "/api/smart_home/bridges/{}",
                url_component(event.bridge_id.as_str())
            )),
            Some(format!(
                "/api/smart_home/state_history/{}",
                url_component(event.event_id.as_str())
            )),
            None,
            event.correlation_id.as_ref().map(|correlation_id| {
                format!(
                    "/api/smart_home/command_results?correlation_id={}",
                    correlation_id.as_str()
                )
            }),
        ),
        RuntimeEvent::CommandResult(result) => (
            None,
            Some(format!(
                "/api/smart_home/bridges/{}",
                url_component(result.bridge_id.as_str())
            )),
            None,
            Some(format!(
                "/api/smart_home/command_results/{}",
                result.command_id.as_str()
            )),
            Some(format!(
                "/api/smart_home/command_results?correlation_id={}",
                result.correlation_id.as_str()
            )),
        ),
        RuntimeEvent::BridgeHealth { bridge_id, .. } => (
            None,
            Some(format!(
                "/api/smart_home/bridges/{}",
                url_component(bridge_id.as_str())
            )),
            None,
            None,
            None,
        ),
        RuntimeEvent::StateExpired { entity_id, .. } => (
            Some(audit_entity_links_json(entity_id)),
            None,
            None,
            None,
            None,
        ),
        RuntimeEvent::DesiredStateDrift {
            bridge_id,
            entity_id,
            ..
        } => (
            Some(audit_entity_links_json(entity_id)),
            Some(format!(
                "/api/smart_home/bridges/{}",
                url_component(bridge_id.as_str())
            )),
            None,
            None,
            None,
        ),
        RuntimeEvent::WorkerNeedsRestart { bridge_id, .. } => (
            None,
            Some(format!(
                "/api/smart_home/bridges/{}",
                url_component(bridge_id.as_str())
            )),
            None,
            None,
            None,
        ),
    };

    format!(
        "{{\"self\":{},\"event_window\":{},\"entity\":{},\"bridge\":{},\"state_history_event\":{},\"command_result\":{},\"command_results_by_correlation\":{}}}",
        json_string(format!("/api/smart_home/events/{}", entry.sequence)),
        json_string(format!(
            "/api/smart_home/events?from_sequence={}&to_sequence={}",
            entry.sequence, entry.sequence
        )),
        entity.unwrap_or_else(|| "null".to_string()),
        optional_link_json(bridge),
        optional_link_json(state_history_event),
        optional_link_json(command_result),
        optional_link_json(correlation_commands),
    )
}

fn runtime_event_json(event: &RuntimeEvent) -> String {
    match event {
        RuntimeEvent::Device(event) => device_event_json(event),
        RuntimeEvent::CommandResult(result) => format!(
            "{{\"kind\":\"command_result\",\"result\":{}}}",
            command_result_json(result)
        ),
        RuntimeEvent::BridgeHealth {
            event_id,
            bridge_id,
            health,
            observed_at_ms,
            received_at_ms,
        } => format!(
            "{{\"kind\":\"bridge_health\",\"event_id\":{},\"bridge_id\":{},\"health\":{},\"observed_at_ms\":{},\"received_at_ms\":{}}}",
            json_string(event_id.as_str()),
            json_string(bridge_id.as_str()),
            json_string(format!("{health:?}").to_ascii_lowercase()),
            observed_at_ms,
            received_at_ms,
        ),
        RuntimeEvent::StateExpired {
            entity_id,
            expired_at_ms,
        } => format!(
            "{{\"kind\":\"state_expired\",\"entity_id\":{},\"expired_at_ms\":{}}}",
            json_string(entity_id.as_str()),
            expired_at_ms,
        ),
        RuntimeEvent::DesiredStateDrift {
            bridge_id,
            entity_id,
            capability_id,
            reason,
            detected_at_ms,
        } => format!(
            "{{\"kind\":\"desired_state_drift\",\"bridge_id\":{},\"entity_id\":{},\"capability_id\":{},\"reason\":{},\"detected_at_ms\":{}}}",
            json_string(bridge_id.as_str()),
            json_string(entity_id.as_str()),
            json_string(capability_id.as_str()),
            json_string(format!("{reason:?}").to_ascii_lowercase()),
            detected_at_ms,
        ),
        RuntimeEvent::WorkerNeedsRestart {
            bridge_id,
            integration_id,
            overdue_at_ms,
        } => format!(
            "{{\"kind\":\"worker_needs_restart\",\"bridge_id\":{},\"integration_id\":{},\"overdue_at_ms\":{}}}",
            json_string(bridge_id.as_str()),
            json_string(integration_id.as_str()),
            overdue_at_ms,
        ),
    }
}

fn device_event_json(event: &DeviceEvent) -> String {
    format!(
        "{{\"kind\":\"device_event\",\"event_id\":{},\"bridge_id\":{},\"device_id\":{},\"entity_id\":{},\"event_type\":{},\"observed_at_ms\":{},\"received_at_ms\":{},\"state_delta\":{},\"raw_ref\":{},\"correlation_id\":{}}}",
        json_string(event.event_id.as_str()),
        json_string(event.bridge_id.as_str()),
        event
            .device_id
            .as_ref()
            .map(|device_id| json_string(device_id.as_str()))
            .unwrap_or_else(|| "null".to_string()),
        event
            .entity_id
            .as_ref()
            .map(|entity_id| json_string(entity_id.as_str()))
            .unwrap_or_else(|| "null".to_string()),
        json_string(device_event_type_label(event.event_type)),
        event.observed_at_ms,
        event.received_at_ms,
        event
            .state_delta
            .as_ref()
            .map(state_delta_json)
            .unwrap_or_else(|| "null".to_string()),
        event
            .raw_ref
            .as_ref()
            .map(json_string)
            .unwrap_or_else(|| "null".to_string()),
        event
            .correlation_id
            .as_ref()
            .map(|correlation_id| json_string(correlation_id.as_str()))
            .unwrap_or_else(|| "null".to_string()),
    )
}

fn command_results_audit_json(
    records: &[RuntimeCommandResultRecord],
    summary: &smart_home_runtime::RuntimeCommandResultSummary,
) -> String {
    format!(
        "{{\"summary\":{},\"results\":[{}]}}",
        command_result_summary_json(summary),
        records
            .iter()
            .map(command_result_record_json)
            .collect::<Vec<_>>()
            .join(",")
    )
}

fn command_result_record_json(record: &RuntimeCommandResultRecord) -> String {
    format!(
        "{{\"sequence\":{},\"next_sequence\":{},\"links\":{},\"result\":{}}}",
        record.sequence,
        record.next_checkpoint.next_sequence(),
        command_result_record_links_json(record),
        command_result_json(&record.result),
    )
}

fn command_result_record_links_json(record: &RuntimeCommandResultRecord) -> String {
    format!(
        "{{\"self\":{},\"event\":{},\"event_window\":{},\"command_results_by_correlation\":{},\"command_results_by_bridge\":{}}}",
        json_string(format!(
            "/api/smart_home/command_results/{}",
            record.result.command_id.as_str()
        )),
        json_string(format!("/api/smart_home/events/{}", record.sequence)),
        json_string(format!(
            "/api/smart_home/events?from_sequence={}&to_sequence={}",
            record.sequence, record.sequence
        )),
        json_string(format!(
            "/api/smart_home/command_results?correlation_id={}",
            record.result.correlation_id.as_str()
        )),
        json_string(format!(
            "/api/smart_home/command_results?bridge_id={}",
            url_component(record.result.bridge_id.as_str())
        )),
    )
}

fn command_result_summary_json(
    summary: &smart_home_runtime::RuntimeCommandResultSummary,
) -> String {
    format!(
        "{{\"total_results\":{},\"accepted_results\":{},\"rejected_results\":{},\"timed_out_results\":{},\"failed_results\":{},\"first_sequence\":{},\"latest_sequence\":{},\"next_sequence\":{}}}",
        summary.total_results,
        summary.accepted_results,
        summary.rejected_results,
        summary.timed_out_results,
        summary.failed_results,
        optional_u64_json(summary.first_sequence),
        optional_u64_json(summary.latest_sequence),
        summary.next_checkpoint.next_sequence(),
    )
}

fn capability_grants_json(
    grants: &[&CapabilityGrant],
    summary: &CapabilityGrantInventorySummary,
    now_ms: u64,
) -> String {
    format!(
        "{{\"summary\":{},\"grants\":[{}]}}",
        capability_grant_summary_json(summary),
        grants
            .iter()
            .map(|grant| capability_grant_json(grant, now_ms))
            .collect::<Vec<_>>()
            .join(",")
    )
}

fn capability_grant_summary_json(summary: &CapabilityGrantInventorySummary) -> String {
    format!(
        "{{\"generated_at_ms\":{},\"total_grants\":{},\"active_grants\":{},\"pending_grants\":{},\"revoked_grants\":{},\"expired_grants\":{},\"tool_grants\":{},\"capability_grants\":{},\"entity_capability_grants\":{},\"all_smart_home_grants\":{},\"read_only_tier_grants\":{},\"low_risk_tier_grants\":{},\"human_approval_tier_grants\":{},\"high_risk_tier_grants\":{},\"expiring_grants\":{},\"unique_principals\":{}}}",
        summary.generated_at_ms,
        summary.total_grants,
        summary.active_grants,
        summary.pending_grants,
        summary.revoked_grants,
        summary.expired_grants,
        summary.tool_grants,
        summary.capability_grants,
        summary.entity_capability_grants,
        summary.all_smart_home_grants,
        summary.read_only_tier_grants,
        summary.low_risk_tier_grants,
        summary.human_approval_tier_grants,
        summary.high_risk_tier_grants,
        summary.expiring_grants,
        summary.unique_principals,
    )
}

fn capability_grant_json(grant: &CapabilityGrant, now_ms: u64) -> String {
    format!(
        "{{\"grant_id\":{},\"principal_id\":{},\"scope\":{},\"max_tier\":{},\"configured_status\":{},\"effective_status\":{},\"active\":{},\"granted_by\":{},\"granted_at_ms\":{},\"expires_at_ms\":{},\"metadata\":[{}]}}",
        json_string(grant.grant_id.as_str()),
        json_string(grant.principal_id.as_str()),
        capability_grant_scope_json(&grant.scope),
        json_string(privilege_tier_label(grant.max_tier)),
        json_string(capability_grant_status_label(grant.status)),
        json_string(capability_grant_status_label(grant.status_at(now_ms))),
        grant.is_active_at(now_ms),
        json_string(&grant.granted_by),
        grant.granted_at_ms,
        optional_u64_json(grant.expires_at_ms),
        metadata_json(&grant.metadata),
    )
}

fn capability_grant_scope_json(scope: &CapabilityGrantScope) -> String {
    match scope {
        CapabilityGrantScope::Tool(tool) => {
            format!(
                "{{\"kind\":\"tool\",\"tool_id\":{}}}",
                json_string(tool.descriptor().tool_id)
            )
        }
        CapabilityGrantScope::Capability(capability_id) => format!(
            "{{\"kind\":\"capability\",\"capability_id\":{}}}",
            json_string(capability_id.as_str()),
        ),
        CapabilityGrantScope::EntityCapability {
            entity_id,
            capability_id,
        } => format!(
            "{{\"kind\":\"entity_capability\",\"entity_id\":{},\"capability_id\":{}}}",
            json_string(entity_id.as_str()),
            json_string(capability_id.as_str()),
        ),
        CapabilityGrantScope::AllSmartHome => "{\"kind\":\"all_smart_home\"}".to_string(),
    }
}

fn command_authorization_preview_json(
    entity: &Entity,
    command: &DeviceCommand,
    tool_decision: &AuthorizationDecision,
    command_decision: &AuthorizationDecision,
) -> String {
    let unsupported_capabilities = unsupported_command_capabilities(entity, command);
    let read_only_capabilities = read_only_command_capabilities(entity, command);
    let supported = unsupported_capabilities.is_empty();
    let commandable = read_only_capabilities.is_empty();
    let authorized = tool_decision.is_allowed() && command_decision.is_allowed();
    let dispatchable = supported && commandable && authorized;
    let missing_capabilities = unique_capability_ids(
        tool_decision
            .missing_capabilities
            .iter()
            .chain(command_decision.missing_capabilities.iter()),
    );
    let matched_grants = unique_grant_ids(
        tool_decision
            .matched_grants
            .iter()
            .chain(command_decision.matched_grants.iter()),
    );

    format!(
        "{{\"principal_id\":{},\"entity_id\":{},\"home_assistant_entity_id\":{},\"command_type\":{},\"required_tier\":{},\"supported\":{},\"commandable\":{},\"authorized\":{},\"dispatchable\":{},\"required_capabilities\":[{}],\"missing_capabilities\":[{}],\"unsupported_capabilities\":[{}],\"read_only_capabilities\":[{}],\"matched_grants\":[{}],\"tool_decision\":{},\"command_decision\":{}}}",
        json_string(command.requested_by.as_str()),
        json_string(command.entity_id.as_str()),
        json_string(home_assistant_entity_id(entity)),
        json_string(command_type_label(command.command_type)),
        json_string(privilege_tier_label(command.required_tier)),
        supported,
        commandable,
        authorized,
        dispatchable,
        json_id_array(command.required_capabilities.iter().map(|capability| capability.as_str())),
        json_id_array(missing_capabilities.iter().map(|capability| capability.as_str())),
        json_id_array(unsupported_capabilities.iter().map(|capability| capability.as_str())),
        json_id_array(read_only_capabilities.iter().map(|capability| capability.as_str())),
        json_id_array(matched_grants.iter().map(|grant_id| grant_id.as_str())),
        authorization_decision_json(tool_decision),
        authorization_decision_json(command_decision),
    )
}

fn desired_state_authorization_preview_json(
    entity: &Entity,
    operation: DesiredStateAuthorizationOperation,
    tool_decision: &AuthorizationDecision,
) -> String {
    let descriptor = operation.tool().descriptor();
    format!(
        "{{\"principal_id\":{},\"entity_id\":{},\"home_assistant_entity_id\":{},\"operation\":{},\"tool_id\":{},\"required_tier\":{},\"preview_only\":true,\"would_mutate_runtime\":true,\"authorized\":{},\"required_capabilities\":[{}],\"missing_capabilities\":[{}],\"matched_grants\":[{}],\"tool_decision\":{}}}",
        json_string(tool_decision.principal_id.as_str()),
        json_string(entity.entity_id.as_str()),
        json_string(home_assistant_entity_id(entity)),
        json_string(operation.label()),
        json_string(descriptor.tool_id),
        json_string(privilege_tier_label(tool_decision.required_tier)),
        tool_decision.is_allowed(),
        json_id_array(
            tool_decision
                .required_capabilities
                .iter()
                .map(|capability| capability.as_str())
        ),
        json_id_array(
            tool_decision
                .missing_capabilities
                .iter()
                .map(|capability| capability.as_str())
        ),
        json_id_array(
            tool_decision
                .matched_grants
                .iter()
                .map(|grant_id| grant_id.as_str())
        ),
        authorization_decision_json(tool_decision),
    )
}

fn scene_authorization_preview_json(
    scene: &Scene,
    tool_decision: &AuthorizationDecision,
    commands: &[AuthorizationCommandPreview],
) -> String {
    let descriptor = SmartHomeTool::Command.descriptor();
    let supported = commands.iter().all(|preview| {
        unsupported_command_capabilities(&preview.entity, &preview.command).is_empty()
    });
    let commandable = commands.iter().all(|preview| {
        read_only_command_capabilities(&preview.entity, &preview.command).is_empty()
    });
    let authorized =
        tool_decision.is_allowed() && commands.iter().all(|preview| preview.decision.is_allowed());
    let dispatchable = supported && commandable && authorized && !commands.is_empty();

    let mut required_capabilities = tool_decision.required_capabilities.clone();
    let mut missing_capabilities = tool_decision.missing_capabilities.clone();
    let mut matched_grants = tool_decision.matched_grants.clone();
    let mut unsupported_capabilities = Vec::new();
    let mut read_only_capabilities = Vec::new();

    for preview in commands {
        required_capabilities.extend(preview.command.required_capabilities.iter().cloned());
        missing_capabilities.extend(preview.decision.missing_capabilities.iter().cloned());
        matched_grants.extend(preview.decision.matched_grants.iter().cloned());
        unsupported_capabilities.extend(unsupported_command_capabilities(
            &preview.entity,
            &preview.command,
        ));
        read_only_capabilities.extend(read_only_command_capabilities(
            &preview.entity,
            &preview.command,
        ));
    }

    required_capabilities.sort();
    required_capabilities.dedup();
    missing_capabilities.sort();
    missing_capabilities.dedup();
    matched_grants.sort();
    matched_grants.dedup();
    unsupported_capabilities.sort();
    unsupported_capabilities.dedup();
    read_only_capabilities.sort();
    read_only_capabilities.dedup();

    format!(
        "{{\"principal_id\":{},\"scene_id\":{},\"home_assistant_scene_id\":{},\"tool_id\":{},\"required_tier\":{},\"preview_only\":true,\"would_mutate_runtime\":true,\"action_count\":{},\"command_count\":{},\"supported\":{},\"commandable\":{},\"authorized\":{},\"dispatchable\":{},\"required_capabilities\":[{}],\"missing_capabilities\":[{}],\"unsupported_capabilities\":[{}],\"read_only_capabilities\":[{}],\"matched_grants\":[{}],\"tool_decision\":{},\"commands\":[{}]}}",
        json_string(tool_decision.principal_id.as_str()),
        json_string(scene.scene_id.as_str()),
        json_string(home_assistant_scene_id(scene)),
        json_string(descriptor.tool_id),
        json_string(privilege_tier_label(tool_decision.required_tier)),
        scene.actions.len(),
        commands.len(),
        supported,
        commandable,
        authorized,
        dispatchable,
        json_id_array(required_capabilities.iter().map(|capability| capability.as_str())),
        json_id_array(missing_capabilities.iter().map(|capability| capability.as_str())),
        json_id_array(unsupported_capabilities.iter().map(|capability| capability.as_str())),
        json_id_array(read_only_capabilities.iter().map(|capability| capability.as_str())),
        json_id_array(matched_grants.iter().map(|grant_id| grant_id.as_str())),
        authorization_decision_json(tool_decision),
        commands
            .iter()
            .map(authorization_command_preview_json)
            .collect::<Vec<_>>()
            .join(","),
    )
}

fn service_authorization_preview_json(
    domain: &str,
    service: &str,
    call: &ServiceCall,
    tool_decision: &AuthorizationDecision,
    commands: &[AuthorizationCommandPreview],
) -> String {
    let descriptor = SmartHomeTool::Command.descriptor();
    let supported = commands.iter().all(|preview| {
        unsupported_command_capabilities(&preview.entity, &preview.command).is_empty()
    });
    let commandable = commands.iter().all(|preview| {
        read_only_command_capabilities(&preview.entity, &preview.command).is_empty()
    });
    let authorized =
        tool_decision.is_allowed() && commands.iter().all(|preview| preview.decision.is_allowed());
    let dispatchable = supported && commandable && authorized && !commands.is_empty();

    let mut required_capabilities = tool_decision.required_capabilities.clone();
    let mut missing_capabilities = tool_decision.missing_capabilities.clone();
    let mut matched_grants = tool_decision.matched_grants.clone();
    let mut unsupported_capabilities = Vec::new();
    let mut read_only_capabilities = Vec::new();

    for preview in commands {
        required_capabilities.extend(preview.command.required_capabilities.iter().cloned());
        missing_capabilities.extend(preview.decision.missing_capabilities.iter().cloned());
        matched_grants.extend(preview.decision.matched_grants.iter().cloned());
        unsupported_capabilities.extend(unsupported_command_capabilities(
            &preview.entity,
            &preview.command,
        ));
        read_only_capabilities.extend(read_only_command_capabilities(
            &preview.entity,
            &preview.command,
        ));
    }

    required_capabilities.sort();
    required_capabilities.dedup();
    missing_capabilities.sort();
    missing_capabilities.dedup();
    matched_grants.sort();
    matched_grants.dedup();
    unsupported_capabilities.sort();
    unsupported_capabilities.dedup();
    read_only_capabilities.sort();
    read_only_capabilities.dedup();

    format!(
        "{{\"principal_id\":{},\"domain\":{},\"service\":{},\"home_assistant_path\":{},\"tool_id\":{},\"required_tier\":{},\"preview_only\":true,\"would_mutate_runtime\":true,\"target_entity_ids\":[{}],\"target_scene_ids\":[{}],\"command_count\":{},\"supported\":{},\"commandable\":{},\"authorized\":{},\"dispatchable\":{},\"required_capabilities\":[{}],\"missing_capabilities\":[{}],\"unsupported_capabilities\":[{}],\"read_only_capabilities\":[{}],\"matched_grants\":[{}],\"tool_decision\":{},\"commands\":[{}]}}",
        json_string(tool_decision.principal_id.as_str()),
        json_string(domain),
        json_string(service),
        json_string(format!("/api/services/{domain}/{service}")),
        json_string(descriptor.tool_id),
        json_string(privilege_tier_label(tool_decision.required_tier)),
        json_string_array(&call.target_entity_ids),
        json_string_array(&call.target_scene_ids),
        commands.len(),
        supported,
        commandable,
        authorized,
        dispatchable,
        json_id_array(required_capabilities.iter().map(|capability| capability.as_str())),
        json_id_array(missing_capabilities.iter().map(|capability| capability.as_str())),
        json_id_array(unsupported_capabilities.iter().map(|capability| capability.as_str())),
        json_id_array(read_only_capabilities.iter().map(|capability| capability.as_str())),
        json_id_array(matched_grants.iter().map(|grant_id| grant_id.as_str())),
        authorization_decision_json(tool_decision),
        commands
            .iter()
            .map(authorization_command_preview_json)
            .collect::<Vec<_>>()
            .join(","),
    )
}

fn authorization_command_preview_json(preview: &AuthorizationCommandPreview) -> String {
    let unsupported_capabilities =
        unsupported_command_capabilities(&preview.entity, &preview.command);
    let read_only_capabilities = read_only_command_capabilities(&preview.entity, &preview.command);
    let supported = unsupported_capabilities.is_empty();
    let commandable = read_only_capabilities.is_empty();
    let authorized = preview.decision.is_allowed();
    let dispatchable = supported && commandable && authorized;

    format!(
        "{{\"entity_id\":{},\"home_assistant_entity_id\":{},\"command_type\":{},\"required_tier\":{},\"arguments\":{},\"supported\":{},\"commandable\":{},\"authorized\":{},\"dispatchable\":{},\"required_capabilities\":[{}],\"missing_capabilities\":[{}],\"unsupported_capabilities\":[{}],\"read_only_capabilities\":[{}],\"matched_grants\":[{}],\"command_decision\":{}}}",
        json_string(preview.command.entity_id.as_str()),
        json_string(home_assistant_entity_id(&preview.entity)),
        json_string(command_type_label(preview.command.command_type)),
        json_string(privilege_tier_label(preview.command.required_tier)),
        value_json(&preview.command.arguments),
        supported,
        commandable,
        authorized,
        dispatchable,
        json_id_array(
            preview
                .command
                .required_capabilities
                .iter()
                .map(|capability| capability.as_str())
        ),
        json_id_array(
            preview
                .decision
                .missing_capabilities
                .iter()
                .map(|capability| capability.as_str())
        ),
        json_id_array(unsupported_capabilities.iter().map(|capability| capability.as_str())),
        json_id_array(read_only_capabilities.iter().map(|capability| capability.as_str())),
        json_id_array(
            preview
                .decision
                .matched_grants
                .iter()
                .map(|grant_id| grant_id.as_str())
        ),
        authorization_decision_json(&preview.decision),
    )
}

fn authorization_decision_json(decision: &AuthorizationDecision) -> String {
    format!(
        "{{\"outcome\":{},\"required_tier\":{},\"required_capabilities\":[{}],\"missing_capabilities\":[{}],\"matched_grants\":[{}]}}",
        json_string(authorization_outcome_label(decision.outcome)),
        json_string(privilege_tier_label(decision.required_tier)),
        json_id_array(decision.required_capabilities.iter().map(|capability| capability.as_str())),
        json_id_array(decision.missing_capabilities.iter().map(|capability| capability.as_str())),
        json_id_array(decision.matched_grants.iter().map(|grant_id| grant_id.as_str())),
    )
}

fn authorization_decisions_json(
    records: &[AuthorizationDecisionRecord<'_>],
    summary: &smart_home_core::AuthorizationDecisionLogSummary,
) -> String {
    format!(
        "{{\"summary\":{},\"decisions\":[{}]}}",
        authorization_decision_summary_json(summary),
        records
            .iter()
            .map(authorization_decision_record_json)
            .collect::<Vec<_>>()
            .join(",")
    )
}

#[derive(Debug, Clone, Copy)]
struct AuthorizationDecisionRecord<'a> {
    decision_index: usize,
    decision: &'a AuthorizationDecision,
}

fn authorization_decision_records<'a>(
    runtime: &'a SmartHomeRuntime,
    decisions: Vec<&'a AuthorizationDecision>,
) -> Vec<AuthorizationDecisionRecord<'a>> {
    let indexed_decisions = runtime
        .registry()
        .authorization_decisions()
        .enumerate()
        .collect::<Vec<_>>();
    decisions
        .into_iter()
        .filter_map(|decision| {
            indexed_decisions
                .iter()
                .find(|(_, candidate)| std::ptr::eq(*candidate, decision))
                .map(|(decision_index, _)| AuthorizationDecisionRecord {
                    decision_index: *decision_index,
                    decision,
                })
        })
        .collect()
}

fn authorization_decision_summary_json(
    summary: &smart_home_core::AuthorizationDecisionLogSummary,
) -> String {
    format!(
        "{{\"total_decisions\":{},\"allowed_decisions\":{},\"denied_decisions\":{},\"tool_decisions\":{},\"command_decisions\":{},\"read_only_tier_decisions\":{},\"low_risk_tier_decisions\":{},\"human_approval_tier_decisions\":{},\"high_risk_tier_decisions\":{},\"decisions_with_missing_capabilities\":{},\"total_required_capabilities\":{},\"total_matched_grants\":{},\"total_missing_capabilities\":{}}}",
        summary.total_decisions,
        summary.allowed_decisions,
        summary.denied_decisions,
        summary.tool_decisions,
        summary.command_decisions,
        summary.read_only_tier_decisions,
        summary.low_risk_tier_decisions,
        summary.human_approval_tier_decisions,
        summary.high_risk_tier_decisions,
        summary.decisions_with_missing_capabilities,
        summary.total_required_capabilities,
        summary.total_matched_grants,
        summary.total_missing_capabilities,
    )
}

fn authorization_decision_record_json(record: &AuthorizationDecisionRecord<'_>) -> String {
    let decision = record.decision;
    format!(
        "{{\"decision_index\":{},\"principal_id\":{},\"subject\":{},\"outcome\":{},\"required_tier\":{},\"required_capabilities\":[{}],\"matched_grants\":[{}],\"missing_capabilities\":[{}],\"decided_at_ms\":{},\"links\":{}}}",
        record.decision_index,
        json_string(decision.principal_id.as_str()),
        authorization_subject_json(&decision.subject),
        json_string(authorization_outcome_label(decision.outcome)),
        json_string(privilege_tier_label(decision.required_tier)),
        json_id_array(decision.required_capabilities.iter().map(|id| id.as_str())),
        json_id_array(decision.matched_grants.iter().map(|id| id.as_str())),
        json_id_array(decision.missing_capabilities.iter().map(|id| id.as_str())),
        decision.decided_at_ms,
        authorization_decision_links_json(record),
    )
}

fn authorization_decision_links_json(record: &AuthorizationDecisionRecord<'_>) -> String {
    let decision = record.decision;
    let (subject_entity, subject_command_result, subject_authorization) = match &decision.subject {
        AuthorizationSubject::Tool(_) => (None, None, None),
        AuthorizationSubject::Command {
            command_id,
            entity_id,
            command_type,
        } => (
            Some(audit_entity_links_json(entity_id)),
            Some(format!(
                "/api/smart_home/command_results/{}",
                command_id.as_str()
            )),
            Some(format!(
                "/api/smart_home/command_authorization?entity_id={}&command_type={}",
                url_component(entity_id.as_str()),
                url_component(command_type_label(*command_type))
            )),
        ),
    };

    format!(
        "{{\"self\":{},\"principal_grants\":{},\"matched_grants\":[{}],\"subject_entity\":{},\"subject_command_result\":{},\"subject_authorization\":{}}}",
        json_string(format!(
            "/api/smart_home/authorization_decisions/{}",
            record.decision_index
        )),
        json_string(format!(
            "/api/smart_home/capability_grants?principal_id={}&status=active",
            decision.principal_id.as_str()
        )),
        decision
            .matched_grants
            .iter()
            .map(|grant_id| {
                json_string(format!(
                    "/api/smart_home/capability_grants/{}",
                    grant_id.as_str()
                ))
            })
            .collect::<Vec<_>>()
            .join(","),
        subject_entity.unwrap_or_else(|| "null".to_string()),
        optional_link_json(subject_command_result),
        optional_link_json(subject_authorization),
    )
}

fn authorization_subject_json(subject: &AuthorizationSubject) -> String {
    match subject {
        AuthorizationSubject::Tool(tool) => {
            format!(
                "{{\"kind\":\"tool\",\"tool_id\":{}}}",
                json_string(tool.descriptor().tool_id)
            )
        }
        AuthorizationSubject::Command {
            command_id,
            entity_id,
            command_type,
        } => format!(
            "{{\"kind\":\"command\",\"command_id\":{},\"entity_id\":{},\"command_type\":{}}}",
            json_string(command_id.as_str()),
            json_string(entity_id.as_str()),
            json_string(command_type_label(*command_type)),
        ),
    }
}

fn states_registry_json(entities: &[&Entity], runtime: &SmartHomeRuntime, now_ms: u64) -> String {
    let with_state = entities
        .iter()
        .filter(|entity| entity.state.is_some())
        .count();
    let stale = entities
        .iter()
        .filter(|entity| {
            entity
                .state
                .as_ref()
                .is_none_or(|snapshot| snapshot.is_stale_at(now_ms))
        })
        .count();
    let optimistic = entities
        .iter()
        .filter(|entity| {
            entity
                .state
                .as_ref()
                .is_some_and(|snapshot| snapshot.confidence == StateConfidence::Optimistic)
        })
        .count();

    format!(
        "{{\"summary\":{{\"total_entities\":{},\"entities_with_state\":{},\"entities_without_state\":{},\"stale_entities\":{},\"optimistic_entities\":{}}},\"states\":[{}]}}",
        entities.len(),
        with_state,
        entities.len().saturating_sub(with_state),
        stale,
        optimistic,
        entities
            .iter()
            .map(|entity| state_registry_json(entity, runtime, now_ms))
            .collect::<Vec<_>>()
            .join(","),
    )
}

fn state_registry_json(entity: &Entity, runtime: &SmartHomeRuntime, now_ms: u64) -> String {
    let device = runtime.registry().device(&entity.device_id);
    let bridge_id = device.map(|device| device.bridge_id.as_str());
    let room_id = device.and_then(|device| device.room_id.as_deref());
    let snapshot = entity.state.as_ref();

    format!(
        "{{\"entity_id\":{},\"home_assistant_entity_id\":{},\"device_id\":{},\"bridge_id\":{},\"room_id\":{},\"name\":{},\"domain\":{},\"entity_kind\":{},\"has_state\":{},\"stale\":{},\"value\":{},\"source\":{},\"confidence\":{},\"observed_at_ms\":{},\"received_at_ms\":{},\"expires_at_ms\":{},\"capability_ids\":[{}],\"capabilities\":[{}]}}",
        json_string(entity.entity_id.as_str()),
        json_string(home_assistant_entity_id(entity)),
        json_string(entity.device_id.as_str()),
        optional_str_json(bridge_id),
        optional_str_json(room_id),
        json_string(&entity.name),
        json_string(entity_domain(entity.kind)),
        json_string(entity_kind_label(entity.kind)),
        snapshot.is_some(),
        snapshot.is_none_or(|snapshot| snapshot.is_stale_at(now_ms)),
        snapshot
            .map(|snapshot| value_json(&snapshot.value))
            .unwrap_or_else(|| "null".to_string()),
        snapshot
            .map(|snapshot| json_string(state_source_label(snapshot.source)))
            .unwrap_or_else(|| "null".to_string()),
        snapshot
            .map(|snapshot| json_string(state_confidence_label(snapshot.confidence)))
            .unwrap_or_else(|| "null".to_string()),
        optional_u64_json(snapshot.map(|snapshot| snapshot.observed_at_ms)),
        optional_u64_json(snapshot.map(|snapshot| snapshot.received_at_ms)),
        optional_u64_json(snapshot.and_then(|snapshot| snapshot.expires_at_ms)),
        json_id_array(
            entity
                .capabilities
                .iter()
                .map(|capability| capability.capability_id.as_str())
        ),
        entity
            .capabilities
            .iter()
            .map(capability_json)
            .collect::<Vec<_>>()
            .join(","),
    )
}

fn entities_registry_json(entities: &[&Entity], runtime: &SmartHomeRuntime, now_ms: u64) -> String {
    let stateful_entities = entities
        .iter()
        .filter(|entity| entity.state.is_some())
        .count();
    let stale_entities = entities
        .iter()
        .filter(|entity| {
            entity
                .state
                .as_ref()
                .is_none_or(|snapshot| snapshot.is_stale_at(now_ms))
        })
        .count();
    let commandable_entities = entities
        .iter()
        .filter(|entity| entity.capabilities.iter().any(capability_allows_command))
        .count();
    let capability_count = entities
        .iter()
        .map(|entity| entity.capabilities.len())
        .sum::<usize>();

    format!(
        "{{\"summary\":{{\"total_entities\":{},\"stateful_entities\":{},\"stale_entities\":{},\"commandable_entities\":{},\"capability_count\":{}}},\"entities\":[{}]}}",
        entities.len(),
        stateful_entities,
        stale_entities,
        commandable_entities,
        capability_count,
        entities
            .iter()
            .map(|entity| entity_registry_json(entity, runtime, now_ms))
            .collect::<Vec<_>>()
            .join(","),
    )
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct EntityInventoryCounts {
    total_entities: usize,
    commandable_entities: usize,
    stateful_entities: usize,
    stale_entities: usize,
    capability_count: usize,
}

impl EntityInventoryCounts {
    fn add(&mut self, other: Self) {
        self.total_entities += other.total_entities;
        self.commandable_entities += other.commandable_entities;
        self.stateful_entities += other.stateful_entities;
        self.stale_entities += other.stale_entities;
        self.capability_count += other.capability_count;
    }
}

#[derive(Debug, Clone)]
struct CapabilityCatalogQuery {
    domain: Option<String>,
    capability_id: Option<String>,
    commandable: Option<bool>,
    observable: Option<bool>,
    limit: usize,
}

impl CapabilityCatalogQuery {
    fn new(limit: usize) -> Self {
        Self {
            domain: None,
            capability_id: None,
            commandable: None,
            observable: None,
            limit,
        }
    }
}

#[derive(Debug, Clone)]
struct CapabilityCatalogItem {
    capability_id: String,
    mode: CapabilityMode,
    value_kind: ValueKind,
    unit: Option<String>,
    min: Option<f64>,
    max: Option<f64>,
    step: Option<f64>,
    domains: Vec<String>,
    entity_kinds: Vec<String>,
    entity_ids: Vec<String>,
    home_assistant_entity_ids: Vec<String>,
    device_ids: Vec<String>,
    room_ids: Vec<String>,
    service_ids: Vec<String>,
}

impl CapabilityCatalogItem {
    fn from_capability(capability: &Capability) -> Self {
        Self {
            capability_id: capability.capability_id.as_str().to_string(),
            mode: capability.mode,
            value_kind: capability.value_kind,
            unit: capability.unit.clone(),
            min: capability.min,
            max: capability.max,
            step: capability.step,
            domains: Vec::new(),
            entity_kinds: Vec::new(),
            entity_ids: Vec::new(),
            home_assistant_entity_ids: Vec::new(),
            device_ids: Vec::new(),
            room_ids: Vec::new(),
            service_ids: Vec::new(),
        }
    }

    fn add_entity(&mut self, runtime: &SmartHomeRuntime, entity: &Entity, capability: &Capability) {
        let domain = entity_domain(entity.kind);
        push_unique_string(&mut self.domains, domain);
        push_unique_string(&mut self.entity_kinds, entity_kind_label(entity.kind));
        push_unique_string(&mut self.entity_ids, entity.entity_id.as_str());
        push_unique_string(
            &mut self.home_assistant_entity_ids,
            &home_assistant_entity_id(entity),
        );
        push_unique_string(&mut self.device_ids, entity.device_id.as_str());

        if let Some(device) = runtime.registry().device(&entity.device_id) {
            if let Some(room_id) = &device.room_id {
                push_unique_string(&mut self.room_ids, room_id);
            }
        }

        if capability_allows_command(capability) {
            for service in services_for_capability(domain, capability) {
                push_unique_string(&mut self.service_ids, &format!("{domain}.{service}"));
            }
        }
    }

    fn sort_links(&mut self) {
        self.domains.sort();
        self.entity_kinds.sort();
        self.entity_ids.sort();
        self.home_assistant_entity_ids.sort();
        self.device_ids.sort();
        self.room_ids.sort();
        self.service_ids.sort();
    }

    fn observable(&self) -> bool {
        matches!(
            self.mode,
            CapabilityMode::Observe | CapabilityMode::ObserveAndCommand
        )
    }

    fn commandable(&self) -> bool {
        matches!(
            self.mode,
            CapabilityMode::Command | CapabilityMode::ObserveAndCommand
        )
    }

    fn ranged(&self) -> bool {
        self.min.is_some() || self.max.is_some() || self.step.is_some()
    }
}

fn capabilities_catalog_json(capabilities: &[CapabilityCatalogItem]) -> String {
    let mut domains = Vec::<String>::new();
    for capability in capabilities {
        for domain in &capability.domains {
            push_unique_string(&mut domains, domain);
        }
    }

    format!(
        "{{\"summary\":{{\"total_capabilities\":{},\"commandable_capabilities\":{},\"observable_capabilities\":{},\"ranged_capabilities\":{},\"domain_count\":{},\"entity_link_count\":{},\"device_link_count\":{},\"room_link_count\":{},\"service_link_count\":{}}},\"capabilities\":[{}]}}",
        capabilities.len(),
        capabilities
            .iter()
            .filter(|capability| capability.commandable())
            .count(),
        capabilities
            .iter()
            .filter(|capability| capability.observable())
            .count(),
        capabilities
            .iter()
            .filter(|capability| capability.ranged())
            .count(),
        domains.len(),
        capabilities
            .iter()
            .map(|capability| capability.entity_ids.len())
            .sum::<usize>(),
        capabilities
            .iter()
            .map(|capability| capability.device_ids.len())
            .sum::<usize>(),
        capabilities
            .iter()
            .map(|capability| capability.room_ids.len())
            .sum::<usize>(),
        capabilities
            .iter()
            .map(|capability| capability.service_ids.len())
            .sum::<usize>(),
        capabilities
            .iter()
            .map(capability_catalog_item_json)
            .collect::<Vec<_>>()
            .join(","),
    )
}

fn capability_catalog_item_json(capability: &CapabilityCatalogItem) -> String {
    format!(
        "{{\"capability_id\":{},\"mode\":{},\"value_kind\":{},\"unit\":{},\"min\":{},\"max\":{},\"step\":{},\"observable\":{},\"commandable\":{},\"domains\":[{}],\"entity_kinds\":[{}],\"entity_count\":{},\"device_count\":{},\"room_count\":{},\"service_count\":{},\"entity_ids\":[{}],\"home_assistant_entity_ids\":[{}],\"device_ids\":[{}],\"room_ids\":[{}],\"service_ids\":[{}]}}",
        json_string(&capability.capability_id),
        json_string(capability_mode_label(capability.mode)),
        json_string(value_kind_label(capability.value_kind)),
        capability
            .unit
            .as_ref()
            .map(json_string)
            .unwrap_or_else(|| "null".to_string()),
        optional_f64_json(capability.min),
        optional_f64_json(capability.max),
        optional_f64_json(capability.step),
        capability.observable(),
        capability.commandable(),
        json_string_array(&capability.domains),
        json_string_array(&capability.entity_kinds),
        capability.entity_ids.len(),
        capability.device_ids.len(),
        capability.room_ids.len(),
        capability.service_ids.len(),
        json_string_array(&capability.entity_ids),
        json_string_array(&capability.home_assistant_entity_ids),
        json_string_array(&capability.device_ids),
        json_string_array(&capability.room_ids),
        json_string_array(&capability.service_ids),
    )
}

fn devices_registry_json(devices: &[&Device], runtime: &SmartHomeRuntime, now_ms: u64) -> String {
    let mut entity_counts = EntityInventoryCounts::default();
    for device in devices {
        entity_counts.add(device_inventory_counts(device, runtime, now_ms));
    }

    format!(
        "{{\"summary\":{{\"total_devices\":{},\"online_devices\":{},\"pairing_candidate_devices\":{},\"attention_devices\":{},\"total_entities\":{},\"commandable_entities\":{},\"stateful_entities\":{},\"stale_entities\":{},\"capability_count\":{}}},\"devices\":[{}]}}",
        devices.len(),
        devices
            .iter()
            .filter(|device| device.health.is_online())
            .count(),
        devices
            .iter()
            .filter(|device| device.health.is_pairing_candidate())
            .count(),
        devices
            .iter()
            .filter(|device| device.health.needs_attention())
            .count(),
        entity_counts.total_entities,
        entity_counts.commandable_entities,
        entity_counts.stateful_entities,
        entity_counts.stale_entities,
        entity_counts.capability_count,
        devices
            .iter()
            .map(|device| device_registry_json(device, runtime, now_ms))
            .collect::<Vec<_>>()
            .join(","),
    )
}

fn device_registry_json(device: &Device, runtime: &SmartHomeRuntime, now_ms: u64) -> String {
    let entities = device_entities(device, runtime);
    let entity_counts = entity_inventory_counts(&entities, now_ms);
    let entity_ids = entities
        .iter()
        .map(|entity| entity.entity_id.as_str())
        .collect::<Vec<_>>();
    let home_assistant_entity_ids = entities
        .iter()
        .map(|entity| home_assistant_entity_id(entity))
        .collect::<Vec<_>>();
    let mut capability_ids = Vec::<String>::new();
    for entity in &entities {
        for capability in &entity.capabilities {
            push_unique_string(&mut capability_ids, capability.capability_id.as_str());
        }
    }

    format!(
        "{{\"device_id\":{},\"bridge_id\":{},\"name\":{},\"manufacturer\":{},\"model\":{},\"serial\":{},\"firmware_version\":{},\"room_id\":{},\"health\":{},\"entity_count\":{},\"commandable_entities\":{},\"stateful_entities\":{},\"stale_entities\":{},\"capability_count\":{},\"entity_ids\":[{}],\"home_assistant_entity_ids\":[{}],\"capability_ids\":[{}]}}",
        json_string(device.device_id.as_str()),
        json_string(device.bridge_id.as_str()),
        json_string(&device.name),
        json_string(&device.manufacturer),
        json_string(&device.model),
        optional_str_json(device.serial.as_deref()),
        optional_str_json(device.firmware_version.as_deref()),
        optional_str_json(device.room_id.as_deref()),
        json_string(health_label(device.health)),
        entity_counts.total_entities,
        entity_counts.commandable_entities,
        entity_counts.stateful_entities,
        entity_counts.stale_entities,
        entity_counts.capability_count,
        json_id_array(entity_ids),
        json_string_array(&home_assistant_entity_ids),
        json_string_array(&capability_ids),
    )
}

fn bridges_registry_json(bridges: &[&Bridge], runtime: &SmartHomeRuntime, now_ms: u64) -> String {
    let mut total_devices = 0usize;
    let mut entity_counts = EntityInventoryCounts::default();
    let mut room_ids = Vec::<String>::new();
    for bridge in bridges {
        let devices = bridge_devices(bridge, runtime);
        total_devices += devices.len();
        for device in devices {
            if let Some(room_id) = &device.room_id {
                push_unique_string(&mut room_ids, room_id);
            }
            entity_counts.add(device_inventory_counts(device, runtime, now_ms));
        }
    }
    room_ids.sort();

    format!(
        "{{\"summary\":{{\"total_bridges\":{},\"online_bridges\":{},\"pairing_candidate_bridges\":{},\"attention_bridges\":{},\"total_devices\":{},\"total_entities\":{},\"commandable_entities\":{},\"stateful_entities\":{},\"stale_entities\":{},\"capability_count\":{},\"room_count\":{}}},\"bridges\":[{}]}}",
        bridges.len(),
        bridges
            .iter()
            .filter(|bridge| bridge.health.is_online())
            .count(),
        bridges
            .iter()
            .filter(|bridge| bridge.health.is_pairing_candidate())
            .count(),
        bridges
            .iter()
            .filter(|bridge| bridge.health.needs_attention())
            .count(),
        total_devices,
        entity_counts.total_entities,
        entity_counts.commandable_entities,
        entity_counts.stateful_entities,
        entity_counts.stale_entities,
        entity_counts.capability_count,
        room_ids.len(),
        bridges
            .iter()
            .map(|bridge| bridge_registry_json(bridge, runtime, now_ms))
            .collect::<Vec<_>>()
            .join(","),
    )
}

fn bridge_registry_json(bridge: &Bridge, runtime: &SmartHomeRuntime, now_ms: u64) -> String {
    let devices = bridge_devices(bridge, runtime);
    let mut entity_counts = EntityInventoryCounts::default();
    let mut room_ids = Vec::<String>::new();
    for device in &devices {
        if let Some(room_id) = &device.room_id {
            push_unique_string(&mut room_ids, room_id);
        }
        entity_counts.add(device_inventory_counts(device, runtime, now_ms));
    }
    room_ids.sort();
    let device_ids = devices
        .iter()
        .map(|device| device.device_id.as_str())
        .collect::<Vec<_>>();

    format!(
        "{{\"bridge_id\":{},\"integration_id\":{},\"transport\":{},\"address\":{},\"hardware_model\":{},\"firmware_version\":{},\"health\":{},\"last_seen_at_ms\":{},\"device_count\":{},\"online_devices\":{},\"pairing_candidate_devices\":{},\"attention_devices\":{},\"entity_count\":{},\"commandable_entities\":{},\"stateful_entities\":{},\"stale_entities\":{},\"capability_count\":{},\"room_count\":{},\"room_ids\":[{}],\"device_ids\":[{}]}}",
        json_string(bridge.bridge_id.as_str()),
        json_string(bridge.integration_id.as_str()),
        json_string(bridge_transport_label(bridge.transport)),
        optional_str_json(bridge.address.as_deref()),
        optional_str_json(bridge.hardware_model.as_deref()),
        optional_str_json(bridge.firmware_version.as_deref()),
        json_string(health_label(bridge.health)),
        optional_u64_json(bridge.last_seen_at_ms),
        devices.len(),
        devices
            .iter()
            .filter(|device| device.health.is_online())
            .count(),
        devices
            .iter()
            .filter(|device| device.health.is_pairing_candidate())
            .count(),
        devices
            .iter()
            .filter(|device| device.health.needs_attention())
            .count(),
        entity_counts.total_entities,
        entity_counts.commandable_entities,
        entity_counts.stateful_entities,
        entity_counts.stale_entities,
        entity_counts.capability_count,
        room_ids.len(),
        json_string_array(&room_ids),
        json_id_array(device_ids),
    )
}

fn bridge_devices<'a>(bridge: &Bridge, runtime: &'a SmartHomeRuntime) -> Vec<&'a Device> {
    let mut devices = runtime
        .registry()
        .devices_for_bridge(&bridge.bridge_id)
        .collect::<Vec<_>>();
    devices.sort_by(|left, right| left.device_id.as_str().cmp(right.device_id.as_str()));
    devices
}

fn device_entities<'a>(device: &Device, runtime: &'a SmartHomeRuntime) -> Vec<&'a Entity> {
    let mut entities = runtime
        .registry()
        .entities_for_device(&device.device_id)
        .collect::<Vec<_>>();
    entities.sort_by(|left, right| left.entity_id.as_str().cmp(right.entity_id.as_str()));
    entities
}

fn device_inventory_counts(
    device: &Device,
    runtime: &SmartHomeRuntime,
    now_ms: u64,
) -> EntityInventoryCounts {
    let entities = device_entities(device, runtime);
    entity_inventory_counts(&entities, now_ms)
}

fn entity_inventory_counts(entities: &[&Entity], now_ms: u64) -> EntityInventoryCounts {
    EntityInventoryCounts {
        total_entities: entities.len(),
        commandable_entities: entities
            .iter()
            .filter(|entity| entity.capabilities.iter().any(capability_allows_command))
            .count(),
        stateful_entities: entities
            .iter()
            .filter(|entity| entity.state.is_some())
            .count(),
        stale_entities: entities
            .iter()
            .filter(|entity| {
                entity
                    .state
                    .as_ref()
                    .is_none_or(|snapshot| snapshot.is_stale_at(now_ms))
            })
            .count(),
        capability_count: entities
            .iter()
            .map(|entity| entity.capabilities.len())
            .sum(),
    }
}

fn rooms_json(rooms: &[RuntimeRoomSummary], runtime: &SmartHomeRuntime) -> String {
    let topology = runtime.topology_summary();
    let state_gap_rooms = rooms.iter().filter(|room| room.has_state_gaps()).count();
    let attention_rooms = rooms
        .iter()
        .filter(|room| room.has_attention_items())
        .count();
    let scene_rooms = rooms.iter().filter(|room| room.has_scene_actions()).count();

    format!(
        "{{\"summary\":{{\"total_rooms\":{},\"attention_rooms\":{},\"state_gap_rooms\":{},\"scene_rooms\":{},\"total_devices\":{},\"total_entities\":{},\"total_scenes\":{},\"topology_unique_rooms\":{}}},\"topology\":{{\"bridges\":{},\"devices\":{},\"entities\":{},\"scenes\":{},\"devices_with_room\":{},\"devices_without_room\":{},\"unique_rooms\":{},\"scene_actions\":{},\"room_scenes\":{}}},\"rooms\":[{}]}}",
        rooms.len(),
        attention_rooms,
        state_gap_rooms,
        scene_rooms,
        rooms.iter().map(|room| room.device_count).sum::<usize>(),
        rooms.iter().map(|room| room.entity_count).sum::<usize>(),
        rooms.iter().map(|room| room.scene_count).sum::<usize>(),
        topology.unique_rooms,
        topology.bridges,
        topology.devices,
        topology.entities,
        topology.scenes,
        topology.devices_with_room,
        topology.devices_without_room,
        topology.unique_rooms,
        topology.scene_actions,
        topology.room_scenes,
        rooms
            .iter()
            .map(room_json)
            .collect::<Vec<_>>()
            .join(","),
    )
}

fn room_json(room: &RuntimeRoomSummary) -> String {
    format!(
        "{{\"room_id\":{},\"device_count\":{},\"online_devices\":{},\"pairing_candidate_devices\":{},\"attention_devices\":{},\"entity_count\":{},\"commandable_entities\":{},\"entities_with_state\":{},\"entities_without_state\":{},\"stale_entities\":{},\"state_gap_count\":{},\"scene_count\":{},\"scene_action_count\":{},\"has_attention\":{},\"has_state_gaps\":{},\"has_scene_actions\":{}}}",
        json_string(&room.room_id),
        room.device_count,
        room.online_devices,
        room.pairing_candidate_devices,
        room.attention_devices,
        room.entity_count,
        room.commandable_entities,
        room.entities_with_state,
        room.entities_without_state,
        room.stale_entities,
        room.state_gap_count(),
        room.scene_count,
        room.scene_action_count,
        room.has_attention_items(),
        room.has_state_gaps(),
        room.has_scene_actions(),
    )
}

fn room_detail_json(room: &RuntimeRoomSummary, runtime: &SmartHomeRuntime, now_ms: u64) -> String {
    let devices = runtime_room_devices(runtime, &room.room_id);
    let entities = runtime_room_entities(runtime, &room.room_id);
    let scenes = runtime_room_scenes(runtime, &room.room_id);
    format!(
        "{{\"room\":{},\"links\":{},\"members\":{{\"device_count\":{},\"entity_count\":{},\"scene_count\":{},\"devices\":[{}],\"entities\":[{}],\"scenes\":[{}]}}}}",
        room_json(room),
        room_links_json(&room.room_id),
        devices.len(),
        entities.len(),
        scenes.len(),
        devices
            .iter()
            .map(|device| device_registry_json(device, runtime, now_ms))
            .collect::<Vec<_>>()
            .join(","),
        entities
            .iter()
            .map(|entity| entity_registry_json(entity, runtime, now_ms))
            .collect::<Vec<_>>()
            .join(","),
        scenes
            .iter()
            .map(|scene| scene_json(scene, runtime))
            .collect::<Vec<_>>()
            .join(","),
    )
}

fn room_links_json(room_id: &str) -> String {
    let room = url_component(room_id);
    format!(
        "{{\"self\":{},\"rooms\":{},\"devices\":{},\"entities\":{},\"states\":{},\"state_gaps\":{},\"scenes\":{},\"history\":{},\"events\":{},\"command_results\":{}}}",
        json_string(format!("/api/smart_home/rooms/{room}")),
        json_string(format!("/api/smart_home/rooms?room_id={room}")),
        json_string(format!("/api/smart_home/devices?room_id={room}")),
        json_string(format!("/api/smart_home/entities?room_id={room}")),
        json_string(format!("/api/smart_home/states?room_id={room}")),
        json_string(format!("/api/smart_home/states?room_id={room}&stale=true")),
        json_string(format!("/api/smart_home/scenes?room_id={room}")),
        json_string(format!("/api/smart_home/state_history?room_id={room}")),
        json_string(format!("/api/smart_home/events?room_id={room}")),
        json_string(format!("/api/smart_home/command_results?room_id={room}")),
    )
}

fn runtime_room_devices<'a>(runtime: &'a SmartHomeRuntime, room_id: &str) -> Vec<&'a Device> {
    let mut devices = runtime
        .registry()
        .devices()
        .filter(|device| device.room_id.as_deref() == Some(room_id))
        .collect::<Vec<_>>();
    devices.sort_by(|left, right| left.device_id.as_str().cmp(right.device_id.as_str()));
    devices
}

fn entity_room_id<'a>(runtime: &'a SmartHomeRuntime, entity: &Entity) -> Option<&'a str> {
    runtime
        .registry()
        .device(&entity.device_id)
        .and_then(|device| device.room_id.as_deref())
}

fn bridge_has_room(runtime: &SmartHomeRuntime, bridge_id: &BridgeId, room_id: &str) -> bool {
    runtime
        .registry()
        .devices()
        .any(|device| &device.bridge_id == bridge_id && device.room_id.as_deref() == Some(room_id))
}

fn entity_id_has_room(runtime: &SmartHomeRuntime, entity_id: &EntityId, room_id: &str) -> bool {
    runtime
        .registry()
        .entity(entity_id)
        .and_then(|entity| entity_room_id(runtime, entity))
        == Some(room_id)
}

fn device_event_matches_room(
    runtime: &SmartHomeRuntime,
    event: &DeviceEvent,
    room_id: &str,
) -> bool {
    event
        .entity_id
        .as_ref()
        .is_some_and(|entity_id| entity_id_has_room(runtime, entity_id, room_id))
        || event
            .device_id
            .as_ref()
            .and_then(|device_id| runtime.registry().device(device_id))
            .and_then(|device| device.room_id.as_deref())
            == Some(room_id)
}

fn runtime_event_matches_room(
    runtime: &SmartHomeRuntime,
    event: &RuntimeEvent,
    room_id: &str,
) -> bool {
    match event {
        RuntimeEvent::Device(event) => device_event_matches_room(runtime, event, room_id),
        RuntimeEvent::CommandResult(result) => bridge_has_room(runtime, &result.bridge_id, room_id),
        RuntimeEvent::BridgeHealth { bridge_id, .. }
        | RuntimeEvent::WorkerNeedsRestart { bridge_id, .. } => {
            bridge_has_room(runtime, bridge_id, room_id)
        }
        RuntimeEvent::StateExpired { entity_id, .. }
        | RuntimeEvent::DesiredStateDrift { entity_id, .. } => {
            entity_id_has_room(runtime, entity_id, room_id)
        }
    }
}

fn runtime_room_entities<'a>(runtime: &'a SmartHomeRuntime, room_id: &str) -> Vec<&'a Entity> {
    let mut entities = runtime
        .registry()
        .entities()
        .filter(|entity| entity_room_id(runtime, entity) == Some(room_id))
        .collect::<Vec<_>>();
    entities.sort_by(|left, right| left.entity_id.as_str().cmp(right.entity_id.as_str()));
    entities
}

fn runtime_room_scenes<'a>(runtime: &'a SmartHomeRuntime, room_id: &str) -> Vec<&'a Scene> {
    let mut scenes = runtime
        .registry()
        .scenes()
        .filter(|scene| {
            scene_room_ids(scene, runtime)
                .iter()
                .any(|candidate| candidate == room_id)
        })
        .collect::<Vec<_>>();
    scenes.sort_by(|left, right| left.scene_id.as_str().cmp(right.scene_id.as_str()));
    scenes
}

fn scenes_json(scenes: &[&Scene], runtime: &SmartHomeRuntime) -> String {
    let action_count = scenes
        .iter()
        .map(|scene| scene.actions.len())
        .sum::<usize>();
    let mut room_ids = Vec::<String>::new();
    for scene in scenes {
        for room_id in scene_room_ids(scene, runtime) {
            push_unique_string(&mut room_ids, &room_id);
        }
    }
    room_ids.sort();
    format!(
        "{{\"summary\":{{\"total_scenes\":{},\"action_count\":{},\"room_count\":{}}},\"scenes\":[{}]}}",
        scenes.len(),
        action_count,
        room_ids.len(),
        scenes
            .iter()
            .map(|scene| scene_json(scene, runtime))
            .collect::<Vec<_>>()
            .join(","),
    )
}

fn scene_json(scene: &Scene, runtime: &SmartHomeRuntime) -> String {
    let room_ids = scene_room_ids(scene, runtime);
    format!(
        "{{\"scene_id\":{},\"home_assistant_scene_id\":{},\"scope\":{},\"native_ref\":{},\"room_ids\":[{}],\"action_count\":{},\"actions\":[{}],\"metadata\":[{}]}}",
        json_string(scene.scene_id.as_str()),
        json_string(home_assistant_scene_id(scene)),
        json_string(scene_scope_label(scene.scope)),
        scene_native_ref_json(scene),
        json_string_array(&room_ids),
        scene.actions.len(),
        scene
            .actions
            .iter()
            .map(|action| scene_action_json(action, runtime))
            .collect::<Vec<_>>()
            .join(","),
        metadata_json(&scene.metadata),
    )
}

fn scene_action_json(action: &smart_home_core::SceneAction, runtime: &SmartHomeRuntime) -> String {
    let home_assistant_entity_id = runtime
        .registry()
        .entity(&action.entity_id)
        .map(home_assistant_entity_id)
        .unwrap_or_else(|| home_assistant_entity_id_for(&action.entity_id));
    format!(
        "{{\"entity_id\":{},\"home_assistant_entity_id\":{},\"desired_state\":{}}}",
        json_string(action.entity_id.as_str()),
        json_string(home_assistant_entity_id),
        value_json(&action.desired_state),
    )
}

fn scene_native_ref_json(scene: &Scene) -> String {
    scene
        .native_ref
        .as_ref()
        .map(|native_ref| {
            format!(
                "{{\"family\":{},\"kind\":{},\"value\":{}}}",
                json_string(native_ref.family.as_str()),
                json_string(&native_ref.kind),
                json_string(&native_ref.value),
            )
        })
        .unwrap_or_else(|| "null".to_string())
}

fn scene_room_ids(scene: &Scene, runtime: &SmartHomeRuntime) -> Vec<String> {
    let mut room_ids = Vec::<String>::new();
    for action in &scene.actions {
        let room_id = runtime
            .registry()
            .entity(&action.entity_id)
            .and_then(|entity| runtime.registry().device(&entity.device_id))
            .and_then(|device| device.room_id.as_ref());
        if let Some(room_id) = room_id {
            push_unique_string(&mut room_ids, room_id);
        }
    }
    room_ids.sort();
    room_ids
}

fn metadata_json(metadata: &[smart_home_core::Metadata]) -> String {
    metadata
        .iter()
        .map(|metadata| {
            format!(
                "{{\"key\":{},\"value\":{}}}",
                json_string(&metadata.key),
                json_string(&metadata.value),
            )
        })
        .collect::<Vec<_>>()
        .join(",")
}

fn entity_registry_json(entity: &Entity, runtime: &SmartHomeRuntime, now_ms: u64) -> String {
    let device = runtime.registry().device(&entity.device_id);
    let bridge_id = device.map(|device| device.bridge_id.as_str());
    let manufacturer = device.map(|device| device.manufacturer.as_str());
    let model = device.map(|device| device.model.as_str());
    let room_id = device.and_then(|device| device.room_id.as_deref());
    let has_state = entity.state.is_some();
    let stale = entity
        .state
        .as_ref()
        .is_none_or(|snapshot| snapshot.is_stale_at(now_ms));
    let state_confidence = entity
        .state
        .as_ref()
        .map(|snapshot| json_string(state_confidence_label(snapshot.confidence)));
    let summary = entity.capability_summary();

    format!(
        "{{\"entity_id\":{},\"home_assistant_entity_id\":{},\"device_id\":{},\"bridge_id\":{},\"name\":{},\"domain\":{},\"entity_kind\":{},\"room_id\":{},\"manufacturer\":{},\"model\":{},\"has_state\":{},\"stale\":{},\"state_confidence\":{},\"capability_summary\":{{\"total\":{},\"observable\":{},\"commandable\":{},\"ranged\":{}}},\"capabilities\":[{}],\"links\":{}}}",
        json_string(entity.entity_id.as_str()),
        json_string(home_assistant_entity_id(entity)),
        json_string(entity.device_id.as_str()),
        optional_str_json(bridge_id),
        json_string(&entity.name),
        json_string(entity_domain(entity.kind)),
        json_string(entity_kind_label(entity.kind)),
        optional_str_json(room_id),
        optional_str_json(manufacturer),
        optional_str_json(model),
        has_state,
        stale,
        state_confidence.unwrap_or_else(|| "null".to_string()),
        summary.total_capabilities,
        summary.observable_capabilities(),
        summary.commandable_capabilities(),
        summary.ranged_capabilities,
        entity
            .capabilities
            .iter()
            .map(capability_json)
            .collect::<Vec<_>>()
            .join(","),
        entity_links_json(entity, runtime),
    )
}

fn entity_links_json(entity: &Entity, runtime: &SmartHomeRuntime) -> String {
    let home_assistant_entity_id = home_assistant_entity_id(entity);
    let entity_ref = url_component(&home_assistant_entity_id);
    let device_ref = url_component(entity.device_id.as_str());
    let device = runtime.registry().device(&entity.device_id);
    let bridge_command_results = device.map(|device| {
        json_string(format!(
            "/api/smart_home/command_results?bridge_id={}&limit=8&sort=status_then_newest",
            url_component(device.bridge_id.as_str())
        ))
    });
    let room_link = device
        .and_then(|device| device.room_id.as_deref())
        .map(|room_id| json_string(format!("/api/smart_home/rooms/{}", url_component(room_id))));
    format!(
        "{{\"self\":{},\"state\":{},\"desired_state\":{},\"history\":{},\"events\":{},\"bridge_command_results\":{},\"device\":{},\"room\":{}}}",
        json_string(format!("/api/smart_home/entities/{entity_ref}")),
        json_string(format!("/api/smart_home/states/{entity_ref}")),
        json_string(format!(
            "/api/smart_home/desired_states?entity_id={entity_ref}"
        )),
        json_string(format!("/api/smart_home/state_history?entity_id={entity_ref}")),
        json_string(format!("/api/smart_home/events?entity_id={entity_ref}")),
        bridge_command_results.unwrap_or_else(|| "null".to_string()),
        json_string(format!("/api/smart_home/devices/{device_ref}")),
        room_link.unwrap_or_else(|| "null".to_string()),
    )
}

fn capability_json(capability: &Capability) -> String {
    format!(
        "{{\"capability_id\":{},\"mode\":{},\"value_kind\":{},\"unit\":{},\"min\":{},\"max\":{},\"step\":{},\"observable\":{},\"commandable\":{}}}",
        json_string(capability.capability_id.as_str()),
        json_string(capability_mode_label(capability.mode)),
        json_string(value_kind_label(capability.value_kind)),
        capability
            .unit
            .as_ref()
            .map(json_string)
            .unwrap_or_else(|| "null".to_string()),
        optional_f64_json(capability.min),
        optional_f64_json(capability.max),
        optional_f64_json(capability.step),
        matches!(
            capability.mode,
            CapabilityMode::Observe | CapabilityMode::ObserveAndCommand
        ),
        capability_allows_command(capability),
    )
}

fn desired_states_json(
    desired_states: &[&DesiredEntityState],
    runtime: &SmartHomeRuntime,
) -> String {
    let desired_capability_count = desired_states
        .iter()
        .map(|desired_state| desired_state.desired.len())
        .sum::<usize>();
    format!(
        "{{\"summary\":{{\"total_desired_states\":{},\"total_desired_capabilities\":{}}},\"desired_states\":[{}]}}",
        desired_states.len(),
        desired_capability_count,
        desired_states
            .iter()
            .map(|desired_state| desired_state_json(desired_state, runtime))
            .collect::<Vec<_>>()
            .join(","),
    )
}

fn desired_state_json(desired_state: &DesiredEntityState, runtime: &SmartHomeRuntime) -> String {
    let home_assistant_entity_id = runtime
        .registry()
        .entity(&desired_state.entity_id)
        .map(home_assistant_entity_id)
        .unwrap_or_else(|| home_assistant_entity_id_for(&desired_state.entity_id));
    format!(
        "{{\"entity_id\":{},\"home_assistant_entity_id\":{},\"requested_by\":{},\"command_timeout_ms\":{},\"desired\":[{}]}}",
        json_string(desired_state.entity_id.as_str()),
        json_string(home_assistant_entity_id),
        json_string(&desired_state.requested_by),
        desired_state.command_timeout_ms,
        desired_state
            .desired
            .iter()
            .map(state_delta_json)
            .collect::<Vec<_>>()
            .join(","),
    )
}

fn runtime_event_query(request: &WebRequest) -> Result<RuntimeEventQuery, ApiError> {
    let mut query = RuntimeEventQuery::new()
        .from_checkpoint(RuntimeEventCheckpoint::from_next_sequence(
            query_u64(request, "from_sequence")?.unwrap_or(0),
        ))
        .with_limit(query_limit(request, 50, 500)?);
    if let Some(to_sequence) = query_u64(request, "to_sequence")? {
        query = query.to_sequence(to_sequence);
    }

    if query_string(request, "sort").is_some_and(|sort| sort == "desc") {
        query = query.sorted_by(RuntimeEventSort::SequenceDesc);
    }
    if let Some(kind) = query_string(request, "kind") {
        query = query.matching(match kind {
            "all" => RuntimeEventFilter::All,
            "commands" | "command_results" => RuntimeEventFilter::Commands,
            "supervision" => RuntimeEventFilter::Supervision,
            other => {
                return Err(ApiError::bad_request(format!(
                    "unsupported event kind `{other}`"
                )));
            }
        });
    }

    Ok(query)
}

fn runtime_command_result_query(
    request: &WebRequest,
) -> Result<RuntimeCommandResultQuery, ApiError> {
    let mut query = RuntimeCommandResultQuery::new()
        .from_checkpoint(RuntimeEventCheckpoint::from_next_sequence(
            query_u64(request, "from_sequence")?.unwrap_or(0),
        ))
        .sorted_by(RuntimeCommandResultSort::SequenceDesc)
        .with_limit(query_limit(request, 50, 500)?);
    if let Some(to_sequence) = query_u64(request, "to_sequence")? {
        query = query.to_sequence(to_sequence);
    }
    if let Some(status) = query_string(request, "status") {
        query = query.with_status(command_status_from_label(status)?);
    }
    if let Some(sort) = query_string(request, "sort") {
        query = query.sorted_by(command_result_sort_from_label(sort)?);
    }
    if let Some(command_id) = query_string(request, "command_id") {
        query = query.for_command(CommandId::trusted(command_id));
    }
    if let Some(bridge_id) = query_string(request, "bridge_id") {
        query = query.for_bridge(BridgeId::trusted(bridge_id));
    }
    if let Some(correlation_id) = query_string(request, "correlation_id") {
        query = query.for_correlation(CorrelationId::trusted(correlation_id));
    }
    Ok(query)
}

fn runtime_pairing_session_query(
    request: &WebRequest,
) -> Result<RuntimePairingSessionQuery, ApiError> {
    let mut query = RuntimePairingSessionQuery::new()
        .with_limit(query_limit(request, 50, 500)?)
        .sorted_by(RuntimePairingSessionSort::StatusThenExpiresAt);
    if let Some(session_id) = query_string(request, "session_id") {
        query = query.for_session(RuntimePairingSessionId::trusted(session_id));
    }
    if let Some(bridge_id) = query_string(request, "bridge_id") {
        query = query.for_bridge(BridgeId::trusted(bridge_id));
    }
    if let Some(integration_id) = query_string(request, "integration_id") {
        query = query.for_integration(IntegrationId::trusted(integration_id));
    }
    if let Some(requested_by) = query_string(request, "requested_by") {
        query = query.requested_by(AgentId::trusted(requested_by));
    }
    if let Some(status) = query_string(request, "status") {
        query = query.with_status(pairing_session_status_from_label(status)?);
    }
    if let Some(expires_before_ms) = query_u64(request, "expires_before_ms")? {
        query = query.expires_before(expires_before_ms);
    }
    if let Some(expiring_at_ms) = query_u64(request, "expiring_at_ms")? {
        query = query.expiring_at(expiring_at_ms);
    }
    if let Some(sort) = query_string(request, "sort") {
        query = query.sorted_by(pairing_session_sort_from_label(sort)?);
    }
    Ok(query)
}

fn runtime_authorization_decision_query(
    request: &WebRequest,
) -> Result<RuntimeAuthorizationDecisionQuery, ApiError> {
    let mut query =
        RuntimeAuthorizationDecisionQuery::new().with_limit(query_limit(request, 50, 500)?);
    if let Some(principal_id) = query_string(request, "principal_id") {
        query = query.for_principal(AgentId::trusted(principal_id));
    }
    if let Some(outcome) = query_string(request, "outcome") {
        query = query.with_outcome(authorization_outcome_from_label(outcome)?);
    }
    if let Some(sort) = query_string(request, "sort") {
        query = query.sorted_by(authorization_decision_sort_from_label(sort)?);
    }
    Ok(query)
}

fn runtime_capability_grant_query(
    runtime: &SmartHomeRuntime,
    request: &WebRequest,
) -> Result<RuntimeCapabilityGrantQuery, ApiError> {
    let mut query = RuntimeCapabilityGrantQuery::new().with_limit(query_limit(request, 50, 500)?);
    if let Some(principal_id) = query_string(request, "principal_id") {
        query = query.for_principal(AgentId::trusted(principal_id));
    }
    if let Some(status) = query_string(request, "status") {
        query = query.with_status(capability_grant_status_from_label(status)?);
    }
    if let Some(scope) = query_string(request, "scope") {
        query = query.with_scope_kind(capability_grant_scope_kind_from_label(scope)?);
    }
    if let Some(capability_id) = query_string(request, "capability_id") {
        query = query.with_capability(CapabilityId::trusted(capability_id));
    }
    if let Some(entity_id) = query_string(request, "entity_id") {
        query = query.for_entity(runtime_entity_id(runtime, entity_id)?);
    }
    if let Some(sort) = query_string(request, "sort") {
        query = query.sorted_by(capability_grant_sort_from_label(sort)?);
    }
    Ok(query)
}

fn desired_state_query(
    runtime: &SmartHomeRuntime,
    request: &WebRequest,
) -> Result<DesiredStateQuery, ApiError> {
    let mut query = DesiredStateQuery::new().with_limit(query_limit(request, 100, 500)?);
    if let Some(entity_id) = query_string(request, "entity_id") {
        query = query.for_entity(runtime_entity_id(runtime, entity_id)?);
    }
    if let Some(requested_by) = query_string(request, "requested_by") {
        query = query.requested_by(requested_by);
    }
    if let Some(capability_id) = query_string(request, "capability_id") {
        query = query.with_capability(CapabilityId::trusted(capability_id));
    }
    Ok(query)
}

fn capability_catalog_query(request: &WebRequest) -> Result<CapabilityCatalogQuery, ApiError> {
    let mut query = CapabilityCatalogQuery::new(query_limit(request, 100, 1_000)?);
    query.domain = query_string(request, "domain").map(str::to_string);
    query.capability_id = query_string(request, "capability_id").map(str::to_string);
    query.commandable = query_bool(request, "commandable")?;
    query.observable = query_bool(request, "observable")?;
    Ok(query)
}

fn runtime_capability_catalog(
    runtime: &SmartHomeRuntime,
    query: &CapabilityCatalogQuery,
) -> Vec<CapabilityCatalogItem> {
    let mut entities = runtime.registry().entities().collect::<Vec<_>>();
    entities.sort_by(|left, right| left.entity_id.as_str().cmp(right.entity_id.as_str()));

    let mut catalog = BTreeMap::<String, CapabilityCatalogItem>::new();
    for entity in entities {
        let domain = entity_domain(entity.kind);
        if query
            .domain
            .as_deref()
            .is_some_and(|filter| filter != domain)
        {
            continue;
        }

        for capability in &entity.capabilities {
            if query
                .capability_id
                .as_deref()
                .is_some_and(|filter| filter != capability.capability_id.as_str())
            {
                continue;
            }
            if query
                .commandable
                .is_some_and(|filter| filter != capability_allows_command(capability))
            {
                continue;
            }
            let observable = matches!(
                capability.mode,
                CapabilityMode::Observe | CapabilityMode::ObserveAndCommand
            );
            if query.observable.is_some_and(|filter| filter != observable) {
                continue;
            }

            let entry = catalog
                .entry(capability.capability_id.as_str().to_string())
                .or_insert_with(|| CapabilityCatalogItem::from_capability(capability));
            entry.add_entity(runtime, entity, capability);
        }
    }

    let mut catalog = catalog.into_values().collect::<Vec<_>>();
    for capability in &mut catalog {
        capability.sort_links();
    }
    catalog.truncate(query.limit);
    catalog
}

fn runtime_entities<'a>(
    runtime: &'a SmartHomeRuntime,
    request: &WebRequest,
) -> Result<Vec<&'a Entity>, ApiError> {
    let domain = query_string(request, "domain");
    let room_id = query_string(request, "room_id");
    let kind = query_string(request, "kind")
        .map(entity_kind_from_label)
        .transpose()?;
    let capability_id = query_string(request, "capability_id");
    let commandable = query_bool(request, "commandable")?;
    let limit = query_limit(request, 100, 1_000)?;

    let mut entities = runtime
        .registry()
        .entities()
        .filter(|entity| domain.is_none_or(|domain| entity_domain(entity.kind) == domain))
        .filter(|entity| {
            room_id.is_none_or(|room_id| entity_room_id(runtime, entity) == Some(room_id))
        })
        .filter(|entity| kind.is_none_or(|kind| entity.kind == kind))
        .filter(|entity| {
            capability_id.is_none_or(|capability_id| {
                entity
                    .capabilities
                    .iter()
                    .any(|capability| capability.capability_id.as_str() == capability_id)
            })
        })
        .filter(|entity| {
            commandable.is_none_or(|commandable| {
                entity.capabilities.iter().any(capability_allows_command) == commandable
            })
        })
        .collect::<Vec<_>>();

    entities.sort_by(|left, right| left.entity_id.as_str().cmp(right.entity_id.as_str()));
    entities.truncate(limit);
    Ok(entities)
}

fn runtime_state_entities<'a>(
    runtime: &'a SmartHomeRuntime,
    request: &WebRequest,
    now_ms: u64,
) -> Result<Vec<&'a Entity>, ApiError> {
    let domain = query_string(request, "domain");
    let room_id = query_string(request, "room_id");
    let kind = query_string(request, "kind")
        .map(entity_kind_from_label)
        .transpose()?;
    let capability_id = query_string(request, "capability_id");
    let has_state = query_bool(request, "has_state")?;
    let stale = query_bool(request, "stale")?;
    let confidence = query_string(request, "confidence")
        .map(state_confidence_from_label)
        .transpose()?;
    let source = query_string(request, "source")
        .map(state_source_from_label)
        .transpose()?;
    let limit = query_limit(request, 100, 1_000)?;

    let mut entities = runtime
        .registry()
        .entities()
        .filter(|entity| domain.is_none_or(|domain| entity_domain(entity.kind) == domain))
        .filter(|entity| {
            room_id.is_none_or(|room_id| entity_room_id(runtime, entity) == Some(room_id))
        })
        .filter(|entity| kind.is_none_or(|kind| entity.kind == kind))
        .filter(|entity| {
            capability_id.is_none_or(|capability_id| {
                entity
                    .capabilities
                    .iter()
                    .any(|capability| capability.capability_id.as_str() == capability_id)
            })
        })
        .filter(|entity| has_state.is_none_or(|has_state| entity.state.is_some() == has_state))
        .filter(|entity| {
            stale.is_none_or(|stale| {
                entity
                    .state
                    .as_ref()
                    .is_none_or(|snapshot| snapshot.is_stale_at(now_ms))
                    == stale
            })
        })
        .filter(|entity| {
            confidence.is_none_or(|confidence| {
                entity
                    .state
                    .as_ref()
                    .is_some_and(|snapshot| snapshot.confidence == confidence)
            })
        })
        .filter(|entity| {
            source.is_none_or(|source| {
                entity
                    .state
                    .as_ref()
                    .is_some_and(|snapshot| snapshot.source == source)
            })
        })
        .collect::<Vec<_>>();

    entities.sort_by(|left, right| left.entity_id.as_str().cmp(right.entity_id.as_str()));
    entities.truncate(limit);
    Ok(entities)
}

fn runtime_state_gap_entities(
    runtime: &SmartHomeRuntime,
    now_ms: u64,
    limit: usize,
) -> Vec<&Entity> {
    let mut entities = runtime
        .registry()
        .entities()
        .filter(|entity| {
            entity
                .state
                .as_ref()
                .is_none_or(|snapshot| snapshot.is_stale_at(now_ms))
        })
        .collect::<Vec<_>>();

    entities.sort_by(|left, right| left.entity_id.as_str().cmp(right.entity_id.as_str()));
    entities.truncate(limit);
    entities
}

fn runtime_scenes<'a>(
    runtime: &'a SmartHomeRuntime,
    request: &WebRequest,
) -> Result<Vec<&'a Scene>, ApiError> {
    let scope = query_string(request, "scope")
        .map(scene_scope_from_label)
        .transpose()?;
    let entity_id = query_string(request, "entity_id")
        .map(|entity_id| runtime_entity_id(runtime, entity_id))
        .transpose()?;
    let room_id = query_string(request, "room_id");
    let limit = query_limit(request, 100, 1_000)?;

    let mut scenes = runtime
        .registry()
        .scenes()
        .filter(|scene| scope.is_none_or(|scope| scene.scope == scope))
        .filter(|scene| {
            entity_id.as_ref().is_none_or(|entity_id| {
                scene
                    .actions
                    .iter()
                    .any(|action| &action.entity_id == entity_id)
            })
        })
        .filter(|scene| {
            room_id.is_none_or(|room_id| {
                scene_room_ids(scene, runtime)
                    .iter()
                    .any(|candidate| candidate == room_id)
            })
        })
        .collect::<Vec<_>>();

    scenes.sort_by(|left, right| left.scene_id.as_str().cmp(right.scene_id.as_str()));
    scenes.truncate(limit);
    Ok(scenes)
}

fn runtime_devices<'a>(
    runtime: &'a SmartHomeRuntime,
    request: &WebRequest,
) -> Result<Vec<&'a Device>, ApiError> {
    let bridge_id = query_string(request, "bridge_id");
    let room_id = query_string(request, "room_id");
    let manufacturer = query_string(request, "manufacturer");
    let health = query_string(request, "health")
        .map(health_from_label)
        .transpose()?;
    let limit = query_limit(request, 100, 1_000)?;

    let mut devices = runtime
        .registry()
        .devices()
        .filter(|device| bridge_id.is_none_or(|bridge_id| device.bridge_id.as_str() == bridge_id))
        .filter(|device| room_id.is_none_or(|room_id| device.room_id.as_deref() == Some(room_id)))
        .filter(|device| {
            manufacturer
                .is_none_or(|manufacturer| device.manufacturer.eq_ignore_ascii_case(manufacturer))
        })
        .filter(|device| health.is_none_or(|health| device.health == health))
        .collect::<Vec<_>>();

    devices.sort_by(|left, right| left.device_id.as_str().cmp(right.device_id.as_str()));
    devices.truncate(limit);
    Ok(devices)
}

fn runtime_bridges<'a>(
    runtime: &'a SmartHomeRuntime,
    request: &WebRequest,
) -> Result<Vec<&'a Bridge>, ApiError> {
    let integration_id = query_string(request, "integration_id");
    let transport = query_string(request, "transport")
        .map(bridge_transport_from_label)
        .transpose()?;
    let health = query_string(request, "health")
        .map(health_from_label)
        .transpose()?;
    let limit = query_limit(request, 100, 1_000)?;

    let mut bridges = runtime
        .registry()
        .bridges()
        .filter(|bridge| {
            integration_id
                .is_none_or(|integration_id| bridge.integration_id.as_str() == integration_id)
        })
        .filter(|bridge| transport.is_none_or(|transport| bridge.transport == transport))
        .filter(|bridge| health.is_none_or(|health| bridge.health == health))
        .collect::<Vec<_>>();

    bridges.sort_by(|left, right| left.bridge_id.as_str().cmp(right.bridge_id.as_str()));
    bridges.truncate(limit);
    Ok(bridges)
}

fn runtime_room_query(request: &WebRequest) -> Result<RuntimeRoomQuery, ApiError> {
    let mut query = RuntimeRoomQuery::new().with_limit(query_limit(request, 100, 1_000)?);
    if let Some(room_id) = query_string(request, "room_id") {
        query = query.for_room(room_id);
    }
    if query_bool(request, "attention_only")?.unwrap_or(false) {
        query = query.attention_only(true);
    }
    if query_bool(request, "state_gaps_only")?.unwrap_or(false) {
        query = query.state_gaps_only(true);
    }
    if let Some(sort) = query_string(request, "sort") {
        query = query.sorted_by(room_sort_from_label(sort)?);
    }
    Ok(query)
}

fn state_history_events<'a>(
    runtime: &'a SmartHomeRuntime,
    request: &WebRequest,
) -> Result<Vec<&'a DeviceEvent>, ApiError> {
    let entity_id = history_entity_filter(request)
        .map(|entity_id| runtime_entity_id(runtime, entity_id))
        .transpose()?;
    let event_type = query_string(request, "event_type")
        .map(device_event_type_from_label)
        .transpose()?;
    let bridge_id = query_string(request, "bridge_id");
    let room_id = query_string(request, "room_id");
    let observed_at_or_after_ms = history_from_ms(request)?;
    let observed_at_or_before_ms = history_to_ms(request)?;
    let received_at_or_after_ms = query_u64(request, "received_at_or_after_ms")?;
    let received_at_or_before_ms = query_u64(request, "received_at_or_before_ms")?;
    let limit = query_limit(request, 100, 1_000)?;

    let mut events = runtime
        .registry()
        .events()
        .filter(|event| {
            entity_id
                .as_ref()
                .is_none_or(|entity_id| event.entity_id.as_ref() == Some(entity_id))
        })
        .filter(|event| bridge_id.is_none_or(|bridge_id| event.bridge_id.as_str() == bridge_id))
        .filter(|event| {
            room_id.is_none_or(|room_id| device_event_matches_room(runtime, event, room_id))
        })
        .filter(|event| event_type.is_none_or(|event_type| event.event_type == event_type))
        .filter(|event| {
            observed_at_or_after_ms
                .is_none_or(|observed_at_ms| event.observed_at_ms >= observed_at_ms)
        })
        .filter(|event| {
            observed_at_or_before_ms
                .is_none_or(|observed_at_ms| event.observed_at_ms <= observed_at_ms)
        })
        .filter(|event| {
            received_at_or_after_ms
                .is_none_or(|received_at_ms| event.received_at_ms >= received_at_ms)
        })
        .filter(|event| {
            received_at_or_before_ms
                .is_none_or(|received_at_ms| event.received_at_ms <= received_at_ms)
        })
        .collect::<Vec<_>>();

    if query_string(request, "sort").is_some_and(|sort| sort == "desc") {
        events.reverse();
    }
    events.truncate(limit);
    Ok(events)
}

fn history_entity_filter(request: &WebRequest) -> Option<&str> {
    query_string(request, "entity_id").or_else(|| query_string(request, "filter_entity_id"))
}

fn history_from_ms(request: &WebRequest) -> Result<Option<u64>, ApiError> {
    if let Some(start_time) = request.route_params.get("start_time") {
        return parse_u64("start_time", start_time).map(Some);
    }
    query_u64(request, "from_ms")?.map_or_else(
        || query_u64(request, "observed_at_or_after_ms"),
        |from_ms| Ok(Some(from_ms)),
    )
}

fn history_to_ms(request: &WebRequest) -> Result<Option<u64>, ApiError> {
    if let Some(to_ms) = query_u64(request, "to_ms")? {
        return Ok(Some(to_ms));
    }
    if let Some(end_time) = query_u64(request, "end_time")? {
        return Ok(Some(end_time));
    }
    query_u64(request, "observed_at_or_before_ms")
}

fn runtime_entity_id(runtime: &SmartHomeRuntime, value: &str) -> Result<EntityId, ApiError> {
    runtime
        .registry()
        .entities()
        .find(|entity| entity_matches_external_id(entity, value))
        .map(|entity| entity.entity_id.clone())
        .ok_or_else(|| ApiError::not_found(format!("entity `{value}` not found")))
}

fn runtime_entity(runtime: &SmartHomeRuntime, value: &str) -> Result<Entity, ApiError> {
    runtime
        .registry()
        .entities()
        .find(|entity| entity_matches_external_id(entity, value))
        .cloned()
        .ok_or_else(|| ApiError::not_found(format!("entity `{value}` not found")))
}

fn home_assistant_entity_id_for_runtime(
    runtime: &SmartHomeRuntime,
    entity_id: &EntityId,
) -> String {
    runtime
        .registry()
        .entity(entity_id)
        .map(home_assistant_entity_id)
        .unwrap_or_else(|| home_assistant_entity_id_for(entity_id))
}

fn state_history_json(events: &[&DeviceEvent], runtime: &SmartHomeRuntime) -> String {
    let mut entity_ids = Vec::<String>::new();
    let mut state_delta_count = 0usize;
    let mut first_observed_at_ms = None;
    let mut latest_observed_at_ms = None;

    for event in events {
        if let Some(entity_id) = &event.entity_id {
            push_unique_string(&mut entity_ids, entity_id.as_str());
        }
        if event.state_delta.is_some() {
            state_delta_count += 1;
        }
        first_observed_at_ms = Some(
            first_observed_at_ms
                .map(|current: u64| current.min(event.observed_at_ms))
                .unwrap_or(event.observed_at_ms),
        );
        latest_observed_at_ms = Some(
            latest_observed_at_ms
                .map(|current: u64| current.max(event.observed_at_ms))
                .unwrap_or(event.observed_at_ms),
        );
    }

    format!(
        "{{\"summary\":{{\"total_events\":{},\"entity_count\":{},\"state_delta_count\":{},\"first_observed_at_ms\":{},\"latest_observed_at_ms\":{}}},\"events\":[{}]}}",
        events.len(),
        entity_ids.len(),
        state_delta_count,
        optional_u64_json(first_observed_at_ms),
        optional_u64_json(latest_observed_at_ms),
        events
            .iter()
            .map(|event| state_history_event_json(event, runtime))
            .collect::<Vec<_>>()
            .join(","),
    )
}

fn home_assistant_history_json(events: &[&DeviceEvent], runtime: &SmartHomeRuntime) -> String {
    let mut by_entity = BTreeMap::<String, Vec<&DeviceEvent>>::new();
    for event in events {
        let key = event
            .entity_id
            .as_ref()
            .and_then(|entity_id| runtime.registry().entity(entity_id))
            .map(home_assistant_entity_id)
            .unwrap_or_else(|| "unknown.unknown".to_string());
        by_entity.entry(key).or_default().push(event);
    }

    format!(
        "[{}]",
        by_entity
            .into_values()
            .map(|events| {
                format!(
                    "[{}]",
                    events
                        .iter()
                        .map(|event| home_assistant_history_event_json(event, runtime))
                        .collect::<Vec<_>>()
                        .join(",")
                )
            })
            .collect::<Vec<_>>()
            .join(",")
    )
}

fn home_assistant_history_event_json(event: &DeviceEvent, runtime: &SmartHomeRuntime) -> String {
    let entity = event
        .entity_id
        .as_ref()
        .and_then(|entity_id| runtime.registry().entity(entity_id));
    let home_assistant_entity_id = entity
        .map(home_assistant_entity_id)
        .unwrap_or_else(|| "unknown.unknown".to_string());
    let canonical_entity_id = event.entity_id.as_ref().map(|entity_id| entity_id.as_str());
    let (state, capability_id, state_delta_value) = match &event.state_delta {
        Some(delta) => (
            value_json(&delta.value),
            json_string(delta.capability_id.as_str()),
            value_json(&delta.value),
        ),
        None => (
            json_string(device_event_type_label(event.event_type)),
            "null".to_string(),
            "null".to_string(),
        ),
    };

    format!(
        "{{\"entity_id\":{},\"state\":{},\"attributes\":{{\"canonical_entity_id\":{},\"event_id\":{},\"bridge_id\":{},\"device_id\":{},\"event_type\":{},\"capability_id\":{},\"state_delta_value\":{},\"raw_ref\":{}}},\"last_changed_ms\":{},\"last_updated_ms\":{},\"context\":{{\"source\":\"event_stream\",\"correlation_id\":{}}}}}",
        json_string(home_assistant_entity_id),
        state,
        canonical_entity_id
            .map(json_string)
            .unwrap_or_else(|| "null".to_string()),
        json_string(event.event_id.as_str()),
        json_string(event.bridge_id.as_str()),
        event
            .device_id
            .as_ref()
            .map(|device_id| json_string(device_id.as_str()))
            .unwrap_or_else(|| "null".to_string()),
        json_string(device_event_type_label(event.event_type)),
        capability_id,
        state_delta_value,
        event
            .raw_ref
            .as_ref()
            .map(json_string)
            .unwrap_or_else(|| "null".to_string()),
        event.observed_at_ms,
        event.received_at_ms,
        event
            .correlation_id
            .as_ref()
            .map(|correlation_id| json_string(correlation_id.as_str()))
            .unwrap_or_else(|| "null".to_string()),
    )
}

fn state_history_event_json(event: &DeviceEvent, runtime: &SmartHomeRuntime) -> String {
    let home_assistant_entity_id = event.entity_id.as_ref().and_then(|entity_id| {
        runtime
            .registry()
            .entity(entity_id)
            .map(home_assistant_entity_id)
    });
    format!(
        "{{\"home_assistant_entity_id\":{},\"event\":{}}}",
        home_assistant_entity_id
            .map(json_string)
            .unwrap_or_else(|| "null".to_string()),
        device_event_json(event),
    )
}

#[derive(Debug, Clone, PartialEq)]
struct ServiceCall {
    target_entity_ids: Vec<String>,
    target_scene_ids: Vec<String>,
    body: JsonValue,
    idempotency_key: Option<String>,
    timeout_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq)]
struct ServiceCommand {
    entity_id: EntityId,
    command_type: CommandType,
    arguments: Value,
    idempotency_key: Option<String>,
    timeout_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq)]
struct AuthorizationCommandPreview {
    entity: Entity,
    command: DeviceCommand,
    decision: AuthorizationDecision,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ApiError {
    status: u16,
    message: String,
}

impl ApiError {
    fn new(status: u16, message: impl Into<String>) -> Self {
        Self {
            status,
            message: message.into(),
        }
    }

    fn bad_request(message: impl Into<String>) -> Self {
        Self::new(400, message)
    }

    fn not_found(message: impl Into<String>) -> Self {
        Self::new(404, message)
    }

    fn forbidden(message: impl Into<String>) -> Self {
        Self::new(403, message)
    }
}

fn preview_command(
    entity: &Entity,
    command_type: CommandType,
    principal_id: &AgentId,
    now_ms: u64,
) -> Result<DeviceCommand, ApiError> {
    DeviceCommand::new(
        CommandId::trusted(format!(
            "preview:{}:{}:{}:{now_ms}",
            principal_id.as_str(),
            entity.entity_id.as_str(),
            command_type_label(command_type)
        )),
        entity.entity_id.clone(),
        command_type,
        Value::Null,
        principal_id.as_str(),
        CorrelationId::trusted(format!(
            "preview:{}:{}:{}:{now_ms}",
            principal_id.as_str(),
            entity.entity_id.as_str(),
            command_type_label(command_type)
        )),
    )
    .map_err(|error| ApiError::bad_request(format!("invalid command preview: {error}")))
}

fn preview_service_command(
    command: &ServiceCommand,
    principal_id: &AgentId,
    now_ms: u64,
    sequence: usize,
) -> Result<DeviceCommand, ApiError> {
    DeviceCommand::new(
        CommandId::trusted(format!(
            "preview:{}:{}:{}:{sequence}:{now_ms}",
            principal_id.as_str(),
            command.entity_id.as_str(),
            command_type_label(command.command_type)
        )),
        command.entity_id.clone(),
        command.command_type,
        command.arguments.clone(),
        principal_id.as_str(),
        CorrelationId::trusted(format!(
            "preview:{}:{}:{}:{sequence}:{now_ms}",
            principal_id.as_str(),
            command.entity_id.as_str(),
            command_type_label(command.command_type)
        )),
    )
    .map_err(|error| ApiError::bad_request(format!("invalid service command preview: {error}")))
}

fn authorization_command_previews(
    runtime: &SmartHomeRuntime,
    service_commands: &[ServiceCommand],
    principal_id: &AgentId,
    grants: &[&CapabilityGrant],
    now_ms: u64,
) -> Result<Vec<AuthorizationCommandPreview>, ApiError> {
    let mut command_previews = Vec::new();
    for (index, service_command) in service_commands.iter().enumerate() {
        let entity = runtime
            .registry()
            .entity(&service_command.entity_id)
            .cloned()
            .ok_or_else(|| {
                ApiError::not_found(format!(
                    "entity `{}` not found",
                    service_command.entity_id.as_str()
                ))
            })?;
        let command = preview_service_command(service_command, principal_id, now_ms, index)?;
        let decision = AuthorizationDecision::for_command(
            principal_id.clone(),
            &command,
            grants.iter().copied(),
            now_ms,
        );
        command_previews.push(AuthorizationCommandPreview {
            entity,
            command,
            decision,
        });
    }
    Ok(command_previews)
}

fn unsupported_command_capabilities(entity: &Entity, command: &DeviceCommand) -> Vec<CapabilityId> {
    command
        .required_capabilities
        .iter()
        .filter(|required| {
            !entity
                .capabilities
                .iter()
                .any(|capability| capability.capability_id == **required)
        })
        .cloned()
        .collect()
}

fn read_only_command_capabilities(entity: &Entity, command: &DeviceCommand) -> Vec<CapabilityId> {
    command
        .required_capabilities
        .iter()
        .filter(|required| {
            entity
                .capabilities
                .iter()
                .find(|capability| capability.capability_id == **required)
                .is_some_and(|capability| !capability_allows_command(capability))
        })
        .cloned()
        .collect()
}

fn unique_capability_ids<'a>(
    capability_ids: impl Iterator<Item = &'a CapabilityId>,
) -> Vec<CapabilityId> {
    let mut capability_ids = capability_ids.cloned().collect::<Vec<_>>();
    capability_ids.sort();
    capability_ids.dedup();
    capability_ids
}

fn unique_grant_ids<'a>(
    grant_ids: impl Iterator<Item = &'a CapabilityGrantId>,
) -> Vec<CapabilityGrantId> {
    let mut grant_ids = grant_ids.cloned().collect::<Vec<_>>();
    grant_ids.sort();
    grant_ids.dedup();
    grant_ids
}

fn set_desired_state_response(
    runtime: &SmartHomePlatformHttpRuntime,
    request: &WebRequest,
    allow_home_assistant_state_body: bool,
) -> WebResponse {
    let Some(target) = request.route_params.get("entity_id") else {
        return json_error(400, "missing entity_id");
    };

    let mut runtime_guard = runtime
        .runtime
        .lock()
        .expect("smart-home runtime mutex should not be poisoned");
    let entity = match runtime_entity(&runtime_guard, target) {
        Ok(entity) => entity,
        Err(error) => return api_error_response(error),
    };
    let desired_state = match parse_desired_state_request(
        request.body(),
        &entity,
        runtime.principal_id.as_str(),
        allow_home_assistant_state_body,
    ) {
        Ok(desired_state) => desired_state,
        Err(error) => return api_error_response(error),
    };

    let now_ms = runtime.now_ms();
    let previous = runtime_guard.clone();
    let output = match runtime_guard.execute_set_desired_state_tool(
        runtime.principal_id.clone(),
        RuntimeSetDesiredStateToolRequest::new(desired_state),
        now_ms,
    ) {
        Ok(output) => output,
        Err(error) => return api_error_response(runtime_error_to_api_error(error)),
    };
    if let Err(error) = runtime.persist_mutation_or_restore(&mut runtime_guard, previous, now_ms) {
        return api_error_response(error);
    }
    let query = DesiredStateQuery::new().for_entity(entity.entity_id.clone());
    let desired_states = runtime_guard.query_desired_states(&query);
    WebResponse::json(set_desired_state_json(&output, &desired_states, &runtime_guard).into_bytes())
}

fn clear_desired_state_response(
    runtime: &SmartHomePlatformHttpRuntime,
    request: &WebRequest,
) -> WebResponse {
    let Some(target) = request.route_params.get("entity_id") else {
        return json_error(400, "missing entity_id");
    };

    let mut runtime_guard = runtime
        .runtime
        .lock()
        .expect("smart-home runtime mutex should not be poisoned");
    let entity_id = match runtime_entity_id(&runtime_guard, target) {
        Ok(entity_id) => entity_id,
        Err(error) => return api_error_response(error),
    };

    let now_ms = runtime.now_ms();
    let previous = runtime_guard.clone();
    let output = match runtime_guard.execute_clear_desired_state_tool(
        runtime.principal_id.clone(),
        RuntimeClearDesiredStateToolRequest::new(entity_id),
        now_ms,
    ) {
        Ok(output) => output,
        Err(error) => return api_error_response(runtime_error_to_api_error(error)),
    };
    if let Err(error) = runtime.persist_mutation_or_restore(&mut runtime_guard, previous, now_ms) {
        return api_error_response(error);
    }
    let query = DesiredStateQuery::new().for_entity(output.entity_id.clone());
    let desired_states = runtime_guard.query_desired_states(&query);
    WebResponse::json(
        clear_desired_state_json(&output, &desired_states, &runtime_guard).into_bytes(),
    )
}

fn parse_desired_state_request(
    body: &[u8],
    entity: &Entity,
    default_requested_by: &str,
    allow_home_assistant_state_body: bool,
) -> Result<DesiredEntityState, ApiError> {
    let body = parse_json_body(body)?;
    let desired = if let Some(value) = body.get("desired_state").or_else(|| body.get("desired")) {
        desired_state_deltas_from_json(value)?
    } else if allow_home_assistant_state_body {
        home_assistant_state_deltas(entity, &body)?
    } else {
        return Err(ApiError::bad_request(
            "desired-state request requires a desired_state object",
        ));
    };

    if desired.is_empty() {
        return Err(ApiError::bad_request(
            "desired-state request must include at least one capability",
        ));
    }

    let requested_by = json_string_field(&body, "requested_by")
        .unwrap_or_else(|| default_requested_by.to_string());
    let mut desired_state =
        DesiredEntityState::new(entity.entity_id.clone(), desired).requested_by(requested_by);
    if let Some(timeout_ms) =
        json_u64_field(&body, "command_timeout_ms").or_else(|| json_u64_field(&body, "timeout_ms"))
    {
        desired_state = desired_state.with_command_timeout(timeout_ms);
    }
    Ok(desired_state)
}

fn parse_json_body(body: &[u8]) -> Result<JsonValue, ApiError> {
    if body.is_empty() {
        return Err(ApiError::bad_request("JSON body is required"));
    }
    serde_json::from_slice(body)
        .map_err(|error| ApiError::bad_request(format!("invalid JSON body: {error}")))
}

fn desired_state_deltas_from_json(value: &JsonValue) -> Result<Vec<StateDelta>, ApiError> {
    let fields = value
        .as_object()
        .ok_or_else(|| ApiError::bad_request("desired_state must be an object"))?;
    let mut deltas = Vec::new();
    for (capability_id, value) in fields {
        deltas.push(StateDelta {
            capability_id: CapabilityId::trusted(capability_id.clone()),
            value: json_capability_value(capability_id, value)?,
        });
    }
    deltas.sort_by(|left, right| {
        left.capability_id
            .as_str()
            .cmp(right.capability_id.as_str())
    });
    Ok(deltas)
}

fn home_assistant_state_deltas(
    entity: &Entity,
    body: &JsonValue,
) -> Result<Vec<StateDelta>, ApiError> {
    let attributes = body.get("attributes").unwrap_or(body);
    let mut deltas = Vec::new();
    match entity_domain(entity.kind) {
        "light" => {
            if let Some(state) = json_string_field(body, "state") {
                match state.as_str() {
                    "on" => deltas.push(state_delta("light.on_off", Value::Bool(true))),
                    "off" => deltas.push(state_delta("light.on_off", Value::Bool(false))),
                    other => {
                        return Err(ApiError::bad_request(format!(
                            "unsupported light state `{other}`"
                        )));
                    }
                }
            }
            if let Some(value) = brightness_value(attributes)? {
                deltas.push(state_delta("light.brightness", value));
            }
            if let Some(value) = color_temperature_value(attributes)? {
                deltas.push(state_delta("light.color_temperature", value));
            }
            if let Some(value) = color_value(attributes)? {
                deltas.push(state_delta("light.color", value));
            }
        }
        "lock" => {
            let state = json_string_field(body, "state")
                .ok_or_else(|| ApiError::bad_request("lock desired state requires state"))?;
            match state.as_str() {
                "locked" | "unlocked" => deltas.push(state_delta("lock.state", Value::Text(state))),
                other => {
                    return Err(ApiError::bad_request(format!(
                        "unsupported lock state `{other}`"
                    )));
                }
            }
        }
        "climate" => {
            let value = number_or_integer_field(attributes, "temperature")
                .or_else(|| number_or_integer_field(body, "temperature"))
                .ok_or_else(|| {
                    ApiError::bad_request("climate desired state requires temperature")
                })?;
            deltas.push(state_delta("climate.setpoint", value));
        }
        domain => {
            return Err(ApiError::bad_request(format!(
                "Home Assistant state body is not supported for domain `{domain}`; use desired_state"
            )));
        }
    }

    deltas.sort_by(|left, right| {
        left.capability_id
            .as_str()
            .cmp(right.capability_id.as_str())
    });
    deltas.dedup_by(|left, right| left.capability_id == right.capability_id);
    Ok(deltas)
}

fn state_delta(capability_id: impl Into<String>, value: Value) -> StateDelta {
    StateDelta {
        capability_id: CapabilityId::trusted(capability_id.into()),
        value,
    }
}

fn json_capability_value(capability_id: &str, value: &JsonValue) -> Result<Value, ApiError> {
    match capability_id {
        "light.on_off" => match value {
            JsonValue::Bool(value) => Ok(Value::Bool(*value)),
            JsonValue::String(value) if value == "on" => Ok(Value::Bool(true)),
            JsonValue::String(value) if value == "off" => Ok(Value::Bool(false)),
            _ => Err(ApiError::bad_request("light.on_off must be boolean")),
        },
        "light.brightness" => json_percentage_value(value, capability_id),
        "light.color_temperature" => json_i64_value(value, capability_id).map(Value::Integer),
        "lock.state" => value
            .as_str()
            .map(|state| Value::Text(state.to_string()))
            .ok_or_else(|| ApiError::bad_request("lock.state must be a string")),
        "climate.setpoint" => json_number_or_integer_value(value, capability_id),
        _ => json_value_to_value(value),
    }
}

fn json_percentage_value(value: &JsonValue, field: &str) -> Result<Value, ApiError> {
    let value = value
        .as_u64()
        .ok_or_else(|| ApiError::bad_request(format!("{field} must be an integer percentage")))?;
    if value > 100 {
        return Err(ApiError::bad_request(format!(
            "{field} must be between 0 and 100"
        )));
    }
    Ok(Value::Percentage(value as u8))
}

fn json_i64_value(value: &JsonValue, field: &str) -> Result<i64, ApiError> {
    value
        .as_i64()
        .ok_or_else(|| ApiError::bad_request(format!("{field} must be an integer")))
}

fn json_number_or_integer_value(value: &JsonValue, field: &str) -> Result<Value, ApiError> {
    value
        .as_i64()
        .map(Value::Integer)
        .or_else(|| value.as_f64().map(Value::Number))
        .ok_or_else(|| ApiError::bad_request(format!("{field} must be numeric")))
}

fn json_value_to_value(value: &JsonValue) -> Result<Value, ApiError> {
    match value {
        JsonValue::Null => Ok(Value::Null),
        JsonValue::Bool(value) => Ok(Value::Bool(*value)),
        JsonValue::Number(value) => value
            .as_i64()
            .map(Value::Integer)
            .or_else(|| value.as_f64().map(Value::Number))
            .ok_or_else(|| ApiError::bad_request("JSON number is not representable")),
        JsonValue::String(value) => Ok(Value::Text(value.clone())),
        JsonValue::Array(values) => values
            .iter()
            .map(json_value_to_value)
            .collect::<Result<Vec<_>, _>>()
            .map(Value::Array),
        JsonValue::Object(fields) => {
            let mut fields = fields
                .iter()
                .map(|(key, value)| Ok((key.clone(), json_value_to_value(value)?)))
                .collect::<Result<Vec<_>, ApiError>>()?;
            fields.sort_by(|left, right| left.0.cmp(&right.0));
            Ok(Value::Object(fields))
        }
    }
}

fn set_desired_state_json(
    output: &RuntimeSetDesiredStateToolOutput,
    desired_states: &[&DesiredEntityState],
    runtime: &SmartHomeRuntime,
) -> String {
    format!(
        "{{\"entity_id\":{},\"home_assistant_entity_id\":{},\"replaced\":{},\"desired_state\":{},\"previous\":{},\"desired_states\":{}}}",
        json_string(output.desired_state.entity_id.as_str()),
        json_string(home_assistant_entity_id_for_runtime(
            runtime,
            &output.desired_state.entity_id,
        )),
        output.replaced,
        desired_state_json(&output.desired_state, runtime),
        output
            .previous
            .as_ref()
            .map(|desired_state| desired_state_json(desired_state, runtime))
            .unwrap_or_else(|| "null".to_string()),
        desired_states_json(desired_states, runtime),
    )
}

fn clear_desired_state_json(
    output: &RuntimeClearDesiredStateToolOutput,
    desired_states: &[&DesiredEntityState],
    runtime: &SmartHomeRuntime,
) -> String {
    format!(
        "{{\"entity_id\":{},\"home_assistant_entity_id\":{},\"removed\":{},\"removed_desired_state\":{},\"desired_states\":{}}}",
        json_string(output.entity_id.as_str()),
        json_string(home_assistant_entity_id_for_runtime(
            runtime,
            &output.entity_id,
        )),
        output.removed(),
        output
            .removed
            .as_ref()
            .map(|desired_state| desired_state_json(desired_state, runtime))
            .unwrap_or_else(|| "null".to_string()),
        desired_states_json(desired_states, runtime),
    )
}

fn service_call_response(
    runtime: &SmartHomePlatformHttpRuntime,
    request: &WebRequest,
) -> WebResponse {
    let domain = match request.route_params.get("domain") {
        Some(domain) => domain.as_str(),
        None => return json_error(400, "missing domain"),
    };
    let service = match request.route_params.get("service") {
        Some(service) => service.as_str(),
        None => return json_error(400, "missing service"),
    };

    let call = match parse_service_call(request.body()) {
        Ok(call) => call,
        Err(error) => return api_error_response(error),
    };

    let mut runtime_guard = runtime
        .runtime
        .lock()
        .expect("smart-home runtime mutex should not be poisoned");
    let now_ms = runtime.now_ms();
    let before = SmartHomePlatformHttpState::from_runtime(
        &runtime_guard,
        runtime.config.clone(),
        runtime.event_types.clone(),
        now_ms,
    );
    let commands = match service_commands(&before, domain, service, &call) {
        Ok(commands) => commands,
        Err(error) => return api_error_response(error),
    };

    let previous = runtime_guard.clone();
    let mut results = Vec::new();
    for command in commands {
        let mut request = RuntimeCommandToolRequest::new(
            command.entity_id,
            command.command_type,
            command.arguments,
        );
        if let Some(idempotency_key) = command.idempotency_key {
            request = request.with_idempotency_key(idempotency_key);
        }
        if let Some(timeout_ms) = command.timeout_ms {
            request = request.with_timeout_ms(timeout_ms);
        }

        match runtime_guard.execute_command_tool(runtime.principal_id.clone(), request, now_ms) {
            Ok(result) => results.push(result),
            Err(error) => {
                *runtime_guard = previous;
                return api_error_response(runtime_error_to_api_error(error));
            }
        }
    }
    if !results.is_empty() {
        if let Err(error) =
            runtime.persist_mutation_or_restore(&mut runtime_guard, previous, now_ms)
        {
            return api_error_response(error);
        }
    }

    let after = SmartHomePlatformHttpState::from_runtime(
        &runtime_guard,
        runtime.config.clone(),
        runtime.event_types.clone(),
        now_ms,
    );
    WebResponse::json(service_call_json(domain, service, &results, &after).into_bytes())
}

fn parse_service_call(body: &[u8]) -> Result<ServiceCall, ApiError> {
    let body = if body.is_empty() {
        JsonValue::Object(Default::default())
    } else {
        serde_json::from_slice(body)
            .map_err(|error| ApiError::bad_request(format!("invalid JSON body: {error}")))?
    };
    service_call_from_json(body)
}

fn service_call_from_json(body: JsonValue) -> Result<ServiceCall, ApiError> {
    let mut target_entity_ids = Vec::new();
    let mut target_scene_ids = Vec::new();
    collect_string_values(&body, "entity_id", &mut target_entity_ids);
    collect_string_values(&body, "entity_ids", &mut target_entity_ids);
    collect_string_values(&body, "scene_id", &mut target_scene_ids);
    collect_string_values(&body, "scene_ids", &mut target_scene_ids);

    if let Some(target) = body.get("target") {
        collect_string_values(target, "entity_id", &mut target_entity_ids);
        collect_string_values(target, "entity_ids", &mut target_entity_ids);
        collect_string_values(target, "scene_id", &mut target_scene_ids);
        collect_string_values(target, "scene_ids", &mut target_scene_ids);
    }

    target_entity_ids.sort();
    target_entity_ids.dedup();
    target_scene_ids.sort();
    target_scene_ids.dedup();

    Ok(ServiceCall {
        idempotency_key: json_string_field(&body, "idempotency_key"),
        timeout_ms: json_u64_field(&body, "timeout_ms"),
        target_entity_ids,
        target_scene_ids,
        body,
    })
}

fn parse_service_call_query(request: &WebRequest) -> Result<ServiceCall, ApiError> {
    let mut body = serde_json::Map::new();
    insert_query_string(&mut body, request, "entity_id");
    insert_query_string_array(&mut body, request, "entity_ids");
    insert_query_string(&mut body, request, "scene_id");
    insert_query_string_array(&mut body, request, "scene_ids");
    insert_query_string(&mut body, request, "idempotency_key");

    for field in [
        "brightness_pct",
        "brightness",
        "color_temp_kelvin",
        "kelvin",
        "color_temp",
        "timeout_ms",
    ] {
        insert_query_u64(&mut body, request, field)?;
    }
    insert_query_number(&mut body, request, "temperature")?;
    insert_query_rgb_color(&mut body, request)?;

    service_call_from_json(JsonValue::Object(body))
}

fn insert_query_string(
    body: &mut serde_json::Map<String, JsonValue>,
    request: &WebRequest,
    field: &str,
) {
    if let Some(value) = query_string(request, field) {
        body.insert(field.to_string(), JsonValue::String(value.to_string()));
    }
}

fn insert_query_string_array(
    body: &mut serde_json::Map<String, JsonValue>,
    request: &WebRequest,
    field: &str,
) {
    if let Some(value) = query_string(request, field) {
        body.insert(
            field.to_string(),
            JsonValue::Array(
                value
                    .split(',')
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(|value| JsonValue::String(value.to_string()))
                    .collect(),
            ),
        );
    }
}

fn insert_query_u64(
    body: &mut serde_json::Map<String, JsonValue>,
    request: &WebRequest,
    field: &str,
) -> Result<(), ApiError> {
    if let Some(value) = query_u64(request, field)? {
        body.insert(
            field.to_string(),
            JsonValue::Number(serde_json::Number::from(value)),
        );
    }
    Ok(())
}

fn insert_query_number(
    body: &mut serde_json::Map<String, JsonValue>,
    request: &WebRequest,
    field: &str,
) -> Result<(), ApiError> {
    if let Some(value) = query_string(request, field) {
        body.insert(
            field.to_string(),
            JsonValue::Number(json_number_from_query(field, value)?),
        );
    }
    Ok(())
}

fn insert_query_rgb_color(
    body: &mut serde_json::Map<String, JsonValue>,
    request: &WebRequest,
) -> Result<(), ApiError> {
    let Some(value) = query_string(request, "rgb_color") else {
        return Ok(());
    };
    let channels = value
        .split(',')
        .map(str::trim)
        .filter(|channel| !channel.is_empty())
        .map(|channel| parse_u64("rgb_color", channel))
        .collect::<Result<Vec<_>, _>>()?;
    body.insert(
        "rgb_color".to_string(),
        JsonValue::Array(
            channels
                .into_iter()
                .map(|channel| JsonValue::Number(serde_json::Number::from(channel)))
                .collect(),
        ),
    );
    Ok(())
}

fn service_commands(
    state: &SmartHomePlatformHttpState,
    domain: &str,
    service: &str,
    call: &ServiceCall,
) -> Result<Vec<ServiceCommand>, ApiError> {
    if domain == "scene" && service == "turn_on" {
        return scene_service_commands(state, call);
    }

    let entities = target_entities(state, domain, call)?;
    let mut commands = Vec::new();
    for entity in entities {
        commands.extend(entity_service_commands(domain, service, entity, call)?);
    }
    Ok(commands)
}

fn target_entities<'a>(
    state: &'a SmartHomePlatformHttpState,
    domain: &str,
    call: &ServiceCall,
) -> Result<Vec<&'a Entity>, ApiError> {
    if call.target_entity_ids.is_empty() {
        return Err(ApiError::bad_request(
            "service call requires an entity target",
        ));
    }

    let mut entities = Vec::new();
    for target in &call.target_entity_ids {
        let entity = state
            .entities
            .iter()
            .find(|entity| entity_matches_external_id(entity, target))
            .ok_or_else(|| ApiError::not_found(format!("entity target `{target}` not found")))?;
        if entity_domain(entity.kind) != domain {
            return Err(ApiError::bad_request(format!(
                "entity target `{target}` is not in domain `{domain}`"
            )));
        }
        entities.push(entity);
    }
    Ok(entities)
}

fn scene_service_commands(
    state: &SmartHomePlatformHttpState,
    call: &ServiceCall,
) -> Result<Vec<ServiceCommand>, ApiError> {
    if call.target_scene_ids.is_empty() && call.target_entity_ids.is_empty() {
        return Err(ApiError::bad_request(
            "scene.turn_on requires a scene target",
        ));
    }

    let mut commands = Vec::new();
    for target in call
        .target_scene_ids
        .iter()
        .chain(call.target_entity_ids.iter())
    {
        let scene = state
            .scenes
            .iter()
            .find(|scene| scene_matches_external_id(scene, target))
            .ok_or_else(|| ApiError::not_found(format!("scene target `{target}` not found")))?;
        for action in &scene.actions {
            for delta in state_deltas_from_value(&action.desired_state)? {
                let (command_type, arguments) =
                    command_from_capability_value(&action.entity_id, &delta)?;
                commands.push(ServiceCommand {
                    entity_id: action.entity_id.clone(),
                    command_type,
                    arguments,
                    idempotency_key: call.idempotency_key.clone(),
                    timeout_ms: call.timeout_ms,
                });
            }
        }
    }
    Ok(commands)
}

fn entity_service_commands(
    domain: &str,
    service: &str,
    entity: &Entity,
    call: &ServiceCall,
) -> Result<Vec<ServiceCommand>, ApiError> {
    let mut commands = Vec::new();
    match (domain, service) {
        ("light", "turn_on") => {
            commands.push(service_command(
                entity,
                CommandType::TurnOn,
                Value::Null,
                call,
            ));
            if let Some(value) = brightness_value(&call.body)? {
                commands.push(service_command(
                    entity,
                    CommandType::SetBrightness,
                    value,
                    call,
                ));
            }
            if let Some(value) = color_temperature_value(&call.body)? {
                commands.push(service_command(
                    entity,
                    CommandType::SetColorTemperature,
                    value,
                    call,
                ));
            }
            if let Some(value) = color_value(&call.body)? {
                commands.push(service_command(entity, CommandType::SetColor, value, call));
            }
        }
        ("light", "turn_off") => {
            commands.push(service_command(
                entity,
                CommandType::TurnOff,
                Value::Null,
                call,
            ));
        }
        ("light", "set_brightness") => {
            let value = brightness_value(&call.body)?.ok_or_else(|| {
                ApiError::bad_request("light.set_brightness requires brightness_pct or brightness")
            })?;
            commands.push(service_command(
                entity,
                CommandType::SetBrightness,
                value,
                call,
            ));
        }
        ("light", "set_color_temperature") => {
            let value = color_temperature_value(&call.body)?.ok_or_else(|| {
                ApiError::bad_request(
                    "light.set_color_temperature requires color_temp, color_temp_kelvin, or kelvin",
                )
            })?;
            commands.push(service_command(
                entity,
                CommandType::SetColorTemperature,
                value,
                call,
            ));
        }
        ("light", "set_color") => {
            let value = color_value(&call.body)?
                .ok_or_else(|| ApiError::bad_request("light.set_color requires rgb_color"))?;
            commands.push(service_command(entity, CommandType::SetColor, value, call));
        }
        ("lock", "lock") => {
            commands.push(service_command(
                entity,
                CommandType::SetLock,
                Value::Text("locked".to_string()),
                call,
            ));
        }
        ("lock", "unlock") => {
            commands.push(service_command(
                entity,
                CommandType::SetLock,
                Value::Text("unlocked".to_string()),
                call,
            ));
        }
        ("climate", "set_temperature") => {
            let value = number_or_integer_field(&call.body, "temperature").ok_or_else(|| {
                ApiError::bad_request("climate.set_temperature requires temperature")
            })?;
            commands.push(service_command(
                entity,
                CommandType::SetThermostatSetpoint,
                value,
                call,
            ));
        }
        _ => {
            return Err(ApiError::bad_request(format!(
                "unsupported service `{domain}.{service}`"
            )));
        }
    }

    Ok(commands)
}

fn service_command(
    entity: &Entity,
    command_type: CommandType,
    arguments: Value,
    call: &ServiceCall,
) -> ServiceCommand {
    ServiceCommand {
        entity_id: entity.entity_id.clone(),
        command_type,
        arguments,
        idempotency_key: call.idempotency_key.clone(),
        timeout_ms: call.timeout_ms,
    }
}

fn state_deltas_from_value(value: &Value) -> Result<Vec<StateDelta>, ApiError> {
    match value {
        Value::Object(fields) => Ok(fields
            .iter()
            .map(|(capability_id, value)| StateDelta {
                capability_id: CapabilityId::trusted(capability_id.clone()),
                value: value.clone(),
            })
            .collect()),
        _ => Err(ApiError::bad_request(
            "scene action desired_state must be an object",
        )),
    }
}

fn command_from_capability_value(
    entity_id: &EntityId,
    delta: &StateDelta,
) -> Result<(CommandType, Value), ApiError> {
    match delta.capability_id.as_str() {
        "light.on_off" => match delta.value {
            Value::Bool(true) => Ok((CommandType::TurnOn, Value::Null)),
            Value::Bool(false) => Ok((CommandType::TurnOff, Value::Null)),
            _ => Err(ApiError::bad_request(format!(
                "entity {entity_id} light.on_off scene value must be boolean"
            ))),
        },
        "light.brightness" => Ok((CommandType::SetBrightness, delta.value.clone())),
        "light.color" => Ok((CommandType::SetColor, delta.value.clone())),
        "light.color_temperature" => Ok((CommandType::SetColorTemperature, delta.value.clone())),
        "lock.state" => Ok((CommandType::SetLock, delta.value.clone())),
        "climate.setpoint" => Ok((CommandType::SetThermostatSetpoint, delta.value.clone())),
        capability_id => Err(ApiError::bad_request(format!(
            "entity {entity_id} desired state for capability `{capability_id}` cannot be mapped"
        ))),
    }
}

fn brightness_value(body: &JsonValue) -> Result<Option<Value>, ApiError> {
    if let Some(value) = json_u64_field(body, "brightness_pct") {
        if value > 100 {
            return Err(ApiError::bad_request(
                "brightness_pct must be between 0 and 100",
            ));
        }
        return Ok(Some(Value::Percentage(value as u8)));
    }

    if let Some(value) = json_u64_field(body, "brightness") {
        if value > 255 {
            return Err(ApiError::bad_request(
                "brightness must be between 0 and 255",
            ));
        }
        let percentage = ((value * 100) + 127) / 255;
        return Ok(Some(Value::Percentage(percentage as u8)));
    }

    Ok(None)
}

fn color_temperature_value(body: &JsonValue) -> Result<Option<Value>, ApiError> {
    for field in ["color_temp_kelvin", "kelvin", "color_temp"] {
        if let Some(value) = json_u64_field(body, field) {
            return Ok(Some(Value::Integer(value as i64)));
        }
    }
    Ok(None)
}

fn color_value(body: &JsonValue) -> Result<Option<Value>, ApiError> {
    let Some(rgb) = body.get("rgb_color") else {
        return Ok(None);
    };
    let values = rgb
        .as_array()
        .ok_or_else(|| ApiError::bad_request("rgb_color must be an array"))?;
    if values.len() != 3 {
        return Err(ApiError::bad_request("rgb_color must have three channels"));
    }
    let mut channels = Vec::new();
    for value in values {
        let channel = value
            .as_u64()
            .ok_or_else(|| ApiError::bad_request("rgb_color channels must be integers"))?;
        if channel > 255 {
            return Err(ApiError::bad_request(
                "rgb_color channels must be between 0 and 255",
            ));
        }
        channels.push(Value::Integer(channel as i64));
    }
    Ok(Some(Value::Array(channels)))
}

fn number_or_integer_field(body: &JsonValue, field: &str) -> Option<Value> {
    body.get(field).and_then(|value| {
        value
            .as_i64()
            .map(Value::Integer)
            .or_else(|| value.as_f64().map(Value::Number))
    })
}

fn collect_string_values(value: &JsonValue, field: &str, output: &mut Vec<String>) {
    let Some(value) = value.get(field) else {
        return;
    };
    match value {
        JsonValue::String(value) => output.push(value.clone()),
        JsonValue::Array(values) => {
            for value in values {
                if let Some(value) = value.as_str() {
                    output.push(value.to_string());
                }
            }
        }
        _ => {}
    }
}

fn json_string_field(value: &JsonValue, field: &str) -> Option<String> {
    value
        .get(field)
        .and_then(JsonValue::as_str)
        .map(str::to_string)
}

fn json_u64_field(value: &JsonValue, field: &str) -> Option<u64> {
    value.get(field).and_then(JsonValue::as_u64)
}

fn service_call_json(
    domain: &str,
    service: &str,
    results: &[CommandResult],
    state: &SmartHomePlatformHttpState,
) -> String {
    format!(
        "{{\"domain\":{},\"service\":{},\"result_count\":{},\"results\":[{}],\"states\":{}}}",
        json_string(domain),
        json_string(service),
        results.len(),
        results
            .iter()
            .map(command_result_json)
            .collect::<Vec<_>>()
            .join(","),
        states_json(&state.entities, state.generated_at_ms),
    )
}

fn command_result_json(result: &CommandResult) -> String {
    format!(
        "{{\"command_id\":{},\"status\":{},\"bridge_id\":{},\"correlation_id\":{},\"message\":{},\"links\":{}}}",
        json_string(result.command_id.as_str()),
        json_string(command_status_label(result.status)),
        json_string(result.bridge_id.as_str()),
        json_string(result.correlation_id.as_str()),
        result
            .message
            .as_ref()
            .map(json_string)
            .unwrap_or_else(|| "null".to_string()),
        command_result_links_json(result),
    )
}

fn command_result_links_json(result: &CommandResult) -> String {
    format!(
        "{{\"self\":{},\"command_results_by_correlation\":{},\"command_results_by_bridge\":{}}}",
        json_string(format!(
            "/api/smart_home/command_results/{}",
            result.command_id.as_str()
        )),
        json_string(format!(
            "/api/smart_home/command_results?correlation_id={}",
            result.correlation_id.as_str()
        )),
        json_string(format!(
            "/api/smart_home/command_results?bridge_id={}",
            url_component(result.bridge_id.as_str())
        )),
    )
}

fn state_delta_json(delta: &StateDelta) -> String {
    format!(
        "{{\"capability_id\":{},\"value\":{}}}",
        json_string(delta.capability_id.as_str()),
        value_json(&delta.value),
    )
}

fn query_string<'a>(request: &'a WebRequest, key: &str) -> Option<&'a str> {
    request.query_params.get(key).map(String::as_str)
}

fn query_u64(request: &WebRequest, key: &str) -> Result<Option<u64>, ApiError> {
    query_string(request, key)
        .map(|value| parse_u64(key, value))
        .transpose()
}

fn route_u64(request: &WebRequest, key: &str) -> Result<u64, ApiError> {
    let Some(value) = request.route_params.get(key) else {
        return Err(ApiError::bad_request(format!("missing {key}")));
    };
    parse_u64(key, value)
}

fn parse_u64(key: &str, value: &str) -> Result<u64, ApiError> {
    value
        .parse::<u64>()
        .map_err(|_| ApiError::bad_request(format!("{key} must be an unsigned integer")))
}

fn json_number_from_query(key: &str, value: &str) -> Result<serde_json::Number, ApiError> {
    if value.contains('.') {
        let value = value
            .parse::<f64>()
            .map_err(|_| ApiError::bad_request(format!("{key} must be a number")))?;
        serde_json::Number::from_f64(value)
            .ok_or_else(|| ApiError::bad_request(format!("{key} must be a finite number")))
    } else {
        value
            .parse::<i64>()
            .map(serde_json::Number::from)
            .map_err(|_| ApiError::bad_request(format!("{key} must be a number")))
    }
}

fn route_usize(request: &WebRequest, key: &str) -> Result<usize, ApiError> {
    let Some(value) = request.route_params.get(key) else {
        return Err(ApiError::bad_request(format!("missing {key}")));
    };
    value
        .parse::<usize>()
        .map_err(|_| ApiError::bad_request(format!("{key} must be an unsigned integer")))
}

fn query_bool(request: &WebRequest, key: &str) -> Result<Option<bool>, ApiError> {
    query_string(request, key)
        .map(|value| match value {
            "true" | "1" => Ok(true),
            "false" | "0" => Ok(false),
            _ => Err(ApiError::bad_request(format!("{key} must be a boolean"))),
        })
        .transpose()
}

fn query_limit(request: &WebRequest, default: usize, max: usize) -> Result<usize, ApiError> {
    let Some(value) = query_string(request, "limit") else {
        return Ok(default.min(max));
    };
    let limit = value
        .parse::<usize>()
        .map_err(|_| ApiError::bad_request("limit must be an unsigned integer"))?;
    Ok(limit.min(max))
}

fn pairing_session_status_from_label(label: &str) -> Result<PairingSessionStatus, ApiError> {
    match label {
        "pending_user_presence" | "pending" => Ok(PairingSessionStatus::PendingUserPresence),
        "completed" => Ok(PairingSessionStatus::Completed),
        "expired" => Ok(PairingSessionStatus::Expired),
        "cancelled" => Ok(PairingSessionStatus::Cancelled),
        other => Err(ApiError::bad_request(format!(
            "unsupported pairing session status `{other}`"
        ))),
    }
}

fn pairing_session_status_label(status: PairingSessionStatus) -> &'static str {
    match status {
        PairingSessionStatus::PendingUserPresence => "pending_user_presence",
        PairingSessionStatus::Completed => "completed",
        PairingSessionStatus::Expired => "expired",
        PairingSessionStatus::Cancelled => "cancelled",
    }
}

fn pairing_session_sort_from_label(label: &str) -> Result<RuntimePairingSessionSort, ApiError> {
    match label {
        "session_id" => Ok(RuntimePairingSessionSort::SessionId),
        "expires_at" => Ok(RuntimePairingSessionSort::ExpiresAt),
        "started_at_desc" => Ok(RuntimePairingSessionSort::StartedAtDesc),
        "status_then_expires_at" => Ok(RuntimePairingSessionSort::StatusThenExpiresAt),
        other => Err(ApiError::bad_request(format!(
            "unsupported pairing session sort `{other}`"
        ))),
    }
}

fn command_status_label(status: CommandStatus) -> &'static str {
    match status {
        CommandStatus::Accepted => "accepted",
        CommandStatus::Rejected => "rejected",
        CommandStatus::TimedOut => "timed_out",
        CommandStatus::Failed => "failed",
    }
}

fn command_status_from_label(status: &str) -> Result<CommandStatus, ApiError> {
    match status {
        "accepted" => Ok(CommandStatus::Accepted),
        "rejected" => Ok(CommandStatus::Rejected),
        "timed_out" => Ok(CommandStatus::TimedOut),
        "failed" => Ok(CommandStatus::Failed),
        other => Err(ApiError::bad_request(format!(
            "unsupported command status `{other}`"
        ))),
    }
}

fn command_result_sort_from_label(sort: &str) -> Result<RuntimeCommandResultSort, ApiError> {
    match sort {
        "sequence_asc" | "oldest_first" => Ok(RuntimeCommandResultSort::SequenceAsc),
        "sequence_desc" | "newest_first" => Ok(RuntimeCommandResultSort::SequenceDesc),
        "status_then_sequence_desc" | "status_then_newest" | "status" => {
            Ok(RuntimeCommandResultSort::StatusThenSequenceDesc)
        }
        other => Err(ApiError::bad_request(format!(
            "unsupported command result sort `{other}`"
        ))),
    }
}

fn authorization_outcome_label(outcome: AuthorizationOutcome) -> &'static str {
    match outcome {
        AuthorizationOutcome::Allowed => "allowed",
        AuthorizationOutcome::Denied => "denied",
    }
}

fn authorization_outcome_from_label(outcome: &str) -> Result<AuthorizationOutcome, ApiError> {
    match outcome {
        "allowed" => Ok(AuthorizationOutcome::Allowed),
        "denied" => Ok(AuthorizationOutcome::Denied),
        other => Err(ApiError::bad_request(format!(
            "unsupported authorization outcome `{other}`"
        ))),
    }
}

fn authorization_decision_sort_from_label(
    sort: &str,
) -> Result<RuntimeAuthorizationDecisionSort, ApiError> {
    match sort {
        "decided_at_asc" | "oldest_first" => Ok(RuntimeAuthorizationDecisionSort::DecidedAtAsc),
        "decided_at_desc" | "newest_first" => Ok(RuntimeAuthorizationDecisionSort::DecidedAtDesc),
        other => Err(ApiError::bad_request(format!(
            "unsupported authorization decision sort `{other}`"
        ))),
    }
}

fn capability_grant_status_label(status: CapabilityGrantStatus) -> &'static str {
    match status {
        CapabilityGrantStatus::Pending => "pending",
        CapabilityGrantStatus::Active => "active",
        CapabilityGrantStatus::Revoked => "revoked",
        CapabilityGrantStatus::Expired => "expired",
    }
}

fn capability_grant_status_from_label(status: &str) -> Result<CapabilityGrantStatus, ApiError> {
    match status {
        "pending" => Ok(CapabilityGrantStatus::Pending),
        "active" => Ok(CapabilityGrantStatus::Active),
        "revoked" => Ok(CapabilityGrantStatus::Revoked),
        "expired" => Ok(CapabilityGrantStatus::Expired),
        other => Err(ApiError::bad_request(format!(
            "unsupported capability grant status `{other}`"
        ))),
    }
}

fn capability_grant_scope_kind_from_label(
    scope: &str,
) -> Result<RuntimeCapabilityGrantScopeKind, ApiError> {
    match scope {
        "tool" => Ok(RuntimeCapabilityGrantScopeKind::Tool),
        "capability" => Ok(RuntimeCapabilityGrantScopeKind::Capability),
        "entity_capability" | "entity" => Ok(RuntimeCapabilityGrantScopeKind::EntityCapability),
        "all_smart_home" | "all" => Ok(RuntimeCapabilityGrantScopeKind::AllSmartHome),
        other => Err(ApiError::bad_request(format!(
            "unsupported capability grant scope `{other}`"
        ))),
    }
}

fn capability_grant_sort_from_label(sort: &str) -> Result<RuntimeCapabilityGrantSort, ApiError> {
    match sort {
        "grant_id" => Ok(RuntimeCapabilityGrantSort::GrantId),
        "principal_id" | "principal" => Ok(RuntimeCapabilityGrantSort::PrincipalId),
        "granted_at_asc" | "oldest_first" => Ok(RuntimeCapabilityGrantSort::GrantedAtAsc),
        "granted_at_desc" | "newest_first" => Ok(RuntimeCapabilityGrantSort::GrantedAtDesc),
        "expires_at_asc" => Ok(RuntimeCapabilityGrantSort::ExpiresAtAsc),
        "expires_at_desc" => Ok(RuntimeCapabilityGrantSort::ExpiresAtDesc),
        other => Err(ApiError::bad_request(format!(
            "unsupported capability grant sort `{other}`"
        ))),
    }
}

fn privilege_tier_label(tier: PrivilegeTier) -> &'static str {
    match tier {
        PrivilegeTier::ReadOnly => "read_only",
        PrivilegeTier::LowRisk => "low_risk",
        PrivilegeTier::HumanApproval => "human_approval",
        PrivilegeTier::HighRisk => "high_risk",
    }
}

fn command_type_label(command_type: CommandType) -> &'static str {
    match command_type {
        CommandType::TurnOn => "turn_on",
        CommandType::TurnOff => "turn_off",
        CommandType::SetBrightness => "set_brightness",
        CommandType::SetColor => "set_color",
        CommandType::SetColorTemperature => "set_color_temperature",
        CommandType::RecallScene => "recall_scene",
        CommandType::SetLock => "set_lock",
        CommandType::SetThermostatSetpoint => "set_thermostat_setpoint",
        CommandType::Media(MediaCommandType::SetPlaybackState) => "media_set_playback_state",
        CommandType::Media(MediaCommandType::PlayNext) => "media_play_next",
        CommandType::Media(MediaCommandType::PlayPrevious) => "media_play_previous",
        CommandType::Media(MediaCommandType::SetVolume) => "media_set_volume",
        CommandType::Media(MediaCommandType::SetMute) => "media_set_mute",
        CommandType::Media(MediaCommandType::SetGroup) => "media_set_group",
        CommandType::Media(MediaCommandType::ClearQueue) => "media_clear_queue",
        CommandType::Media(MediaCommandType::PlayQueueItem) => "media_play_queue_item",
        CommandType::Media(MediaCommandType::RemoveQueueItem) => "media_remove_queue_item",
        CommandType::Media(MediaCommandType::MoveQueueItem) => "media_move_queue_item",
        CommandType::DeviceControl(DeviceControlCommandType::SetIndicatorMode) => {
            "device_set_indicator_mode"
        }
        CommandType::DeviceControl(DeviceControlCommandType::SetIndicatorBrightness) => {
            "device_set_indicator_brightness"
        }
        CommandType::DeviceControl(DeviceControlCommandType::SetDisplayBrightness) => {
            "device_set_display_brightness"
        }
        CommandType::DeviceControl(DeviceControlCommandType::CalibrateSensor) => {
            "sensor_calibrate"
        }
        CommandType::DeviceControl(DeviceControlCommandType::SetTemperatureUnit) => {
            "device_set_temperature_unit"
        }
        CommandType::DeviceControl(DeviceControlCommandType::SetParticulateDisplayStandard) => {
            "device_set_particulate_display_standard"
        }
        CommandType::DeviceControl(DeviceControlCommandType::SetAutomaticCo2BaselineDays) => {
            "device_set_automatic_co2_baseline_days"
        }
        CommandType::DeviceControl(DeviceControlCommandType::SetGasLearningOffsets) => {
            "device_set_gas_learning_offsets"
        }
        CommandType::DeviceControl(DeviceControlCommandType::SetCompensatedDisplay) => {
            "device_set_compensated_display"
        }
        CommandType::DeviceControl(DeviceControlCommandType::TestIndicator) => {
            "device_test_indicator"
        }
        CommandType::DeviceControl(DeviceControlCommandType::SetCorrectionProfile) => {
            "device_set_correction_profile"
        }
        CommandType::DeviceControl(DeviceControlCommandType::SetCameraRecording) => {
            "camera_set_recording"
        }
    }
}

fn command_type_from_label(command_type: &str) -> Result<CommandType, ApiError> {
    match command_type {
        "turn_on" => Ok(CommandType::TurnOn),
        "turn_off" => Ok(CommandType::TurnOff),
        "set_brightness" => Ok(CommandType::SetBrightness),
        "set_color" => Ok(CommandType::SetColor),
        "set_color_temperature" => Ok(CommandType::SetColorTemperature),
        "recall_scene" => Ok(CommandType::RecallScene),
        "set_lock" => Ok(CommandType::SetLock),
        "set_thermostat_setpoint" => Ok(CommandType::SetThermostatSetpoint),
        "media_set_playback_state" => Ok(CommandType::Media(MediaCommandType::SetPlaybackState)),
        "media_play_next" => Ok(CommandType::Media(MediaCommandType::PlayNext)),
        "media_play_previous" => Ok(CommandType::Media(MediaCommandType::PlayPrevious)),
        "media_set_volume" => Ok(CommandType::Media(MediaCommandType::SetVolume)),
        "media_set_mute" => Ok(CommandType::Media(MediaCommandType::SetMute)),
        "media_set_group" => Ok(CommandType::Media(MediaCommandType::SetGroup)),
        "media_clear_queue" => Ok(CommandType::Media(MediaCommandType::ClearQueue)),
        "media_play_queue_item" => Ok(CommandType::Media(MediaCommandType::PlayQueueItem)),
        "media_remove_queue_item" => Ok(CommandType::Media(MediaCommandType::RemoveQueueItem)),
        "media_move_queue_item" => Ok(CommandType::Media(MediaCommandType::MoveQueueItem)),
        "device_set_indicator_mode" => Ok(CommandType::DeviceControl(
            DeviceControlCommandType::SetIndicatorMode,
        )),
        "device_set_indicator_brightness" => Ok(CommandType::DeviceControl(
            DeviceControlCommandType::SetIndicatorBrightness,
        )),
        "device_set_display_brightness" => Ok(CommandType::DeviceControl(
            DeviceControlCommandType::SetDisplayBrightness,
        )),
        "sensor_calibrate" => Ok(CommandType::DeviceControl(
            DeviceControlCommandType::CalibrateSensor,
        )),
        "device_set_temperature_unit" => Ok(CommandType::DeviceControl(
            DeviceControlCommandType::SetTemperatureUnit,
        )),
        "device_set_particulate_display_standard" => Ok(CommandType::DeviceControl(
            DeviceControlCommandType::SetParticulateDisplayStandard,
        )),
        "device_set_automatic_co2_baseline_days" => Ok(CommandType::DeviceControl(
            DeviceControlCommandType::SetAutomaticCo2BaselineDays,
        )),
        "device_set_gas_learning_offsets" => Ok(CommandType::DeviceControl(
            DeviceControlCommandType::SetGasLearningOffsets,
        )),
        "device_set_compensated_display" => Ok(CommandType::DeviceControl(
            DeviceControlCommandType::SetCompensatedDisplay,
        )),
        "device_test_indicator" => Ok(CommandType::DeviceControl(
            DeviceControlCommandType::TestIndicator,
        )),
        "device_set_correction_profile" => Ok(CommandType::DeviceControl(
            DeviceControlCommandType::SetCorrectionProfile,
        )),
        "camera_set_recording" => Ok(CommandType::DeviceControl(
            DeviceControlCommandType::SetCameraRecording,
        )),
        other => Err(ApiError::bad_request(format!(
            "unsupported command_type `{other}`"
        ))),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DesiredStateAuthorizationOperation {
    Set,
    Clear,
}

impl DesiredStateAuthorizationOperation {
    fn label(self) -> &'static str {
        match self {
            Self::Set => "set",
            Self::Clear => "clear",
        }
    }

    fn tool(self) -> SmartHomeTool {
        match self {
            Self::Set => SmartHomeTool::SetDesiredState,
            Self::Clear => SmartHomeTool::ClearDesiredState,
        }
    }
}

fn desired_state_authorization_operation_from_label(
    operation: &str,
) -> Result<DesiredStateAuthorizationOperation, ApiError> {
    match operation {
        "set" | "write" | "upsert" => Ok(DesiredStateAuthorizationOperation::Set),
        "clear" | "delete" | "remove" => Ok(DesiredStateAuthorizationOperation::Clear),
        other => Err(ApiError::bad_request(format!(
            "unsupported desired-state authorization operation `{other}`"
        ))),
    }
}

fn capability_mode_label(mode: CapabilityMode) -> &'static str {
    match mode {
        CapabilityMode::Observe => "observe",
        CapabilityMode::Command => "command",
        CapabilityMode::ObserveAndCommand => "observe_and_command",
    }
}

fn value_kind_label(kind: ValueKind) -> &'static str {
    match kind {
        ValueKind::Null => "null",
        ValueKind::Boolean => "boolean",
        ValueKind::Integer => "integer",
        ValueKind::Number => "number",
        ValueKind::Percentage => "percentage",
        ValueKind::Text => "text",
        ValueKind::Object => "object",
        ValueKind::Array => "array",
    }
}

fn bridge_transport_label(transport: BridgeTransport) -> &'static str {
    match transport {
        BridgeTransport::LanHttp => "lan_http",
        BridgeTransport::LanTcp => "lan_tcp",
        BridgeTransport::LanUdp => "lan_udp",
        BridgeTransport::Mdns => "mdns",
        BridgeTransport::Serial => "serial",
        BridgeTransport::Ble => "ble",
        BridgeTransport::Cloud => "cloud",
        BridgeTransport::LocalProcess => "local_process",
    }
}

fn bridge_transport_from_label(transport: &str) -> Result<BridgeTransport, ApiError> {
    match transport {
        "lan_http" | "lan-http" | "http" => Ok(BridgeTransport::LanHttp),
        "lan_tcp" | "lan-tcp" | "tcp" => Ok(BridgeTransport::LanTcp),
        "lan_udp" | "lan-udp" | "udp" => Ok(BridgeTransport::LanUdp),
        "mdns" => Ok(BridgeTransport::Mdns),
        "serial" => Ok(BridgeTransport::Serial),
        "ble" => Ok(BridgeTransport::Ble),
        "cloud" => Ok(BridgeTransport::Cloud),
        "local_process" | "local-process" => Ok(BridgeTransport::LocalProcess),
        other => Err(ApiError::bad_request(format!(
            "unsupported bridge transport `{other}`"
        ))),
    }
}

fn health_label(health: Health) -> &'static str {
    match health {
        Health::Unknown => "unknown",
        Health::Discoverable => "discoverable",
        Health::Unpaired => "unpaired",
        Health::Online => "online",
        Health::Degraded => "degraded",
        Health::Offline => "offline",
        Health::AuthFailed => "auth_failed",
        Health::Unsupported => "unsupported",
        Health::Removed => "removed",
    }
}

fn health_from_label(health: &str) -> Result<Health, ApiError> {
    match health {
        "unknown" => Ok(Health::Unknown),
        "discoverable" => Ok(Health::Discoverable),
        "unpaired" => Ok(Health::Unpaired),
        "online" => Ok(Health::Online),
        "degraded" => Ok(Health::Degraded),
        "offline" => Ok(Health::Offline),
        "auth_failed" | "auth-failed" => Ok(Health::AuthFailed),
        "unsupported" => Ok(Health::Unsupported),
        "removed" => Ok(Health::Removed),
        other => Err(ApiError::bad_request(format!(
            "unsupported health `{other}`"
        ))),
    }
}

fn room_sort_from_label(sort: &str) -> Result<RuntimeRoomSort, ApiError> {
    match sort {
        "room_id" | "id" => Ok(RuntimeRoomSort::RoomId),
        "attention" | "attention_desc" => Ok(RuntimeRoomSort::AttentionDesc),
        "entity_count" | "entity_count_desc" => Ok(RuntimeRoomSort::EntityCountDesc),
        "scene_count" | "scene_count_desc" => Ok(RuntimeRoomSort::SceneCountDesc),
        other => Err(ApiError::bad_request(format!(
            "unsupported room sort `{other}`"
        ))),
    }
}

fn scene_scope_label(scope: SceneScope) -> &'static str {
    match scope {
        SceneScope::Room => "room",
        SceneScope::Zone => "zone",
        SceneScope::Home => "home",
        SceneScope::Bridge => "bridge",
        SceneScope::Custom => "custom",
    }
}

fn scene_scope_from_label(scope: &str) -> Result<SceneScope, ApiError> {
    match scope {
        "room" => Ok(SceneScope::Room),
        "zone" => Ok(SceneScope::Zone),
        "home" => Ok(SceneScope::Home),
        "bridge" => Ok(SceneScope::Bridge),
        "custom" => Ok(SceneScope::Custom),
        other => Err(ApiError::bad_request(format!(
            "unsupported scene scope `{other}`"
        ))),
    }
}

fn device_event_type_label(event_type: DeviceEventType) -> &'static str {
    match event_type {
        DeviceEventType::Discovered => "discovered",
        DeviceEventType::Updated => "updated",
        DeviceEventType::Removed => "removed",
        DeviceEventType::Unavailable => "unavailable",
        DeviceEventType::Error => "error",
        DeviceEventType::Health => "health",
    }
}

fn device_event_type_from_label(event_type: &str) -> Result<DeviceEventType, ApiError> {
    match event_type {
        "discovered" => Ok(DeviceEventType::Discovered),
        "updated" => Ok(DeviceEventType::Updated),
        "removed" => Ok(DeviceEventType::Removed),
        "unavailable" => Ok(DeviceEventType::Unavailable),
        "error" => Ok(DeviceEventType::Error),
        "health" => Ok(DeviceEventType::Health),
        other => Err(ApiError::bad_request(format!(
            "unsupported device event type `{other}`"
        ))),
    }
}

fn runtime_error_to_api_error(error: RuntimeError) -> ApiError {
    match error {
        RuntimeError::UnauthorizedCommand { .. } | RuntimeError::UnauthorizedTool { .. } => {
            ApiError::forbidden(error.to_string())
        }
        RuntimeError::UnknownEntity(_) | RuntimeError::UnknownScene(_) => {
            ApiError::not_found(error.to_string())
        }
        RuntimeError::UnsupportedCapability { .. }
        | RuntimeError::ReadOnlyCapability { .. }
        | RuntimeError::UnsupportedDesiredState { .. } => ApiError::bad_request(error.to_string()),
        _ => ApiError::new(500, error.to_string()),
    }
}

fn api_error_response(error: ApiError) -> WebResponse {
    json_error(error.status, error.message)
}

fn json_error(status: u16, message: impl AsRef<str>) -> WebResponse {
    WebResponse::new(
        status,
        format!("{{\"error\":{}}}", json_string(message.as_ref())).into_bytes(),
    )
    .with_content_type("application/json")
}

fn default_event_types() -> Vec<String> {
    sorted_unique_strings([
        "call_service",
        "command_result",
        "state_changed",
        "state_expired",
    ])
}

fn sorted_unique_strings(values: impl IntoIterator<Item = impl Into<String>>) -> Vec<String> {
    let mut values = values.into_iter().map(Into::into).collect::<Vec<_>>();
    values.sort();
    values.dedup();
    values
}

fn services_for_capability(domain: &str, capability: &Capability) -> Vec<&'static str> {
    match capability.capability_id.as_str() {
        "light.on_off" => vec!["turn_on", "turn_off"],
        "light.brightness" => vec!["set_brightness"],
        "light.color" => vec!["set_color"],
        "light.color_temperature" => vec!["set_color_temperature"],
        "lock.state" => vec!["lock", "unlock"],
        "climate.setpoint" => vec!["set_temperature"],
        "scene.recall" => vec!["turn_on"],
        _ if domain == "input" => vec!["set_value"],
        _ => vec!["set_value"],
    }
}

fn capability_allows_command(capability: &Capability) -> bool {
    matches!(
        capability.mode,
        CapabilityMode::Command | CapabilityMode::ObserveAndCommand
    )
}

fn entity_domain(kind: EntityKind) -> &'static str {
    match kind {
        EntityKind::Camera => "camera",
        EntityKind::Light => "light",
        EntityKind::LightGroup => "light",
        EntityKind::Switch => "switch",
        EntityKind::Sensor => "sensor",
        EntityKind::Lock => "lock",
        EntityKind::Thermostat => "climate",
        EntityKind::Scene => "scene",
        EntityKind::Input => "input",
        EntityKind::BridgeHealth => "binary_sensor",
        EntityKind::NetworkDiagnostic => "diagnostic",
        EntityKind::Unknown => "unknown",
    }
}

fn entity_matches_external_id(entity: &Entity, target: &str) -> bool {
    entity.entity_id.as_str() == target || home_assistant_entity_id(entity) == target
}

fn scene_matches_external_id(scene: &Scene, target: &str) -> bool {
    scene.scene_id.as_str() == target || home_assistant_scene_id(scene) == target
}

fn home_assistant_entity_id(entity: &Entity) -> String {
    format!(
        "{}.{}",
        entity_domain(entity.kind),
        object_id(entity.entity_id.as_str())
    )
}

fn home_assistant_entity_id_for(entity_id: &EntityId) -> String {
    format!("entity.{}", object_id(entity_id.as_str()))
}

fn home_assistant_scene_id(scene: &Scene) -> String {
    format!("scene.{}", object_id(scene.scene_id.as_str()))
}

fn object_id(value: &str) -> String {
    let mut object_id = String::new();
    let mut previous_was_separator = false;
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() {
            object_id.push(ch.to_ascii_lowercase());
            previous_was_separator = false;
        } else if !previous_was_separator {
            object_id.push('_');
            previous_was_separator = true;
        }
    }
    let object_id = object_id.trim_matches('_');
    if object_id.is_empty() {
        "unnamed".to_string()
    } else {
        object_id.to_string()
    }
}

fn entity_kind_label(kind: EntityKind) -> &'static str {
    match kind {
        EntityKind::Camera => "camera",
        EntityKind::Light => "light",
        EntityKind::LightGroup => "light_group",
        EntityKind::Switch => "switch",
        EntityKind::Sensor => "sensor",
        EntityKind::Lock => "lock",
        EntityKind::Thermostat => "thermostat",
        EntityKind::Scene => "scene",
        EntityKind::Input => "input",
        EntityKind::BridgeHealth => "bridge_health",
        EntityKind::NetworkDiagnostic => "network_diagnostic",
        EntityKind::Unknown => "unknown",
    }
}

fn entity_kind_from_label(kind: &str) -> Result<EntityKind, ApiError> {
    match kind {
        "camera" => Ok(EntityKind::Camera),
        "light" => Ok(EntityKind::Light),
        "light_group" => Ok(EntityKind::LightGroup),
        "switch" => Ok(EntityKind::Switch),
        "sensor" => Ok(EntityKind::Sensor),
        "lock" => Ok(EntityKind::Lock),
        "thermostat" | "climate" => Ok(EntityKind::Thermostat),
        "scene" => Ok(EntityKind::Scene),
        "input" => Ok(EntityKind::Input),
        "bridge_health" | "binary_sensor" => Ok(EntityKind::BridgeHealth),
        "network_diagnostic" | "diagnostic" => Ok(EntityKind::NetworkDiagnostic),
        "unknown" => Ok(EntityKind::Unknown),
        other => Err(ApiError::bad_request(format!(
            "unsupported entity kind `{other}`"
        ))),
    }
}

fn state_source_label(source: StateSource) -> &'static str {
    match source {
        StateSource::EventStream => "event_stream",
        StateSource::Poll => "poll",
        StateSource::OptimisticCommand => "optimistic_command",
        StateSource::Manual => "manual",
    }
}

fn state_source_from_label(source: &str) -> Result<StateSource, ApiError> {
    match source {
        "event_stream" | "event-stream" => Ok(StateSource::EventStream),
        "poll" => Ok(StateSource::Poll),
        "optimistic_command" | "optimistic-command" => Ok(StateSource::OptimisticCommand),
        "manual" => Ok(StateSource::Manual),
        other => Err(ApiError::bad_request(format!(
            "unsupported state source `{other}`"
        ))),
    }
}

fn state_confidence_label(confidence: StateConfidence) -> &'static str {
    match confidence {
        StateConfidence::Confirmed => "confirmed",
        StateConfidence::Optimistic => "optimistic",
        StateConfidence::Stale => "stale",
        StateConfidence::Unknown => "unknown",
    }
}

fn state_confidence_from_label(confidence: &str) -> Result<StateConfidence, ApiError> {
    match confidence {
        "confirmed" => Ok(StateConfidence::Confirmed),
        "optimistic" => Ok(StateConfidence::Optimistic),
        "stale" => Ok(StateConfidence::Stale),
        "unknown" => Ok(StateConfidence::Unknown),
        other => Err(ApiError::bad_request(format!(
            "unsupported state confidence `{other}`"
        ))),
    }
}

fn value_json(value: &Value) -> String {
    match value {
        Value::Null => "null".to_string(),
        Value::Bool(value) => value.to_string(),
        Value::Integer(value) => value.to_string(),
        Value::Number(value) if value.is_finite() => value.to_string(),
        Value::Number(_) => "null".to_string(),
        Value::Percentage(value) => value.to_string(),
        Value::Text(value) => json_string(value),
        Value::Object(fields) => format!(
            "{{{}}}",
            fields
                .iter()
                .map(|(key, value)| format!("{}:{}", json_string(key), value_json(value)))
                .collect::<Vec<_>>()
                .join(",")
        ),
        Value::Array(values) => format!(
            "[{}]",
            values.iter().map(value_json).collect::<Vec<_>>().join(",")
        ),
    }
}

fn json_string_array(values: &[String]) -> String {
    values.iter().map(json_string).collect::<Vec<_>>().join(",")
}

fn json_id_array<'a>(values: impl IntoIterator<Item = &'a str>) -> String {
    values
        .into_iter()
        .map(json_string)
        .collect::<Vec<_>>()
        .join(",")
}

fn optional_u64_json(value: Option<u64>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "null".to_string())
}

fn optional_f64_json(value: Option<f64>) -> String {
    match value {
        Some(value) if value.is_finite() => value.to_string(),
        _ => "null".to_string(),
    }
}

fn optional_str_json(value: Option<&str>) -> String {
    value.map(json_string).unwrap_or_else(|| "null".to_string())
}

fn optional_link_json(value: Option<String>) -> String {
    value.map(json_string).unwrap_or_else(|| "null".to_string())
}

fn audit_entity_links_json(entity_id: &EntityId) -> String {
    let entity = url_component(entity_id.as_str());
    format!(
        "{{\"self\":{},\"state\":{},\"desired_state\":{},\"state_history\":{},\"events\":{}}}",
        json_string(format!("/api/smart_home/entities/{entity}")),
        json_string(format!("/api/smart_home/states/{entity}")),
        json_string(format!("/api/smart_home/desired_states?entity_id={entity}")),
        json_string(format!("/api/smart_home/state_history?entity_id={entity}")),
        json_string(format!("/api/smart_home/events?entity_id={entity}")),
    )
}

fn shell_single_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}

fn url_component(value: &str) -> String {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let mut encoded = String::new();
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            encoded.push(byte as char);
        } else {
            encoded.push('%');
            encoded.push(HEX[(byte >> 4) as usize] as char);
            encoded.push(HEX[(byte & 0x0f) as usize] as char);
        }
    }
    encoded
}

fn json_string(value: impl AsRef<str>) -> String {
    let mut escaped = String::from("\"");
    for ch in value.as_ref().chars() {
        match ch {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            ch if ch.is_control() => escaped.push_str(&format!("\\u{:04x}", ch as u32)),
            ch => escaped.push(ch),
        }
    }
    escaped.push('"');
    escaped
}

fn push_unique_string(values: &mut Vec<String>, value: &str) {
    if !values.iter().any(|existing| existing == value) {
        values.push(value.to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use embeddable_http_server::{HttpRequest, HttpServerOptions};
    use http_core::{Header, HttpVersion, RequestHead};
    use smart_home_automation_runtime::{AutomationAction, AutomationTrigger};
    use smart_home_core::{BridgeId, DeviceId, EventId};
    use smart_home_dashboard_core::{
        NativeDashboard, NativeDashboardCard, NativeDashboardCardKind, NativeDashboardView,
        DASHBOARD_MANIFEST_SCHEMA_VERSION,
    };
    use smart_home_runtime_store::SmartHomeRuntimeStore;
    use smart_home_testkit::hue_lighting_runtime;
    use std::fs;
    use std::io::{BufRead, BufReader, Read, Write};
    use std::net::{SocketAddr, TcpStream};
    use std::path::PathBuf;
    use std::sync::Arc;
    use std::thread;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};
    use storage_local_folder::LocalFolderStorageBackend;
    use tcp_runtime::{ConnectionId, TcpConnectionInfo};
    use web_core::WebServer;

    const DASHBOARD_PENDING_WRITE_BYTES: usize = 256 * 1024;

    fn temp_root(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("test clock should be after unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!(
            "smart-home-platform-http-{}-{name}-{nanos}",
            std::process::id()
        ))
    }

    fn dashboard_server_options() -> HttpServerOptions {
        let mut options = HttpServerOptions::default();
        options.tcp.max_pending_write_bytes = DASHBOARD_PENDING_WRITE_BYTES;
        options
    }

    fn request(method: &str, target: &str) -> HttpRequest {
        request_with_body(method, target, "")
    }

    fn request_with_body(method: &str, target: &str, body: &str) -> HttpRequest {
        let mut headers = vec![Header {
            name: "Host".to_string(),
            value: "localhost".to_string(),
        }];
        if !body.is_empty() {
            headers.push(Header {
                name: "Content-Type".to_string(),
                value: "application/json".to_string(),
            });
            headers.push(Header {
                name: "Content-Length".to_string(),
                value: body.len().to_string(),
            });
        }

        HttpRequest {
            connection: TcpConnectionInfo {
                id: ConnectionId(0),
                peer_addr: SocketAddr::from(([127, 0, 0, 1], 10_000)),
                local_addr: SocketAddr::from(([127, 0, 0, 1], 8123)),
            },
            head: RequestHead {
                method: method.to_string(),
                target: target.to_string(),
                version: HttpVersion { major: 1, minor: 1 },
                headers,
            },
            body: body.as_bytes().to_vec(),
        }
    }

    fn response_body(response: web_core::WebResponse) -> String {
        String::from_utf8(response.body).expect("json response is utf8")
    }

    fn http_get(port: u16, path: &str) -> (u16, String) {
        http_request(port, "GET", path, "")
    }

    fn http_post(port: u16, path: &str, body: &str) -> (u16, String) {
        http_request(port, "POST", path, body)
    }

    fn http_request(port: u16, method: &str, path: &str, body: &str) -> (u16, String) {
        let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connect");
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .expect("set read timeout");

        let request = format!(
            "{method} {path} HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        stream.write_all(request.as_bytes()).expect("write request");

        let mut reader = BufReader::new(&stream);
        let mut status_line = String::new();
        reader
            .read_line(&mut status_line)
            .expect("read status line");
        let status = status_line
            .split_whitespace()
            .nth(1)
            .expect("status code")
            .parse()
            .expect("parse status code");

        let mut content_length = 0usize;
        loop {
            let mut line = String::new();
            reader.read_line(&mut line).expect("read header");
            let trimmed = line.trim();
            if trimmed.is_empty() {
                break;
            }
            if trimmed.to_ascii_lowercase().starts_with("content-length:") {
                content_length = trimmed
                    .split_once(':')
                    .map(|(_, value)| value.trim().parse().unwrap_or(0))
                    .unwrap_or(0);
            }
        }

        let mut body = vec![0; content_length];
        reader.read_exact(&mut body).expect("read response body");
        (
            status,
            String::from_utf8(body).expect("json response is utf8"),
        )
    }

    fn start_server(app: WebApp) -> (u16, tcp_runtime::StopHandle) {
        let app = Arc::new(app);

        #[cfg(any(
            target_os = "macos",
            target_os = "freebsd",
            target_os = "openbsd",
            target_os = "netbsd",
            target_os = "dragonfly"
        ))]
        let mut server =
            WebServer::bind_kqueue("127.0.0.1:0", dashboard_server_options(), Arc::clone(&app))
                .expect("bind kqueue");

        #[cfg(target_os = "linux")]
        let mut server =
            WebServer::bind_epoll("127.0.0.1:0", dashboard_server_options(), Arc::clone(&app))
                .expect("bind epoll");

        #[cfg(target_os = "windows")]
        let mut server =
            WebServer::bind_windows("127.0.0.1:0", dashboard_server_options(), Arc::clone(&app))
                .expect("bind windows");

        let port = server.local_addr().port();
        let stop = server.stop_handle();
        thread::spawn(move || {
            let _ = server.serve();
        });
        thread::sleep(Duration::from_millis(20));
        (port, stop)
    }

    fn fixture_state() -> SmartHomePlatformHttpState {
        let runtime = hue_lighting_runtime();
        SmartHomePlatformHttpState::from_runtime(
            &runtime,
            SmartHomePlatformHttpConfig::new("Codex Home").with_time_zone("America/Los_Angeles"),
            ["state_changed", "call_service"],
            5_000,
        )
    }

    fn fixture_runtime(grant_access: bool) -> SmartHomePlatformHttpRuntime {
        let runtime = SmartHomePlatformHttpRuntime::new(
            hue_lighting_runtime(),
            SmartHomePlatformHttpConfig::new("Codex Home").with_time_zone("America/Los_Angeles"),
        )
        .with_event_types(["state_changed", "call_service", "command_result"])
        .with_now_ms(5_000);

        if grant_access {
            runtime.grant_local_full_access("test", 1_000)
        } else {
            runtime
        }
    }

    fn fixture_dashboard_manifest() -> NativeDashboardManifest {
        NativeDashboardManifest {
            schema_version: DASHBOARD_MANIFEST_SCHEMA_VERSION,
            source_instance_id: "fixture-home-assistant".to_string(),
            generated_at_ms: 5_000,
            dashboards: vec![NativeDashboard {
                dashboard_id: "overview".to_string(),
                url_path: "lovelace".to_string(),
                title: "Overview".to_string(),
                icon: None,
                require_admin: false,
                show_in_sidebar: true,
                views: vec![NativeDashboardView {
                    view_id: "living-room".to_string(),
                    title: "Living Room".to_string(),
                    path: Some("living-room".to_string()),
                    icon: None,
                    cards: vec![NativeDashboardCard {
                        card_id: "living-room-light".to_string(),
                        kind: NativeDashboardCardKind::EntityControl,
                        source_type: "light".to_string(),
                        title: Some("Living Room Light".to_string()),
                        entity_ids: vec!["entity-light-1".to_string()],
                    }],
                }],
            }],
            source_resources: Vec::new(),
        }
    }

    fn fixture_runtime_with_desired_state() -> SmartHomePlatformHttpRuntime {
        let mut runtime = hue_lighting_runtime();
        runtime
            .upsert_desired_state(
                DesiredEntityState::new(
                    EntityId::trusted("entity-light-1"),
                    vec![StateDelta {
                        capability_id: CapabilityId::trusted("light.on_off"),
                        value: Value::Bool(true),
                    }],
                )
                .requested_by("agent:chief-of-staff")
                .with_command_timeout(2_500),
            )
            .expect("fixture desired state should validate");

        SmartHomePlatformHttpRuntime::new(
            runtime,
            SmartHomePlatformHttpConfig::new("Codex Home").with_time_zone("America/Los_Angeles"),
        )
        .with_now_ms(5_000)
    }

    fn fixture_runtime_with_state_history() -> SmartHomePlatformHttpRuntime {
        let mut runtime = hue_lighting_runtime();
        runtime
            .apply_device_event(DeviceEvent {
                event_id: EventId::trusted("event-light-1-on"),
                bridge_id: BridgeId::trusted("bridge-1"),
                device_id: Some(DeviceId::trusted("device-1")),
                entity_id: Some(EntityId::trusted("entity-light-1")),
                observed_at_ms: 2_000,
                received_at_ms: 2_010,
                event_type: DeviceEventType::Updated,
                state_delta: Some(StateDelta {
                    capability_id: CapabilityId::trusted("light.on_off"),
                    value: Value::Bool(true),
                }),
                raw_ref: Some("event-log://fixture/light/1".to_string()),
                correlation_id: None,
                metadata: Vec::new(),
            })
            .expect("fixture event should validate");

        SmartHomePlatformHttpRuntime::new(
            runtime,
            SmartHomePlatformHttpConfig::new("Codex Home").with_time_zone("America/Los_Angeles"),
        )
        .with_now_ms(5_000)
    }

    #[test]
    fn platform_http_summary_counts_runtime_snapshot_shape() {
        let state = fixture_state();
        let summary = state.summary();

        assert_eq!(summary.state_count, 2);
        assert_eq!(summary.scene_count, 1);
        assert_eq!(summary.unknown_state_count, 2);
        assert_eq!(summary.event_type_count, 2);
        assert!(summary.service_count >= 4);
    }

    #[test]
    fn runtime_web_app_serves_browser_dashboard_shell() {
        let app = home_assistant_runtime_web_app(fixture_runtime(true));

        for path in ["/", "/dashboard", "/smart-home"] {
            let response: web_core::WebResponse = app.handle(request("GET", path)).into();
            let headers = response.headers.clone();
            let body = response_body(response);

            assert!(
                headers.iter().any(|(name, value)| {
                    name.eq_ignore_ascii_case("content-type") && value == "text/html; charset=utf-8"
                }),
                "dashboard shell should be served as HTML"
            );
            assert!(body.contains("<title>Codex Home</title>"));
            assert!(body.contains("json(\"/api/smart_home/bootstrap\")"));
            assert!(body.contains("json(\"/api/smart_home/readiness\")"));
            assert!(body.contains("json(\"/api/smart_home/dashboard_manifest\")"));
            assert!(body.contains("json(\"/api/smart_home/automations\")"));
            assert!(body.contains("/api/smart_home/pairing_sessions?limit=12"));
            assert!(body.contains("id=\"dashboards-panel\""));
            assert!(body.contains("id=\"automations-panel\""));
            assert!(body.contains("id=\"pairing-panel\""));
            assert!(body.contains("data-dashboard-filter=\"search\""));
            assert!(body.contains("data-dashboard-filter=\"room\""));
            assert!(body.contains("data-dashboard-filter=\"domain\""));
            assert!(body.contains("data-dashboard-filter=\"capability-id\""));
            assert!(body.contains("data-dashboard-filter=\"capability-commandable\""));
            assert!(body.contains("data-dashboard-filter=\"capability-observable\""));
            assert!(body.contains("data-dashboard-filter=\"desired-entity\""));
            assert!(body.contains("data-dashboard-filter=\"desired-requested-by\""));
            assert!(body.contains("data-dashboard-filter=\"device-bridge\""));
            assert!(body.contains("data-dashboard-filter=\"device-manufacturer\""));
            assert!(body.contains("data-dashboard-filter=\"device-health\""));
            assert!(body.contains("data-dashboard-filter=\"bridge-integration\""));
            assert!(body.contains("data-dashboard-filter=\"bridge-transport\""));
            assert!(body.contains("data-dashboard-filter=\"bridge-health\""));
            assert!(body.contains("data-dashboard-filter=\"scene-scope\""));
            assert!(body.contains("data-dashboard-filter=\"scene-entity\""));
            assert!(body.contains("data-dashboard-filter=\"service-name\""));
            assert!(body.contains("data-dashboard-filter=\"service-capability\""));
            assert!(body.contains("data-dashboard-filter=\"service-entity\""));
            assert!(body.contains("data-dashboard-filter=\"service-scene\""));
            assert!(body.contains("data-dashboard-filter=\"api-surface\""));
            assert!(body.contains("data-dashboard-filter=\"api-method\""));
            assert!(body.contains("data-dashboard-filter=\"api-category\""));
            assert!(body.contains("data-dashboard-filter=\"api-mutating\""));
            assert!(body.contains("data-dashboard-filter=\"api-authorized\""));
            assert!(body.contains("data-dashboard-filter=\"activity-entity\""));
            assert!(body.contains("data-dashboard-filter=\"history-type\""));
            assert!(body.contains("data-dashboard-filter=\"history-bridge\""));
            assert!(body.contains("data-dashboard-filter=\"history-from-ms\""));
            assert!(body.contains("data-dashboard-filter=\"history-to-ms\""));
            assert!(body.contains("data-dashboard-filter=\"history-received-from-ms\""));
            assert!(body.contains("data-dashboard-filter=\"history-received-to-ms\""));
            assert!(body.contains("data-dashboard-filter=\"event-from-sequence\""));
            assert!(body.contains("data-dashboard-filter=\"event-to-sequence\""));
            assert!(body.contains("data-dashboard-filter=\"command-status\""));
            assert!(body.contains("data-dashboard-filter=\"command-from-sequence\""));
            assert!(body.contains("data-dashboard-filter=\"command-to-sequence\""));
            assert!(body.contains("const FILTER_QUERY_PARAMS = ["));
            assert!(body.contains("[\"api_surface\", els.filterApiSurface]"));
            assert!(body.contains("[\"api_method\", els.filterApiMethod]"));
            assert!(body.contains("[\"api_category\", els.filterApiCategory]"));
            assert!(body.contains("[\"api_mutating\", els.filterApiMutating]"));
            assert!(body.contains("[\"api_authorized\", els.filterApiAuthorized]"));
            assert!(body.contains("[\"capability_id\", els.filterCapabilityId]"));
            assert!(body.contains("[\"capability_commandable\", els.filterCapabilityCommandable]"));
            assert!(body.contains("[\"capability_observable\", els.filterCapabilityObservable]"));
            assert!(body.contains("[\"desired_entity\", els.filterDesiredEntity]"));
            assert!(body.contains("[\"desired_requested_by\", els.filterDesiredRequestedBy]"));
            assert!(body.contains("[\"device_bridge\", els.filterDeviceBridge]"));
            assert!(body.contains("[\"device_manufacturer\", els.filterDeviceManufacturer]"));
            assert!(body.contains("[\"device_health\", els.filterDeviceHealth]"));
            assert!(body.contains("[\"bridge_integration\", els.filterBridgeIntegration]"));
            assert!(body.contains("[\"bridge_transport\", els.filterBridgeTransport]"));
            assert!(body.contains("[\"bridge_health\", els.filterBridgeHealth]"));
            assert!(body.contains("[\"scene_scope\", els.filterSceneScope]"));
            assert!(body.contains("[\"scene_entity\", els.filterSceneEntity]"));
            assert!(body.contains("[\"service_name\", els.filterServiceName]"));
            assert!(body.contains("[\"service_capability\", els.filterServiceCapability]"));
            assert!(body.contains("[\"service_entity\", els.filterServiceEntity]"));
            assert!(body.contains("[\"service_scene\", els.filterServiceScene]"));
            assert!(body.contains("[\"event_kind\", els.filterEventKind]"));
            assert!(body.contains("[\"event_from_sequence\", els.filterEventFromSequence]"));
            assert!(body.contains("[\"event_to_sequence\", els.filterEventToSequence]"));
            assert!(body.contains("[\"activity_entity\", els.filterActivityEntity]"));
            assert!(body.contains("[\"history_type\", els.filterHistoryType]"));
            assert!(body.contains("[\"history_bridge\", els.filterHistoryBridge]"));
            assert!(body.contains("[\"history_from_ms\", els.filterHistoryFromMs]"));
            assert!(body.contains("[\"history_to_ms\", els.filterHistoryToMs]"));
            assert!(
                body.contains("[\"history_received_from_ms\", els.filterHistoryReceivedFromMs]")
            );
            assert!(body.contains("[\"history_received_to_ms\", els.filterHistoryReceivedToMs]"));
            assert!(body.contains("[\"command_status\", els.filterCommandStatus]"));
            assert!(body.contains("[\"command_from_sequence\", els.filterCommandFromSequence]"));
            assert!(body.contains("[\"command_to_sequence\", els.filterCommandToSequence]"));
            assert!(body.contains("restoreFiltersFromUrl()"));
            assert!(body.contains("window.history.replaceState(null, \"\", nextUrl)"));
            assert!(body.contains(
                "queryUrl(\"/api/smart_home/states\", {limit: 24, domain: filters.domain, room_id: roomId, stale, capability_id: capabilityId})"
            ));
            assert!(body.contains(
                "queryUrl(\"/api/smart_home/states\", {limit: 24, room_id: roomId, stale: true, capability_id: capabilityId})"
            ));
            assert!(body.contains("queryUrl(\"/api/smart_home/scenes\", {"));
            assert!(body.contains("room_id: roomId"));
            assert!(body.contains("scope: filters.sceneScope"));
            assert!(body.contains("entity_id: filters.sceneEntity"));
            assert!(body.contains("queryUrl(\"/api/smart_home/desired_states\", {"));
            assert!(body.contains("entity_id: filters.desiredEntity"));
            assert!(body.contains("capability_id: capabilityId"));
            assert!(body.contains("requested_by: filters.desiredRequestedBy"));
            assert!(body.contains("const activityEntity = filters.activityEntity || undefined"));
            assert!(body.contains("const historyType = filters.historyType || undefined"));
            assert!(body.contains("queryUrl(\"/api/smart_home/api\", {"));
            assert!(body.contains("surface: filters.apiSurface"));
            assert!(body.contains("method: filters.apiMethod"));
            assert!(body.contains("category: filters.apiCategory"));
            assert!(body.contains("mutating: filters.apiMutating"));
            assert!(body.contains("authorized: filters.apiAuthorized"));
            assert!(body.contains("queryUrl(\"/api/smart_home/state_history\", {"));
            assert!(body.contains("entity_id: activityEntity"));
            assert!(body.contains("event_type: historyType"));
            assert!(body.contains("bridge_id: filters.historyBridge"));
            assert!(body.contains("from_ms: filters.historyFromMs"));
            assert!(body.contains("to_ms: filters.historyToMs"));
            assert!(body.contains("received_at_or_after_ms: filters.historyReceivedFromMs"));
            assert!(body.contains("received_at_or_before_ms: filters.historyReceivedToMs"));
            assert!(body.matches("entity_id: activityEntity").count() >= 2);
            assert!(body.contains("queryUrl(\"/api/smart_home/services\", {"));
            assert!(body.contains("service: filters.serviceName"));
            assert!(body.contains("capability_id: filters.serviceCapability"));
            assert!(body.contains("entity_id: filters.serviceEntity"));
            assert!(body.contains("scene_id: filters.serviceScene"));
            assert!(body.contains("queryUrl(\"/api/smart_home/capabilities\", {"));
            assert!(body.contains("capability_id: filters.capabilityId"));
            assert!(body.contains("commandable: filters.capabilityCommandable"));
            assert!(body.contains("observable: filters.capabilityObservable"));
            assert!(body.contains("json(\"/api/smart_home/rooms?sort=scene_count\")"));
            assert!(body.contains("queryUrl(\"/api/smart_home/devices\", {"));
            assert!(body.contains("bridge_id: filters.deviceBridge"));
            assert!(body.contains("manufacturer: filters.deviceManufacturer"));
            assert!(body.contains("health: filters.deviceHealth"));
            assert!(body.contains("queryUrl(\"/api/smart_home/bridges\", {"));
            assert!(body.contains("integration_id: filters.bridgeIntegration"));
            assert!(body.contains("transport: filters.bridgeTransport"));
            assert!(body.contains("health: filters.bridgeHealth"));
            assert!(body.contains("queryUrl(\"/api/smart_home/events\", {"));
            assert!(body.contains("from_sequence: filters.eventFromSequence"));
            assert!(body.contains("to_sequence: filters.eventToSequence"));
            assert!(body.contains("queryUrl(\"/api/smart_home/command_results\", {"));
            assert!(body.contains("room_id: roomId"));
            assert!(body.contains("status: filters.commandStatus"));
            assert!(body.contains("from_sequence: filters.commandFromSequence"));
            assert!(body.contains("to_sequence: filters.commandToSequence"));
            assert!(body.contains("queryUrl(\"/api/smart_home/authorization_decisions\", {"));
            assert!(body.contains("outcome: filters.authorizationOutcome"));
            assert!(body.contains("renderRoomOptions(rooms, filters.room)"));
            assert!(body.contains("entityMatchesFilters(filters, entity)"));
            assert!(body.contains("renderCapabilities(capabilities)"));
            assert!(body.contains("filterRows(history.events || [], filters)"));
            assert!(body.contains("id=\"capabilities\""));
            assert!(body.contains("capabilityDetailUrl(capability)"));
            assert!(body.contains("capabilityServicesUrl(capability)"));
            assert!(body.contains("capabilityEntitiesUrl(capability)"));
            assert!(body.contains("<tbody id=\"events\"></tbody>"));
            assert!(body.contains("id=\"detail-body\""));
            assert!(body.contains("renderDetail(label, url, response.status, response.ok, body)"));
            assert!(body.contains("inspectDetail(inspectDetailButton)"));
            assert!(body.contains("data-inspect-url"));
            assert!(body.contains("const commandActionFollowUp = (body, entityId)"));
            assert!(body.contains("const desiredStateFollowUp = (entityId)"));
            assert!(body
                .contains("const renderActionDetail = (label, url, status, ok, body, followUp)"));
            assert!(body.contains("const actionJson = async (url, options, label"));
            assert!(body.contains("follow_up: followUp"));
            assert!(body.contains("accepted: ok"));
            assert!(
                body.contains("/api/smart_home/command_results/${encodeURIComponent(commandId)}")
            );
            assert!(body.contains(
                "/api/smart_home/command_results?correlation_id=${encodeURIComponent(correlationId)}"
            ));
            assert!(body.contains("const eventLinks = entry.links || {}"));
            assert!(body.contains("const eventDetailUrl = eventLinks.self ||"));
            assert!(body.contains("const resultLinks = result.links || {}"));
            assert!(body.contains("const recordLinks = record.links || {}"));
            assert!(body.contains("const eventUrl = recordLinks.event"));
            assert!(body.contains("const links = record.links || {}"));
            assert!(body.contains("const grantsUrl = links.principal_grants ||"));
            assert!(body.contains("links.subject_command_result"));
            assert!(body.contains("stateDetailUrl(entity)"));
            assert!(body.contains("entityDetailUrl(entity)"));
            assert!(body.contains("entityDesiredStateUrl(entity)"));
            assert!(body.contains("entityHistoryUrl(entity)"));
            assert!(body.contains("entityEventsUrl(entity)"));
            assert!(body.contains("entityBridgeCommandsUrl(entity)"));
            assert!(body.contains("sceneDetailUrl(scene)"));
            assert!(body.contains("serviceDetailUrl(service)"));
            assert!(body.contains("roomDetailUrl(room)"));
            assert!(
                body.contains("/api/smart_home/devices/${encodeURIComponent(device.device_id)}")
            );
            assert!(
                body.contains("/api/smart_home/bridges/${encodeURIComponent(bridge.bridge_id)}")
            );
            assert!(body.contains("/api/smart_home/state_history/"));
            assert!(body.contains("/api/smart_home/events/${entry.sequence}"));
            assert!(body.contains("/api/smart_home/command_results/"));
            assert!(
                body.contains("/api/smart_home/authorization_decisions/${record.decision_index}")
            );
            assert!(body.contains("/api/services/light/"));
            assert!(body.contains("light.${serviceButton.dataset.service} response"));
            assert!(
                body.contains("commandActionFollowUp(responseBody, serviceButton.dataset.entity)")
            );
            assert!(body.contains("scene.turn_on response"));
            assert!(body.contains("data-service=\"set_brightness\""));
            assert!(body.contains("brightness_pct"));
            assert!(body.contains("/api/services/scene/turn_on"));
            assert!(body.contains("/api/smart_home/desired_states/"));
            assert!(body.contains("clear desired state response"));
            assert!(body.contains("desired ${desiredButton.dataset.desiredAction} response"));
        }
    }

    #[test]
    fn runtime_web_app_serves_native_manifest_and_pairing_inventory() {
        let mut d23 = hue_lighting_runtime();
        let bridge = d23.registry().bridges().next().cloned().unwrap();
        d23.start_pairing_session(smart_home_runtime::RuntimePairingSession::pending(
            RuntimePairingSessionId::trusted("pairing-dashboard-1"),
            &bridge,
            AgentId::trusted("agent:operator"),
            4_000,
            8_000,
            Vec::new(),
        ))
        .unwrap();
        let runtime =
            SmartHomePlatformHttpRuntime::new(d23, SmartHomePlatformHttpConfig::new("Codex Home"))
                .with_now_ms(5_000)
                .with_dashboard_manifest(fixture_dashboard_manifest());
        let app = home_assistant_runtime_web_app(runtime);

        let manifest: serde_json::Value = serde_json::from_str(&response_body(
            app.handle(request("GET", "/api/smart_home/dashboard_manifest"))
                .into(),
        ))
        .unwrap();
        assert_eq!(manifest["configured"], true);
        assert_eq!(manifest["summary"]["dashboards"], 1);
        assert_eq!(manifest["summary"]["entity_references"], 1);
        assert_eq!(
            manifest["manifest"]["dashboards"][0]["views"][0]["title"],
            "Living Room"
        );

        let pairing: serde_json::Value = serde_json::from_str(&response_body(
            app.handle(request(
                "GET",
                "/api/smart_home/pairing_sessions?status=pending&sort=expires_at",
            ))
            .into(),
        ))
        .unwrap();
        assert_eq!(pairing["summary"]["total_sessions"], 1);
        assert_eq!(pairing["sessions"][0]["session_id"], "pairing-dashboard-1");
        assert_eq!(pairing["sessions"][0]["status"], "pending_user_presence");

        let detail: serde_json::Value = serde_json::from_str(&response_body(
            app.handle(request(
                "GET",
                "/api/smart_home/pairing_sessions/pairing-dashboard-1",
            ))
            .into(),
        ))
        .unwrap();
        assert_eq!(detail["bridge_id"], bridge.bridge_id.as_str());

        let invalid: web_core::WebResponse = app
            .handle(request(
                "GET",
                "/api/smart_home/pairing_sessions?status=unknown",
            ))
            .into();
        assert_eq!(invalid.status, 400);
    }

    #[test]
    fn home_assistant_web_app_serves_config_states_services_and_events() {
        let state = fixture_state();
        let app = home_assistant_web_app(state);

        let root = response_body(app.handle(request("GET", "/api/")).into());
        assert_eq!(root, r#"{"message":"API running."}"#);

        let config = response_body(app.handle(request("GET", "/api/config")).into());
        assert!(config.contains(r#""location_name":"Codex Home""#));
        assert!(config.contains(r#""state_count":2"#));

        let states = response_body(app.handle(request("GET", "/api/states")).into());
        assert!(states.contains(r#""entity_id":"entity-light-1""#));
        assert!(states.contains(r#""domain":"light""#));
        assert!(states.contains(r#""state":"unknown""#));

        let one_state = response_body(
            app.handle(request("GET", "/api/states/entity-light-1"))
                .into(),
        );
        assert!(one_state.contains(r#""friendly_name":"Kitchen Light""#));
        assert!(one_state.contains(r#""light.on_off""#));
        assert!(one_state.contains(r#""light.brightness""#));
        assert!(one_state.contains(r#""light.color_temperature""#));

        let services = response_body(app.handle(request("GET", "/api/services")).into());
        assert!(services.contains(r#""domain":"light""#));
        assert!(services.contains(r#""service":"turn_on""#));
        assert!(services.contains(r#""service":"set_brightness""#));
        assert!(services.contains(r#""domain":"scene""#));

        let events = response_body(app.handle(request("GET", "/api/events")).into());
        assert!(events.contains(r#""event":"call_service""#));
        assert!(events.contains(r#""event":"state_changed""#));
    }

    #[test]
    fn home_assistant_web_app_serves_over_repo_http_server() {
        let (port, stop) = start_server(home_assistant_web_app(fixture_state()));
        let (status, body) = http_get(port, "/api/states/entity-light-1");
        stop.stop();

        assert_eq!(status, 200);
        assert!(body.contains(r#""entity_id":"entity-light-1""#));
        assert!(body.contains(r#""domain":"light""#));
        assert!(body.contains(r#""friendly_name":"Kitchen Light""#));
    }

    #[test]
    fn runtime_web_app_dispatches_authorized_light_service_calls() {
        let app = home_assistant_runtime_web_app(fixture_runtime(true));
        let response: web_core::WebResponse = app
            .handle(request_with_body(
                "POST",
                "/api/services/light/turn_on",
                r#"{"entity_id":"light.entity_light_1","brightness_pct":75,"idempotency_key":"ha:turn-on:kitchen"}"#,
            ))
            .into();

        let body = response_body(response.clone());
        assert_eq!(response.status, 200);
        assert!(body.contains(r#""domain":"light""#));
        assert!(body.contains(r#""service":"turn_on""#));
        assert!(body.contains(r#""result_count":2"#));
        assert!(body.contains(r#""status":"accepted""#));

        let state = response_body(
            app.handle(request("GET", "/api/states/light.entity_light_1"))
                .into(),
        );
        assert!(state.contains(r#""confidence":"optimistic""#));
        assert!(state.contains(r#""light.brightness":75"#));
    }

    #[test]
    fn runtime_web_app_serves_dashboard_ready_audit_routes() {
        let app = home_assistant_runtime_web_app(fixture_runtime(true));

        let snapshot = response_body(app.handle(request("GET", "/api/smart_home/runtime")).into());
        assert!(snapshot.contains(r#""registry":{"bridges":1"#));
        assert!(snapshot.contains(r#""event_bus":{"subscription_count":0"#));
        assert!(snapshot.contains(r#""pending_work":{"total":"#));
        assert!(snapshot.contains(r#""state_refresh_target_count":2"#));

        let response: web_core::WebResponse = app
            .handle(request_with_body(
                "POST",
                "/api/services/light/turn_on",
                r#"{"entity_id":"entity-light-1"}"#,
            ))
            .into();
        assert_eq!(response.status, 200);

        let command_results = response_body(
            app.handle(request(
                "GET",
                "/api/smart_home/command_results?status=accepted&limit=5",
            ))
            .into(),
        );
        assert!(command_results.contains(r#""total_results":1"#));
        assert!(command_results.contains(r#""status":"accepted""#));
        assert!(command_results.contains(r#""sequence":0"#));
        let command_results_json: JsonValue =
            serde_json::from_str(&command_results).expect("command results response is JSON");
        let command_id = command_results_json["results"][0]["result"]["command_id"]
            .as_str()
            .expect("command result exposes command_id");
        let command_bridge_id = command_results_json["results"][0]["result"]["bridge_id"]
            .as_str()
            .expect("command result exposes bridge_id");
        let command_correlation_id = command_results_json["results"][0]["result"]["correlation_id"]
            .as_str()
            .expect("command result exposes correlation_id");

        let command_detail_path = format!("/api/smart_home/command_results/{command_id}");
        let command_correlation_path =
            format!("/api/smart_home/command_results?correlation_id={command_correlation_id}");
        assert_eq!(
            command_results_json["results"][0]["links"]["self"].as_str(),
            Some(command_detail_path.as_str())
        );
        assert_eq!(
            command_results_json["results"][0]["links"]["event"].as_str(),
            Some("/api/smart_home/events/0")
        );
        assert_eq!(
            command_results_json["results"][0]["links"]["event_window"].as_str(),
            Some("/api/smart_home/events?from_sequence=0&to_sequence=0")
        );
        assert_eq!(
            command_results_json["results"][0]["result"]["links"]["command_results_by_correlation"]
                .as_str(),
            Some(command_correlation_path.as_str())
        );
        let command_detail = response_body(app.handle(request("GET", &command_detail_path)).into());
        assert!(command_detail.contains(r#""sequence":0"#));
        assert!(command_detail.contains(&format!(r#""command_id":"{command_id}""#)));
        assert!(command_detail.contains(r#""event":"/api/smart_home/events/0""#));

        let by_command_id = response_body(
            app.handle(request(
                "GET",
                &format!("/api/smart_home/command_results?command_id={command_id}"),
            ))
            .into(),
        );
        assert!(by_command_id.contains(r#""total_results":1"#));
        assert!(by_command_id.contains(&format!(r#""command_id":"{command_id}""#)));

        let by_bridge_id = response_body(
            app.handle(request(
                "GET",
                &format!("/api/smart_home/command_results?bridge_id={command_bridge_id}"),
            ))
            .into(),
        );
        assert!(by_bridge_id.contains(r#""total_results":1"#));
        assert!(by_bridge_id.contains(&format!(r#""bridge_id":"{command_bridge_id}""#)));

        let by_room = response_body(
            app.handle(request(
                "GET",
                "/api/smart_home/command_results?room_id=kitchen&limit=5",
            ))
            .into(),
        );
        assert!(by_room.contains(r#""total_results":1"#));
        assert!(by_room.contains(&format!(r#""bridge_id":"{command_bridge_id}""#)));

        let by_missing_room = response_body(
            app.handle(request(
                "GET",
                "/api/smart_home/command_results?room_id=garage&limit=5",
            ))
            .into(),
        );
        assert!(by_missing_room.contains(r#""total_results":0"#));

        let by_correlation_id = response_body(
            app.handle(request(
                "GET",
                &format!("/api/smart_home/command_results?correlation_id={command_correlation_id}"),
            ))
            .into(),
        );
        assert!(by_correlation_id.contains(r#""total_results":1"#));
        assert!(
            by_correlation_id.contains(&format!(r#""correlation_id":"{command_correlation_id}""#))
        );

        let command_sequence_window = response_body(
            app.handle(request(
                "GET",
                "/api/smart_home/command_results?from_sequence=0&to_sequence=0&limit=5",
            ))
            .into(),
        );
        assert!(command_sequence_window.contains(r#""total_results":1"#));
        assert!(command_sequence_window.contains(r#""sequence":0"#));

        let empty_command_sequence_window = response_body(
            app.handle(request(
                "GET",
                "/api/smart_home/command_results?from_sequence=1&to_sequence=1&limit=5",
            ))
            .into(),
        );
        assert!(empty_command_sequence_window.contains(r#""total_results":0"#));

        let unknown_command = response_body(
            app.handle(request(
                "GET",
                "/api/smart_home/command_results?command_id=missing-command",
            ))
            .into(),
        );
        assert!(unknown_command.contains(r#""total_results":0"#));

        let status_sorted_command_results = response_body(
            app.handle(request(
                "GET",
                "/api/smart_home/command_results?sort=status_then_newest&limit=5",
            ))
            .into(),
        );
        assert!(status_sorted_command_results.contains(r#""total_results":1"#));
        assert!(status_sorted_command_results.contains(r#""status":"accepted""#));

        let events = response_body(
            app.handle(request(
                "GET",
                "/api/smart_home/events?kind=commands&room_id=kitchen&sort=desc&limit=5",
            ))
            .into(),
        );
        assert!(events.contains(r#""command_results":1"#));
        assert!(events.contains(r#""kind":"command_result""#));

        let event_sequence_window = response_body(
            app.handle(request(
                "GET",
                "/api/smart_home/events?kind=commands&from_sequence=0&to_sequence=0&limit=5",
            ))
            .into(),
        );
        assert!(event_sequence_window.contains(r#""total_events":1"#));
        assert!(event_sequence_window.contains(r#""sequence":0"#));

        let empty_event_sequence_window = response_body(
            app.handle(request(
                "GET",
                "/api/smart_home/events?kind=commands&from_sequence=1&to_sequence=1&limit=5",
            ))
            .into(),
        );
        assert!(empty_event_sequence_window.contains(r#""total_events":0"#));

        let missing_room_events = response_body(
            app.handle(request(
                "GET",
                "/api/smart_home/events?kind=commands&room_id=garage&sort=desc&limit=5",
            ))
            .into(),
        );
        assert!(missing_room_events.contains(r#""total_events":0"#));

        let event_detail = response_body(
            app.handle(request("GET", "/api/smart_home/events/0"))
                .into(),
        );
        assert!(event_detail.contains(r#""sequence":0"#));
        assert!(event_detail.contains(r#""kind":"command_result""#));
        assert!(event_detail
            .contains(r#""event_window":"/api/smart_home/events?from_sequence=0&to_sequence=0""#));
        assert!(event_detail.contains(&format!(
            r#""command_result":"/api/smart_home/command_results/{command_id}""#
        )));

        let missing_event: web_core::WebResponse = app
            .handle(request("GET", "/api/smart_home/events/999"))
            .into();
        assert_eq!(missing_event.status, 404);

        let missing_command: web_core::WebResponse = app
            .handle(request(
                "GET",
                "/api/smart_home/command_results/missing-command",
            ))
            .into();
        assert_eq!(missing_command.status, 404);

        let decisions = response_body(
            app.handle(request(
                "GET",
                "/api/smart_home/authorization_decisions?outcome=allowed&limit=5",
            ))
            .into(),
        );
        assert!(decisions.contains(r#""allowed_decisions":2"#));
        assert!(decisions.contains(r#""principal_id":"agent:home-assistant-local-api""#));
        assert!(decisions.contains(r#""kind":"command""#));
        assert!(decisions.contains(r#""decision_index":"#));
        assert!(decisions.contains(r#""principal_grants":"/api/smart_home/capability_grants?principal_id=agent:home-assistant-local-api&status=active""#));
        assert!(decisions.contains(r#""subject_command_result":"/api/smart_home/command_results/"#));
        assert!(decisions.contains(r#""subject_authorization":"/api/smart_home/command_authorization?entity_id=entity-light-1&command_type=turn_on""#));
        let decisions_json: JsonValue =
            serde_json::from_str(&decisions).expect("authorization decisions response is JSON");
        let decision_index = decisions_json["decisions"][0]["decision_index"]
            .as_u64()
            .expect("authorization decision exposes decision_index");

        let principal_decisions = response_body(
            app.handle(request(
                "GET",
                "/api/smart_home/authorization_decisions?principal_id=agent:home-assistant-local-api&sort=oldest_first&limit=5",
            ))
            .into(),
        );
        assert!(principal_decisions.contains(r#""total_decisions":2"#));
        assert!(principal_decisions.contains(r#""principal_id":"agent:home-assistant-local-api""#));

        let decision_detail_path =
            format!("/api/smart_home/authorization_decisions/{decision_index}");
        let decision_detail =
            response_body(app.handle(request("GET", &decision_detail_path)).into());
        assert!(decision_detail.contains(&format!(r#""decision_index":{decision_index}"#)));
        assert!(decision_detail.contains(r#""principal_id":"agent:home-assistant-local-api""#));
        assert!(decision_detail.contains(&format!(
            r#""self":"/api/smart_home/authorization_decisions/{decision_index}""#
        )));

        let missing_decision: web_core::WebResponse = app
            .handle(request(
                "GET",
                "/api/smart_home/authorization_decisions/999",
            ))
            .into();
        assert_eq!(missing_decision.status, 404);
    }

    #[test]
    fn runtime_web_app_previews_command_authorization_without_dispatch() {
        let app = home_assistant_runtime_web_app(fixture_runtime(true));
        let preview = response_body(
            app.handle(request(
                "GET",
                "/api/smart_home/command_authorization?entity_id=light.entity_light_1&command_type=turn_on",
            ))
            .into(),
        );
        assert!(preview.contains(r#""principal_id":"agent:home-assistant-local-api""#));
        assert!(preview.contains(r#""entity_id":"entity-light-1""#));
        assert!(preview.contains(r#""home_assistant_entity_id":"light.entity_light_1""#));
        assert!(preview.contains(r#""command_type":"turn_on""#));
        assert!(preview.contains(r#""required_tier":"low_risk""#));
        assert!(preview.contains(r#""supported":true"#));
        assert!(preview.contains(r#""commandable":true"#));
        assert!(preview.contains(r#""authorized":true"#));
        assert!(preview.contains(r#""dispatchable":true"#));
        assert!(preview.contains(r#""required_capabilities":["light.on_off"]"#));
        assert!(preview.contains(r#""missing_capabilities":[]"#));
        assert!(preview.contains(
            r#""matched_grants":["grant:agent:home-assistant-local-api:local-api-full-access"]"#
        ));
        assert!(preview.contains(r#""tool_decision":{"outcome":"allowed""#));
        assert!(preview.contains(r#""command_decision":{"outcome":"allowed""#));

        let denied_app = home_assistant_runtime_web_app(fixture_runtime(false));
        let denied = response_body(
            denied_app
                .handle(request(
                    "GET",
                    "/api/smart_home/command_authorization?entity_id=light.entity_light_1&command_type=set_brightness",
                ))
                .into(),
        );
        assert!(denied.contains(r#""authorized":false"#));
        assert!(denied.contains(r#""dispatchable":false"#));
        assert!(denied.contains(r#""smart_home.command.light""#));
        assert!(denied.contains(r#""light.brightness""#));
        assert!(denied.contains(r#""tool_decision":{"outcome":"denied""#));
        assert!(denied.contains(r#""command_decision":{"outcome":"denied""#));

        let unsupported = response_body(
            app.handle(request(
                "GET",
                "/api/smart_home/command_authorization?entity_id=sensor.entity_sensor_1&command_type=turn_on",
            ))
            .into(),
        );
        assert!(unsupported.contains(r#""supported":false"#));
        assert!(unsupported.contains(r#""dispatchable":false"#));
        assert!(unsupported.contains(r#""unsupported_capabilities":["light.on_off"]"#));

        let invalid_command: web_core::WebResponse = app
            .handle(request(
                "GET",
                "/api/smart_home/command_authorization?entity_id=light.entity_light_1&command_type=blink",
            ))
            .into();
        assert_eq!(invalid_command.status, 400);
    }

    #[test]
    fn runtime_web_app_previews_desired_state_authorization_without_mutation() {
        let app = home_assistant_runtime_web_app(fixture_runtime(true));
        let preview = response_body(
            app.handle(request(
                "GET",
                "/api/smart_home/desired_state_authorization?entity_id=light.entity_light_1&operation=set",
            ))
            .into(),
        );
        assert!(preview.contains(r#""principal_id":"agent:home-assistant-local-api""#));
        assert!(preview.contains(r#""entity_id":"entity-light-1""#));
        assert!(preview.contains(r#""home_assistant_entity_id":"light.entity_light_1""#));
        assert!(preview.contains(r#""operation":"set""#));
        assert!(preview.contains(r#""tool_id":"smart_home.set_desired_state""#));
        assert!(preview.contains(r#""required_tier":"low_risk""#));
        assert!(preview.contains(r#""preview_only":true"#));
        assert!(preview.contains(r#""would_mutate_runtime":true"#));
        assert!(preview.contains(r#""authorized":true"#));
        assert!(preview.contains(r#""required_capabilities":["smart_home.command.light"]"#));
        assert!(preview.contains(r#""missing_capabilities":[]"#));
        assert!(preview.contains(
            r#""matched_grants":["grant:agent:home-assistant-local-api:local-api-full-access"]"#
        ));
        assert!(preview.contains(r#""tool_decision":{"outcome":"allowed""#));

        let clear_preview = response_body(
            app.handle(request(
                "GET",
                "/api/smart_home/desired_state_authorization?entity_id=light.entity_light_1&operation=clear",
            ))
            .into(),
        );
        assert!(clear_preview.contains(r#""operation":"clear""#));
        assert!(clear_preview.contains(r#""tool_id":"smart_home.clear_desired_state""#));
        assert!(clear_preview.contains(r#""authorized":true"#));

        let desired_states = response_body(
            app.handle(request(
                "GET",
                "/api/smart_home/desired_states?entity_id=light.entity_light_1",
            ))
            .into(),
        );
        assert!(desired_states.contains(r#""total_desired_states":0"#));

        let denied_app = home_assistant_runtime_web_app(fixture_runtime(false));
        let denied = response_body(
            denied_app
                .handle(request(
                    "GET",
                    "/api/smart_home/desired_state_authorization?entity_id=light.entity_light_1",
                ))
                .into(),
        );
        assert!(denied.contains(r#""operation":"set""#));
        assert!(denied.contains(r#""authorized":false"#));
        assert!(denied.contains(r#""missing_capabilities":["smart_home.command.light"]"#));
        assert!(denied.contains(r#""tool_decision":{"outcome":"denied""#));

        let invalid_operation: web_core::WebResponse = app
            .handle(request(
                "GET",
                "/api/smart_home/desired_state_authorization?entity_id=light.entity_light_1&operation=blink",
            ))
            .into();
        assert_eq!(invalid_operation.status, 400);
    }

    #[test]
    fn runtime_web_app_previews_scene_authorization_without_dispatch() {
        let app = home_assistant_runtime_web_app(fixture_runtime(true));
        let preview = response_body(
            app.handle(request(
                "GET",
                "/api/smart_home/scene_authorization?scene_id=scene.scene_kitchen_bright",
            ))
            .into(),
        );
        assert!(preview.contains(r#""principal_id":"agent:home-assistant-local-api""#));
        assert!(preview.contains(r#""scene_id":"scene-kitchen-bright""#));
        assert!(preview.contains(r#""home_assistant_scene_id":"scene.scene_kitchen_bright""#));
        assert!(preview.contains(r#""tool_id":"smart_home.command""#));
        assert!(preview.contains(r#""preview_only":true"#));
        assert!(preview.contains(r#""would_mutate_runtime":true"#));
        assert!(preview.contains(r#""command_count":2"#));
        assert!(preview.contains(r#""supported":true"#));
        assert!(preview.contains(r#""commandable":true"#));
        assert!(preview.contains(r#""authorized":true"#));
        assert!(preview.contains(r#""dispatchable":true"#));
        assert!(preview.contains(r#""light.on_off""#));
        assert!(preview.contains(r#""light.brightness""#));
        assert!(preview.contains(r#""smart_home.command.light""#));
        assert!(preview.contains(r#""missing_capabilities":[]"#));
        assert!(preview.contains(
            r#""matched_grants":["grant:agent:home-assistant-local-api:local-api-full-access"]"#
        ));
        assert!(preview.contains(r#""tool_decision":{"outcome":"allowed""#));
        assert!(preview.contains(r#""command_decision":{"outcome":"allowed""#));
        assert!(preview.contains(r#""home_assistant_entity_id":"light.entity_light_1""#));
        assert!(preview.contains(r#""command_type":"turn_on""#));
        assert!(preview.contains(r#""command_type":"set_brightness""#));

        let command_results = response_body(
            app.handle(request("GET", "/api/smart_home/command_results?limit=10"))
                .into(),
        );
        assert!(command_results.contains(r#""total_results":0"#));

        let denied_app = home_assistant_runtime_web_app(fixture_runtime(false));
        let denied = response_body(
            denied_app
                .handle(request(
                    "GET",
                    "/api/smart_home/scene_authorization?scene_id=scene.scene_kitchen_bright",
                ))
                .into(),
        );
        assert!(denied.contains(r#""authorized":false"#));
        assert!(denied.contains(r#""dispatchable":false"#));
        assert!(denied.contains(r#""smart_home.command.light""#));
        assert!(denied.contains(r#""light.brightness""#));
        assert!(denied.contains(r#""tool_decision":{"outcome":"denied""#));
        assert!(denied.contains(r#""command_decision":{"outcome":"denied""#));

        let missing_scene: web_core::WebResponse = app
            .handle(request("GET", "/api/smart_home/scene_authorization"))
            .into();
        assert_eq!(missing_scene.status, 400);

        let unknown_scene: web_core::WebResponse = app
            .handle(request(
                "GET",
                "/api/smart_home/scene_authorization?scene_id=scene.missing",
            ))
            .into();
        assert_eq!(unknown_scene.status, 404);
    }

    #[test]
    fn runtime_web_app_previews_service_authorization_without_dispatch() {
        let app = home_assistant_runtime_web_app(fixture_runtime(true));
        let preview = response_body(
            app.handle(request(
                "GET",
                "/api/smart_home/service_authorization/light/turn_on?entity_id=light.entity_light_1&brightness_pct=75",
            ))
            .into(),
        );
        assert!(preview.contains(r#""principal_id":"agent:home-assistant-local-api""#));
        assert!(preview.contains(r#""domain":"light""#));
        assert!(preview.contains(r#""service":"turn_on""#));
        assert!(preview.contains(r#""home_assistant_path":"/api/services/light/turn_on""#));
        assert!(preview.contains(r#""tool_id":"smart_home.command""#));
        assert!(preview.contains(r#""preview_only":true"#));
        assert!(preview.contains(r#""would_mutate_runtime":true"#));
        assert!(preview.contains(r#""target_entity_ids":["light.entity_light_1"]"#));
        assert!(preview.contains(r#""target_scene_ids":[]"#));
        assert!(preview.contains(r#""command_count":2"#));
        assert!(preview.contains(r#""supported":true"#));
        assert!(preview.contains(r#""commandable":true"#));
        assert!(preview.contains(r#""authorized":true"#));
        assert!(preview.contains(r#""dispatchable":true"#));
        assert!(preview.contains(r#""light.on_off""#));
        assert!(preview.contains(r#""light.brightness""#));
        assert!(preview.contains(r#""smart_home.command.light""#));
        assert!(preview.contains(r#""missing_capabilities":[]"#));
        assert!(preview.contains(
            r#""matched_grants":["grant:agent:home-assistant-local-api:local-api-full-access"]"#
        ));
        assert!(preview.contains(r#""tool_decision":{"outcome":"allowed""#));
        assert!(preview.contains(r#""command_decision":{"outcome":"allowed""#));
        assert!(preview.contains(r#""home_assistant_entity_id":"light.entity_light_1""#));
        assert!(preview.contains(r#""command_type":"turn_on""#));
        assert!(preview.contains(r#""command_type":"set_brightness""#));

        let command_results = response_body(
            app.handle(request("GET", "/api/smart_home/command_results?limit=10"))
                .into(),
        );
        assert!(command_results.contains(r#""total_results":0"#));

        let denied_app = home_assistant_runtime_web_app(fixture_runtime(false));
        let denied = response_body(
            denied_app
                .handle(request(
                    "GET",
                    "/api/smart_home/service_authorization/light/turn_on?entity_id=light.entity_light_1&brightness_pct=75",
                ))
                .into(),
        );
        assert!(denied.contains(r#""authorized":false"#));
        assert!(denied.contains(r#""dispatchable":false"#));
        assert!(denied.contains(r#""smart_home.command.light""#));
        assert!(denied.contains(r#""light.brightness""#));
        assert!(denied.contains(r#""tool_decision":{"outcome":"denied""#));
        assert!(denied.contains(r#""command_decision":{"outcome":"denied""#));

        let missing_target: web_core::WebResponse = app
            .handle(request(
                "GET",
                "/api/smart_home/service_authorization/light/turn_on",
            ))
            .into();
        assert_eq!(missing_target.status, 400);

        let invalid_brightness: web_core::WebResponse = app
            .handle(request(
                "GET",
                "/api/smart_home/service_authorization/light/turn_on?entity_id=light.entity_light_1&brightness_pct=101",
            ))
            .into();
        assert_eq!(invalid_brightness.status, 400);
    }

    #[test]
    fn runtime_web_app_serves_dashboard_ready_capability_grants() {
        let app = home_assistant_runtime_web_app(fixture_runtime(true));

        let grants = response_body(
            app.handle(request(
                "GET",
                "/api/smart_home/capability_grants?principal_id=agent:home-assistant-local-api&status=active&scope=all_smart_home&sort=principal_id&limit=5",
            ))
            .into(),
        );
        assert!(grants.contains(r#""generated_at_ms":5000"#));
        assert!(grants.contains(r#""total_grants":1"#));
        assert!(grants.contains(r#""active_grants":1"#));
        assert!(grants.contains(r#""all_smart_home_grants":1"#));
        assert!(grants.contains(r#""high_risk_tier_grants":1"#));
        assert!(grants.contains(r#""unique_principals":1"#));
        assert!(grants.contains(r#""principal_id":"agent:home-assistant-local-api""#));
        assert!(grants.contains(r#""scope":{"kind":"all_smart_home"}"#));
        assert!(grants.contains(r#""configured_status":"active""#));
        assert!(grants.contains(r#""effective_status":"active""#));
        assert!(grants.contains(r#""active":true"#));

        let grants_json: JsonValue =
            serde_json::from_str(&grants).expect("capability grants response is JSON");
        let grant_id = grants_json["grants"][0]["grant_id"]
            .as_str()
            .expect("capability grant exposes grant_id");

        let grant_detail_path = format!("/api/smart_home/capability_grants/{grant_id}");
        let grant_detail = response_body(app.handle(request("GET", &grant_detail_path)).into());
        assert!(grant_detail.contains(&format!(r#""grant_id":"{grant_id}""#)));
        assert!(grant_detail.contains(r#""max_tier":"high_risk""#));
        assert!(grant_detail.contains(r#""granted_by":"test""#));

        let missing_grant: web_core::WebResponse = app
            .handle(request(
                "GET",
                "/api/smart_home/capability_grants/missing-grant",
            ))
            .into();
        assert_eq!(missing_grant.status, 404);

        let invalid_status: web_core::WebResponse = app
            .handle(request(
                "GET",
                "/api/smart_home/capability_grants?status=unknown",
            ))
            .into();
        assert_eq!(invalid_status.status, 400);
    }

    #[test]
    fn runtime_web_app_serves_dashboard_ready_health_probe() {
        let app = home_assistant_runtime_web_app(fixture_runtime(true));
        let health = response_body(app.handle(request("GET", "/api/smart_home/health")).into());

        assert!(health.contains(r#""generated_at_ms":5000"#));
        assert!(health.contains(r#""status":"attention""#));
        assert!(health.contains(r#""live":true"#));
        assert!(health.contains(r#""ready":true"#));
        assert!(health.contains(r#""has_pending_work":true"#));
        assert!(health.contains(r#""has_attention":false"#));
        assert!(health.contains(r#""has_state_gaps":true"#));
        assert!(health.contains(r#""bridge_count":1"#));
        assert!(health.contains(r#""online_bridges":1"#));
        assert!(health.contains(r#""device_count":1"#));
        assert!(health.contains(r#""online_devices":1"#));
        assert!(health.contains(r#""entity_count":2"#));
        assert!(health.contains(r#""entities_without_state":2"#));
        assert!(health.contains(r#""stale_entities":0"#));
        assert!(health.contains(r#""pending_work_total":2"#));
        assert!(health.contains(r#""state_refresh_target_count":2"#));
        assert!(health.contains(r#""registry":"ok""#));
        assert!(health.contains(r#""event_bus":"ok""#));
        assert!(health.contains(r#""discovery":"ok""#));
        assert!(health.contains(r#""supervisor":"ok""#));
        assert!(health.contains(r#""state":"attention""#));
    }

    #[test]
    fn runtime_web_app_serves_readiness_checklist() {
        let app = home_assistant_runtime_web_app(fixture_runtime(true));
        let readiness = response_body(
            app.handle(request("GET", "/api/smart_home/readiness"))
                .into(),
        );

        assert!(readiness.contains(r#""generated_at_ms":5000"#));
        assert!(readiness.contains(r#""status":"attention""#));
        assert!(readiness.contains(r#""ready":true"#));
        assert!(readiness.contains(r#""total_checks":8"#));
        assert!(readiness.contains(r#""passing_checks":7"#));
        assert!(readiness.contains(r#""attention_checks":1"#));
        assert!(readiness.contains(r#""blocking_checks":0"#));
        assert!(readiness.contains(r#""health":"/api/smart_home/health""#));
        assert!(readiness.contains(r#""smoke":"/api/smart_home/smoke""#));
        assert!(readiness.contains(r#""controller_handoff":"/api/smart_home/controller_handoff""#));
        assert!(readiness.contains(r#""state_gaps":"/api/smart_home/states?stale=true""#));
        assert!(readiness.contains(r#""check_id":"registry""#));
        assert!(readiness.contains(r#""status":"ok""#));
        assert!(readiness.contains(r#""check_id":"state_coverage""#));
        assert!(readiness.contains(r#""route":"/api/smart_home/states?stale=true""#));
        assert!(readiness.contains(r#"2 entities need state refresh"#));
        assert!(readiness.contains(r#""check_id":"authorization""#));
        assert!(readiness.contains(r#"1 capability grants are available"#));
    }

    #[test]
    fn runtime_web_app_serves_controller_handoff_manifest() {
        let app = home_assistant_runtime_web_app(fixture_runtime(true));
        let handoff = response_body(app.handle(request("GET", CONTROLLER_HANDOFF_PATH)).into());

        assert!(handoff.contains(r#""generated_at_ms":5000"#));
        assert!(handoff.contains(r#""version":"0.1.0""#));
        assert!(handoff.contains(r#""status":"ready""#));
        assert!(handoff.contains(r#""ready":true"#));
        assert!(handoff.contains(r#""principal_id":"agent:home-assistant-local-api""#));
        assert!(handoff.contains(r#""self":"/api/smart_home/controller_handoff""#));
        assert!(handoff.contains(r#""readiness":"/api/smart_home/readiness""#));
        assert!(handoff.contains(r#""bootstrap":"/api/smart_home/bootstrap""#));
        assert!(handoff.contains(r#""smoke":"/api/smart_home/smoke""#));
        assert!(handoff.contains(r#""api":"/api/smart_home/api""#));
        assert!(handoff.contains(r#""category_id":"repo_http_stack""#));
        assert!(handoff.contains(r#""category_id":"browser_dashboard""#));
        assert!(handoff.contains(r#""category_id":"fixture_controller""#));
        assert!(handoff.contains(r#""category_id":"state_history_events""#));
        assert!(handoff.contains(r#""category_id":"commands_services_scenes""#));
        assert!(handoff.contains(r#""category_id":"authorization_boundaries""#));
        assert!(handoff.contains(r#""evidence":["/api/smart_home/capability_grants""#));
        assert!(handoff.contains(r#""/api/smart_home/service_authorization/:domain/:service""#));

        let handoff_json: JsonValue =
            serde_json::from_str(&handoff).expect("controller handoff response is JSON");
        assert_eq!(handoff_json["summary"]["total_categories"], 6);
        assert_eq!(handoff_json["summary"]["ready_categories"], 6);
        assert_eq!(handoff_json["summary"]["attention_categories"], 0);
        assert_eq!(handoff_json["summary"]["blocked_categories"], 0);
        assert!(
            handoff_json["summary"]["smart_home_routes"]
                .as_u64()
                .unwrap_or_default()
                >= 30
        );
        assert_eq!(handoff_json["summary"]["browser_routes"], 3);
        assert_eq!(handoff_json["summary"]["runtime_authorized_routes"], 6);
        assert_eq!(handoff_json["summary"]["readiness_checks"], 8);
        assert_eq!(handoff_json["summary"]["smoke_checks"], 15);
    }

    #[test]
    fn runtime_web_app_serves_dashboard_overview() {
        let app = home_assistant_runtime_web_app(fixture_runtime(true));
        let response: web_core::WebResponse = app
            .handle(request_with_body(
                "POST",
                "/api/services/light/turn_on",
                r#"{"entity_id":"entity-light-1"}"#,
            ))
            .into();
        assert_eq!(response.status, 200);

        let dashboard = response_body(
            app.handle(request("GET", "/api/smart_home/dashboard"))
                .into(),
        );

        assert!(dashboard.contains(r#""generated_at_ms":5000"#));
        assert!(dashboard.contains(r#""config":{"location_name":"Codex Home""#));
        assert!(dashboard.contains(r#""summary":{"state_count":2"#));
        assert!(dashboard.contains(r#""bridge_count":1"#));
        assert!(dashboard.contains(r#""device_count":1"#));
        assert!(dashboard.contains(r#""entity_count":2"#));
        assert!(dashboard.contains(r#""room_count":1"#));
        assert!(dashboard.contains(r#""scene_count":1"#));
        assert!(dashboard.contains(r#""pending_work_total":"#));
        assert!(dashboard.contains(r#""has_state_gaps":true"#));
        assert!(dashboard.contains(r#""health":{"generated_at_ms":5000"#));
        assert!(dashboard.contains(r#""status":"attention""#));
        assert!(dashboard.contains(r#""ready":true"#));
        assert!(dashboard.contains(r#""runtime":{"generated_at_ms":5000"#));
        assert!(dashboard.contains(r#""topology":{"bridges":1"#));
        assert!(dashboard.contains(r#""bridges":{"summary":{"total_bridges":1"#));
        assert!(dashboard.contains(r#""devices":{"summary":{"total_devices":1"#));
        assert!(dashboard.contains(r#""entities":{"summary":{"total_entities":2"#));
        assert!(dashboard.contains(r#""capabilities":{"summary":{"total_capabilities":4"#));
        assert!(dashboard.contains(r#""rooms":{"summary":{"total_rooms":1"#));
        assert!(dashboard.contains(r#""desired_states":{"summary":{"total_desired_states":0"#));
        assert!(dashboard.contains(r#""events":{"summary":{"total_events":1"#));
        assert!(dashboard.contains(r#""command_results":{"summary":{"total_results":1"#));
        assert!(dashboard.contains(r#""authorization_decisions":{"summary":{"total_decisions":2"#));
    }

    #[test]
    fn runtime_web_app_serves_dashboard_bootstrap_payload() {
        let app = home_assistant_runtime_web_app(fixture_runtime(true));
        let response: web_core::WebResponse = app
            .handle(request_with_body(
                "POST",
                "/api/services/light/turn_on",
                r#"{"entity_id":"entity-light-1"}"#,
            ))
            .into();
        assert_eq!(response.status, 200);

        let bootstrap = response_body(
            app.handle(request("GET", "/api/smart_home/bootstrap"))
                .into(),
        );

        assert!(bootstrap.contains(r#""generated_at_ms":5000"#));
        assert!(bootstrap.contains(r#""version":"0.1.0""#));
        assert!(bootstrap.contains(r#""readiness":"/api/smart_home/readiness""#));
        assert!(bootstrap.contains(r#""controller_handoff":"/api/smart_home/controller_handoff""#));
        assert!(bootstrap.contains(r#""dashboard":"/api/smart_home/dashboard""#));
        assert!(bootstrap.contains(r#""smoke":"/api/smart_home/smoke""#));
        assert!(bootstrap.contains(r#""smoke_script":"/api/smart_home/smoke_script""#));
        assert!(bootstrap.contains(r#""states":"/api/smart_home/states""#));
        assert!(bootstrap
            .contains(r#""command_authorization":"/api/smart_home/command_authorization""#));
        assert!(bootstrap.contains(
            r#""desired_state_authorization":"/api/smart_home/desired_state_authorization""#
        ));
        assert!(
            bootstrap.contains(r#""scene_authorization":"/api/smart_home/scene_authorization""#)
        );
        assert!(bootstrap
            .contains(r#""service_authorization":"/api/smart_home/service_authorization""#));
        assert!(bootstrap.contains(r#""health":{"generated_at_ms":5000"#));
        assert!(bootstrap.contains(r#""dashboard":{"generated_at_ms":5000"#));
        assert!(bootstrap.contains(r#""api":{"version":"0.1.0""#));
        assert!(bootstrap.contains(r#""path":"/api/smart_home/bootstrap""#));
        assert!(bootstrap.contains(r#""state_gaps":{"summary":{"total_entities":1"#));
        assert!(bootstrap.contains(r#""entity_id":"entity-sensor-1""#));
        assert!(bootstrap.contains(r#""recent_activity":{"events":{"summary":{"total_events":1"#));
        assert!(bootstrap.contains(r#""command_results":{"summary":{"total_results":1"#));
        assert!(bootstrap.contains(r#""authorization_decisions":{"summary":{"total_decisions":2"#));
    }

    #[test]
    fn runtime_web_app_serves_fixture_controller_smoke_plan() {
        let app = home_assistant_runtime_web_app(fixture_runtime(true));
        let smoke = response_body(app.handle(request("GET", "/api/smart_home/smoke")).into());

        assert!(smoke.contains(r#""generated_at_ms":5000"#));
        assert!(smoke.contains(r#""status":"attention""#));
        assert!(smoke.contains(r#""ready":true"#));
        assert!(smoke.contains(r#""principal_id":"agent:home-assistant-local-api""#));
        assert!(smoke.contains(r#""self":"/api/smart_home/smoke""#));
        assert!(smoke.contains(r#""script":"/api/smart_home/smoke_script""#));
        assert!(smoke.contains(r#""controller_handoff":"/api/smart_home/controller_handoff""#));
        assert!(
            smoke.contains(r#""command_authorization":"/api/smart_home/command_authorization""#)
        );
        assert!(smoke.contains(
            r#""desired_state_authorization":"/api/smart_home/desired_state_authorization""#
        ));
        assert!(smoke.contains(r#""scene_authorization":"/api/smart_home/scene_authorization""#));
        assert!(
            smoke.contains(r#""service_authorization":"/api/smart_home/service_authorization""#)
        );
        assert!(smoke.contains(r#""check_id":"command_authorization_preview""#));
        assert!(smoke.contains(
            r#""path":"/api/smart_home/command_authorization?entity_id=light.entity_light_1&command_type=turn_on""#
        ));
        assert!(smoke.contains(r#""check_id":"desired_state_authorization_preview""#));
        assert!(smoke.contains(
            r#""path":"/api/smart_home/desired_state_authorization?entity_id=light.entity_light_1&operation=set""#
        ));
        assert!(smoke.contains(r#""check_id":"scene_authorization_preview""#));
        assert!(smoke.contains(
            r#""path":"/api/smart_home/scene_authorization?scene_id=scene.scene_kitchen_bright""#
        ));
        assert!(smoke.contains(r#""check_id":"service_authorization_preview""#));
        assert!(smoke.contains(
            r#""path":"/api/smart_home/service_authorization/light/turn_on?entity_id=light.entity_light_1&brightness_pct=75""#
        ));
        assert!(smoke.contains(r#""check_id":"command_probe""#));
        assert!(smoke.contains(r#""path":"/api/services/light/turn_on""#));
        assert!(smoke.contains(r#""check_id":"controller_handoff""#));
        assert!(smoke.contains(r#""path":"/api/smart_home/controller_handoff""#));
        assert!(smoke.contains(r#""runtime_authorized":true"#));
        assert!(smoke.contains(r#""entity_id":"light.entity_light_1""#));
        assert!(smoke.contains(r#""brightness_pct":75"#));

        let smoke_json: JsonValue =
            serde_json::from_str(&smoke).expect("smoke plan response is JSON");
        assert_eq!(smoke_json["summary"]["total_checks"], 15);
        assert_eq!(smoke_json["summary"]["safe_get_checks"], 14);
        assert_eq!(smoke_json["summary"]["mutating_checks"], 1);
        assert_eq!(smoke_json["summary"]["runtime_authorized_checks"], 1);
        assert_eq!(smoke_json["summary"]["blocking_readiness_checks"], 0);
        assert_eq!(smoke_json["summary"]["attention_readiness_checks"], 1);
        assert_eq!(
            smoke_json["checks"][10]["request_body"]["entity_id"],
            "light.entity_light_1"
        );
        assert_eq!(
            smoke_json["checks"][10]["request_body"]["brightness_pct"],
            75
        );
    }

    #[test]
    fn runtime_web_app_serves_fixture_controller_smoke_script() {
        let app = home_assistant_runtime_web_app(fixture_runtime(true));
        let response: web_core::WebResponse = app
            .handle(request("GET", "/api/smart_home/smoke_script"))
            .into();
        assert_eq!(response.status, 200);
        assert!(response.headers.iter().any(|(name, value)| {
            name == "content-type" && value == "text/plain; charset=utf-8"
        }));
        let script = response_body(response);

        assert!(script.contains("#!/usr/bin/env sh"));
        assert!(script.contains("set -eu"));
        assert!(script.contains("SMART_HOME_BASE_URL"));
        assert!(script.contains("BASE_URL=${SMART_HOME_BASE_URL:-'http://127.0.0.1:8123'}"));
        assert!(script.contains("CURL=${CURL:-curl}"));
        assert!(script.contains(r#"run_check 'Dashboard shell' 'GET' '/' '200'"#));
        assert!(script.contains(
            r#"run_check 'Command authorization preview' 'GET' '/api/smart_home/command_authorization?entity_id=light.entity_light_1&command_type=turn_on' '200'"#
        ));
        assert!(script.contains(
            r#"run_check 'Desired-state authorization preview' 'GET' '/api/smart_home/desired_state_authorization?entity_id=light.entity_light_1&operation=set' '200'"#
        ));
        assert!(script.contains(
            r#"run_check 'Scene authorization preview' 'GET' '/api/smart_home/scene_authorization?scene_id=scene.scene_kitchen_bright' '200'"#
        ));
        assert!(script.contains(
            r#"run_check 'Service authorization preview' 'GET' '/api/smart_home/service_authorization/light/turn_on?entity_id=light.entity_light_1&brightness_pct=75' '200'"#
        ));
        assert!(script.contains(
            r#"run_check 'Command probe' 'POST' '/api/services/light/turn_on' '200' '{"entity_id":"light.entity_light_1","brightness_pct":75}'"#
        ));
        assert!(script.contains(
            r#"run_check 'Controller handoff' 'GET' '/api/smart_home/controller_handoff' '200'"#
        ));
        assert!(script.contains("All smart-home smoke checks passed (15 checks)"));
    }

    #[test]
    fn runtime_web_app_serves_dashboard_ready_api_catalog() {
        let app = home_assistant_runtime_web_app(fixture_runtime(true));

        let catalog = response_body(app.handle(request("GET", "/api/smart_home/api")).into());
        assert!(catalog.contains(r#""path":"/api/smart_home/readiness""#));
        assert!(catalog.contains(r#""path":"/api/smart_home/controller_handoff""#));
        assert!(catalog.contains(r#""path":"/api/smart_home/dashboard""#));
        assert!(catalog.contains(r#""path":"/api/smart_home/smoke""#));
        assert!(catalog.contains(r#""path":"/api/smart_home/smoke_script""#));
        assert!(catalog.contains(r#""path":"/api/smart_home/rooms/:room_id""#));
        assert!(catalog.contains(r#""path":"/api/services/:domain/:service""#));
        assert!(catalog
            .contains(r#""query_params":["authorized","category","method","mutating","surface"]"#));
        assert!(catalog.contains(
            r#""path":"/api/smart_home/entities","category":"entities","surface":"smart_home","mutates_runtime":false,"runtime_authorized":false,"query_params":["capability_id","commandable","domain","kind","limit","room_id"]"#
        ));
        assert!(catalog.contains(
            r#""path":"/api/smart_home/states","category":"states","surface":"smart_home","mutates_runtime":false,"runtime_authorized":false,"query_params":["capability_id","confidence","domain","has_state","kind","limit","room_id","source","stale"]"#
        ));
        assert!(catalog.contains(
            r#""path":"/api/smart_home/events","category":"events","surface":"smart_home","mutates_runtime":false,"runtime_authorized":false,"query_params":["entity_id","from_sequence","kind","limit","room_id","sort","to_sequence"]"#
        ));
        assert!(catalog.contains(
            r#""path":"/api/smart_home/command_results","category":"command_results","surface":"smart_home","mutates_runtime":false,"runtime_authorized":false,"query_params":["bridge_id","command_id","correlation_id","from_sequence","limit","room_id","sort","status","to_sequence"]"#
        ));
        assert!(catalog.contains(
            r#""path":"/api/smart_home/capability_grants","category":"authorization","surface":"smart_home","mutates_runtime":false,"runtime_authorized":false,"query_params":["capability_id","entity_id","limit","principal_id","scope","sort","status"]"#
        ));
        assert!(catalog.contains(
            r#""path":"/api/smart_home/command_authorization","category":"authorization","surface":"smart_home","mutates_runtime":false,"runtime_authorized":false,"query_params":["command_type","entity_id","principal_id"]"#
        ));
        assert!(catalog.contains(
            r#""path":"/api/smart_home/desired_state_authorization","category":"authorization","surface":"smart_home","mutates_runtime":false,"runtime_authorized":false,"query_params":["entity_id","operation","principal_id"]"#
        ));
        assert!(catalog.contains(
            r#""path":"/api/smart_home/scene_authorization","category":"authorization","surface":"smart_home","mutates_runtime":false,"runtime_authorized":false,"query_params":["scene_id","principal_id"]"#
        ));
        assert!(catalog.contains(
            r#""path":"/api/smart_home/service_authorization/:domain/:service","category":"authorization","surface":"smart_home","mutates_runtime":false,"runtime_authorized":false,"query_params":["brightness","brightness_pct","color_temp","color_temp_kelvin","entity_id","entity_ids","idempotency_key","kelvin","principal_id","rgb_color","scene_id","scene_ids","temperature","timeout_ms"]"#
        ));
        assert!(catalog.contains(
            r#""path":"/api/smart_home/state_history","category":"state_history","surface":"smart_home","mutates_runtime":false,"runtime_authorized":false,"query_params":["bridge_id","entity_id","event_type","from_ms","limit","observed_at_or_after_ms","observed_at_or_before_ms","received_at_or_after_ms","received_at_or_before_ms","room_id","sort","to_ms"]"#
        ));
        let catalog_json: JsonValue =
            serde_json::from_str(&catalog).expect("API catalog response is JSON");
        assert!(
            catalog_json["route_count"].as_u64().unwrap_or_default() >= 30,
            "catalog exposes the local controller route surface"
        );

        let handoff_routes = response_body(
            app.handle(request("GET", "/api/smart_home/api?category=handoff"))
                .into(),
        );
        let handoff_routes_json: JsonValue =
            serde_json::from_str(&handoff_routes).expect("handoff API catalog response is JSON");
        assert_eq!(handoff_routes_json["route_count"], 1);
        assert!(handoff_routes.contains(r#""path":"/api/smart_home/controller_handoff""#));

        let browser = response_body(
            app.handle(request("GET", "/api/smart_home/api?surface=browser"))
                .into(),
        );
        let browser_json: JsonValue =
            serde_json::from_str(&browser).expect("browser API catalog response is JSON");
        assert_eq!(browser_json["route_count"], 3);
        assert!(browser.contains(r#""path":"/""#));
        assert!(browser.contains(r#""path":"/dashboard""#));
        assert!(browser.contains(r#""path":"/smart-home""#));

        let mutating = response_body(
            app.handle(request(
                "GET",
                "/api/smart_home/api?mutating=true&authorized=true",
            ))
            .into(),
        );
        let mutating_json: JsonValue =
            serde_json::from_str(&mutating).expect("mutating API catalog response is JSON");
        assert_eq!(mutating_json["route_count"], 6);
        for route in mutating_json["routes"]
            .as_array()
            .expect("mutating route list is an array")
        {
            assert_eq!(route["mutates_runtime"], true);
            assert_eq!(route["runtime_authorized"], true);
        }

        let home_assistant_posts = response_body(
            app.handle(request(
                "GET",
                "/api/smart_home/api?surface=home_assistant&method=post",
            ))
            .into(),
        );
        assert!(home_assistant_posts.contains(r#""route_count":2"#));
        assert!(home_assistant_posts.contains(r#""path":"/api/states/:entity_id""#));
        assert!(home_assistant_posts.contains(r#""path":"/api/services/:domain/:service""#));

        let invalid_surface: web_core::WebResponse = app
            .handle(request("GET", "/api/smart_home/api?surface=unknown"))
            .into();
        assert_eq!(invalid_surface.status, 400);
    }

    #[test]
    fn runtime_web_app_creates_previews_executes_and_audits_automation() {
        let automations = Arc::new(Mutex::new(SmartHomeAutomationRuntime::new()));
        let runtime = fixture_runtime(true).with_automation_runtime(Arc::clone(&automations));
        let app = home_assistant_runtime_web_app(runtime);
        let definition = serde_json::to_string(&schedule_automation_definition()).unwrap();

        let created: WebResponse = app
            .handle(request_with_body(
                "POST",
                "/api/smart_home/automations",
                &definition,
            ))
            .into();
        assert_eq!(created.status, 200);
        assert!(response_body(created).contains(r#""automation_id":"nightly-off""#));

        let preview: WebResponse = app
            .handle(request_with_body(
                "POST",
                "/api/smart_home/automations/evaluate",
                r#"{"dry_run":true}"#,
            ))
            .into();
        assert_eq!(preview.status, 200);
        let preview = response_body(preview);
        assert!(preview.contains(r#""outcome":"planned""#));
        assert!(
            preview.contains(r#""idempotency_key":"automation:nightly-off:schedule:5:action:0""#)
        );
        assert_eq!(automations.lock().unwrap().audit_records().len(), 0);

        let executed: WebResponse = app
            .handle(request_with_body(
                "POST",
                "/api/smart_home/automations/evaluate",
                "{}",
            ))
            .into();
        assert_eq!(executed.status, 200);
        assert!(response_body(executed).contains(r#""outcome":"executed""#));

        let repeated: WebResponse = app
            .handle(request_with_body(
                "POST",
                "/api/smart_home/automations/evaluate",
                "{}",
            ))
            .into();
        assert_eq!(repeated.status, 200);
        assert!(response_body(repeated).contains(r#""records":[]"#));

        let audit = response_body(
            app.handle(request("GET", "/api/smart_home/automation_audit"))
                .into(),
        );
        assert!(audit.contains(r#""record_count":1"#));
        assert!(audit.contains(r#""automation_id":"nightly-off""#));
        assert!(audit.contains(r#""outcome":"executed""#));
    }

    #[test]
    fn runtime_web_app_rolls_back_automation_when_persistence_fails() {
        let mut automation_runtime = SmartHomeAutomationRuntime::new();
        automation_runtime
            .upsert_definition(schedule_automation_definition())
            .unwrap();
        let automations = Arc::new(Mutex::new(automation_runtime));
        let runtime = fixture_runtime(true)
            .with_automation_runtime(Arc::clone(&automations))
            .with_automation_persistence(|_, _, _| Err("disk full".to_string()));
        let before = runtime.snapshot();
        let app = home_assistant_runtime_web_app(runtime.clone());

        let response: WebResponse = app
            .handle(request_with_body(
                "POST",
                "/api/smart_home/automations/evaluate",
                "{}",
            ))
            .into();

        assert_eq!(response.status, 503);
        assert!(response_body(response).contains("disk full"));
        assert_eq!(runtime.snapshot(), before);
        assert!(automations.lock().unwrap().audit_records().is_empty());
    }

    fn schedule_automation_definition() -> AutomationDefinition {
        AutomationDefinition {
            automation_id: "nightly-off".to_string(),
            enabled: true,
            trigger: AutomationTrigger::Schedule {
                every_ms: 1_000,
                offset_ms: 0,
            },
            conditions: Vec::new(),
            actions: vec![AutomationAction::Command {
                entity_id: EntityId::trusted("entity-light-1"),
                command_type: CommandType::TurnOff,
                arguments: Value::Null,
                timeout_ms: None,
            }],
        }
    }

    #[test]
    fn runtime_web_app_serves_dashboard_ready_service_catalog() {
        let app = home_assistant_runtime_web_app(fixture_runtime(true));

        let services = response_body(
            app.handle(request("GET", "/api/smart_home/services?domain=light"))
                .into(),
        );
        assert!(services.contains(r#""summary":{"total_services":4"#));
        assert!(services.contains(r#""service_id":"light.turn_on""#));
        assert!(services.contains(r#""home_assistant_path":"/api/services/light/turn_on""#));
        assert!(services.contains(r#""runtime_authorized":true"#));
        assert!(services.contains(r#""home_assistant_entity_ids":["light.entity_light_1"]"#));

        let detail = response_body(
            app.handle(request(
                "GET",
                "/api/smart_home/services/light/set_brightness",
            ))
            .into(),
        );
        assert!(detail.contains(r#""service_id":"light.set_brightness""#));
        assert!(detail.contains(r#""capability_ids":["light.brightness"]"#));

        let entity_filtered = response_body(
            app.handle(request(
                "GET",
                "/api/smart_home/services?entity_id=light.entity_light_1&capability_id=light.on_off",
            ))
            .into(),
        );
        assert!(entity_filtered.contains(r#""total_services":2"#));
        assert!(entity_filtered.contains(r#""service_id":"light.turn_on""#));
        assert!(entity_filtered.contains(r#""service_id":"light.turn_off""#));
        assert!(!entity_filtered.contains(r#""service_id":"light.set_brightness""#));

        let scene_filtered = response_body(
            app.handle(request(
                "GET",
                "/api/smart_home/services?scene_id=scene.scene_kitchen_bright",
            ))
            .into(),
        );
        assert!(scene_filtered.contains(r#""service_id":"scene.turn_on""#));
        assert!(
            scene_filtered.contains(r#""home_assistant_scene_ids":["scene.scene_kitchen_bright"]"#)
        );

        let missing_service: web_core::WebResponse = app
            .handle(request("GET", "/api/smart_home/services/light/missing"))
            .into();
        assert_eq!(missing_service.status, 404);
    }

    #[test]
    fn runtime_web_app_serves_dashboard_ready_entity_registry() {
        let app = home_assistant_runtime_web_app(fixture_runtime(true));
        let body = response_body(
            app.handle(request(
                "GET",
                "/api/smart_home/entities?domain=light&capability_id=light.brightness&commandable=true",
            ))
            .into(),
        );

        assert!(body.contains(r#""total_entities":1"#));
        assert!(body.contains(r#""commandable_entities":1"#));
        assert!(body.contains(r#""capability_count":3"#));
        assert!(body.contains(r#""entity_id":"entity-light-1""#));
        assert!(body.contains(r#""home_assistant_entity_id":"light.entity_light_1""#));
        assert!(body.contains(r#""device_id":"device-1""#));
        assert!(body.contains(r#""bridge_id":"bridge-1""#));
        assert!(body.contains(r#""room_id":"kitchen""#));
        assert!(body.contains(r#""manufacturer":"Signify""#));
        assert!(body.contains(r#""model":"Hue bulb""#));
        assert!(body.contains(r#""capability_id":"light.brightness""#));
        assert!(body.contains(r#""mode":"observe_and_command""#));
        assert!(body.contains(r#""value_kind":"percentage""#));
        assert!(body.contains(r#""min":0"#));
        assert!(body.contains(r#""max":100"#));
        assert!(body.contains(r#""links":{"self":"/api/smart_home/entities/light.entity_light_1""#));
        assert!(body.contains(r#""state":"/api/smart_home/states/light.entity_light_1""#));
        assert!(body.contains(
            r#""desired_state":"/api/smart_home/desired_states?entity_id=light.entity_light_1""#
        ));
        assert!(body.contains(
            r#""history":"/api/smart_home/state_history?entity_id=light.entity_light_1""#
        ));
        assert!(
            body.contains(r#""events":"/api/smart_home/events?entity_id=light.entity_light_1""#)
        );
        assert!(body.contains(
            r#""bridge_command_results":"/api/smart_home/command_results?bridge_id=bridge-1&limit=8&sort=status_then_newest""#
        ));
        assert!(body.contains(r#""device":"/api/smart_home/devices/device-1""#));
        assert!(body.contains(r#""room":"/api/smart_home/rooms/kitchen""#));

        let room_entities = response_body(
            app.handle(request(
                "GET",
                "/api/smart_home/entities?room_id=kitchen&kind=sensor",
            ))
            .into(),
        );
        assert!(room_entities.contains(r#""total_entities":1"#));
        assert!(room_entities.contains(r#""entity_id":"entity-sensor-1""#));
        assert!(room_entities.contains(r#""room_id":"kitchen""#));
        assert!(!room_entities.contains(r#""entity_id":"entity-light-1""#));

        let one_response: web_core::WebResponse = app
            .handle(request(
                "GET",
                "/api/smart_home/entities/light.entity_light_1",
            ))
            .into();
        let one_body = response_body(one_response.clone());
        assert_eq!(one_response.status, 200);
        assert!(one_body.contains(r#""name":"Kitchen Light""#));
        assert!(one_body.contains(r#""domain":"light""#));
        assert!(one_body.contains(
            r#""history":"/api/smart_home/state_history?entity_id=light.entity_light_1""#
        ));
    }

    #[test]
    fn runtime_web_app_serves_dashboard_ready_state_registry() {
        let app = home_assistant_runtime_web_app(fixture_runtime(true));
        let missing_states = response_body(
            app.handle(request(
                "GET",
                "/api/smart_home/states?domain=light&has_state=false&stale=true",
            ))
            .into(),
        );

        assert!(missing_states.contains(r#""total_entities":1"#));
        assert!(missing_states.contains(r#""entities_without_state":1"#));
        assert!(missing_states.contains(r#""entity_id":"entity-light-1""#));
        assert!(missing_states.contains(r#""home_assistant_entity_id":"light.entity_light_1""#));
        assert!(missing_states.contains(r#""has_state":false"#));
        assert!(missing_states.contains(r#""value":null"#));
        assert!(missing_states.contains(r#""stale":true"#));
        assert!(missing_states.contains(
            r#""capability_ids":["light.on_off","light.brightness","light.color_temperature"]"#
        ));
        assert!(missing_states.contains(r#""capabilities":[{"capability_id":"light.on_off""#));
        assert!(missing_states
            .contains(r#""capability_id":"light.brightness","mode":"observe_and_command""#));
        assert!(missing_states.contains(r#""min":0,"max":100,"step":1"#));

        let room_states = response_body(
            app.handle(request(
                "GET",
                "/api/smart_home/states?room_id=kitchen&stale=true",
            ))
            .into(),
        );
        assert!(room_states.contains(r#""total_entities":2"#));
        assert!(room_states.contains(r#""entities_without_state":2"#));
        assert!(room_states.contains(r#""home_assistant_entity_id":"light.entity_light_1""#));
        assert!(room_states.contains(r#""home_assistant_entity_id":"sensor.entity_sensor_1""#));

        let response: web_core::WebResponse = app
            .handle(request_with_body(
                "POST",
                "/api/services/light/turn_on",
                r#"{"entity_id":"light.entity_light_1","brightness_pct":75}"#,
            ))
            .into();
        assert_eq!(response.status, 200);

        let optimistic_states = response_body(
            app.handle(request(
                "GET",
                "/api/smart_home/states?domain=light&has_state=true&confidence=optimistic&source=optimistic_command",
            ))
            .into(),
        );
        assert!(optimistic_states.contains(r#""total_entities":1"#));
        assert!(optimistic_states.contains(r#""entities_with_state":1"#));
        assert!(optimistic_states.contains(r#""optimistic_entities":1"#));
        assert!(optimistic_states.contains(r#""home_assistant_entity_id":"light.entity_light_1""#));
        assert!(optimistic_states.contains(r#""source":"optimistic_command""#));
        assert!(optimistic_states.contains(r#""confidence":"optimistic""#));
        assert!(optimistic_states.contains(r#""light.brightness":75"#));

        let detail_response: web_core::WebResponse = app
            .handle(request(
                "GET",
                "/api/smart_home/states/light.entity_light_1",
            ))
            .into();
        let detail = response_body(detail_response.clone());
        assert_eq!(detail_response.status, 200);
        assert!(detail.contains(r#""entity_id":"entity-light-1""#));
        assert!(detail.contains(r#""has_state":true"#));

        let invalid_confidence: web_core::WebResponse = app
            .handle(request("GET", "/api/smart_home/states?confidence=maybe"))
            .into();
        assert_eq!(invalid_confidence.status, 400);
    }

    #[test]
    fn runtime_web_app_serves_dashboard_ready_capability_catalog() {
        let app = home_assistant_runtime_web_app(fixture_runtime(true));
        let light_capabilities = response_body(
            app.handle(request(
                "GET",
                "/api/smart_home/capabilities?domain=light&commandable=true",
            ))
            .into(),
        );

        assert!(light_capabilities.contains(r#""total_capabilities":3"#));
        assert!(light_capabilities.contains(r#""commandable_capabilities":3"#));
        assert!(light_capabilities.contains(r#""observable_capabilities":3"#));
        assert!(light_capabilities.contains(r#""ranged_capabilities":1"#));
        assert!(light_capabilities.contains(r#""domain_count":1"#));
        assert!(light_capabilities.contains(r#""capability_id":"light.brightness""#));
        assert!(light_capabilities.contains(r#""mode":"observe_and_command""#));
        assert!(light_capabilities.contains(r#""value_kind":"percentage""#));
        assert!(light_capabilities.contains(r#""min":0"#));
        assert!(light_capabilities.contains(r#""max":100"#));
        assert!(light_capabilities.contains(r#""domains":["light"]"#));
        assert!(light_capabilities.contains(r#""entity_kinds":["light"]"#));
        assert!(light_capabilities.contains(r#""entity_ids":["entity-light-1"]"#));
        assert!(
            light_capabilities.contains(r#""home_assistant_entity_ids":["light.entity_light_1"]"#)
        );
        assert!(light_capabilities.contains(r#""device_ids":["device-1"]"#));
        assert!(light_capabilities.contains(r#""room_ids":["kitchen"]"#));
        assert!(light_capabilities.contains(r#""service_ids":["light.set_brightness"]"#));

        let sensor_capability = response_body(
            app.handle(request(
                "GET",
                "/api/smart_home/capabilities?capability_id=sensor.occupancy&observable=true",
            ))
            .into(),
        );

        assert!(sensor_capability.contains(r#""total_capabilities":1"#));
        assert!(sensor_capability.contains(r#""capability_id":"sensor.occupancy""#));
        assert!(sensor_capability.contains(r#""domains":["sensor"]"#));
        assert!(sensor_capability.contains(r#""entity_ids":["entity-sensor-1"]"#));
        assert!(sensor_capability.contains(r#""commandable":false"#));
        assert!(sensor_capability.contains(r#""service_count":0"#));
    }

    #[test]
    fn runtime_web_app_serves_dashboard_ready_room_summaries() {
        let app = home_assistant_runtime_web_app(fixture_runtime(true));
        let body = response_body(
            app.handle(request(
                "GET",
                "/api/smart_home/rooms?room_id=kitchen&state_gaps_only=true&sort=scene_count",
            ))
            .into(),
        );

        assert!(body.contains(r#""total_rooms":1"#));
        assert!(body.contains(r#""state_gap_rooms":1"#));
        assert!(body.contains(r#""scene_rooms":1"#));
        assert!(body.contains(r#""topology_unique_rooms":1"#));
        assert!(body.contains(r#""devices_with_room":1"#));
        assert!(body.contains(r#""room_id":"kitchen""#));
        assert!(body.contains(r#""device_count":1"#));
        assert!(body.contains(r#""online_devices":1"#));
        assert!(body.contains(r#""entity_count":2"#));
        assert!(body.contains(r#""commandable_entities":1"#));
        assert!(body.contains(r#""entities_without_state":2"#));
        assert!(body.contains(r#""state_gap_count":2"#));
        assert!(body.contains(r#""scene_count":1"#));
        assert!(body.contains(r#""scene_action_count":1"#));
        assert!(body.contains(r#""has_state_gaps":true"#));
        assert!(body.contains(r#""has_scene_actions":true"#));

        let detail = response_body(
            app.handle(request("GET", "/api/smart_home/rooms/kitchen"))
                .into(),
        );
        assert!(detail.contains(r#""room_id":"kitchen""#));
        assert!(detail.contains(r#""device_count":1"#));
        assert!(detail.contains(r#""entity_count":2"#));
        assert!(detail.contains(r#""scene_action_count":1"#));
        assert!(detail.contains(r#""room":{"room_id":"kitchen""#));
        assert!(detail.contains(
            r#""links":{"self":"/api/smart_home/rooms/kitchen","rooms":"/api/smart_home/rooms?room_id=kitchen","devices":"/api/smart_home/devices?room_id=kitchen","entities":"/api/smart_home/entities?room_id=kitchen","states":"/api/smart_home/states?room_id=kitchen","state_gaps":"/api/smart_home/states?room_id=kitchen&stale=true","scenes":"/api/smart_home/scenes?room_id=kitchen","history":"/api/smart_home/state_history?room_id=kitchen","events":"/api/smart_home/events?room_id=kitchen","command_results":"/api/smart_home/command_results?room_id=kitchen"}"#
        ));
        assert!(detail.contains(r#""members":{"device_count":1,"entity_count":2,"scene_count":1"#));
        assert!(detail.contains(r#""devices":[{"device_id":"device-1""#));
        assert!(detail.contains(
            r#""entities":[{"entity_id":"entity-light-1","home_assistant_entity_id":"light.entity_light_1""#
        ));
        assert!(detail.contains(r#""entity_id":"entity-sensor-1""#));
        assert!(detail.contains(r#""scenes":[{"scene_id":"scene-kitchen-bright""#));
        assert!(detail.contains(r#""home_assistant_scene_id":"scene.scene_kitchen_bright""#));
        assert!(detail.contains(r#""home_assistant_entity_id":"light.entity_light_1""#));

        let missing: web_core::WebResponse = app
            .handle(request("GET", "/api/smart_home/rooms/missing"))
            .into();
        assert_eq!(missing.status, 404);
    }

    #[test]
    fn runtime_web_app_serves_dashboard_ready_scene_registry() {
        let app = home_assistant_runtime_web_app(fixture_runtime(true));
        let scenes = response_body(
            app.handle(request(
                "GET",
                "/api/smart_home/scenes?room_id=kitchen&scope=room&entity_id=light.entity_light_1",
            ))
            .into(),
        );

        assert!(scenes.contains(r#""total_scenes":1"#));
        assert!(scenes.contains(r#""action_count":1"#));
        assert!(scenes.contains(r#""room_count":1"#));
        assert!(scenes.contains(r#""scene_id":"scene-kitchen-bright""#));
        assert!(scenes.contains(r#""home_assistant_scene_id":"scene.scene_kitchen_bright""#));
        assert!(scenes.contains(r#""scope":"room""#));
        assert!(scenes.contains(
            r#""native_ref":{"family":"hue","kind":"scene","value":"scene-kitchen-bright"}"#
        ));
        assert!(scenes.contains(r#""room_ids":["kitchen"]"#));
        assert!(scenes.contains(r#""home_assistant_entity_id":"light.entity_light_1""#));
        assert!(scenes.contains(r#""light.on_off":true"#));
        assert!(scenes.contains(r#""light.brightness":80"#));
        assert!(scenes.contains(r#""key":"fixture.room_id","value":"kitchen""#));

        let detail_response: web_core::WebResponse = app
            .handle(request(
                "GET",
                "/api/smart_home/scenes/scene.scene_kitchen_bright",
            ))
            .into();
        let detail = response_body(detail_response.clone());
        assert_eq!(detail_response.status, 200);
        assert!(detail.contains(r#""scene_id":"scene-kitchen-bright""#));
        assert!(detail.contains(r#""action_count":1"#));

        let missing_scene: web_core::WebResponse = app
            .handle(request("GET", "/api/smart_home/scenes/scene.missing"))
            .into();
        assert_eq!(missing_scene.status, 404);

        let invalid_scope: web_core::WebResponse = app
            .handle(request("GET", "/api/smart_home/scenes?scope=planet"))
            .into();
        assert_eq!(invalid_scope.status, 400);
    }

    #[test]
    fn runtime_web_app_serves_dashboard_ready_device_and_bridge_registry() {
        let app = home_assistant_runtime_web_app(fixture_runtime(true));
        let devices = response_body(
            app.handle(request(
                "GET",
                "/api/smart_home/devices?bridge_id=bridge-1&room_id=kitchen&health=online",
            ))
            .into(),
        );

        assert!(devices.contains(r#""total_devices":1"#));
        assert!(devices.contains(r#""online_devices":1"#));
        assert!(devices.contains(r#""total_entities":2"#));
        assert!(devices.contains(r#""commandable_entities":1"#));
        assert!(devices.contains(r#""stale_entities":2"#));
        assert!(devices.contains(r#""capability_count":4"#));
        assert!(devices.contains(r#""device_id":"device-1""#));
        assert!(devices.contains(r#""bridge_id":"bridge-1""#));
        assert!(devices.contains(r#""name":"Kitchen""#));
        assert!(devices.contains(r#""manufacturer":"Signify""#));
        assert!(devices.contains(r#""model":"Hue bulb""#));
        assert!(devices.contains(r#""serial":"device-native-1""#));
        assert!(devices.contains(r#""firmware_version":"1.0.0""#));
        assert!(devices.contains(r#""room_id":"kitchen""#));
        assert!(devices.contains(r#""health":"online""#));
        assert!(devices.contains(r#""entity_ids":["entity-light-1","entity-sensor-1"]"#));
        assert!(devices.contains(
            r#""home_assistant_entity_ids":["light.entity_light_1","sensor.entity_sensor_1"]"#
        ));
        assert!(devices.contains(r#""capability_ids":["light.on_off","light.brightness","light.color_temperature","sensor.occupancy"]"#));

        let device_response: web_core::WebResponse = app
            .handle(request("GET", "/api/smart_home/devices/device-1"))
            .into();
        let device = response_body(device_response.clone());
        assert_eq!(device_response.status, 200);
        assert!(device.contains(r#""device_id":"device-1""#));
        assert!(device.contains(r#""entity_count":2"#));

        let bridges = response_body(
            app.handle(request(
                "GET",
                "/api/smart_home/bridges?integration_id=hue&transport=lan_http&health=online",
            ))
            .into(),
        );

        assert!(bridges.contains(r#""total_bridges":1"#));
        assert!(bridges.contains(r#""online_bridges":1"#));
        assert!(bridges.contains(r#""total_devices":1"#));
        assert!(bridges.contains(r#""room_count":1"#));
        assert!(bridges.contains(r#""bridge_id":"bridge-1""#));
        assert!(bridges.contains(r#""integration_id":"hue""#));
        assert!(bridges.contains(r#""transport":"lan_http""#));
        assert!(bridges.contains(r#""address":"https://192.0.2.10""#));
        assert!(bridges.contains(r#""hardware_model":"BSB002""#));
        assert!(bridges.contains(r#""firmware_version":"1.66.1960062030""#));
        assert!(bridges.contains(r#""last_seen_at_ms":1000"#));
        assert!(bridges.contains(r#""device_count":1"#));
        assert!(bridges.contains(r#""entity_count":2"#));
        assert!(bridges.contains(r#""room_ids":["kitchen"]"#));
        assert!(bridges.contains(r#""device_ids":["device-1"]"#));

        let bridge_response: web_core::WebResponse = app
            .handle(request("GET", "/api/smart_home/bridges/bridge-1"))
            .into();
        let bridge = response_body(bridge_response.clone());
        assert_eq!(bridge_response.status, 200);
        assert!(bridge.contains(r#""bridge_id":"bridge-1""#));
        assert!(bridge.contains(r#""commandable_entities":1"#));
    }

    #[test]
    fn runtime_web_app_serves_desired_state_targets() {
        let app = home_assistant_runtime_web_app(fixture_runtime_with_desired_state());
        let body = response_body(
            app.handle(request(
                "GET",
                "/api/smart_home/desired_states?entity_id=light.entity_light_1",
            ))
            .into(),
        );

        assert!(body.contains(r#""total_desired_states":1"#));
        assert!(body.contains(r#""entity_id":"entity-light-1""#));
        assert!(body.contains(r#""home_assistant_entity_id":"light.entity_light_1""#));
        assert!(body.contains(r#""requested_by":"agent:chief-of-staff""#));
        assert!(body.contains(r#""capability_id":"light.on_off""#));
    }

    #[test]
    fn runtime_web_app_sets_desired_state_through_runtime_authorization() {
        let app = home_assistant_runtime_web_app(fixture_runtime(true));
        let response: web_core::WebResponse = app
            .handle(request_with_body(
                "POST",
                "/api/smart_home/desired_states/light.entity_light_1",
                r#"{"desired_state":{"light.on_off":true,"light.brightness":80},"requested_by":"agent:dashboard","command_timeout_ms":3000}"#,
            ))
            .into();

        let body = response_body(response.clone());
        assert_eq!(response.status, 200);
        assert!(body.contains(r#""entity_id":"entity-light-1""#));
        assert!(body.contains(r#""home_assistant_entity_id":"light.entity_light_1""#));
        assert!(body.contains(r#""replaced":false"#));
        assert!(body.contains(r#""requested_by":"agent:dashboard""#));
        assert!(body.contains(r#""command_timeout_ms":3000"#));
        assert!(body.contains(r#""capability_id":"light.on_off""#));
        assert!(body.contains(r#""capability_id":"light.brightness""#));

        let desired_states = response_body(
            app.handle(request(
                "GET",
                "/api/smart_home/desired_states?entity_id=light.entity_light_1",
            ))
            .into(),
        );
        assert!(desired_states.contains(r#""total_desired_states":1"#));
        assert!(desired_states.contains(r#""total_desired_capabilities":2"#));
    }

    #[test]
    fn runtime_web_app_persists_accepted_mutations_for_restart() {
        let root = temp_root("durable-mutation");
        let store = Arc::new(SmartHomeRuntimeStore::new(LocalFolderStorageBackend::new(
            &root,
        )));
        let persistence_store = Arc::clone(&store);
        let runtime = fixture_runtime(true).with_mutation_persistence(move |runtime, now_ms| {
            persistence_store
                .save(runtime, &[], now_ms)
                .map(|_| ())
                .map_err(|error| error.to_string())
        });
        let app = home_assistant_runtime_web_app(runtime);

        let response: web_core::WebResponse = app
            .handle(request_with_body(
                "POST",
                "/api/smart_home/desired_states/light.entity_light_1",
                r#"{"desired_state":{"light.on_off":true}}"#,
            ))
            .into();
        assert_eq!(response.status, 200);

        drop(app);
        drop(store);
        let restarted_store = SmartHomeRuntimeStore::new(LocalFolderStorageBackend::new(&root));
        let restored = restarted_store
            .load()
            .expect("load persisted runtime")
            .expect("accepted mutation should create a durable snapshot");
        let desired_states = restored.runtime.query_desired_states(
            &DesiredStateQuery::new().for_entity(EntityId::trusted("entity-light-1")),
        );
        assert_eq!(restored.saved_at_ms, 5_000);
        assert_eq!(desired_states.len(), 1);
        assert_eq!(
            desired_states[0].desired,
            vec![StateDelta {
                capability_id: CapabilityId::trusted("light.on_off"),
                value: Value::Bool(true),
            }]
        );

        fs::remove_dir_all(root).expect("remove durable mutation test root");
    }

    #[test]
    fn runtime_web_app_persists_service_command_audit_for_restart() {
        let root = temp_root("durable-service");
        let store = Arc::new(SmartHomeRuntimeStore::new(LocalFolderStorageBackend::new(
            &root,
        )));
        let persistence_store = Arc::clone(&store);
        let runtime = fixture_runtime(true).with_mutation_persistence(move |runtime, now_ms| {
            persistence_store
                .save(runtime, &[], now_ms)
                .map(|_| ())
                .map_err(|error| error.to_string())
        });
        let app = home_assistant_runtime_web_app(runtime);

        let response: web_core::WebResponse = app
            .handle(request_with_body(
                "POST",
                "/api/services/light/turn_on",
                r#"{"entity_id":"light.entity_light_1"}"#,
            ))
            .into();
        assert_eq!(response.status, 200);

        drop(app);
        drop(store);
        let restarted_store = SmartHomeRuntimeStore::new(LocalFolderStorageBackend::new(&root));
        let restored = restarted_store
            .load()
            .expect("load persisted runtime")
            .expect("accepted service call should create a durable snapshot");
        let command_results = restored
            .runtime
            .query_command_results(&RuntimeCommandResultQuery::new());
        assert_eq!(command_results.len(), 1);
        assert_eq!(command_results[0].result.status, CommandStatus::Accepted);

        fs::remove_dir_all(root).expect("remove durable service test root");
    }

    #[test]
    fn runtime_web_app_rolls_back_when_mutation_persistence_fails() {
        let runtime = fixture_runtime(true)
            .with_mutation_persistence(|_, _| Err("storage offline".to_string()));
        let shared_runtime = Arc::clone(&runtime.runtime);
        let app = home_assistant_runtime_web_app(runtime);

        let response: web_core::WebResponse = app
            .handle(request_with_body(
                "POST",
                "/api/smart_home/desired_states/light.entity_light_1",
                r#"{"desired_state":{"light.on_off":true}}"#,
            ))
            .into();
        assert_eq!(response.status, 503);
        assert!(response_body(response).contains("storage offline"));

        let runtime = shared_runtime
            .lock()
            .expect("smart-home runtime mutex should not be poisoned");
        assert!(runtime
            .query_desired_states(
                &DesiredStateQuery::new().for_entity(EntityId::trusted("entity-light-1")),
            )
            .is_empty());
    }

    #[test]
    fn runtime_web_app_posts_home_assistant_state_as_desired_state() {
        let app = home_assistant_runtime_web_app(fixture_runtime(true));
        let response: web_core::WebResponse = app
            .handle(request_with_body(
                "POST",
                "/api/states/light.entity_light_1",
                r#"{"state":"on","attributes":{"brightness":191,"color_temp_kelvin":2700}}"#,
            ))
            .into();

        let body = response_body(response.clone());
        assert_eq!(response.status, 200);
        assert!(body.contains(r#""entity_id":"entity-light-1""#));
        assert!(body.contains(r#""requested_by":"agent:home-assistant-local-api""#));
        assert!(body.contains(r#""capability_id":"light.on_off""#));
        assert!(body.contains(r#""capability_id":"light.brightness""#));
        assert!(body.contains(r#""capability_id":"light.color_temperature""#));
        assert!(body.contains(r#""value":75"#));
        assert!(body.contains(r#""value":2700"#));
    }

    #[test]
    fn runtime_web_app_clears_desired_state_through_runtime_authorization() {
        let app = home_assistant_runtime_web_app(
            fixture_runtime_with_desired_state().grant_local_full_access("test", 1_000),
        );
        let response: web_core::WebResponse = app
            .handle(request(
                "DELETE",
                "/api/smart_home/desired_states/light.entity_light_1",
            ))
            .into();

        let body = response_body(response.clone());
        assert_eq!(response.status, 200);
        assert!(body.contains(r#""entity_id":"entity-light-1""#));
        assert!(body.contains(r#""removed":true"#));
        assert!(body.contains(r#""removed_desired_state":{"entity_id":"entity-light-1""#));
        assert!(body.contains(r#""total_desired_states":0"#));
    }

    #[test]
    fn runtime_web_app_rejects_desired_state_without_runtime_grants() {
        let app = home_assistant_runtime_web_app(fixture_runtime(false));
        let response: web_core::WebResponse = app
            .handle(request_with_body(
                "POST",
                "/api/smart_home/desired_states/entity-light-1",
                r#"{"desired_state":{"light.on_off":true}}"#,
            ))
            .into();

        assert_eq!(response.status, 403);
        assert!(response_body(response).contains("not authorized"));
    }

    #[test]
    fn runtime_web_app_serves_state_history_with_alias_filters() {
        let app = home_assistant_runtime_web_app(fixture_runtime_with_state_history());
        let body = response_body(
            app.handle(request(
                "GET",
                "/api/smart_home/state_history?entity_id=light.entity_light_1&event_type=updated",
            ))
            .into(),
        );

        assert!(body.contains(r#""total_events":1"#));
        assert!(body.contains(r#""entity_count":1"#));
        assert!(body.contains(r#""state_delta_count":1"#));
        assert!(body.contains(r#""home_assistant_entity_id":"light.entity_light_1""#));
        assert!(body.contains(r#""event_id":"event-light-1-on""#));
        assert!(body.contains(r#""event_type":"updated""#));
        assert!(body.contains(r#""capability_id":"light.on_off""#));
        assert!(body.contains(r#""value":true"#));

        let window_body = response_body(
            app.handle(request(
                "GET",
                "/api/smart_home/state_history?from_ms=2000&to_ms=2000&limit=5",
            ))
            .into(),
        );
        assert!(window_body.contains(r#""total_events":1"#));
        assert!(window_body.contains(r#""event_id":"event-light-1-on""#));

        let future_window_body = response_body(
            app.handle(request(
                "GET",
                "/api/smart_home/state_history?from_ms=2001&limit=5",
            ))
            .into(),
        );
        assert!(future_window_body.contains(r#""total_events":0"#));

        let past_window_body = response_body(
            app.handle(request(
                "GET",
                "/api/smart_home/state_history?to_ms=1999&limit=5",
            ))
            .into(),
        );
        assert!(past_window_body.contains(r#""total_events":0"#));

        let observed_window_body = response_body(
            app.handle(request(
                "GET",
                "/api/smart_home/state_history?observed_at_or_after_ms=2000&observed_at_or_before_ms=2000&limit=5",
            ))
            .into(),
        );
        assert!(observed_window_body.contains(r#""total_events":1"#));
        assert!(observed_window_body.contains(r#""event_id":"event-light-1-on""#));

        let past_observed_window_body = response_body(
            app.handle(request(
                "GET",
                "/api/smart_home/state_history?observed_at_or_before_ms=1999&limit=5",
            ))
            .into(),
        );
        assert!(past_observed_window_body.contains(r#""total_events":0"#));

        let received_window_body = response_body(
            app.handle(request(
                "GET",
                "/api/smart_home/state_history?received_at_or_after_ms=2010&received_at_or_before_ms=2010&limit=5",
            ))
            .into(),
        );
        assert!(received_window_body.contains(r#""total_events":1"#));
        assert!(received_window_body.contains(r#""event_id":"event-light-1-on""#));

        let future_received_window_body = response_body(
            app.handle(request(
                "GET",
                "/api/smart_home/state_history?received_at_or_after_ms=2011&limit=5",
            ))
            .into(),
        );
        assert!(future_received_window_body.contains(r#""total_events":0"#));

        let past_received_window_body = response_body(
            app.handle(request(
                "GET",
                "/api/smart_home/state_history?received_at_or_before_ms=2009&limit=5",
            ))
            .into(),
        );
        assert!(past_received_window_body.contains(r#""total_events":0"#));

        let bridge_body = response_body(
            app.handle(request(
                "GET",
                "/api/smart_home/state_history?bridge_id=bridge-1&limit=5",
            ))
            .into(),
        );
        assert!(bridge_body.contains(r#""total_events":1"#));
        assert!(bridge_body.contains(r#""event_id":"event-light-1-on""#));

        let missing_bridge_body = response_body(
            app.handle(request(
                "GET",
                "/api/smart_home/state_history?bridge_id=bridge-2&limit=5",
            ))
            .into(),
        );
        assert!(missing_bridge_body.contains(r#""total_events":0"#));

        let room_body = response_body(
            app.handle(request(
                "GET",
                "/api/smart_home/state_history?room_id=kitchen&limit=5",
            ))
            .into(),
        );
        assert!(room_body.contains(r#""total_events":1"#));
        assert!(room_body.contains(r#""event_id":"event-light-1-on""#));

        let entity_events = response_body(
            app.handle(request(
                "GET",
                "/api/smart_home/events?entity_id=light.entity_light_1&limit=5",
            ))
            .into(),
        );
        assert!(entity_events.contains(r#""total_events":1"#));
        assert!(entity_events.contains(r#""kind":"device_event""#));
        assert!(entity_events.contains(r#""event_id":"event-light-1-on""#));

        let missing_entity_events = response_body(
            app.handle(request(
                "GET",
                "/api/smart_home/events?entity_id=sensor.entity_sensor_1&limit=5",
            ))
            .into(),
        );
        assert!(missing_entity_events.contains(r#""total_events":0"#));

        let missing_room_body = response_body(
            app.handle(request(
                "GET",
                "/api/smart_home/state_history?room_id=garage&limit=5",
            ))
            .into(),
        );
        assert!(missing_room_body.contains(r#""total_events":0"#));

        let detail = response_body(
            app.handle(request(
                "GET",
                "/api/smart_home/state_history/event-light-1-on",
            ))
            .into(),
        );
        assert!(detail.contains(r#""home_assistant_entity_id":"light.entity_light_1""#));
        assert!(detail.contains(r#""event_id":"event-light-1-on""#));
        assert!(detail.contains(r#""event_type":"updated""#));

        let missing_event: web_core::WebResponse = app
            .handle(request(
                "GET",
                "/api/smart_home/state_history/missing-event",
            ))
            .into();
        assert_eq!(missing_event.status, 404);
    }

    #[test]
    fn runtime_web_app_serves_home_assistant_history_period_route() {
        let app = home_assistant_runtime_web_app(fixture_runtime_with_state_history());
        let body = response_body(
            app.handle(request(
                "GET",
                "/api/history/period?filter_entity_id=light.entity_light_1",
            ))
            .into(),
        );

        assert!(body.starts_with("[["));
        assert!(body.contains(r#""entity_id":"light.entity_light_1""#));
        assert!(body.contains(r#""state":true"#));
        assert!(body.contains(r#""canonical_entity_id":"entity-light-1""#));
        assert!(body.contains(r#""event_id":"event-light-1-on""#));
        assert!(body.contains(r#""capability_id":"light.on_off""#));

        let room_body = response_body(
            app.handle(request("GET", "/api/history/period?room_id=kitchen"))
                .into(),
        );
        assert_eq!(room_body, body);

        let missing_room_body = response_body(
            app.handle(request("GET", "/api/history/period?room_id=garage"))
                .into(),
        );
        assert_eq!(missing_room_body, "[]");

        let period_body = response_body(
            app.handle(request(
                "GET",
                "/api/history/period/2000?filter_entity_id=light.entity_light_1",
            ))
            .into(),
        );
        assert_eq!(period_body, body);

        let future_period_body = response_body(
            app.handle(request(
                "GET",
                "/api/history/period/2001?filter_entity_id=light.entity_light_1",
            ))
            .into(),
        );
        assert_eq!(future_period_body, "[]");

        let ended_period_body = response_body(
            app.handle(request(
                "GET",
                "/api/history/period/2000?filter_entity_id=light.entity_light_1&end_time=1999",
            ))
            .into(),
        );
        assert_eq!(ended_period_body, "[]");
    }

    #[test]
    fn runtime_web_app_rejects_service_calls_without_runtime_grants() {
        let app = home_assistant_runtime_web_app(fixture_runtime(false));
        let response: web_core::WebResponse = app
            .handle(request_with_body(
                "POST",
                "/api/services/light/turn_on",
                r#"{"entity_id":"entity-light-1"}"#,
            ))
            .into();

        assert_eq!(response.status, 403);
        assert!(response_body(response).contains("not authorized"));
    }

    #[test]
    fn runtime_web_app_expands_scene_turn_on_into_commands() {
        let app = home_assistant_runtime_web_app(fixture_runtime(true));
        let response: web_core::WebResponse = app
            .handle(request_with_body(
                "POST",
                "/api/services/scene/turn_on",
                r#"{"entity_id":"scene.scene_kitchen_bright"}"#,
            ))
            .into();

        let body = response_body(response.clone());
        assert_eq!(response.status, 200);
        assert!(body.contains(r#""domain":"scene""#));
        assert!(body.contains(r#""result_count":2"#));
        assert!(body.contains(r#""status":"accepted""#));
    }

    #[test]
    fn runtime_web_app_serves_post_services_over_repo_http_server() {
        let (port, stop) = start_server(home_assistant_runtime_web_app(fixture_runtime(true)));
        let (status, body) = http_post(
            port,
            "/api/services/light/set_brightness",
            r#"{"entity_id":"entity-light-1","brightness":128}"#,
        );
        stop.stop();

        assert_eq!(status, 200);
        assert!(body.contains(r#""service":"set_brightness""#));
        assert!(body.contains(r#""result_count":1"#));
        assert!(body.contains(r#""status":"accepted""#));
    }

    #[test]
    fn runtime_web_app_serves_dashboard_shell_over_repo_http_server() {
        let (port, stop) = start_server(home_assistant_runtime_web_app(fixture_runtime(true)));
        let (status, body) = http_get(port, "/");
        stop.stop();

        assert_eq!(status, 200);
        assert!(body.contains("<title>Codex Home</title>"));
        assert!(body.contains("json(\"/api/smart_home/bootstrap\")"));
        assert!(body.contains("data-dashboard-filter=\"search\""));
        assert!(body.contains("data-dashboard-filter=\"room\""));
        assert!(body.contains("data-dashboard-filter=\"capability-id\""));
        assert!(body.contains("data-dashboard-filter=\"capability-commandable\""));
        assert!(body.contains("data-dashboard-filter=\"capability-observable\""));
        assert!(body.contains("data-dashboard-filter=\"desired-entity\""));
        assert!(body.contains("data-dashboard-filter=\"desired-requested-by\""));
        assert!(body.contains("data-dashboard-filter=\"device-bridge\""));
        assert!(body.contains("data-dashboard-filter=\"device-manufacturer\""));
        assert!(body.contains("data-dashboard-filter=\"device-health\""));
        assert!(body.contains("data-dashboard-filter=\"bridge-integration\""));
        assert!(body.contains("data-dashboard-filter=\"bridge-transport\""));
        assert!(body.contains("data-dashboard-filter=\"bridge-health\""));
        assert!(body.contains("data-dashboard-filter=\"scene-scope\""));
        assert!(body.contains("data-dashboard-filter=\"scene-entity\""));
        assert!(body.contains("data-dashboard-filter=\"service-name\""));
        assert!(body.contains("data-dashboard-filter=\"service-capability\""));
        assert!(body.contains("data-dashboard-filter=\"service-entity\""));
        assert!(body.contains("data-dashboard-filter=\"service-scene\""));
        assert!(body.contains("data-dashboard-filter=\"api-surface\""));
        assert!(body.contains("data-dashboard-filter=\"api-method\""));
        assert!(body.contains("data-dashboard-filter=\"api-category\""));
        assert!(body.contains("data-dashboard-filter=\"api-mutating\""));
        assert!(body.contains("data-dashboard-filter=\"api-authorized\""));
        assert!(body.contains("data-dashboard-filter=\"grant-status\""));
        assert!(body.contains("data-dashboard-filter=\"grant-scope\""));
        assert!(body.contains("data-dashboard-filter=\"grant-principal\""));
        assert!(body.contains("data-dashboard-filter=\"authorization-principal\""));
        assert!(body.contains("data-dashboard-filter=\"activity-entity\""));
        assert!(body.contains("data-dashboard-filter=\"history-type\""));
        assert!(body.contains("data-dashboard-filter=\"history-bridge\""));
        assert!(body.contains("data-dashboard-filter=\"history-from-ms\""));
        assert!(body.contains("data-dashboard-filter=\"history-to-ms\""));
        assert!(body.contains("data-dashboard-filter=\"history-received-from-ms\""));
        assert!(body.contains("data-dashboard-filter=\"history-received-to-ms\""));
        assert!(body.contains("data-dashboard-filter=\"event-from-sequence\""));
        assert!(body.contains("data-dashboard-filter=\"event-to-sequence\""));
        assert!(body.contains("data-dashboard-filter=\"command-id\""));
        assert!(body.contains("data-dashboard-filter=\"command-bridge\""));
        assert!(body.contains("data-dashboard-filter=\"command-correlation\""));
        assert!(body.contains("data-dashboard-filter=\"command-from-sequence\""));
        assert!(body.contains("data-dashboard-filter=\"command-to-sequence\""));
        assert!(body.contains("const FILTER_QUERY_PARAMS = ["));
        assert!(body.contains("[\"api_surface\", els.filterApiSurface]"));
        assert!(body.contains("[\"api_method\", els.filterApiMethod]"));
        assert!(body.contains("[\"api_category\", els.filterApiCategory]"));
        assert!(body.contains("[\"api_mutating\", els.filterApiMutating]"));
        assert!(body.contains("[\"api_authorized\", els.filterApiAuthorized]"));
        assert!(body.contains("[\"capability_id\", els.filterCapabilityId]"));
        assert!(body.contains("[\"capability_commandable\", els.filterCapabilityCommandable]"));
        assert!(body.contains("[\"capability_observable\", els.filterCapabilityObservable]"));
        assert!(body.contains("[\"desired_entity\", els.filterDesiredEntity]"));
        assert!(body.contains("[\"desired_requested_by\", els.filterDesiredRequestedBy]"));
        assert!(body.contains("[\"device_bridge\", els.filterDeviceBridge]"));
        assert!(body.contains("[\"device_manufacturer\", els.filterDeviceManufacturer]"));
        assert!(body.contains("[\"device_health\", els.filterDeviceHealth]"));
        assert!(body.contains("[\"bridge_integration\", els.filterBridgeIntegration]"));
        assert!(body.contains("[\"bridge_transport\", els.filterBridgeTransport]"));
        assert!(body.contains("[\"bridge_health\", els.filterBridgeHealth]"));
        assert!(body.contains("[\"scene_scope\", els.filterSceneScope]"));
        assert!(body.contains("[\"scene_entity\", els.filterSceneEntity]"));
        assert!(body.contains("[\"service_name\", els.filterServiceName]"));
        assert!(body.contains("[\"service_capability\", els.filterServiceCapability]"));
        assert!(body.contains("[\"service_entity\", els.filterServiceEntity]"));
        assert!(body.contains("[\"service_scene\", els.filterServiceScene]"));
        assert!(body.contains("[\"event_from_sequence\", els.filterEventFromSequence]"));
        assert!(body.contains("[\"event_to_sequence\", els.filterEventToSequence]"));
        assert!(body.contains("[\"activity_entity\", els.filterActivityEntity]"));
        assert!(body.contains("[\"history_type\", els.filterHistoryType]"));
        assert!(body.contains("[\"history_bridge\", els.filterHistoryBridge]"));
        assert!(body.contains("[\"history_from_ms\", els.filterHistoryFromMs]"));
        assert!(body.contains("[\"history_to_ms\", els.filterHistoryToMs]"));
        assert!(body.contains("[\"history_received_from_ms\", els.filterHistoryReceivedFromMs]"));
        assert!(body.contains("[\"history_received_to_ms\", els.filterHistoryReceivedToMs]"));
        assert!(body.contains("[\"command_id\", els.filterCommandId]"));
        assert!(body.contains("[\"command_bridge\", els.filterCommandBridge]"));
        assert!(body.contains("[\"command_correlation\", els.filterCommandCorrelation]"));
        assert!(body.contains("[\"command_from_sequence\", els.filterCommandFromSequence]"));
        assert!(body.contains("[\"command_to_sequence\", els.filterCommandToSequence]"));
        assert!(body.contains("[\"authorization_principal\", els.filterAuthorizationPrincipal]"));
        assert!(body.contains("[\"grant_status\", els.filterGrantStatus]"));
        assert!(body.contains("window.addEventListener(\"popstate\""));
        assert!(body.contains("window.history.replaceState(null, \"\", nextUrl)"));
        assert!(body.contains("queryUrl(\"/api/smart_home/scenes\", {"));
        assert!(body.contains("room_id: roomId"));
        assert!(body.contains("scope: filters.sceneScope"));
        assert!(body.contains("entity_id: filters.sceneEntity"));
        assert!(body.contains("queryUrl(\"/api/smart_home/desired_states\", {"));
        assert!(body.contains("entity_id: filters.desiredEntity"));
        assert!(body.contains("capability_id: capabilityId"));
        assert!(body.contains("requested_by: filters.desiredRequestedBy"));
        assert!(body.contains("queryUrl(\"/api/smart_home/api\", {"));
        assert!(body.contains("surface: filters.apiSurface"));
        assert!(body.contains("method: filters.apiMethod"));
        assert!(body.contains("category: filters.apiCategory"));
        assert!(body.contains("mutating: filters.apiMutating"));
        assert!(body.contains("authorized: filters.apiAuthorized"));
        assert!(body.contains("const activityEntity = filters.activityEntity || undefined"));
        assert!(body.contains("const historyType = filters.historyType || undefined"));
        assert!(body.contains("queryUrl(\"/api/smart_home/state_history\", {"));
        assert!(body.contains("queryUrl(\"/api/smart_home/events\", {"));
        assert!(body.contains("from_sequence: filters.eventFromSequence"));
        assert!(body.contains("to_sequence: filters.eventToSequence"));
        assert!(body.contains("entity_id: activityEntity"));
        assert!(body.contains("event_type: historyType"));
        assert!(body.contains("bridge_id: filters.historyBridge"));
        assert!(body.contains("from_ms: filters.historyFromMs"));
        assert!(body.contains("to_ms: filters.historyToMs"));
        assert!(body.contains("received_at_or_after_ms: filters.historyReceivedFromMs"));
        assert!(body.contains("received_at_or_before_ms: filters.historyReceivedToMs"));
        assert!(body.matches("entity_id: activityEntity").count() >= 2);
        assert!(body.contains("queryUrl(\"/api/smart_home/services\", {"));
        assert!(body.contains("service: filters.serviceName"));
        assert!(body.contains("capability_id: filters.serviceCapability"));
        assert!(body.contains("entity_id: filters.serviceEntity"));
        assert!(body.contains("scene_id: filters.serviceScene"));
        assert!(body.contains("queryUrl(\"/api/smart_home/capabilities\", {"));
        assert!(body.contains("capability_id: filters.capabilityId"));
        assert!(body.contains("commandable: filters.capabilityCommandable"));
        assert!(body.contains("observable: filters.capabilityObservable"));
        assert!(body.contains("json(\"/api/smart_home/rooms?sort=scene_count\")"));
        assert!(body.contains("queryUrl(\"/api/smart_home/devices\", {"));
        assert!(body.contains("bridge_id: filters.deviceBridge"));
        assert!(body.contains("manufacturer: filters.deviceManufacturer"));
        assert!(body.contains("health: filters.deviceHealth"));
        assert!(body.contains("queryUrl(\"/api/smart_home/bridges\", {"));
        assert!(body.contains("integration_id: filters.bridgeIntegration"));
        assert!(body.contains("transport: filters.bridgeTransport"));
        assert!(body.contains("health: filters.bridgeHealth"));
        assert!(body.contains("queryUrl(\"/api/smart_home/command_results\", {"));
        assert!(body.contains("command_id: filters.commandId"));
        assert!(body.contains("bridge_id: filters.commandBridge"));
        assert!(body.contains("correlation_id: filters.commandCorrelation"));
        assert!(body.contains("from_sequence: filters.commandFromSequence"));
        assert!(body.contains("to_sequence: filters.commandToSequence"));
        assert!(body.contains("queryUrl(\"/api/smart_home/authorization_decisions\", {"));
        assert!(body.contains("principal_id: filters.authorizationPrincipal"));
        assert!(body.contains("queryUrl(\"/api/smart_home/capability_grants\", {"));
        assert!(body.contains("id=\"detail-body\""));
        assert!(body.contains("renderDetail(label, url, response.status, response.ok, body)"));
        assert!(body.contains("id=\"capability-grants\""));
        assert!(body.contains("id=\"capabilities\""));
        assert!(body.contains("renderCapabilities(capabilities)"));
        assert!(body.contains("capabilityDetailUrl(capability)"));
        assert!(body.contains("renderCapabilityGrants(capabilityGrants, filters)"));
        assert!(body.contains("principalCapabilityGrantsUrl(record.principal_id)"));
        assert!(body.contains("capabilityGrantDetailUrl(grant)"));
        assert!(body.contains("stateDetailUrl(entity)"));
        assert!(body.contains("entityHistoryUrl(entity)"));
        assert!(body.contains("entityEventsUrl(entity)"));
        assert!(body.contains("entityBridgeCommandsUrl(entity)"));
        assert!(body.contains("commandAuthorizationUrl(entity, \"turn_on\")"));
        assert!(body.contains("commandAuthorizationUrl(entity, \"turn_off\")"));
        assert!(body.contains("commandAuthorizationUrl(entity, \"set_brightness\")"));
        assert!(body.contains("/api/smart_home/command_authorization?entity_id="));
        assert!(body.contains("desiredStateAuthorizationUrl(entity, \"set\")"));
        assert!(body.contains("desiredStateAuthorizationUrl(entity, \"clear\")"));
        assert!(body.contains("desiredStateAuthorizationUrl(target, \"clear\")"));
        assert!(body.contains("/api/smart_home/desired_state_authorization?entity_id="));
        assert!(body.contains("sceneAuthorizationUrl(scene)"));
        assert!(body.contains("/api/smart_home/scene_authorization?scene_id="));
        assert!(body.contains("serviceAuthorizationUrl(service)"));
        assert!(body.contains("/api/smart_home/service_authorization/"));
        assert!(body.contains("Auth on"));
        assert!(body.contains("Auth off"));
        assert!(body.contains("Auth brightness"));
        assert!(body.contains("Auth target"));
        assert!(body.contains("Auth clear"));
        assert!(body.contains("Auth scene"));
        assert!(body.contains("Auth service"));
        assert!(body.contains("commandable capabilities"));
        assert!(body.contains("serviceDetailUrl(service)"));
        assert!(body.contains("roomDetailUrl(room)"));
        assert!(body.contains("/api/smart_home/devices/${encodeURIComponent(device.device_id)}"));
        assert!(body.contains("/api/services/light/"));
        assert!(body.contains("data-brightness-input"));
        assert!(body.contains("brightness_pct"));
        assert!(body.contains("data-desired-action"));
        assert!(body.contains(r#"requested_by: "agent:dashboard""#));
        assert!(body.contains("command_timeout_ms: 3000"));
        assert!(body.contains("/api/services/scene/turn_on"));
        assert!(body.contains("/api/smart_home/desired_states/"));
    }

    #[test]
    fn runtime_web_app_serves_runtime_snapshot_over_repo_http_server() {
        let (port, stop) = start_server(home_assistant_runtime_web_app(fixture_runtime(true)));
        let (status, body) = http_get(port, "/api/smart_home/runtime");
        stop.stop();

        assert_eq!(status, 200);
        assert!(body.contains(r#""registry":{"bridges":1"#));
        assert!(body.contains(r#""desired_state":{"target_count":0"#));
    }

    #[test]
    fn runtime_web_app_serves_smoke_script_over_repo_http_server() {
        let (port, stop) = start_server(home_assistant_runtime_web_app(fixture_runtime(true)));
        let (status, body) = http_get(port, "/api/smart_home/smoke_script");
        stop.stop();

        assert_eq!(status, 200);
        assert!(body.contains(&format!(
            "BASE_URL=${{SMART_HOME_BASE_URL:-'http://127.0.0.1:{port}'}}"
        )));
        assert!(
            body.contains(r#"run_check 'Startup bundle' 'GET' '/api/smart_home/bootstrap' '200'"#)
        );
        assert!(body.contains(
            r#"run_check 'Controller handoff' 'GET' '/api/smart_home/controller_handoff' '200'"#
        ));
        assert!(body.contains("All smart-home smoke checks passed (15 checks)"));
    }

    #[test]
    fn home_assistant_web_app_reports_missing_state_as_json_404() {
        let app = home_assistant_web_app(fixture_state());
        let response: web_core::WebResponse = app
            .handle(request("GET", "/api/states/missing.entity"))
            .into();

        assert_eq!(response.status, 404);
        assert_eq!(response_body(response), r#"{"error":"entity not found"}"#);
    }

    #[test]
    fn value_json_escapes_strings_and_projects_nested_values() {
        let value = Value::Object(vec![
            ("name".to_string(), Value::Text("Kitchen \"A\"".to_string())),
            (
                "levels".to_string(),
                Value::Array(vec![Value::Percentage(50)]),
            ),
        ]);

        assert_eq!(
            value_json(&value),
            r#"{"name":"Kitchen \"A\"","levels":[50]}"#
        );
    }

    #[test]
    fn lan_udp_bridge_transport_round_trips_through_api_labels() {
        assert_eq!(bridge_transport_label(BridgeTransport::LanUdp), "lan_udp");
        assert_eq!(
            bridge_transport_from_label("lan_udp").unwrap(),
            BridgeTransport::LanUdp
        );
        assert_eq!(
            bridge_transport_from_label("udp").unwrap(),
            BridgeTransport::LanUdp
        );
    }

    #[test]
    fn lan_tcp_bridge_transport_round_trips_through_api_labels() {
        assert_eq!(bridge_transport_label(BridgeTransport::LanTcp), "lan_tcp");
        assert_eq!(
            bridge_transport_from_label("lan_tcp").unwrap(),
            BridgeTransport::LanTcp
        );
        assert_eq!(
            bridge_transport_from_label("tcp").unwrap(),
            BridgeTransport::LanTcp
        );
    }

    #[test]
    fn media_command_labels_round_trip_through_local_api_labels() {
        let commands = [
            ("media_set_playback_state", MediaCommandType::SetPlaybackState),
            ("media_play_next", MediaCommandType::PlayNext),
            ("media_play_previous", MediaCommandType::PlayPrevious),
            ("media_set_volume", MediaCommandType::SetVolume),
            ("media_set_mute", MediaCommandType::SetMute),
            ("media_set_group", MediaCommandType::SetGroup),
            ("media_clear_queue", MediaCommandType::ClearQueue),
            ("media_play_queue_item", MediaCommandType::PlayQueueItem),
            ("media_remove_queue_item", MediaCommandType::RemoveQueueItem),
            ("media_move_queue_item", MediaCommandType::MoveQueueItem),
        ];
        for (label, media_command) in commands {
            let command = CommandType::Media(media_command);
            assert_eq!(command_type_from_label(label).unwrap(), command);
            assert_eq!(command_type_label(command), label);
        }
    }

    #[test]
    fn device_control_command_labels_round_trip_through_local_api_labels() {
        let commands = [
            (
                "device_set_indicator_mode",
                DeviceControlCommandType::SetIndicatorMode,
            ),
            (
                "device_set_indicator_brightness",
                DeviceControlCommandType::SetIndicatorBrightness,
            ),
            (
                "device_set_display_brightness",
                DeviceControlCommandType::SetDisplayBrightness,
            ),
            (
                "sensor_calibrate",
                DeviceControlCommandType::CalibrateSensor,
            ),
            (
                "device_set_temperature_unit",
                DeviceControlCommandType::SetTemperatureUnit,
            ),
            (
                "device_set_particulate_display_standard",
                DeviceControlCommandType::SetParticulateDisplayStandard,
            ),
            (
                "device_set_automatic_co2_baseline_days",
                DeviceControlCommandType::SetAutomaticCo2BaselineDays,
            ),
            (
                "device_set_gas_learning_offsets",
                DeviceControlCommandType::SetGasLearningOffsets,
            ),
            (
                "device_set_compensated_display",
                DeviceControlCommandType::SetCompensatedDisplay,
            ),
            (
                "device_test_indicator",
                DeviceControlCommandType::TestIndicator,
            ),
            (
                "device_set_correction_profile",
                DeviceControlCommandType::SetCorrectionProfile,
            ),
            (
                "camera_set_recording",
                DeviceControlCommandType::SetCameraRecording,
            ),
        ];
        for (label, device_command) in commands {
            let command = CommandType::DeviceControl(device_command);
            assert_eq!(command_type_from_label(label).unwrap(), command);
            assert_eq!(command_type_label(command), label);
        }
    }
}
