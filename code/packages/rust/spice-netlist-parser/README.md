# spice-netlist-parser

Small SPICE3 netlist parser that builds `spice_engine::Circuit` values.

```rust
use spice_netlist_parser::parse_netlist;

let parsed = parse_netlist(r#"
* RC low pass
V1 in 0 PULSE(0 1 0 1n 1n 10n 20n)
R1 in out 1k
C1 out 0 1u
.tran 1n 20n method=gear2
.end
"#)?;

assert_eq!(parsed.tran_cards().len(), 1);
```

`parse_netlist` lowers decks through the Berkeley SPICE logical-card syntax
facade, so normal simulator parsing honors leading `+` continuations, strips
inline comments from normalized cards, and reports stable syntax diagnostics
before semantic lowering starts.

For editor, Mosaic, and parser-generator frontends, the crate also exposes the
same facade directly:

```rust
use spice_netlist_parser::{
    berkeley_app_package_manifest_json, parse_berkeley_app_deck,
    BerkeleyAppPersistedEditorState, BerkeleyCardKind,
};

let deck = parse_berkeley_app_deck(r#"
* divider
V1 in 0 DC 1
R1 in out 1k
R2 out 0 1k
.op
.end
"#);

assert!(!deck.has_errors());
assert_eq!(deck.analysis_inventory()[0].analysis, "op");
assert_eq!(deck.syntax.cards[1].kind, BerkeleyCardKind::Element);

let execution = deck.run_artifacts()?;
assert_eq!(execution.analyses[0].syntax_card_index, Some(3));
assert!(execution.analyses[0].table_columns.contains(&"Index".to_string()));
assert_eq!(execution.analyses[0].waveform_series[0].x_column, "Index");

let session = deck.run_session_state(Some(3))?;
assert!(session.execution_available);
assert_eq!(session.selected_waveform_series_count, Some(1));

let controls = deck.run_editor_controls(Some(3))?;
assert!(controls.selected_control.unwrap().waveform_available);

let command_plan = deck.run_editor_command_plan(Some(3))?;
assert!(command_plan
    .commands
    .iter()
    .any(|command| command.id == "analysis.3.inspect-waveform" && command.enabled));

let view = deck.run_editor_state_snapshot(Default::default())?;
assert_eq!(view.resolved_state.selected_syntax_card_index, Some(3));

let host = deck.run_host_surface(BerkeleyAppPersistedEditorState {
    selected_syntax_card_index: Some(3),
    active_command_id: Some("analysis.3.inspect-waveform".to_string()),
})?;
assert_eq!(host.active_panel.unwrap().id, "waveform");

let host_wire_json = deck.run_host_surface_wire_json(BerkeleyAppPersistedEditorState {
    selected_syntax_card_index: Some(3),
    active_command_id: Some("analysis.3.inspect-waveform".to_string()),
})?;
assert!(host_wire_json.contains(r#""activePanelId":"waveform""#));

let package_manifest_json = berkeley_app_package_manifest_json();
assert!(package_manifest_json.contains(r#""packageName":"berkeley-spice-mosaic-app""#));

let bootstrap_json = deck.run_app_bootstrap_json(BerkeleyAppPersistedEditorState {
    selected_syntax_card_index: Some(3),
    active_command_id: Some("analysis.3.inspect-waveform".to_string()),
})?;
assert!(bootstrap_json.contains(r#""packageManifest":{"#));
assert!(bootstrap_json.contains(r#""hostSurface":{"#));

let startup_summary_json = deck.run_app_startup_summary_json(BerkeleyAppPersistedEditorState {
    selected_syntax_card_index: Some(3),
    active_command_id: Some("analysis.3.inspect-waveform".to_string()),
})?;
assert!(startup_summary_json.contains(r#""ready":true"#));

let launch_plan_json = deck.run_app_launch_plan_json(BerkeleyAppPersistedEditorState {
    selected_syntax_card_index: Some(3),
    active_command_id: Some("analysis.3.inspect-waveform".to_string()),
})?;
assert!(launch_plan_json.contains(r#""startupRoute":"ready""#));
assert!(launch_plan_json.contains(r#""entryPanelId":"waveform""#));

let readiness_report_json = deck.run_app_readiness_report_json(BerkeleyAppPersistedEditorState {
    selected_syntax_card_index: Some(3),
    active_command_id: Some("analysis.3.inspect-waveform".to_string()),
})?;
assert!(readiness_report_json.contains(r#""errorCount":0"#));

let shell_handoff_json = deck.run_app_shell_handoff_json(BerkeleyAppPersistedEditorState {
    selected_syntax_card_index: Some(3),
    active_command_id: Some("analysis.3.inspect-waveform".to_string()),
})?;
assert!(shell_handoff_json.contains(r#""packageManifest":{"#));
assert!(shell_handoff_json.contains(r#""readinessReport":{"#));

let shell_status_json = deck.run_app_shell_status_json(BerkeleyAppPersistedEditorState {
    selected_syntax_card_index: Some(3),
    active_command_id: Some("analysis.3.inspect-waveform".to_string()),
})?;
assert!(shell_status_json.contains(r#""severity":"ready""#));
assert!(shell_status_json.contains(r#""message":"Ready to launch waveform panel""#));

let shell_telemetry_json = deck.run_app_shell_telemetry_json(BerkeleyAppPersistedEditorState {
    selected_syntax_card_index: Some(3),
    active_command_id: Some("analysis.3.inspect-waveform".to_string()),
})?;
assert!(shell_telemetry_json.contains(r#""enabledPanelCount":4"#));
assert!(shell_telemetry_json.contains(r#""artifactCapabilityCount":"#));

let shell_events_json = deck.run_app_shell_event_log_json(BerkeleyAppPersistedEditorState {
    selected_syntax_card_index: Some(3),
    active_command_id: Some("analysis.3.inspect-waveform".to_string()),
})?;
assert!(shell_events_json.contains(r#""eventCount":6"#));
assert!(shell_events_json.contains(r#""id":"shell.status""#));

let shell_event_summary_json = deck.run_app_shell_event_summary_json(
    BerkeleyAppPersistedEditorState {
        selected_syntax_card_index: Some(3),
        active_command_id: Some("analysis.3.inspect-waveform".to_string()),
    },
)?;
assert!(shell_event_summary_json.contains(r#""readyEventCount":3"#));
assert!(shell_event_summary_json.contains(r#""artifactCapabilityCount":"#));

let shell_event_digest_json = deck.run_app_shell_event_digest_json(
    BerkeleyAppPersistedEditorState {
        selected_syntax_card_index: Some(3),
        active_command_id: Some("analysis.3.inspect-waveform".to_string()),
    },
)?;
assert!(shell_event_digest_json.contains(r#""headlineEventId":"shell.status""#));
assert!(shell_event_digest_json.contains(r#""metricEventCount":3"#));

let shell_event_dashboard_json = deck.run_app_shell_event_dashboard_json(
    BerkeleyAppPersistedEditorState {
        selected_syntax_card_index: Some(3),
        active_command_id: Some("analysis.3.inspect-waveform".to_string()),
    },
)?;
assert!(shell_event_dashboard_json.contains(r#""sectionCount":3"#));
assert!(shell_event_dashboard_json.contains(r#""attentionRequired":false"#));

let shell_dashboard_package_json = deck.run_app_shell_dashboard_package_json(
    BerkeleyAppPersistedEditorState {
        selected_syntax_card_index: Some(3),
        active_command_id: Some("analysis.3.inspect-waveform".to_string()),
    },
)?;
assert!(shell_dashboard_package_json.contains(r#""packageManifest":{"#));
assert!(shell_dashboard_package_json.contains(r#""eventDashboard":{"#));

let shell_dashboard_cards_json = deck.run_app_shell_dashboard_cards_json(
    BerkeleyAppPersistedEditorState {
        selected_syntax_card_index: Some(3),
        active_command_id: Some("analysis.3.inspect-waveform".to_string()),
    },
)?;
assert!(shell_dashboard_cards_json.contains(r#""primaryCardId":"dashboard.status""#));
assert!(shell_dashboard_cards_json.contains(r#""cardsCapabilityId":"app-shell-dashboard-cards-json""#));

let shell_dashboard_view_json = deck.run_app_shell_dashboard_view_json(
    BerkeleyAppPersistedEditorState {
        selected_syntax_card_index: Some(3),
        active_command_id: Some("analysis.3.inspect-waveform".to_string()),
    },
)?;
assert!(shell_dashboard_view_json.contains(r#""primaryCardTitle":"Startup status""#));
assert!(shell_dashboard_view_json.contains(r#""viewCapabilityId":"app-shell-dashboard-view-json""#));

let shell_dashboard_layout_json = deck.run_app_shell_dashboard_layout_json(
    BerkeleyAppPersistedEditorState {
        selected_syntax_card_index: Some(3),
        active_command_id: Some("analysis.3.inspect-waveform".to_string()),
    },
)?;
assert!(shell_dashboard_layout_json.contains(r#""primaryRegionId":"dashboard.layout.status""#));
assert!(shell_dashboard_layout_json.contains(r#""layoutCapabilityId":"app-shell-dashboard-layout-json""#));

let shell_dashboard_navigation_json = deck.run_app_shell_dashboard_navigation_json(
    BerkeleyAppPersistedEditorState {
        selected_syntax_card_index: Some(3),
        active_command_id: Some("analysis.3.inspect-waveform".to_string()),
    },
)?;
assert!(shell_dashboard_navigation_json.contains(r#""activeItemId":"dashboard.nav.status""#));
assert!(shell_dashboard_navigation_json.contains(r#""navigationCapabilityId":"app-shell-dashboard-navigation-json""#));

let shell_dashboard_routes_json = deck.run_app_shell_dashboard_routes_json(
    BerkeleyAppPersistedEditorState {
        selected_syntax_card_index: Some(3),
        active_command_id: Some("analysis.3.inspect-waveform".to_string()),
    },
)?;
assert!(shell_dashboard_routes_json.contains(r#""activeRoutePath":"/dashboard/status""#));
assert!(shell_dashboard_routes_json.contains(r#""routesCapabilityId":"app-shell-dashboard-routes-json""#));

let shell_dashboard_breadcrumbs_json = deck.run_app_shell_dashboard_breadcrumbs_json(
    BerkeleyAppPersistedEditorState {
        selected_syntax_card_index: Some(3),
        active_command_id: Some("analysis.3.inspect-waveform".to_string()),
    },
)?;
assert!(shell_dashboard_breadcrumbs_json
    .contains(r#""activeBreadcrumbId":"dashboard.breadcrumb.status""#));
assert!(shell_dashboard_breadcrumbs_json
    .contains(r#""breadcrumbsCapabilityId":"app-shell-dashboard-breadcrumbs-json""#));

let shell_dashboard_tabs_json = deck.run_app_shell_dashboard_tabs_json(
    BerkeleyAppPersistedEditorState {
        selected_syntax_card_index: Some(3),
        active_command_id: Some("analysis.3.inspect-waveform".to_string()),
    },
)?;
assert!(shell_dashboard_tabs_json.contains(r#""selectedTabId":"dashboard.tab.status""#));
assert!(shell_dashboard_tabs_json
    .contains(r#""tabsCapabilityId":"app-shell-dashboard-tabs-json""#));

let shell_dashboard_tab_panels_json = deck.run_app_shell_dashboard_tab_panels_json(
    BerkeleyAppPersistedEditorState {
        selected_syntax_card_index: Some(3),
        active_command_id: Some("analysis.3.inspect-waveform".to_string()),
    },
)?;
assert!(shell_dashboard_tab_panels_json
    .contains(r#""selectedPanelId":"dashboard.tab-panel.status""#));
assert!(shell_dashboard_tab_panels_json
    .contains(r#""tabPanelsCapabilityId":"app-shell-dashboard-tab-panels-json""#));

let shell_dashboard_panel_cards_json = deck.run_app_shell_dashboard_panel_cards_json(
    BerkeleyAppPersistedEditorState {
        selected_syntax_card_index: Some(3),
        active_command_id: Some("analysis.3.inspect-waveform".to_string()),
    },
)?;
assert!(shell_dashboard_panel_cards_json
    .contains(r#""selectedPanelCardId":"dashboard.panel-card.status""#));
assert!(shell_dashboard_panel_cards_json
    .contains(r#""panelCardsCapabilityId":"app-shell-dashboard-panel-cards-json""#));

let shell_dashboard_panel_card_actions_json =
    deck.run_app_shell_dashboard_panel_card_actions_json(
        BerkeleyAppPersistedEditorState {
            selected_syntax_card_index: Some(3),
            active_command_id: Some("analysis.3.inspect-waveform".to_string()),
        },
    )?;
assert!(shell_dashboard_panel_card_actions_json
    .contains(r#""selectedPanelCardActionId":"dashboard.panel-card-action.status""#));
assert!(shell_dashboard_panel_card_actions_json
    .contains(r#""panelCardActionsCapabilityId":"app-shell-dashboard-panel-card-actions-json""#));

let shell_dashboard_action_dispatch_json =
    deck.run_app_shell_dashboard_action_dispatch_json(
        BerkeleyAppPersistedEditorState {
            selected_syntax_card_index: Some(3),
            active_command_id: Some("analysis.3.inspect-waveform".to_string()),
        },
    )?;
assert!(shell_dashboard_action_dispatch_json
    .contains(r#""selectedActionDispatchId":"dashboard.action-dispatch.status""#));
assert!(shell_dashboard_action_dispatch_json
    .contains(r#""actionDispatchCapabilityId":"app-shell-dashboard-action-dispatch-json""#));

let shell_dashboard_dispatch_events_json =
    deck.run_app_shell_dashboard_dispatch_events_json(
        BerkeleyAppPersistedEditorState {
            selected_syntax_card_index: Some(3),
            active_command_id: Some("analysis.3.inspect-waveform".to_string()),
        },
    )?;
assert!(shell_dashboard_dispatch_events_json
    .contains(r#""selectedDispatchEventId":"dashboard.dispatch-event.status""#));
assert!(shell_dashboard_dispatch_events_json
    .contains(r#""dispatchEventsCapabilityId":"app-shell-dashboard-dispatch-events-json""#));

let shell_dashboard_dispatch_queue_json =
    deck.run_app_shell_dashboard_dispatch_queue_json(
        BerkeleyAppPersistedEditorState {
            selected_syntax_card_index: Some(3),
            active_command_id: Some("analysis.3.inspect-waveform".to_string()),
        },
    )?;
assert!(shell_dashboard_dispatch_queue_json
    .contains(r#""selectedDispatchQueueItemId":"dashboard.dispatch-queue.status""#));
assert!(shell_dashboard_dispatch_queue_json
    .contains(r#""dispatchQueueCapabilityId":"app-shell-dashboard-dispatch-queue-json""#));

let shell_dashboard_dispatch_queue_summary_json =
    deck.run_app_shell_dashboard_dispatch_queue_summary_json(
        BerkeleyAppPersistedEditorState {
            selected_syntax_card_index: Some(3),
            active_command_id: Some("analysis.3.inspect-waveform".to_string()),
        },
    )?;
assert!(shell_dashboard_dispatch_queue_summary_json
    .contains(r#""firstQueuedDispatchQueueItemId":"dashboard.dispatch-queue.status""#));
assert!(shell_dashboard_dispatch_queue_summary_json.contains(
    r#""dispatchQueueSummaryCapabilityId":"app-shell-dashboard-dispatch-queue-summary-json""#
));

let shell_dashboard_dispatch_queue_lanes_json =
    deck.run_app_shell_dashboard_dispatch_queue_lanes_json(
        BerkeleyAppPersistedEditorState {
            selected_syntax_card_index: Some(3),
            active_command_id: Some("analysis.3.inspect-waveform".to_string()),
        },
    )?;
assert!(shell_dashboard_dispatch_queue_lanes_json
    .contains(r#""activeLaneId":"dashboard.dispatch-queue-lane.queued""#));
assert!(shell_dashboard_dispatch_queue_lanes_json.contains(
    r#""dispatchQueueLanesCapabilityId":"app-shell-dashboard-dispatch-queue-lanes-json""#
));

let shell_dashboard_dispatch_queue_lane_tabs_json =
    deck.run_app_shell_dashboard_dispatch_queue_lane_tabs_json(
        BerkeleyAppPersistedEditorState {
            selected_syntax_card_index: Some(3),
            active_command_id: Some("analysis.3.inspect-waveform".to_string()),
        },
    )?;
assert!(shell_dashboard_dispatch_queue_lane_tabs_json
    .contains(r#""activeTabId":"dashboard.dispatch-queue-lane-tab.queued""#));
assert!(shell_dashboard_dispatch_queue_lane_tabs_json.contains(
    r#""dispatchQueueLaneTabsCapabilityId":"app-shell-dashboard-dispatch-queue-lane-tabs-json""#
));

let shell_dashboard_dispatch_queue_lane_tab_panels_json =
    deck.run_app_shell_dashboard_dispatch_queue_lane_tab_panels_json(
        BerkeleyAppPersistedEditorState {
            selected_syntax_card_index: Some(3),
            active_command_id: Some("analysis.3.inspect-waveform".to_string()),
        },
    )?;
assert!(shell_dashboard_dispatch_queue_lane_tab_panels_json
    .contains(r#""activePanelId":"dashboard.dispatch-queue-lane-tab-panel.queued""#));
assert!(shell_dashboard_dispatch_queue_lane_tab_panels_json.contains(
    r#""dispatchQueueLaneTabPanelsCapabilityId":"app-shell-dashboard-dispatch-queue-lane-tab-panels-json""#
));

let shell_dashboard_dispatch_queue_lane_tab_panel_cards_json =
    deck.run_app_shell_dashboard_dispatch_queue_lane_tab_panel_cards_json(
        BerkeleyAppPersistedEditorState {
            selected_syntax_card_index: Some(3),
            active_command_id: Some("analysis.3.inspect-waveform".to_string()),
        },
    )?;
assert!(shell_dashboard_dispatch_queue_lane_tab_panel_cards_json
    .contains(r#""activePanelCardId":"dashboard.dispatch-queue-lane-tab-panel-card.queued""#));
assert!(shell_dashboard_dispatch_queue_lane_tab_panel_cards_json.contains(
    r#""dispatchQueueLaneTabPanelCardsCapabilityId":"app-shell-dashboard-dispatch-queue-lane-tab-panel-cards-json""#
));

let shell_dashboard_dispatch_queue_lane_tab_panel_card_actions_json =
    deck.run_app_shell_dashboard_dispatch_queue_lane_tab_panel_card_actions_json(
        BerkeleyAppPersistedEditorState {
            selected_syntax_card_index: Some(3),
            active_command_id: Some("analysis.3.inspect-waveform".to_string()),
        },
    )?;
assert!(shell_dashboard_dispatch_queue_lane_tab_panel_card_actions_json.contains(
    r#""activePanelCardActionId":"dashboard.dispatch-queue-lane-tab-panel-card-action.queued""#
));
assert!(shell_dashboard_dispatch_queue_lane_tab_panel_card_actions_json.contains(
    r#""dispatchQueueLaneTabPanelCardActionsCapabilityId":"app-shell-dashboard-dispatch-queue-lane-tab-panel-card-actions-json""#
));

let shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_json =
    deck.run_app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_json(
        BerkeleyAppPersistedEditorState {
            selected_syntax_card_index: Some(3),
            active_command_id: Some("analysis.3.inspect-waveform".to_string()),
        },
    )?;
assert!(shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_json
    .contains(r#""activeMenuItemId":"dashboard.dispatch-queue-lane-tab-panel-card-action-menu-item.queued""#));
assert!(shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_json.contains(
    r#""dispatchQueueLaneTabPanelCardActionMenuCapabilityId":"app-shell-dashboard-dispatch-queue-lane-tab-panel-card-action-menu-json""#
));

let shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_groups_json =
    deck.run_app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_groups_json(
        BerkeleyAppPersistedEditorState {
            selected_syntax_card_index: Some(3),
            active_command_id: Some("analysis.3.inspect-waveform".to_string()),
        },
    )?;
assert!(shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_groups_json
    .contains(r#""activeMenuGroupId":"dashboard.dispatch-queue-lane-tab-panel-card-action-menu-group.queued""#));
assert!(shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_groups_json.contains(
    r#""dispatchQueueLaneTabPanelCardActionMenuGroupsCapabilityId":"app-shell-dashboard-dispatch-queue-lane-tab-panel-card-action-menu-groups-json""#
));

let shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcuts_json =
    deck.run_app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcuts_json(
        BerkeleyAppPersistedEditorState {
            selected_syntax_card_index: Some(3),
            active_command_id: Some("analysis.3.inspect-waveform".to_string()),
        },
    )?;
assert!(shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcuts_json
    .contains(r#""activeMenuGroupShortcutId":"dashboard.dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut.queued""#));
assert!(shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcuts_json
    .contains(r#""dispatchQueueLaneTabPanelCardActionMenuGroupShortcutsCapabilityId":"app-shell-dashboard-dispatch-queue-lane-tab-panel-card-action-menu-group-shortcuts-json""#));

let shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_bindings_json =
    deck.run_app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_bindings_json(
        BerkeleyAppPersistedEditorState {
            selected_syntax_card_index: Some(3),
            active_command_id: Some("analysis.3.inspect-waveform".to_string()),
        },
    )?;
assert!(shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_bindings_json
    .contains(r#""activeMenuGroupShortcutBindingId":"dashboard.dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-binding.queued""#));
assert!(shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_bindings_json
    .contains(r#""dispatchQueueLaneTabPanelCardActionMenuGroupShortcutBindingsCapabilityId":"app-shell-dashboard-dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-bindings-json""#));

let shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_registry_json =
    deck.run_app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_registry_json(
        BerkeleyAppPersistedEditorState {
            selected_syntax_card_index: Some(3),
            active_command_id: Some("analysis.3.inspect-waveform".to_string()),
        },
    )?;
assert!(shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_registry_json
    .contains(r#""activeMenuGroupShortcutCommandId":"dashboard.dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command.queued""#));
assert!(shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_registry_json
    .contains(r#""dispatchQueueLaneTabPanelCardActionMenuGroupShortcutCommandRegistryCapabilityId":"app-shell-dashboard-dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-registry-json""#));

let shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_json =
    deck.run_app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_json(
        BerkeleyAppPersistedEditorState {
            selected_syntax_card_index: Some(3),
            active_command_id: Some("analysis.3.inspect-waveform".to_string()),
        },
    )?;
assert!(shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_json
    .contains(r#""activeMenuGroupShortcutCommandPaletteItemId":"dashboard.dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-item.queued""#));
assert!(shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_json
    .contains(r#""dispatchQueueLaneTabPanelCardActionMenuGroupShortcutCommandPaletteCapabilityId":"app-shell-dashboard-dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-json""#));

let shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_index_json =
    deck.run_app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_index_json(
        BerkeleyAppPersistedEditorState {
            selected_syntax_card_index: Some(3),
            active_command_id: Some("analysis.3.inspect-waveform".to_string()),
        },
    )?;
assert!(shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_index_json
    .contains(r#""activeMenuGroupShortcutCommandSearchIndexEntryId":"dashboard.dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-index-entry.queued""#));
assert!(shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_index_json
    .contains(r#""dispatchQueueLaneTabPanelCardActionMenuGroupShortcutCommandPaletteSearchIndexCapabilityId":"app-shell-dashboard-dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-index-json""#));

let shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_results_json =
    deck.run_app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_results_json(
        BerkeleyAppPersistedEditorState {
            selected_syntax_card_index: Some(3),
            active_command_id: Some("analysis.3.inspect-waveform".to_string()),
        },
        "queued dispatch",
    )?;
assert!(shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_results_json
    .contains(r#""activeMenuGroupShortcutCommandSearchResultId":"dashboard.dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-result.queued""#));
assert!(shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_results_json
    .contains(r#""dispatchQueueLaneTabPanelCardActionMenuGroupShortcutCommandPaletteSearchResultsCapabilityId":"app-shell-dashboard-dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-results-json""#));

let shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_selection_json =
    deck.run_app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_selection_json(
        BerkeleyAppPersistedEditorState {
            selected_syntax_card_index: Some(3),
            active_command_id: Some("analysis.3.inspect-waveform".to_string()),
        },
        "queued dispatch",
        None,
    )?;
assert!(shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_selection_json
    .contains(r#""selectedCommandId":"berkeley.app-shell.dashboard.dispatch-queue.menu-group-shortcut.queued""#));
assert!(shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_selection_json
    .contains(r#""dispatchQueueLaneTabPanelCardActionMenuGroupShortcutCommandPaletteSearchSelectionCapabilityId":"app-shell-dashboard-dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-selection-json""#));

let shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_json =
    deck.run_app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_json(
        BerkeleyAppPersistedEditorState {
            selected_syntax_card_index: Some(3),
            active_command_id: Some("analysis.3.inspect-waveform".to_string()),
        },
        "queued dispatch",
        None,
    )?;
assert!(shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_json
    .contains(r#""invocationAction":"invoke-command""#));
assert!(shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_json
    .contains(r#""dispatchQueueLaneTabPanelCardActionMenuGroupShortcutCommandPaletteSearchInvocationCapabilityId":"app-shell-dashboard-dispatch-queue-lane-tab-panel-card-action-menu-group-shortcut-command-palette-search-invocation-json""#));
```

