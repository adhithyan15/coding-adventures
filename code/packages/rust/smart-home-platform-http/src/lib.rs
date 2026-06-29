//! Home Assistant-compatible local HTTP API routes for the smart-home platform.
//!
//! The crate builds `web-core::WebApp` routes over runtime-owned smart-home
//! registry snapshots. It deliberately uses the repo's own HTTP server stack;
//! service calls are wired through runtime command authorization instead of a
//! parallel mutation path.

#![forbid(unsafe_code)]

use serde_json::Value as JsonValue;
use smart_home_core::{
    AgentId, AuthorizationDecision, AuthorizationOutcome, AuthorizationSubject, Bridge, BridgeId,
    BridgeTransport, Capability, CapabilityGrant, CapabilityGrantId,
    CapabilityGrantInventorySummary, CapabilityGrantScope, CapabilityGrantStatus, CapabilityId,
    CapabilityMode, CommandId, CommandResult, CommandStatus, CommandType, CorrelationId, Device,
    DeviceEvent, DeviceEventType, Entity, EntityId, EntityKind, EventId, Health, PrivilegeTier,
    Scene, SceneScope, StateConfidence, StateDelta, StateSource, Value, ValueKind,
};
use smart_home_runtime::{
    DesiredEntityState, DesiredStateQuery, RuntimeAuthorizationDecisionQuery,
    RuntimeAuthorizationDecisionSort, RuntimeCapabilityGrantQuery, RuntimeCapabilityGrantScopeKind,
    RuntimeCapabilityGrantSort, RuntimeClearDesiredStateToolOutput,
    RuntimeClearDesiredStateToolRequest, RuntimeCommandResultQuery, RuntimeCommandResultRecord,
    RuntimeCommandResultSort, RuntimeCommandToolRequest, RuntimeError, RuntimeEvent,
    RuntimeEventCheckpoint, RuntimeEventFilter, RuntimeEventLogEntry, RuntimeEventQuery,
    RuntimeEventSort, RuntimeReadSnapshot, RuntimeRoomQuery, RuntimeRoomSort, RuntimeRoomSummary,
    RuntimeSetDesiredStateToolOutput, RuntimeSetDesiredStateToolRequest, SmartHomeRuntime,
};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use web_core::{WebApp, WebRequest, WebResponse};