The facade preserves normalized logical cards, source spans, token names
aligned with `code/grammars/spice/berkeley.tokens`, stable syntax diagnostics,
analysis inventory, source-order execution through the existing parser, and
card-indexed app artifacts with stable result tables, output-plan artifacts,
run-artifact summaries, rawfile / wrdata artifact metadata from the engine deck
execution layer, and numeric waveform series derived from result tables for
Mosaic plot surfaces. It also exposes app session snapshots with deterministic
source fingerprints, selected-analysis state, run/blocked status, diagnostics,
table columns, output probes, and selected waveform availability so Mosaic UI
hosts can render editor state without owning simulator internals. It also
derives stable per-analysis editor controls for selecting, running, and
inspecting tables or waveforms with explicit disabled reasons, plus command
plans with stable command IDs and target names for host menu or button wiring.
Product shells can also consume compact shell event summaries, digests, and
dashboards with stable event-kind, severity, diagnostic, repaired-state,
capability, headline-event, attention-event, metric-event, and dashboard-section
fields without walking the full event stream, then join dashboard panel cards to
stable launch actions and action dispatch descriptors for first-render button
and menu wiring. Dashboard dispatch events turn those descriptors into compact
ready/blocked event rows so product shells can append dispatch telemetry without
interpreting panel-card-action internals, while dashboard dispatch queues map
those events into queued/blocked queue items with stable queue-item routing and
capability metadata. Dashboard dispatch queue summaries condense the same queue
into selected/default queue routing, first queued/blocked/attention item IDs,
queued/blocked/attention ID lists, counts, and summary capability metadata,
while dashboard dispatch queue digests pick a single headline queue item with
its message, target, state, dispatch joins, and digest capability metadata.
Dashboard dispatch queue lanes bucket the queue into stable queued, blocked,
and attention lanes with active-lane routing and lane capability metadata.
Dashboard dispatch queue lane tabs project those lanes into stable tab
descriptors with active/attention tab IDs, enabled and disabled tab counts, and
lane-tab capability metadata.
Dashboard dispatch queue lane tab panels project those tabs into stable panel
descriptors with active/attention panel IDs, enabled, disabled, and empty panel
counts, and lane-tab-panel capability metadata.
Dashboard dispatch queue lane tab panel cards project those panels into compact
card descriptors with active/attention card IDs, enabled, disabled, and empty
card counts, summaries, badge counts, and lane-tab-panel-card capability
metadata.
Dashboard dispatch queue lane tab panel card actions project those cards into
stable action descriptors with active/attention action IDs, labels, targets,
enabled, disabled, and empty action counts, disabled reasons, and
lane-tab-panel-card-action capability metadata.
Dashboard dispatch queue lane tab panel card action menus project those actions
into menu items with active/attention/default item IDs, enabled, disabled,
empty, primary, selected, and attention item counts, positions, action links,
and lane-tab-panel-card-action-menu capability metadata.
Dashboard dispatch queue lane tab panel card action menu groups bucket those
menu items by queue state with active/attention/default/primary group IDs,
item/action ID lists, enabled, disabled, empty, primary, selected, and
attention group counts, badge totals, and lane-tab-panel-card-action-menu-group
capability metadata.
Dashboard dispatch queue lane tab panel card action menu group shortcuts expose
host accelerator hints over those queue-state groups with stable shortcut IDs,
targets, active/attention/default/primary shortcut routing, accelerator labels,
disabled reasons, and lane-tab-panel-card-action-menu-group-shortcuts
capability metadata.
Shortcut command palette search indexes turn command-palette items into stable
searchable entries with normalized text, token lists, active/attention/default/
primary entry routing, and search-index capability metadata. Search results
filter those entries by normalized query tokens, preserving matched tokens,
active/attention/default/primary result routing, result counts, and empty-state
text for command-palette product shells.
Search invocation receipt notification stack summary product handoffs wrap the
latest stack summary with stable product surface, render region, handoff route,
product-shell action, live-region, announcement, badge, and nested
`stackSummary` metadata so WebAssembly and product shells can render
post-dispatch feedback without making a second summary call.
Product handoff delivery packages add a stable delivery package ID, package
kind, delivery route, WebAssembly export symbol, hydration target, top-level
notification counts, nested `productHandoff`, and delivery-package capability
metadata so Mosaic and product-shell hosts can bootstrap the post-dispatch
handoff without re-walking the command-palette receipt stack.
Delivery package embed manifests add a stable embed manifest ID, WebAssembly
module/import/export metadata, hydration mode, preload/instantiate/mount flags,
nested `deliveryPackage`, and embed-manifest capability metadata so
WebAssembly-backed product shells can decide when to load, instantiate, and
mount the post-dispatch handoff.
Embed loader plans add stable module-request metadata, loader phase/strategy,
module cache/integrity hints, deterministic load order, nested `embedManifest`,
and loader-plan capability metadata so WebAssembly-backed product shells can
preload, instantiate, mount, or defer the post-dispatch handoff without
recomputing the embed manifest.
Embed runtime plans add stable runtime host/bootstrap/mount/readiness metadata,
runtime phase/strategy, runtime entrypoints, hydration scheduler hints,
deterministic runtime steps, nested `embedLoaderPlan`, and runtime-plan
capability metadata so WebAssembly-backed product shells can start, publish, or
defer the post-dispatch handoff runtime without recomputing the loader plan.
Embed runtime session plans add stable session IDs, lifecycle state,
activation, ownership, publish-channel metadata, deterministic session steps,
nested `embedRuntimePlan`, and runtime-session-plan capability metadata so
WebAssembly-backed product shells can open, activate, publish, or defer a
post-dispatch runtime session without recomputing the runtime plan.
Embed runtime activation plans add stable activation requests, targets, gates,
channels, activation entrypoints, deterministic activation steps, nested
`embedRuntimeSessionPlan`, and runtime-activation-plan capability metadata so
WebAssembly-backed product shells can request, publish, or defer runtime
activation without recomputing the session plan.
Embed runtime activation receipts add stable receipt IDs, accepted/deferred
outcomes, receipt messages, deterministic receipt steps, nested
`embedRuntimeActivationPlan`, and activation-receipt capability metadata so
WebAssembly-backed product shells can record and replay runtime activation
handoffs without recomputing the activation plan.
Embed runtime activation receipt journals add stable journal and entry IDs,
committed/deferred outcomes, deterministic journal steps, nested
`embedRuntimeActivationReceipt`, and journal capability metadata so
WebAssembly-backed product shells can append and replay activation receipt
handoff records without recomputing receipt payloads.
Embed runtime activation receipt journal summaries add stable summary IDs,
latest-entry metadata, committed/deferred entry counts, deterministic summary
steps, nested `embedRuntimeActivationReceiptJournal`, and summary capability
metadata so WebAssembly-backed product shells can render compact activation
receipt journal status without re-walking journal entries.
Embed runtime activation receipt journal summary handoffs add stable handoff
IDs, publish/defer dispositions, handoff actions, deterministic handoff steps,
nested `embedRuntimeActivationReceiptJournalSummary`, and handoff capability
metadata so WebAssembly-backed product shells can render or defer compact
activation receipt journal summaries without recomputing summary payloads.
Embed runtime activation receipt journal summary handoff receipts add stable
receipt IDs, acknowledge/defer dispositions, receipt actions, deterministic
receipt steps, nested `embedRuntimeActivationReceiptJournalSummaryHandoff`, and
handoff-receipt capability metadata so WebAssembly-backed product shells can
acknowledge or defer compact activation receipt journal summary handoffs without
recomputing handoff payloads.
Embed runtime activation receipt journal summary handoff receipt
acknowledgements add stable acknowledgement IDs, acknowledge/defer
dispositions, acknowledgement actions, deterministic acknowledgement steps,
nested `embedRuntimeActivationReceiptJournalSummaryHandoffReceipt`, and
handoff-receipt-acknowledgement capability metadata so WebAssembly-backed
product shells can close out compact activation receipt journal summary handoff
receipts without re-walking receipt payloads.
Embed runtime activation receipt journal summary handoff receipt
acknowledgement records add stable record IDs, recorded/deferred dispositions,
record actions, deterministic record steps, nested
`embedRuntimeActivationReceiptJournalSummaryHandoffReceiptAcknowledgement`, and
handoff-receipt-acknowledgement-record capability metadata so WebAssembly-backed
product shells can replay compact activation receipt journal summary handoff
receipt acknowledgements without re-walking acknowledgement payloads.
Embed runtime activation receipt journal summary handoff receipt
acknowledgement record receipts add stable receipt IDs, acknowledge/defer
dispositions, receipt actions, deterministic receipt steps, nested
`embedRuntimeActivationReceiptJournalSummaryHandoffReceiptAcknowledgementRecord`,
and handoff-receipt-acknowledgement-record-receipt capability metadata so
WebAssembly-backed product shells can close out compact activation receipt
journal summary handoff receipt acknowledgement records without re-walking
record payloads.
Embed runtime activation receipt journal summary handoff receipt
acknowledgement record receipt acknowledgements add stable acknowledgement IDs,
acknowledge/defer dispositions, acknowledgement actions, deterministic
acknowledgement steps, nested
`embedRuntimeActivationReceiptJournalSummaryHandoffReceiptAcknowledgementRecordReceipt`,
and handoff-receipt-acknowledgement-record-receipt-acknowledgement capability
metadata so WebAssembly-backed product shells can close out compact activation
receipt journal summary handoff receipt acknowledgement record receipts without
re-walking receipt payloads.
Embed runtime activation receipt journal summary handoff receipt
acknowledgement record receipt acknowledgement records add stable record IDs,
record/defer dispositions, record actions, deterministic record steps, nested
`embedRuntimeActivationReceiptJournalSummaryHandoffReceiptAcknowledgementRecordReceiptAcknowledgement`,
and handoff-receipt-acknowledgement-record-receipt-acknowledgement-record
capability metadata so WebAssembly-backed product shells can record compact
activation receipt journal summary handoff receipt acknowledgement record
receipt acknowledgements without re-walking acknowledgement payloads.
Embed runtime activation receipt journal summary handoff receipt
acknowledgement record receipt acknowledgement record summaries add stable
summary IDs, summarize/defer dispositions, summary actions, deterministic
summary steps, nested
`embedRuntimeActivationReceiptJournalSummaryHandoffReceiptAcknowledgementRecordReceiptAcknowledgementRecord`,
and handoff-receipt-acknowledgement-record-receipt-acknowledgement-record-
summary capability metadata so WebAssembly-backed product shells can render
compact activation receipt journal summary handoff receipt acknowledgement
record receipt acknowledgement record status cards without re-walking record
payloads.
Embed runtime activation receipt journal summary handoff receipt
acknowledgement record receipt acknowledgement record summary digests add stable
digest IDs, route/hold dispositions, badge labels and tones, routing targets,
compact notification/count metadata, and summary-digest capability metadata so
WebAssembly-backed product shells can route acknowledgement status cards without
parsing the larger summary payload.
Persisted editor-state snapshots resolve saved selection and active-command IDs
against the current deck, repairing stale UI state after source edits. Host
surfaces turn those snapshots into stable source, diagnostics, analysis, table,
and waveform panel descriptors with explicit targets, enabled states, active
state, and disabled reasons for Mosaic shell integration. Host-surface wire
exports flatten the same contract into schema-versioned JSON with repaired
selection metadata and lower-case panel / diagnostic kinds for WebAssembly and
product-shell embedding. The static package manifest advertises the Berkeley
grammar version, host-surface wire schema, panel kinds, command targets,
runnable analysis directives, and artifact capabilities so Mosaic and
WebAssembly hosts can negotiate the app package before opening a deck. It is the
Rust app/runtime entrypoint for Mosaic-backed UI work. App bootstrap snapshots
combine that manifest with the schema-versioned host-surface wire export so
WebAssembly and product shells can load one startup payload with package
capabilities, repaired editor-state metadata, active panels, and diagnostics
before taking ownership of the UI. Startup summaries derive a compact ready /
blocked route from the same bootstrap payload, including source fingerprint,
active panel, repaired editor-state IDs, stale-state flags, diagnostic count,
and blocking reason. Launch plans derive product-shell entry actions from the
bootstrap payload, including a primary entry panel, target, route status, and
panel action descriptors so hosts can start on the right Mosaic surface without
walking the full host-surface export. Readiness reports summarize the same
startup path with panel/action availability counts, diagnostic severity counts,
repaired-state flags, and blocking reasons for product-shell telemetry and
readiness gates. Shell handoffs package the manifest, startup summary, launch
plan, and readiness report into one compact JSON envelope for WebAssembly and
product shells that do not need the full host-surface export during startup.
Shell statuses derive a compact route, severity, and status message from the
handoff so product shells can render startup chrome and telemetry without
inspecting every launch/readiness field. Shell telemetry adds compact startup
metrics for route, severity, entry action, panel/action availability,
diagnostic counts, repaired-state flags, and advertised capability count without
requiring hosts to parse the full shell handoff. Shell event logs turn the same
handoff into stable status, route, primary-action, diagnostic, repaired-state,
and capability events so Mosaic and product shells can append startup streams
without reinventing app-state traversal. Shell event digests condense those
logs into one headline event plus attention and metric event ID lists for
startup dashboards. Shell event dashboards group the same digest into status,
attention, and metrics sections for first-render Mosaic dashboards. Shell
dashboard packages combine the manifest and event dashboard into one compact
schema-versioned payload for WebAssembly and product hosts that want a
first-render dashboard without stitching package metadata to dashboard sections.
Shell dashboard cards derive stable card IDs, primary-card routing, attention
flags, severity, and event IDs from the dashboard package so product hosts can
render first-pass dashboard cards without interpreting section internals. Shell
dashboard views summarize those cards into primary-card labels, visible card
IDs, attention card IDs, metric card IDs, and capability metadata for hosts that
only need the first-render dashboard routing contract. Shell dashboard layouts
turn those card and view descriptors into stable status, attention, and metrics
regions with primary and visible flags for first-render host composition. Shell
dashboard navigation derives stable status, attention, and metrics
navigation items with active, visible, enabled, and badge-count metadata from
those layout regions. Shell dashboard routes derive stable route IDs, paths,
active/default route selection, and route capability metadata from navigation
items for product-shell router setup; the panel-card action, action-dispatch,
dispatch-event, dispatch-queue, and dispatch-queue summary surfaces then expose
button wiring plus ready/blocked dispatch telemetry, queue-state metadata, and
compact queue summary routing. The
grammar-backed parser generator and Python/TypeScript parity surfaces continue
to mature.