pub const VERSION: &str = "0.1.0";

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
    }

    .panel {
      background: var(--panel);
      border: 1px solid var(--line);
      border-radius: 8px;
      padding: 14px;
    }

    .toolbar, .row, .metric-grid {
      display: flex;
      gap: 8px;
      align-items: center;
      flex-wrap: wrap;
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
      <div class="panel">
        <h2>Home</h2>
        <div id="summary" class="metric-grid"></div>
      </div>
      <div class="panel">
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
          <label>Events
            <select id="filter-event-kind" data-dashboard-filter="event-kind">
              <option value="">All events</option>
              <option value="commands">Commands</option>
              <option value="supervision">Supervision</option>
            </select>
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
          <label>Authorization
            <select id="filter-authorization-outcome" data-dashboard-filter="authorization-outcome">
              <option value="">All decisions</option>
              <option value="allowed">Allowed</option>
              <option value="denied">Denied</option>
            </select>
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
      <div class="panel">
        <div class="row">
          <h2>Rooms</h2>
          <span class="muted">Topology and coverage</span>
        </div>
        <div id="rooms" class="cards"></div>
      </div>
      <div class="panel">
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
      <div class="panel">
        <div class="row">
          <h2>Entities</h2>
          <span id="state-count" class="muted"></span>
        </div>
        <div id="entities" class="cards"></div>
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
      <div class="panel">
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
      <div class="panel">
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
      authorizationDecisions: document.querySelector("#authorization-decisions"),
      bridges: document.querySelector("#bridges"),
      capabilityGrants: document.querySelector("#capability-grants"),
      checks: document.querySelector("#checks"),
      commandResults: document.querySelector("#command-results"),
      detailBody: document.querySelector("#detail-body"),
      detailEndpoint: document.querySelector("#detail-endpoint"),
      detailStatus: document.querySelector("#detail-status"),
      detailTitle: document.querySelector("#detail-title"),
      desired: document.querySelector("#desired"),
      devices: document.querySelector("#devices"),
      entities: document.querySelector("#entities"),
      events: document.querySelector("#events"),
      filterAuthorizationOutcome: document.querySelector("#filter-authorization-outcome"),
      filterCommandStatus: document.querySelector("#filter-command-status"),
      filterControl: document.querySelector("#filter-control"),
      filterDomain: document.querySelector("#filter-domain"),
      filterEventKind: document.querySelector("#filter-event-kind"),
      filterGrantPrincipal: document.querySelector("#filter-grant-principal"),
      filterGrantScope: document.querySelector("#filter-grant-scope"),
      filterGrantStatus: document.querySelector("#filter-grant-status"),
      filterRoom: document.querySelector("#filter-room"),
      filterSearch: document.querySelector("#filter-search"),
      filterState: document.querySelector("#filter-state"),
      gaps: document.querySelector("#gaps"),
      history: document.querySelector("#history"),
      location: document.querySelector("#location"),
      log: document.querySelector("#log"),
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
      ["event_kind", els.filterEventKind],
      ["command_status", els.filterCommandStatus],
      ["authorization_outcome", els.filterAuthorizationOutcome],
      ["grant_status", els.filterGrantStatus],
      ["grant_scope", els.filterGrantScope],
      ["grant_principal", els.filterGrantPrincipal]
    ];

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
      eventKind: els.filterEventKind.value,
      commandStatus: els.filterCommandStatus.value,
      authorizationOutcome: els.filterAuthorizationOutcome.value,
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
    const sceneDetailUrl = (scene) =>
      `/api/smart_home/scenes/${encodeURIComponent(scene.home_assistant_scene_id || scene.scene_id)}`;
    const serviceDetailUrl = (service) => {
      const [domain, serviceName] = String(service.service_id || "").split(".");
      return domain && serviceName
        ? `/api/smart_home/services/${encodeURIComponent(domain)}/${encodeURIComponent(serviceName)}`
        : "/api/smart_home/services";
    };
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
            <div class="actions row">
              ${inspectButton(stateDetailUrl(entity), "state detail")}
              ${inspectButton(entityDetailUrl(entity), "entity detail")}
              ${inspectButton(entityHistoryUrl(entity), "entity history", "History")}
              ${inspectButton(entityEventsUrl(entity), "entity events", "Events")}
              ${inspectButton(entityDesiredStateUrl(entity), "desired state", "Desired")}
              ${inspectButton(entityBridgeCommandsUrl(entity), "bridge command results", "Commands")}
              ${canToggle ? `<button type="button" data-service="turn_on" data-entity="${entity.home_assistant_entity_id}">Turn on</button><button type="button" data-service="turn_off" data-entity="${entity.home_assistant_entity_id}">Turn off</button>` : ""}
              ${canToggle ? `<button type="button" data-desired-action="on" data-entity="${entity.home_assistant_entity_id}">Target on</button><button type="button" data-desired-action="off" data-entity="${entity.home_assistant_entity_id}">Target off</button>` : ""}
            </div>
            ${canSetBrightness ? `
              <label class="range-control">
                <span class="muted">Brightness <strong data-brightness-value="${entity.home_assistant_entity_id}">${brightnessCurrent}%</strong></span>
                <input type="range" min="${brightnessMin}" max="${brightnessMax}" step="${brightnessStep}" value="${brightnessCurrent}" data-brightness-input="${entity.home_assistant_entity_id}">
                <button type="button" data-service="set_brightness" data-entity="${entity.home_assistant_entity_id}" data-brightness-for="${entity.home_assistant_entity_id}">Set brightness</button>
                <button type="button" data-desired-action="brightness" data-entity="${entity.home_assistant_entity_id}" data-brightness-for="${entity.home_assistant_entity_id}">Target brightness</button>
              </label>
            ` : ""}
          </article>
        `;
      }).join("") || `<p class="muted">No matching entities</p>`;
    };

    const renderDesiredStates = (desiredStates, filters) => {
      const targets = filterRows(desiredStates.desired_states || [], filters);
      els.desired.innerHTML = targets.map((target) => `
        <tr>
          <td>${target.home_assistant_entity_id}<br><span class="muted">${target.entity_id}</span></td>
          <td>${deltasText(target.desired)}</td>
          <td>${target.requested_by}<br><span class="muted">${target.command_timeout_ms} ms</span></td>
          <td><button type="button" data-clear-desired="${target.home_assistant_entity_id}">Clear</button></td>
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
        return `
          <tr>
            <td>${entry.sequence}<br><span class="muted">next ${entry.next_sequence}</span></td>
            <td>${event.kind || "event"}</td>
            <td>${eventSubject(event)}</td>
            <td><span class="${statusClass(eventStatus(event))}">${eventStatus(event)}</span></td>
            <td>${inspectButton(`/api/smart_home/events/${entry.sequence}`, "runtime event")}</td>
          </tr>
        `;
      }).join("") || `<tr><td colspan="5" class="muted">No runtime events</td></tr>`;
    };

    const renderCommandResults = (audit, filters) => {
      const results = filterRows(audit.results || [], filters);
      els.commandResults.innerHTML = results.map((record) => {
        const result = record.result || {};
        const detailUrl = `/api/smart_home/command_results/${encodeURIComponent(result.command_id || "")}`;
        return `
          <tr>
            <td>${result.command_id || "unknown"}<br><span class="muted">${result.correlation_id || ""}</span></td>
            <td><span class="${statusClass(result.status || "ok")}">${result.status || "unknown"}</span></td>
            <td>${result.bridge_id || ""}</td>
            <td>${record.sequence}</td>
            <td>${result.command_id ? inspectButton(detailUrl, "command result") : ""}</td>
          </tr>
        `;
      }).join("") || `<tr><td colspan="5" class="muted">No command results</td></tr>`;
    };

    const renderAuthorizationDecisions = (audit, filters) => {
      const decisions = filterRows(audit.decisions || [], filters);
      els.authorizationDecisions.innerHTML = decisions.map((record) => `
        <tr>
          <td>${record.principal_id}<br><span class="muted">${observedText(record.decided_at_ms)}</span></td>
          <td>${subjectText(record.subject)}</td>
          <td><span class="${statusClass(record.outcome || "ok")}">${record.outcome}</span></td>
          <td>${record.required_tier}</td>
          <td>${inspectButton(`/api/smart_home/authorization_decisions/${record.decision_index}`, "authorization decision")} ${inspectButton(principalCapabilityGrantsUrl(record.principal_id), "principal grants", "Grants")}</td>
        </tr>
      `).join("") || `<tr><td colspan="5" class="muted">No authorization decisions</td></tr>`;
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
          routes,
          rooms,
          devices,
          bridges,
          events,
          commandResults,
          authorizationDecisions,
          capabilityGrants
        ] = await Promise.all([
          json("/api/smart_home/bootstrap"),
          json("/api/smart_home/readiness"),
          json(queryUrl("/api/smart_home/states", {limit: 24, domain: filters.domain, room_id: roomId, stale})),
          json(queryUrl("/api/smart_home/states", {limit: 24, room_id: roomId, stale: true})),
          json(queryUrl("/api/smart_home/scenes", {limit: 12, room_id: roomId})),
          json(queryUrl("/api/smart_home/desired_states", {limit: 12})),
          json(queryUrl("/api/smart_home/state_history", {limit: 12, room_id: roomId})),
          json("/api/smart_home/services?limit=8"),
          json("/api/smart_home/api?mutating=true&authorized=true"),
          json("/api/smart_home/rooms?sort=scene_count"),
          json(queryUrl("/api/smart_home/devices", {limit: 8, room_id: roomId})),
          json("/api/smart_home/bridges?limit=8"),
          json(queryUrl("/api/smart_home/events", {limit: 12, kind: filters.eventKind, room_id: roomId})),
          json(queryUrl("/api/smart_home/command_results", {
            limit: 8,
            room_id: roomId,
            status: filters.commandStatus
          })),
          json(queryUrl("/api/smart_home/authorization_decisions", {
            limit: 8,
            outcome: filters.authorizationOutcome
          })),
          json(queryUrl("/api/smart_home/capability_grants", {
            limit: 8,
            principal_id: filters.grantPrincipal,
            status: filters.grantStatus,
            scope: filters.grantScope,
            sort: "principal_id"
          }))
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
        renderServices(services);
        renderRoutes(routes);
        renderRoomOptions(rooms, filters.room);
        renderRooms(rooms, filters);
        renderDevices(devices);
        renderBridges(bridges);
        renderScenes(scenes);
        renderEntities(states, filters);
        renderDesiredStates(desiredStates, filters);
        renderGaps(stateGaps, filters);
        renderHistory(history, filters);
        renderEvents(events, filters);
        renderCommandResults(commandResults, filters);
        renderAuthorizationDecisions(authorizationDecisions, filters);
        renderCapabilityGrants(capabilityGrants, filters);
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
      const button = serviceButton || sceneButton || clearDesiredButton || desiredButton || inspectDetailButton;
      if (!button) {
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
          await json(`/api/services/light/${serviceButton.dataset.service}`, {
            method: "POST",
            headers: {"content-type": "application/json"},
            body: JSON.stringify(body)
          });
          log(`${serviceButton.dataset.service} accepted for ${serviceButton.dataset.entity}`);
        } else if (sceneButton) {
          await json("/api/services/scene/turn_on", {
            method: "POST",
            headers: {"content-type": "application/json"},
            body: JSON.stringify({entity_id: sceneButton.dataset.scene})
          });
          log(`scene.turn_on accepted for ${sceneButton.dataset.scene}`);
        } else if (clearDesiredButton) {
          await json(`/api/smart_home/desired_states/${encodeURIComponent(clearDesiredButton.dataset.clearDesired)}`, {
            method: "DELETE"
          });
          log(`desired state cleared for ${clearDesiredButton.dataset.clearDesired}`);
        } else {
          const desiredState = {};
          if (desiredButton.dataset.desiredAction === "brightness") {
            const input = brightnessInputFor(desiredButton.dataset.brightnessFor);
            desiredState["light.brightness"] = input ? Number(input.value) : 100;
          } else {
            desiredState["light.on_off"] = desiredButton.dataset.desiredAction === "on";
          }
          await json(`/api/smart_home/desired_states/${encodeURIComponent(desiredButton.dataset.entity)}`, {
            method: "POST",
            headers: {"content-type": "application/json"},
            body: JSON.stringify({
              desired_state: desiredState,
              requested_by: "agent:dashboard",
              command_timeout_ms: 3000
            })
          });
          log(`desired ${desiredButton.dataset.desiredAction} target accepted for ${desiredButton.dataset.entity}`);
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
    config: SmartHomePlatformHttpConfig,
    event_types: Vec<String>,
    principal_id: AgentId,
    now_ms: u64,
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
            config,
            event_types: default_event_types(),
            principal_id: AgentId::trusted("agent:home-assistant-local-api"),
            now_ms: 0,
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
        self.now_ms = now_ms;
        self
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
            self.now_ms,
        )
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
        app.get("/api/smart_home/dashboard", move |_| {
            runtime_dashboard_response(&runtime)
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
        app.post("/api/services/:domain/:service", move |request| {
            service_call_response(&runtime, request)
        });
    }

    app
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
        query_params: &["filter_entity_id", "minimal_response", "room_id"],
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
        path: "/api/smart_home/dashboard",
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
            "room_id",
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
                .map_or(true, |method| route.method == method)
        })
        .filter(|route| category.map_or(true, |category| route.category == category))
        .filter(|route| {
            surface.map_or(true, |surface| surface == "all" || route.surface == surface)
        })
        .filter(|route| mutating.map_or(true, |mutating| route.mutates_runtime == mutating))
        .filter(|route| {
            authorized.map_or(true, |authorized| route.runtime_authorized == authorized)
        })
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
        runtime_snapshot_json(&runtime_guard.read_snapshot_at(runtime.now_ms)).into_bytes(),
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

fn runtime_dashboard_response(runtime: &SmartHomePlatformHttpRuntime) -> WebResponse {
    let runtime_guard = runtime
        .runtime
        .lock()
        .expect("smart-home runtime mutex should not be poisoned");
    WebResponse::json(runtime_dashboard_json(runtime, &runtime_guard).into_bytes())
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
    let entities = match runtime_state_entities(&runtime_guard, request, runtime.now_ms) {
        Ok(entities) => entities,
        Err(error) => return api_error_response(error),
    };
    WebResponse::json(states_registry_json(&entities, &runtime_guard, runtime.now_ms).into_bytes())
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
    WebResponse::json(state_registry_json(entity, &runtime_guard, runtime.now_ms).into_bytes())
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
        entities_registry_json(&entities, &runtime_guard, runtime.now_ms).into_bytes(),
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
    WebResponse::json(entity_registry_json(entity, &runtime_guard, runtime.now_ms).into_bytes())
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
    let grants = runtime_guard.query_capability_grants_at(&query, runtime.now_ms);
    let summary = runtime_guard.capability_grant_summary_at(&query, runtime.now_ms);
    WebResponse::json(capability_grants_json(&grants, &summary, runtime.now_ms).into_bytes())
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
    WebResponse::json(capability_grant_json(grant, runtime.now_ms).into_bytes())
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
    WebResponse::json(devices_registry_json(&devices, &runtime_guard, runtime.now_ms).into_bytes())
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
    WebResponse::json(device_registry_json(device, &runtime_guard, runtime.now_ms).into_bytes())
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
    WebResponse::json(bridges_registry_json(&bridges, &runtime_guard, runtime.now_ms).into_bytes())
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
    WebResponse::json(bridge_registry_json(bridge, &runtime_guard, runtime.now_ms).into_bytes())
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
    let rooms = runtime_guard.query_room_summaries_at(&query, runtime.now_ms);
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
    let rooms = runtime_guard.query_room_summaries_at(&query, runtime.now_ms);
    let Some(room) = rooms.first() else {
        return api_error_response(ApiError::not_found(format!("room `{room_id}` not found")));
    };
    WebResponse::json(room_detail_json(room, &runtime_guard, runtime.now_ms).into_bytes())
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
    let snapshot = runtime_guard.read_snapshot_at(runtime.now_ms);
    let topology = runtime_guard.topology_summary();
    let pending = snapshot.pending_work_summary();
    let stale_entities = runtime_guard
        .registry()
        .entities()
        .filter(|entity| {
            entity
                .state
                .as_ref()
                .is_some_and(|snapshot| snapshot.is_stale_at(runtime.now_ms))
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
        runtime.now_ms,
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
        "{{\"generated_at_ms\":{},\"status\":{},\"ready\":{},\"summary\":{{\"total_checks\":{},\"passing_checks\":{},\"attention_checks\":{},\"blocking_checks\":{}}},\"links\":{{\"health\":{},\"dashboard\":{},\"bootstrap\":{},\"smoke\":{},\"api\":{},\"state_gaps\":{},\"command_results\":{},\"authorization_decisions\":{},\"capability_grants\":{}}},\"checks\":[{}]}}",
        runtime.now_ms,
        json_string(status),
        blocking_checks == 0,
        checks.len(),
        passing_checks,
        attention_checks,
        blocking_checks,
        json_string("/api/smart_home/health"),
        json_string("/api/smart_home/dashboard"),
        json_string("/api/smart_home/bootstrap"),
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
    let snapshot = runtime_guard.read_snapshot_at(runtime.now_ms);
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

fn runtime_dashboard_json(
    runtime: &SmartHomePlatformHttpRuntime,
    runtime_guard: &SmartHomeRuntime,
) -> String {
    let state = SmartHomePlatformHttpState::from_runtime(
        runtime_guard,
        runtime.config.clone(),
        runtime.event_types.clone(),
        runtime.now_ms,
    );
    let state_summary = state.summary();
    let snapshot = runtime_guard.read_snapshot_at(runtime.now_ms);
    let topology = runtime_guard.topology_summary();
    let pending = snapshot.pending_work_summary();
    let rooms = runtime_guard.query_room_summaries_at(
        &RuntimeRoomQuery::new()
            .sorted_by(RuntimeRoomSort::AttentionDesc)
            .with_limit(50),
        runtime.now_ms,
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
        runtime.now_ms,
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
        bridges_registry_json(&bridges, runtime_guard, runtime.now_ms),
        devices_registry_json(&devices, runtime_guard, runtime.now_ms),
        entities_registry_json(&entities, runtime_guard, runtime.now_ms),
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
    let state_gaps = runtime_state_gap_entities(runtime_guard, runtime.now_ms, 25);
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
        "{{\"generated_at_ms\":{},\"version\":{},\"links\":{{\"readiness\":{},\"dashboard\":{},\"smoke\":{},\"api\":{},\"states\":{},\"state_history\":{},\"command_results\":{},\"authorization_decisions\":{},\"capability_grants\":{}}},\"health\":{},\"dashboard\":{},\"api\":{},\"state_gaps\":{},\"recent_activity\":{{\"events\":{{\"summary\":{}}},\"command_results\":{{\"summary\":{}}},\"authorization_decisions\":{{\"summary\":{}}}}}}}",
        runtime.now_ms,
        json_string(VERSION),
        json_string("/api/smart_home/readiness"),
        json_string("/api/smart_home/dashboard"),
        json_string("/api/smart_home/smoke"),
        json_string("/api/smart_home/api"),
        json_string("/api/smart_home/states"),
        json_string("/api/smart_home/state_history"),
        json_string("/api/smart_home/command_results"),
        json_string("/api/smart_home/authorization_decisions"),
        json_string("/api/smart_home/capability_grants"),
        runtime_health_json(runtime, runtime_guard),
        runtime_dashboard_json(runtime, runtime_guard),
        api_catalog_json(&routes),
        states_registry_json(&state_gaps, runtime_guard, runtime.now_ms),
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
        "{{\"generated_at_ms\":{},\"version\":{},\"status\":{},\"ready\":{},\"principal_id\":{},\"summary\":{{\"total_checks\":{},\"safe_get_checks\":{},\"mutating_checks\":{},\"runtime_authorized_checks\":{},\"blocking_readiness_checks\":{},\"attention_readiness_checks\":{}}},\"links\":{{\"self\":{},\"dashboard\":{},\"readiness\":{},\"bootstrap\":{},\"api\":{},\"command_results\":{},\"authorization_decisions\":{},\"capability_grants\":{}}},\"checks\":[{}]}}",
        runtime.now_ms,
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
        json_string("/"),
        json_string("/api/smart_home/readiness"),
        json_string("/api/smart_home/bootstrap"),
        json_string("/api/smart_home/api"),
        json_string("/api/smart_home/command_results"),
        json_string("/api/smart_home/authorization_decisions"),
        json_string("/api/smart_home/capability_grants"),
        checks
            .iter()
            .map(runtime_smoke_check_json)
            .collect::<Vec<_>>()
            .join(","),
    )
}

fn runtime_smoke_checks(
    runtime: &SmartHomePlatformHttpRuntime,
    runtime_guard: &SmartHomeRuntime,
) -> Vec<RuntimeSmokeCheck> {
    let state = SmartHomePlatformHttpState::from_runtime(
        runtime_guard,
        runtime.config.clone(),
        runtime.event_types.clone(),
        runtime.now_ms,
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
    checks.push(runtime_smoke_command_probe(&state));
    checks.extend([
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
        "{{\"sequence\":{},\"next_sequence\":{},\"event\":{}}}",
        entry.sequence,
        entry.next_checkpoint.next_sequence(),
        runtime_event_json(entry.event),
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
        "{{\"sequence\":{},\"next_sequence\":{},\"result\":{}}}",
        record.sequence,
        record.next_checkpoint.next_sequence(),
        command_result_json(&record.result),
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
        "{{\"decision_index\":{},\"principal_id\":{},\"subject\":{},\"outcome\":{},\"required_tier\":{},\"required_capabilities\":[{}],\"matched_grants\":[{}],\"missing_capabilities\":[{}],\"decided_at_ms\":{}}}",
        record.decision_index,
        json_string(decision.principal_id.as_str()),
        authorization_subject_json(&decision.subject),
        json_string(authorization_outcome_label(decision.outcome)),
        json_string(privilege_tier_label(decision.required_tier)),
        json_id_array(decision.required_capabilities.iter().map(|id| id.as_str())),
        json_id_array(decision.matched_grants.iter().map(|id| id.as_str())),
        json_id_array(decision.missing_capabilities.iter().map(|id| id.as_str())),
        decision.decided_at_ms,
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
    let capability_ids = entity
        .capabilities
        .iter()
        .map(|capability| capability.capability_id.as_str());
    let snapshot = entity.state.as_ref();

    format!(
        "{{\"entity_id\":{},\"home_assistant_entity_id\":{},\"device_id\":{},\"bridge_id\":{},\"room_id\":{},\"name\":{},\"domain\":{},\"entity_kind\":{},\"has_state\":{},\"stale\":{},\"value\":{},\"source\":{},\"confidence\":{},\"observed_at_ms\":{},\"received_at_ms\":{},\"expires_at_ms\":{},\"capability_ids\":[{}]}}",
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
        json_id_array(capability_ids),
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
    let room_id = query_string(request, "room_id");
    let observed_at_or_after_ms = query_u64(request, "observed_at_or_after_ms")?;
    let received_at_or_after_ms = query_u64(request, "received_at_or_after_ms")?;
    let limit = query_limit(request, 100, 1_000)?;

    let mut events = runtime
        .registry()
        .events()
        .filter(|event| {
            entity_id
                .as_ref()
                .is_none_or(|entity_id| event.entity_id.as_ref() == Some(entity_id))
        })
        .filter(|event| {
            room_id.is_none_or(|room_id| device_event_matches_room(runtime, event, room_id))
        })
        .filter(|event| event_type.is_none_or(|event_type| event.event_type == event_type))
        .filter(|event| {
            observed_at_or_after_ms
                .is_none_or(|observed_at_ms| event.observed_at_ms >= observed_at_ms)
        })
        .filter(|event| {
            received_at_or_after_ms
                .is_none_or(|received_at_ms| event.received_at_ms >= received_at_ms)
        })
        .collect::<Vec<_>>();

    if query_string(request, "sort").is_some_and(|sort| sort == "desc") {
        events.reverse();
    }
    events.truncate(limit);
    Ok(events)
}

fn history_entity_filter<'a>(request: &'a WebRequest) -> Option<&'a str> {
    query_string(request, "entity_id").or_else(|| query_string(request, "filter_entity_id"))
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

    let output = match runtime_guard.execute_set_desired_state_tool(
        runtime.principal_id.clone(),
        RuntimeSetDesiredStateToolRequest::new(desired_state),
        runtime.now_ms,
    ) {
        Ok(output) => output,
        Err(error) => return api_error_response(runtime_error_to_api_error(error)),
    };
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

    let output = match runtime_guard.execute_clear_desired_state_tool(
        runtime.principal_id.clone(),
        RuntimeClearDesiredStateToolRequest::new(entity_id),
        runtime.now_ms,
    ) {
        Ok(output) => output,
        Err(error) => return api_error_response(runtime_error_to_api_error(error)),
    };
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
    let before = SmartHomePlatformHttpState::from_runtime(
        &runtime_guard,
        runtime.config.clone(),
        runtime.event_types.clone(),
        runtime.now_ms,
    );
    let commands = match service_commands(&before, domain, service, &call) {
        Ok(commands) => commands,
        Err(error) => return api_error_response(error),
    };

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

        match runtime_guard.execute_command_tool(
            runtime.principal_id.clone(),
            request,
            runtime.now_ms,
        ) {
            Ok(result) => results.push(result),
            Err(error) => return api_error_response(runtime_error_to_api_error(error)),
        }
    }

    let after = SmartHomePlatformHttpState::from_runtime(
        &runtime_guard,
        runtime.config.clone(),
        runtime.event_types.clone(),
        runtime.now_ms,
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
        "{{\"command_id\":{},\"status\":{},\"bridge_id\":{},\"correlation_id\":{},\"message\":{}}}",
        json_string(result.command_id.as_str()),
        json_string(command_status_label(result.status)),
        json_string(result.bridge_id.as_str()),
        json_string(result.correlation_id.as_str()),
        result
            .message
            .as_ref()
            .map(json_string)
            .unwrap_or_else(|| "null".to_string()),
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
        .map(|value| {
            value
                .parse::<u64>()
                .map_err(|_| ApiError::bad_request(format!("{key} must be an unsigned integer")))
        })
        .transpose()
}

fn route_u64(request: &WebRequest, key: &str) -> Result<u64, ApiError> {
    let Some(value) = request.route_params.get(key) else {
        return Err(ApiError::bad_request(format!("missing {key}")));
    };
    value
        .parse::<u64>()
        .map_err(|_| ApiError::bad_request(format!("{key} must be an unsigned integer")))
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
    use smart_home_core::{BridgeId, DeviceId, EventId};
    use smart_home_testkit::hue_lighting_runtime;
    use std::io::{BufRead, BufReader, Read, Write};
    use std::net::{SocketAddr, TcpStream};
    use std::sync::Arc;
    use std::thread;
    use std::time::Duration;
    use tcp_runtime::{ConnectionId, TcpConnectionInfo};
    use web_core::WebServer;

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
        let mut server = WebServer::bind_kqueue(
            "127.0.0.1:0",
            HttpServerOptions::default(),
            Arc::clone(&app),
        )
        .expect("bind kqueue");

        #[cfg(target_os = "linux")]
        let mut server = WebServer::bind_epoll(
            "127.0.0.1:0",
            HttpServerOptions::default(),
            Arc::clone(&app),
        )
        .expect("bind epoll");

        #[cfg(target_os = "windows")]
        let mut server = WebServer::bind_windows(
            "127.0.0.1:0",
            HttpServerOptions::default(),
            Arc::clone(&app),
        )
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
            assert!(body.contains("data-dashboard-filter=\"search\""));
            assert!(body.contains("data-dashboard-filter=\"room\""));
            assert!(body.contains("data-dashboard-filter=\"domain\""));
            assert!(body.contains("data-dashboard-filter=\"command-status\""));
            assert!(body.contains("const FILTER_QUERY_PARAMS = ["));
            assert!(body.contains("[\"event_kind\", els.filterEventKind]"));
            assert!(body.contains("[\"command_status\", els.filterCommandStatus]"));
            assert!(body.contains("restoreFiltersFromUrl()"));
            assert!(body.contains("window.history.replaceState(null, \"\", nextUrl)"));
            assert!(body.contains(
                "queryUrl(\"/api/smart_home/states\", {limit: 24, domain: filters.domain, room_id: roomId, stale})"
            ));
            assert!(body.contains(
                "queryUrl(\"/api/smart_home/states\", {limit: 24, room_id: roomId, stale: true})"
            ));
            assert!(
                body.contains("queryUrl(\"/api/smart_home/scenes\", {limit: 12, room_id: roomId})")
            );
            assert!(body.contains("queryUrl(\"/api/smart_home/desired_states\", {limit: 12})"));
            assert!(body.contains(
                "queryUrl(\"/api/smart_home/state_history\", {limit: 12, room_id: roomId})"
            ));
            assert!(body.contains("json(\"/api/smart_home/services?limit=8\")"));
            assert!(body.contains("json(\"/api/smart_home/api?mutating=true&authorized=true\")"));
            assert!(body.contains("json(\"/api/smart_home/rooms?sort=scene_count\")"));
            assert!(
                body.contains("queryUrl(\"/api/smart_home/devices\", {limit: 8, room_id: roomId})")
            );
            assert!(body.contains("json(\"/api/smart_home/bridges?limit=8\")"));
            assert!(body.contains(
                "queryUrl(\"/api/smart_home/events\", {limit: 12, kind: filters.eventKind, room_id: roomId})"
            ));
            assert!(body.contains("queryUrl(\"/api/smart_home/command_results\", {"));
            assert!(body.contains("room_id: roomId"));
            assert!(body.contains("status: filters.commandStatus"));
            assert!(body.contains("queryUrl(\"/api/smart_home/authorization_decisions\", {"));
            assert!(body.contains("outcome: filters.authorizationOutcome"));
            assert!(body.contains("renderRoomOptions(rooms, filters.room)"));
            assert!(body.contains("entityMatchesFilters(filters, entity)"));
            assert!(body.contains("filterRows(history.events || [], filters)"));
            assert!(body.contains("<tbody id=\"events\"></tbody>"));
            assert!(body.contains("id=\"detail-body\""));
            assert!(body.contains("renderDetail(label, url, response.status, response.ok, body)"));
            assert!(body.contains("inspectDetail(inspectDetailButton)"));
            assert!(body.contains("data-inspect-url"));
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
            assert!(body.contains("data-service=\"set_brightness\""));
            assert!(body.contains("brightness_pct"));
            assert!(body.contains("/api/services/scene/turn_on"));
            assert!(body.contains("/api/smart_home/desired_states/"));
        }
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
        let command_detail = response_body(app.handle(request("GET", &command_detail_path)).into());
        assert!(command_detail.contains(r#""sequence":0"#));
        assert!(command_detail.contains(&format!(r#""command_id":"{command_id}""#)));

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

        let missing_decision: web_core::WebResponse = app
            .handle(request(
                "GET",
                "/api/smart_home/authorization_decisions/999",
            ))
            .into();
        assert_eq!(missing_decision.status, 404);
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
        assert!(bootstrap.contains(r#""dashboard":"/api/smart_home/dashboard""#));
        assert!(bootstrap.contains(r#""smoke":"/api/smart_home/smoke""#));
        assert!(bootstrap.contains(r#""states":"/api/smart_home/states""#));
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
        assert!(smoke.contains(r#""check_id":"command_probe""#));
        assert!(smoke.contains(r#""path":"/api/services/light/turn_on""#));
        assert!(smoke.contains(r#""runtime_authorized":true"#));
        assert!(smoke.contains(r#""entity_id":"light.entity_light_1""#));
        assert!(smoke.contains(r#""brightness_pct":75"#));

        let smoke_json: JsonValue =
            serde_json::from_str(&smoke).expect("smoke plan response is JSON");
        assert_eq!(smoke_json["summary"]["total_checks"], 10);
        assert_eq!(smoke_json["summary"]["safe_get_checks"], 9);
        assert_eq!(smoke_json["summary"]["mutating_checks"], 1);
        assert_eq!(smoke_json["summary"]["runtime_authorized_checks"], 1);
        assert_eq!(smoke_json["summary"]["blocking_readiness_checks"], 0);
        assert_eq!(smoke_json["summary"]["attention_readiness_checks"], 1);
        assert_eq!(
            smoke_json["checks"][6]["request_body"]["entity_id"],
            "light.entity_light_1"
        );
        assert_eq!(
            smoke_json["checks"][6]["request_body"]["brightness_pct"],
            75
        );
    }

    #[test]
    fn runtime_web_app_serves_dashboard_ready_api_catalog() {
        let app = home_assistant_runtime_web_app(fixture_runtime(true));

        let catalog = response_body(app.handle(request("GET", "/api/smart_home/api")).into());
        assert!(catalog.contains(r#""path":"/api/smart_home/readiness""#));
        assert!(catalog.contains(r#""path":"/api/smart_home/dashboard""#));
        assert!(catalog.contains(r#""path":"/api/smart_home/smoke""#));
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
            r#""path":"/api/smart_home/events","category":"events","surface":"smart_home","mutates_runtime":false,"runtime_authorized":false,"query_params":["entity_id","from_sequence","kind","limit","room_id","sort"]"#
        ));
        assert!(catalog.contains(
            r#""path":"/api/smart_home/command_results","category":"command_results","surface":"smart_home","mutates_runtime":false,"runtime_authorized":false,"query_params":["bridge_id","command_id","correlation_id","from_sequence","limit","room_id","sort","status"]"#
        ));
        assert!(catalog.contains(
            r#""path":"/api/smart_home/capability_grants","category":"authorization","surface":"smart_home","mutates_runtime":false,"runtime_authorized":false,"query_params":["capability_id","entity_id","limit","principal_id","scope","sort","status"]"#
        ));
        assert!(catalog.contains(
            r#""path":"/api/smart_home/state_history","category":"state_history","surface":"smart_home","mutates_runtime":false,"runtime_authorized":false,"query_params":["bridge_id","entity_id","event_type","from_ms","limit","room_id","to_ms"]"#
        ));
        let catalog_json: JsonValue =
            serde_json::from_str(&catalog).expect("API catalog response is JSON");
        assert!(
            catalog_json["route_count"].as_u64().unwrap_or_default() >= 30,
            "catalog exposes the local controller route surface"
        );

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
        assert_eq!(mutating_json["route_count"], 4);
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
        assert!(body.contains("data-dashboard-filter=\"grant-status\""));
        assert!(body.contains("data-dashboard-filter=\"grant-scope\""));
        assert!(body.contains("data-dashboard-filter=\"grant-principal\""));
        assert!(body.contains("const FILTER_QUERY_PARAMS = ["));
        assert!(body.contains("[\"grant_status\", els.filterGrantStatus]"));
        assert!(body.contains("window.addEventListener(\"popstate\""));
        assert!(body.contains("window.history.replaceState(null, \"\", nextUrl)"));
        assert!(body.contains("queryUrl(\"/api/smart_home/scenes\", {limit: 12, room_id: roomId})"));
        assert!(body.contains("queryUrl(\"/api/smart_home/desired_states\", {limit: 12})"));
        assert!(body
            .contains("queryUrl(\"/api/smart_home/state_history\", {limit: 12, room_id: roomId})"));
        assert!(body.contains("json(\"/api/smart_home/services?limit=8\")"));
        assert!(body.contains("json(\"/api/smart_home/api?mutating=true&authorized=true\")"));
        assert!(body.contains("json(\"/api/smart_home/rooms?sort=scene_count\")"));
        assert!(body.contains("queryUrl(\"/api/smart_home/devices\", {limit: 8, room_id: roomId})"));
        assert!(body.contains("json(\"/api/smart_home/bridges?limit=8\")"));
        assert!(body.contains("queryUrl(\"/api/smart_home/command_results\", {"));
        assert!(body.contains("queryUrl(\"/api/smart_home/authorization_decisions\", {"));
        assert!(body.contains("queryUrl(\"/api/smart_home/capability_grants\", {"));
        assert!(body.contains("id=\"detail-body\""));
        assert!(body.contains("renderDetail(label, url, response.status, response.ok, body)"));
        assert!(body.contains("id=\"capability-grants\""));
        assert!(body.contains("renderCapabilityGrants(capabilityGrants, filters)"));
        assert!(body.contains("principalCapabilityGrantsUrl(record.principal_id)"));
        assert!(body.contains("capabilityGrantDetailUrl(grant)"));
        assert!(body.contains("stateDetailUrl(entity)"));
        assert!(body.contains("entityHistoryUrl(entity)"));
        assert!(body.contains("entityEventsUrl(entity)"));
        assert!(body.contains("entityBridgeCommandsUrl(entity)"));
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
}