This parser supports `R`, `C`, `L`, `V`, `I`, `D`, `Q`, `M`, `G`, `E`, `F`, and
`H` elements, `.model <name> D(...)` diode cards with `IS` and `VT`
parameters, `.model <name> NPN|PNP(...)` BJT cards with `IS`, `BF` /
`BETA_F`, `VT`, `CJE`, `CJC`, `TF`, and `TR` parameters,
`.model <name> NMOS|PMOS(...)` Level-1 MOSFET
cards with common SPICE aliases (`VT0` / `VTO`, `KP`, `LAMBDA`, `GAMMA`, `PHI`,
`W`, `L`, `RSH`, `IS`, `N_SUB` / `NSUB`, `T_NOM` / `TNOM`, `CGSO`, `CGDO`,
`CGBO`, `CBS`, and `CBD`), MOS instance `NRD=<squares>`, SPICE engineering
suffixes, capacitor `IC=<voltage>` and inductor `IC=<current>` initial
conditions, independent-source `AC <magnitude> [phase]` forms,
PWL/PULSE/SIN/EXP source forms, comments, `.end`, `.subckt` / `X` instance
expansion, and `.op`, `.tran`, `.dc`, `.ac`, `.tf`, `.sens`, `.mc`, `.noise`,
`.temp`, `.print`, `.plot`, `.save`, `.probe`, `.measure`, `.four`, and
`.options` analysis cards.
Transient cards can carry `method=euler|trap|gear2`; when omitted,
`parsed.transient_method(None)?` falls back to `.options method=<...>` if
present.
Selected `.options` keys can also be turned into engine-call options with
`parsed.dc_op_options()?` and `parsed.adaptive_transient_options(None)?`.
Runnable `.op`, `.dc`, `.ac dec` / `.ac log`, and `.tran` cards can be planned
and executed directly with `parsed.analysis_plan()`, `parsed.run_analysis_plan()`,
or `run_netlist(deck)`.
`.save`, `.probe`, `.print`, and `.plot` cards can be applied to executed
analysis results with `parsed.select_outputs(&results)?`. Supported `.measure`
cards can be evaluated with `parsed.measure_results(&results)?`; the first
execution subset supports `FIND ... AT=<value>` plus `MAX`, `MIN`, `AVG`, and
`RMS` over optional `FROM=<value>` / `TO=<value>` ranges.
Deck-level `.temp` cards can be resolved into Kelvin with
`parsed.operating_temperature_kelvin(0, 300.0)?`, and
`parsed.noise_temperature_kelvin(Some(noise_card), 0, 300.0)?` applies the
SPICE precedence where an explicit `.noise temp=<kelvin>` overrides the deck
operating temperature.
