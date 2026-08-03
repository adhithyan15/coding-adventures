# Changelog

## Unreleased

- Reject negative and non-finite Level-1 MOS model-card `AF` values before
  lowering netlist elements into the engine.
- Reject negative and non-finite Level-1 MOS model-card `KF` values before
  lowering netlist elements into the engine.
- Reject negative and non-finite Level-1 MOS model-card `RSH` values before
  lowering netlist elements into the engine.
- Reject negative and non-finite Level-1 MOS model-card `RS` values before
  lowering netlist elements into the engine.
- Reject negative and non-finite Level-1 MOS model-card `RD` values before
  lowering netlist elements into the engine.
- Reject invalid Level-1 MOS model-card `LD` values that are negative,
  non-finite, or leave a non-positive effective channel length.
- Reject zero, negative, and non-finite Level-1 MOS model-card `L` values
  before lowering netlist elements into the engine.
- Reject zero, negative, and non-finite Level-1 MOS model-card `W` values
  before lowering netlist elements into the engine.
- Reject negative and non-finite Level-1 MOS model-card `JS` values before
  lowering netlist elements into the engine.
- Reject zero, negative, and non-finite Level-1 MOS model-card `IS` values
  before lowering netlist elements into the engine.
- Reject negative and non-finite Level-1 MOS model-card `CGBO` values before
  lowering netlist elements into the engine.
- Reject negative and non-finite Level-1 MOS model-card `CGDO` values before
  lowering netlist elements into the engine.
- Reject negative and non-finite Level-1 MOS model-card `CGSO` values before
  lowering netlist elements into the engine.
- Reject negative and non-finite Level-1 MOS model-card `CBD` / `CJD` values
  before lowering netlist elements into the engine.
- Reject negative and non-finite Level-1 MOS model-card `CBS` / `CJS` values
  before lowering netlist elements into the engine.
- Reject negative and non-finite Level-1 MOS model-card `CJSW` values before
  lowering netlist elements into the engine.
- Reject negative and non-finite Level-1 MOS model-card `CJ` values before
  lowering netlist elements into the engine.
- Reject negative and non-finite Level-1 MOS model-card `MJSW` values before
  lowering netlist elements into the engine.
- Reject non-finite Level-1 MOS model-card `FC` values outside `[0, 1)` before
  lowering netlist elements into the engine.
- Reject negative and non-finite Level-1 MOS model-card `MJ` values before
  lowering netlist elements into the engine.
- Reject zero, negative, and non-finite Level-1 MOS model-card `PB` values
  before lowering netlist elements into the engine.
- Reject negative and non-finite Level-1 MOS model-card `GAMMA` values before
  lowering netlist elements into the engine.
- Reject zero, negative, and non-finite Level-1 MOS model-card `PHI` values
  before lowering netlist elements into the engine.
- Reject non-finite Level-1 MOS model-card `LAMBDA` / `LAM` values before
  lowering netlist elements into the engine.
- Reject non-finite Level-1 MOS model-card `VT0` / `VTO` / `VTH` values before
  lowering netlist elements into the engine.
- Reject zero, negative, and non-finite Level-1 MOS model-card `KP` values
  before lowering netlist elements into the engine.
- Reject negative and non-finite Level-1 MOS model-card `U0` / `UO` values
  before lowering netlist elements into the engine.
- Reject zero, negative, and non-finite Level-1 MOS model-card `TOX` values
  before lowering netlist elements into the engine.
- Reject negative and non-finite Level-1 MOS instance `PS` values before
  lowering netlist elements into the engine.
- Reject negative and non-finite Level-1 MOS instance `PD` values before
  lowering netlist elements into the engine.
- Reject negative and non-finite Level-1 MOS instance `AS` values before
  lowering netlist elements into the engine.
- Reject negative and non-finite Level-1 MOS instance `AD` values before
  lowering netlist elements into the engine.
- Reject negative and non-finite Level-1 MOS instance `NRS` values before
  lowering netlist elements into the engine.
- Reject negative and non-finite Level-1 MOS instance `NRD` values before
  lowering netlist elements into the engine.
- Reject zero, negative, and non-finite Level-1 MOS instance `L` values before
  lowering netlist elements into the engine.
- Reject zero, negative, and non-finite Level-1 MOS instance `W` values before
  lowering netlist elements into the engine.
- Reject unsupported Level-1 MOS instance parameters instead of silently
  ignoring misspelled geometry or diffusion fields.
- Preserve Level-1 MOS model-card `U0` / `UO` and derive `KP` from surface
  mobility and explicit `TOX` when the card omits `KP`.
- Preserve Level-1 MOS model-card `JS` independently from scalar `IS` so
  Berkeley netlists can drive diffusion-area-scaled bulk-junction leakage.
- Preserve the remaining supported Level-1 MOS model-card fields `LD`, `TOX`,
  `RD`, `RS`, `KF`, and `AF`, plus `VTH`, `LAM`, `CJS`, and `CJD` aliases,
  when lowering Berkeley netlists into engine parameters.
- Preserve Level-1 MOS model-card `PB`, `MJ`, and `FC` values when lowering
  Berkeley netlists into the engine parameter bundle.
- Accept Level-1 MOS model-card `MJSW=<grading>` through the normalized engine
  model-card contract for independent sidewall depletion shaping.
- Parse Level-1 MOS instance `PS=<perimeter>` into the engine parameter bundle
  for source-body sidewall capacitance.
- Parse Level-1 MOS instance `PD=<perimeter>` and model-card
  `CJSW=<capacitance/length>` into the engine parameter bundle for drain-body
  sidewall capacitance.
- Parse Level-1 MOS instance `AS=<area>` into the engine parameter bundle for
  source-body junction capacitance.
- Parse Level-1 MOS instance `AD=<area>` and model-card `CJ=<capacitance/area>`
  into the engine parameter bundle for drain-body junction capacitance.
- Parse Level-1 MOS instance `NRS=<squares>` into the engine parameter bundle
  so netlist-driven source resistance uses `RSH * NRS`.
- Parse Level-1 MOS model `RSH` and instance `NRD=<squares>` into the engine
  parameter bundle so netlist-driven drain resistance uses `RSH * NRD`.
- Add Berkeley SPICE app-deck shell dashboard dispatch queue lane tab panel
  card action menu group shortcut command palette search invocation receipt
  notification stack summary product handoff delivery package embed runtime
  activation receipt journal summary handoff receipt acknowledgement record
  receipt acknowledgement record summary digests for Mosaic and WebAssembly
  product-shell routing. `BerkeleyAppDeck::app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_acknowledgement_record_summary_digest()`,
  `run_app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_acknowledgement_record_summary_digest()`,
  and their JSON helpers wrap acknowledgement-record summary status cards with
  stable digest IDs, route/hold dispositions, badge labels and tones, routing
  targets, compact notification/count metadata, and summary-digest capability
  metadata without nesting the larger summary payload.
- Add Berkeley SPICE app-deck shell dashboard dispatch queue lane tab panel
  card action menu group shortcut command palette search invocation receipt
  notification stack summary product handoff delivery package embed runtime
  activation receipt journal summary handoff receipt acknowledgement record
  receipt acknowledgement record summaries for Mosaic and WebAssembly runtime
  activation acknowledgement-record-receipt-acknowledgement-record closeout.
  `BerkeleyAppDeck::app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_acknowledgement_record_summary()`,
  `run_app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_acknowledgement_record_summary()`,
  and their JSON helpers wrap runtime activation receipt journal summary
  handoff receipt acknowledgement record receipt acknowledgement records with
  stable summary IDs, summarize/defer dispositions, summary actions,
  deterministic summary steps, nested
  `embedRuntimeActivationReceiptJournalSummaryHandoffReceiptAcknowledgementRecordReceiptAcknowledgementRecord`,
  and handoff-receipt-acknowledgement-record-receipt-acknowledgement-record-
  summary capability metadata for product-shell WebAssembly status cards.
- Add Berkeley SPICE app-deck shell dashboard dispatch queue lane tab panel
  card action menu group shortcut command palette search invocation receipt
  notification stack summary product handoff delivery package embed runtime
  activation receipt journal summary handoff receipt acknowledgement record
  receipt acknowledgement records for Mosaic and WebAssembly runtime
  activation acknowledgement-record-receipt-acknowledgement closeout.
  `BerkeleyAppDeck::app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_acknowledgement_record()`,
  `run_app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_acknowledgement_record()`,
  and their JSON helpers wrap runtime activation receipt journal summary
  handoff receipt acknowledgement record receipt acknowledgements with stable
  record IDs, record/defer dispositions, record actions, deterministic record
  steps, nested
  `embedRuntimeActivationReceiptJournalSummaryHandoffReceiptAcknowledgementRecordReceiptAcknowledgement`,
  and handoff-receipt-acknowledgement-record-receipt-acknowledgement-record
  capability metadata for product-shell WebAssembly bootstrapping.
- Add Berkeley SPICE app-deck shell dashboard dispatch queue lane tab panel
  card action menu group shortcut command palette search invocation receipt
  notification stack summary product handoff delivery package embed runtime
  activation receipt journal summary handoff receipt acknowledgement record
  receipt acknowledgements for Mosaic and WebAssembly runtime activation
  acknowledgement-record-receipt closeout.
  `BerkeleyAppDeck::app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_acknowledgement()`,
  `run_app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt_acknowledgement()`,
  and their JSON helpers wrap runtime activation receipt journal summary
  handoff receipt acknowledgement record receipts with stable acknowledgement
  IDs, acknowledge/defer dispositions, acknowledgement actions,
  deterministic acknowledgement steps, nested
  `embedRuntimeActivationReceiptJournalSummaryHandoffReceiptAcknowledgementRecordReceipt`,
  and handoff-receipt-acknowledgement-record-receipt-acknowledgement
  capability metadata for product-shell WebAssembly bootstrapping.
- Add Berkeley SPICE app-deck shell dashboard dispatch queue lane tab panel
  card action menu group shortcut command palette search invocation receipt
  notification stack summary product handoff delivery package embed runtime
  activation receipt journal summary handoff receipt acknowledgement record
  receipts for Mosaic and WebAssembly runtime activation acknowledgement-record
  closeout.
  `BerkeleyAppDeck::app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt()`,
  `run_app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record_receipt()`,
  and their JSON helpers wrap runtime activation receipt journal summary
  handoff receipt acknowledgement records with stable receipt IDs,
  acknowledge/defer dispositions, receipt actions, deterministic receipt steps,
  nested `embedRuntimeActivationReceiptJournalSummaryHandoffReceiptAcknowledgementRecord`,
  and handoff-receipt-acknowledgement-record-receipt capability metadata for
  product-shell WebAssembly bootstrapping.
- Add Berkeley SPICE app-deck shell dashboard dispatch queue lane tab panel
  card action menu group shortcut command palette search invocation receipt
  notification stack summary product handoff delivery package embed runtime
  activation receipt journal summary handoff receipt acknowledgement records
  for Mosaic and WebAssembly runtime activation acknowledgement replay.
  `BerkeleyAppDeck::app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record()`,
  `run_app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement_record()`,
  and their JSON helpers wrap runtime activation receipt journal summary
  handoff receipt acknowledgements with stable record IDs, recorded/deferred
  dispositions, record actions, deterministic record steps, nested
  `embedRuntimeActivationReceiptJournalSummaryHandoffReceiptAcknowledgement`,
  and handoff-receipt-acknowledgement-record capability metadata for
  product-shell WebAssembly bootstrapping.
- Add Berkeley SPICE app-deck shell dashboard dispatch queue lane tab panel
  card action menu group shortcut command palette search invocation receipt
  notification stack summary product handoff delivery package embed runtime
  activation receipt journal summary handoff receipt acknowledgements for
  Mosaic and WebAssembly runtime activation handoff receipt closeout.
  `BerkeleyAppDeck::app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement()`,
  `run_app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt_acknowledgement()`,
  and their JSON helpers wrap runtime activation receipt journal summary
  handoff receipts with stable acknowledgement IDs, acknowledge/defer
  dispositions, acknowledgement actions, deterministic acknowledgement steps,
  nested `embedRuntimeActivationReceiptJournalSummaryHandoffReceipt`, and
  handoff-receipt-acknowledgement capability metadata for product-shell
  WebAssembly bootstrapping.
- Add Berkeley SPICE app-deck shell dashboard dispatch queue lane tab panel
  card action menu group shortcut command palette search invocation receipt
  notification stack summary product handoff delivery package embed runtime
  activation receipt journal summary handoff receipts for Mosaic and
  WebAssembly runtime activation handoff acknowledgement.
  `BerkeleyAppDeck::app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt()`,
  `run_app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff_receipt()`,
  and their JSON helpers wrap runtime activation receipt journal summary
  handoffs with stable receipt IDs, acknowledge/defer dispositions, receipt
  actions, deterministic receipt steps, nested
  `embedRuntimeActivationReceiptJournalSummaryHandoff`, and
  journal-summary-handoff-receipt capability metadata for product-shell
  WebAssembly bootstrapping.
- Add Berkeley SPICE app-deck shell dashboard dispatch queue lane tab panel
  card action menu group shortcut command palette search invocation receipt
  notification stack summary product handoff delivery package embed runtime
  activation receipt journal summary handoffs for Mosaic and WebAssembly
  runtime activation handoff rendering.
  `BerkeleyAppDeck::app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff()`,
  `run_app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary_handoff()`,
  and their JSON helpers wrap runtime activation receipt journal summaries with
  stable handoff IDs, publish/defer dispositions, handoff actions,
  deterministic handoff steps, nested
  `embedRuntimeActivationReceiptJournalSummary`, and journal-summary-handoff
  capability metadata for product-shell WebAssembly bootstrapping.
- Add Berkeley SPICE app-deck shell dashboard dispatch queue lane tab panel
  card action menu group shortcut command palette search invocation receipt
  notification stack summary product handoff delivery package embed runtime
  activation receipt journal summaries for Mosaic and WebAssembly runtime
  activation handoff status.
  `BerkeleyAppDeck::app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary()`,
  `run_app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal_summary()`,
  and their JSON helpers wrap runtime activation receipt journals with stable
  summary IDs, latest-entry metadata, committed/deferred counts,
  deterministic summary steps, nested `embedRuntimeActivationReceiptJournal`,
  and journal-summary capability metadata for product-shell WebAssembly
  bootstrapping.
- Add Berkeley SPICE app-deck shell dashboard dispatch queue lane tab panel
  card action menu group shortcut command palette search invocation receipt
  notification stack summary product handoff delivery package embed runtime
  activation receipt journals for Mosaic and WebAssembly runtime activation
  handoff audit trails.
  `BerkeleyAppDeck::app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal()`,
  `run_app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt_journal()`,
  and their JSON helpers wrap runtime activation receipts with stable journal
  IDs, entry metadata, committed/deferred outcomes, deterministic journal
  steps, nested `embedRuntimeActivationReceipt`, and journal capability
  metadata for product-shell WebAssembly bootstrapping.
- Add Berkeley SPICE app-deck shell dashboard dispatch queue lane tab panel
  card action menu group shortcut command palette search invocation receipt
  notification stack summary product handoff delivery package embed runtime
  activation receipts for Mosaic and WebAssembly runtime activation handoff
  audit trails.
  `BerkeleyAppDeck::app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt()`,
  `run_app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_receipt()`,
  and their JSON helpers wrap runtime activation plans with stable receipt IDs,
  accepted/deferred outcomes, receipt messages, deterministic receipt steps,
  nested `embedRuntimeActivationPlan`, and activation-receipt capability
  metadata for product-shell WebAssembly bootstrapping.
- Add Berkeley SPICE app-deck shell dashboard dispatch queue lane tab panel
  card action menu group shortcut command palette search invocation receipt
  notification stack summary product handoff delivery package embed runtime
  activation plans for Mosaic and WebAssembly runtime activation handoff.
  `BerkeleyAppDeck::app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_plan()`,
  `run_app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification_stack_summary_product_handoff_delivery_package_embed_runtime_activation_plan()`,
  and their JSON helpers wrap runtime session plans with stable activation
  requests, targets, gates, channels, activation entrypoints, deterministic
  activation steps, nested `embedRuntimeSessionPlan`, and runtime-activation-
  plan capability metadata for product-shell WebAssembly bootstrapping.
- Add Berkeley SPICE app-deck shell dashboard dispatch queue lane tab panel
  card action menu group shortcut command palette search invocation receipt
  notification stack summary product handoff delivery package embed runtime
  session plans for Mosaic and WebAssembly runtime ownership.
  `BerkeleyAppDeck::app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification_stack_summary_product_handoff_delivery_package_embed_runtime_session_plan()`,
  `run_app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification_stack_summary_product_handoff_delivery_package_embed_runtime_session_plan()`,
  and their JSON helpers wrap runtime plans with stable runtime-session IDs,
  lifecycle state, activation, ownership, publish-channel metadata,
  deterministic session steps, nested `embedRuntimePlan`, and runtime-session-
  plan capability metadata for product-shell WebAssembly bootstrapping.
- Add Berkeley SPICE app-deck shell dashboard dispatch queue lane tab panel
  card action menu group shortcut command palette search invocation receipt
  notification stack summary product handoff delivery package embed runtime
  plans for Mosaic and WebAssembly first-render hosts.
  `BerkeleyAppDeck::app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification_stack_summary_product_handoff_delivery_package_embed_runtime_plan()`,
  `run_app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification_stack_summary_product_handoff_delivery_package_embed_runtime_plan()`,
  and their JSON helpers wrap embed loader plans with stable runtime-plan IDs,
  runtime host/bootstrap/mount/readiness metadata, runtime phase/strategy,
  runtime entrypoints, hydration scheduler hints, deterministic runtime steps,
  nested `embedLoaderPlan`, and runtime-plan capability metadata for product-
  shell WebAssembly bootstrapping.
- Add Berkeley SPICE app-deck shell dashboard dispatch queue lane tab panel
  card action menu group shortcut command palette search invocation receipt
  notification stack summary product handoff delivery package embed loader
  plans for Mosaic and WebAssembly first-render hosts.
  `BerkeleyAppDeck::app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification_stack_summary_product_handoff_delivery_package_embed_loader_plan()`,
  `run_app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification_stack_summary_product_handoff_delivery_package_embed_loader_plan()`,
  and their JSON helpers wrap embed manifests with stable loader-plan IDs,
  module-request metadata, loader phase/strategy, module cache and integrity
  hints, deterministic load order, nested `embedManifest`, and loader-plan
  capability metadata for product-shell WebAssembly bootstrapping.
- Add Berkeley SPICE app-deck shell dashboard dispatch queue lane tab panel
  card action menu group shortcut command palette search invocation receipt
  notification stack summary product handoff delivery package embed manifests
  for Mosaic and WebAssembly first-render hosts.
  `BerkeleyAppDeck::app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification_stack_summary_product_handoff_delivery_package_embed_manifest()`,
  `run_app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification_stack_summary_product_handoff_delivery_package_embed_manifest()`,
  and their JSON helpers wrap delivery packages with stable embed-manifest IDs,
  WebAssembly module/import/export metadata, hydration mode, preload/
  instantiate/mount flags, nested `deliveryPackage`, and embed-manifest
  capability metadata for product-shell WebAssembly bootstrapping.
- Add Berkeley SPICE app-deck shell dashboard dispatch queue lane tab panel
  card action menu group shortcut command palette search invocation receipt
  notification stack summary product handoff delivery packages for Mosaic and
  WebAssembly first-render hosts.
  `BerkeleyAppDeck::app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification_stack_summary_product_handoff_delivery_package()`,
  `run_app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification_stack_summary_product_handoff_delivery_package()`,
  and their JSON helpers wrap product handoffs with stable delivery-package
  IDs, package kind, delivery route, WebAssembly export symbol, hydration
  target, top-level notification counts, nested `productHandoff`, and delivery
  package capability metadata for product-shell handoff bootstrapping.
- Add Berkeley SPICE app-deck shell dashboard dispatch queue lane tab panel
  card action menu group shortcut command palette search invocation receipt
  notification stack summary product handoffs for Mosaic first-render hosts.
  `BerkeleyAppDeck::app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification_stack_summary_product_handoff()`,
  `run_app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation_receipt_notification_stack_summary_product_handoff()`,
  and their JSON helpers wrap receipt notification stack summaries with stable
  product surface, render region, handoff route, product-shell action,
  live-region, announcement, badge, and nested `stackSummary` metadata for
  WebAssembly and product-shell post-dispatch feedback.
- Add Berkeley SPICE app-deck shell dashboard dispatch queue lane tab panel
  card action menu group shortcut command palette search invocations for Mosaic
  first-render hosts.
  `BerkeleyAppDeck::app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation()`,
  `run_app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_invocation()`,
  and their JSON helpers package selected command-palette search results into
  deterministic invocation state, action, message, blocked reason, dispatch
  readiness, command, handler, and target metadata for product-shell command
  dispatch UIs.
- Add Berkeley SPICE app-deck shell dashboard dispatch queue lane tab panel
  card action menu group shortcut command palette search selections for Mosaic
  first-render hosts.
  `BerkeleyAppDeck::app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_selection()`,
  `run_app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_selection()`,
  and their JSON helpers resolve a requested, active, attention, default,
  primary, or first visible search result into selected command, handler,
  target, query-match, `canInvoke`, and blocked-reason metadata for
  product-shell command palette activation UIs.
- Add Berkeley SPICE app-deck shell dashboard dispatch queue lane tab panel
  card action menu group shortcut command palette search results for Mosaic
  first-render hosts.
  `BerkeleyAppDeck::app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_results()`,
  `run_app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_results()`,
  and their JSON helpers filter command-palette search-index entries by
  normalized query tokens with matched-token metadata, active/attention/default/
  primary result routing, result counts, stable empty-state text, and
  search-results capability metadata for product-shell command palette UIs.
- Add Berkeley SPICE app-deck shell dashboard dispatch queue lane tab panel
  card action menu group shortcut command palette search indexes for Mosaic
  first-render hosts.
  `BerkeleyAppDeck::app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_index()`,
  `run_app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette_search_index()`,
  and their JSON helpers project command-palette items into stable searchable
  entries with normalized search text, token lists, active/attention/default/
  primary search-entry routing, selectable/visible/enabled counts, disabled
  reasons, and search-index capability metadata for product-shell command
  palette search and filter UIs.
- Add Berkeley SPICE app-deck shell dashboard dispatch queue lane tab panel
  card action menu group shortcut command palettes for Mosaic first-render
  hosts.
  `BerkeleyAppDeck::app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette()`,
  `run_app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_palette()`,
  and their JSON helpers project shortcut command registries into stable
  command-palette items with command-entry IDs, search text, keywords, ranks,
  selectable/visible/enabled flags, active/attention/default/primary
  palette-item routing, disabled reasons, and command-palette capability
  metadata for product-shell command palette and search UIs.
- Add Berkeley SPICE app-deck shell dashboard dispatch queue lane tab panel
  card action menu group shortcut command registries for Mosaic first-render
  hosts.
  `BerkeleyAppDeck::app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_registry()`,
  `run_app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_command_registry()`,
  and their JSON helpers project shortcut bindings into stable command
  registry entries with registry IDs, handler IDs, command groups, invocation
  kinds, active/attention/default/primary command routing, visible/enabled/
  disabled command counts, disabled reasons, and command-registry capability
  metadata for product-shell command palettes and dispatch registries.
- Add Berkeley SPICE app-deck shell dashboard dispatch queue lane tab panel
  card action menu group shortcut bindings for Mosaic first-render hosts.
  `BerkeleyAppDeck::app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_bindings()`,
  `run_app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcut_bindings()`,
  and their JSON helpers project queue-state shortcuts into stable
  keyboard/command binding records with command IDs, scope and target-kind
  metadata, active/attention/default/primary binding routing, menu-group
  targets, disabled reasons, and shortcut-bindings capability metadata for
  product-shell command registries.
- Add Berkeley SPICE app-deck shell dashboard dispatch queue lane tab panel
  card action menu group shortcuts for Mosaic first-render hosts.
  `BerkeleyAppDeck::app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcuts()`,
  `run_app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_group_shortcuts()`,
  and their JSON helpers project queue-state menu groups into host shortcut
  descriptors with active/attention/default/primary shortcut routing,
  accelerator labels, menu-group targets, disabled reasons, and
  lane-tab-panel-card-action-menu-group-shortcuts capability metadata for
  product-shell command surfaces.
- Add Berkeley SPICE app-deck shell dashboard dispatch queue lane tab panel
  card action menu groups for Mosaic first-render hosts.
  `BerkeleyAppDeck::app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_groups()`,
  `run_app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu_groups()`,
  and their JSON helpers bucket dispatch queue lane-tab panel card action menu
  items into queue-state menu groups with active/attention/default/primary
  group routing, item/action ID lists, enabled/disabled/empty/primary/selected/
  attention group counts, disabled reasons, and
  lane-tab-panel-card-action-menu-groups capability metadata for product-shell
  menus.
- Add Berkeley SPICE app-deck shell dashboard dispatch queue lane tab panel
  card action menus for Mosaic first-render hosts.
  `BerkeleyAppDeck::app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu()`,
  `run_app_shell_dashboard_dispatch_queue_lane_tab_panel_card_action_menu()`,
  and their JSON helpers project dispatch queue lane-tab panel card actions
  into menu items with active/attention/default routing, positions,
  enabled/disabled/empty/primary/selected/attention item counts, action links,
  disabled reasons, and lane-tab-panel-card-action-menu capability metadata for
  product-shell menus.
- Add Berkeley SPICE app-deck shell dashboard dispatch queue lane tab panel
  card actions for Mosaic first-render hosts.
  `BerkeleyAppDeck::app_shell_dashboard_dispatch_queue_lane_tab_panel_card_actions()`,
  `run_app_shell_dashboard_dispatch_queue_lane_tab_panel_card_actions()`, and
  their JSON helpers project dispatch queue lane-tab panel cards into stable
  action descriptors with active/attention action routing, labels, targets,
  enabled/disabled/empty action counts, disabled reasons, and
  lane-tab-panel-card-actions capability metadata for product-shell navigation.
- Add Berkeley SPICE app-deck shell dashboard dispatch queue lane tab panel
  cards for Mosaic first-render hosts.
  `BerkeleyAppDeck::app_shell_dashboard_dispatch_queue_lane_tab_panel_cards()`,
  `run_app_shell_dashboard_dispatch_queue_lane_tab_panel_cards()`, and their
  JSON helpers project dispatch queue lane-tab panels into stable card
  descriptors with active/attention card routing, enabled/disabled/empty card
  counts, compact summaries, badge counts, and lane-tab-panel-cards capability
  metadata for product-shell navigation.
- Add Berkeley SPICE app-deck shell dashboard dispatch queue lane tab panels
  for Mosaic first-render hosts.
  `BerkeleyAppDeck::app_shell_dashboard_dispatch_queue_lane_tab_panels()`,
  `run_app_shell_dashboard_dispatch_queue_lane_tab_panels()`, and their JSON
  helpers project dispatch queue lane tabs into stable panel descriptors with
  active/attention panel routing, enabled/disabled/empty panel counts,
  empty-state messages, and lane-tab-panels capability metadata for
  product-shell navigation.
- Add Berkeley SPICE app-deck shell dashboard dispatch queue lane tabs for
  Mosaic first-render hosts.
  `BerkeleyAppDeck::app_shell_dashboard_dispatch_queue_lane_tabs()`,
  `run_app_shell_dashboard_dispatch_queue_lane_tabs()`, and their JSON helpers
  project dispatch queue lanes into stable tab descriptors with active-tab
  routing, attention-tab routing, enabled/disabled tab counts, lane links, and
  lane-tabs capability metadata for product-shell navigation.
- Add Berkeley SPICE app-deck shell dashboard dispatch queue lanes for Mosaic
  first-render hosts.
  `BerkeleyAppDeck::app_shell_dashboard_dispatch_queue_lanes()`,
  `run_app_shell_dashboard_dispatch_queue_lanes()`, and their JSON helpers
  bucket dashboard dispatch queues into stable queued, blocked, and attention
  lanes with active-lane routing, lane item IDs, headline queue metadata, and
  lanes capability metadata for product-shell dispatch telemetry.
- Add Berkeley SPICE app-deck shell dashboard dispatch queue digests for Mosaic
  first-render hosts.
  `BerkeleyAppDeck::app_shell_dashboard_dispatch_queue_digest()`,
  `run_app_shell_dashboard_dispatch_queue_digest()`, and their JSON helpers
  derive a compact headline queue item with queue state, message, target,
  dispatch/action joins, first queue item routing, counts, and digest
  capability metadata from dashboard dispatch queues.
- Add Berkeley SPICE app-deck shell dashboard dispatch queue summaries for
  Mosaic first-render hosts.
  `BerkeleyAppDeck::app_shell_dashboard_dispatch_queue_summary()`,
  `run_app_shell_dashboard_dispatch_queue_summary()`, and their JSON helpers
  derive compact selected/default queue routing, first queued/blocked/attention
  queue item IDs, queue item ID lists, counts, and summary capability metadata
  from dashboard dispatch queues.
- Add Berkeley SPICE app-deck shell dashboard dispatch queues for Mosaic
  first-render hosts. `BerkeleyAppDeck::app_shell_dashboard_dispatch_queue()`,
  `run_app_shell_dashboard_dispatch_queue()`, and their JSON helpers derive
  stable queue item IDs, selected/default queue routing, queued/blocked state,
  dispatch queue messages, event/action joins, and queue capability metadata
  from dashboard dispatch events.
- Add Berkeley SPICE app-deck shell dashboard dispatch events for Mosaic
  first-render hosts. `BerkeleyAppDeck::app_shell_dashboard_dispatch_events()`,
  `run_app_shell_dashboard_dispatch_events()`, and their JSON helpers derive
  stable ready/blocked dispatch event rows, selected/default event routing, and
  dispatch event capability metadata from dashboard action dispatches.
- Add Berkeley SPICE app-deck shell dashboard action dispatch for Mosaic
  first-render hosts. `BerkeleyAppDeck::app_shell_dashboard_action_dispatch()`,
  `run_app_shell_dashboard_action_dispatch()`, and their JSON helpers derive
  stable action dispatch IDs, selected/default dispatch routing, dispatchable
  state, disabled reasons, and action-dispatch capability metadata from
  dashboard panel-card actions.
- Add Berkeley SPICE app-deck shell dashboard panel-card actions for Mosaic
  first-render hosts. `BerkeleyAppDeck::app_shell_dashboard_panel_card_actions()`,
  `run_app_shell_dashboard_panel_card_actions()`, and their JSON helpers join
  dashboard panel cards to launch actions with stable selected/default
  panel-card action IDs, selected/default action IDs, labels, targets, enabled
  state, disabled reasons, and panel-card action capability metadata.
- Add Berkeley SPICE app-deck shell dashboard panel cards for Mosaic
  first-render hosts. `BerkeleyAppDeck::app_shell_dashboard_panel_cards()`,
  `run_app_shell_dashboard_panel_cards()`, and their JSON helpers derive stable
  selected/default panel-card IDs, selected/default card IDs, panel/card joins,
  event metadata, counts, and panel-card capability metadata from dashboard tab
  panels and cards.
- Add Berkeley SPICE app-deck shell dashboard tab panels for Mosaic
  first-render hosts. `BerkeleyAppDeck::app_shell_dashboard_tab_panels()`,
  `run_app_shell_dashboard_tab_panels()`, and their JSON helpers derive stable
  selected/default render-panel IDs, tab/breadcrumb/route/item/region mapping,
  counts, and tab-panel capability metadata from dashboard tabs.
- Add Berkeley SPICE app-deck shell dashboard tabs for Mosaic first-render
  hosts. `BerkeleyAppDeck::app_shell_dashboard_tabs()`,
  `run_app_shell_dashboard_tabs()`, and their JSON helpers derive stable tab
  IDs, selected/default tab routing, breadcrumb/route/item/region mapping,
  counts, and tab capability metadata from dashboard breadcrumbs.
- Add Berkeley SPICE app-deck shell dashboard breadcrumbs for Mosaic
  first-render hosts. `BerkeleyAppDeck::app_shell_dashboard_breadcrumbs()`,
  `run_app_shell_dashboard_breadcrumbs()`, and their JSON helpers derive stable
  breadcrumb IDs, positions, active/default breadcrumb selection,
  route/item/region mapping, counts, and breadcrumb capability metadata from
  dashboard routes.
- Add Berkeley SPICE app-deck shell dashboard routes for Mosaic first-render
  hosts. `BerkeleyAppDeck::app_shell_dashboard_routes()`,
  `run_app_shell_dashboard_routes()`, and their JSON helpers derive stable
  route IDs, paths, active/default route selection, item/region/card mapping,
  route counts, and route capability metadata from dashboard navigation.
- Add Berkeley SPICE app-deck shell dashboard navigation for Mosaic
  first-render hosts. `BerkeleyAppDeck::app_shell_dashboard_navigation()`,
  `run_app_shell_dashboard_navigation()`, and their JSON helpers derive stable
  status, attention, and metrics navigation items, active-item routing,
  enabled/visible counts, badge counts, and navigation capability metadata from
  dashboard layouts.
- Add Berkeley SPICE app-deck shell dashboard layouts for Mosaic first-render
  hosts. `BerkeleyAppDeck::app_shell_dashboard_layout()`,
  `run_app_shell_dashboard_layout()`, and their JSON helpers derive stable
  status, attention, and metrics regions, primary-region routing, visible-region
  counts, and layout capability metadata from dashboard cards and views.
- Add Berkeley SPICE app-deck shell dashboard views for Mosaic first-render
  hosts. `BerkeleyAppDeck::app_shell_dashboard_view()`,
  `run_app_shell_dashboard_view()`, and their JSON helpers summarize dashboard
  cards into primary-card labels, visible card IDs, attention card IDs, metric
  card IDs, and view capability metadata.
- Add Berkeley SPICE app-deck shell dashboard cards for Mosaic first-render
  hosts. `BerkeleyAppDeck::app_shell_dashboard_cards()`,
  `run_app_shell_dashboard_cards()`, and their JSON helpers derive stable card
  IDs, primary-card routing, attention flags, severities, and event IDs from the
  shell dashboard package.
- Add Berkeley SPICE app-deck shell dashboard packages for Mosaic WebAssembly
  and product hosts. `BerkeleyAppDeck::app_shell_dashboard_package()`,
  `run_app_shell_dashboard_package()`, and their JSON helpers combine the
  package manifest and first-render event dashboard into one schema-versioned
  payload.
- Add Berkeley SPICE app-deck shell event dashboards for Mosaic first-render
  startup panels. `BerkeleyAppDeck::app_shell_event_dashboard()`,
  `run_app_shell_event_dashboard()`, and their JSON helpers group event digests
  into stable status, attention, and metrics sections.
- Add Berkeley SPICE app-deck shell event digests for Mosaic startup
  dashboards. `BerkeleyAppDeck::app_shell_event_digest()`,
  `run_app_shell_event_digest()`, and their JSON helpers derive a headline
  event, attention event IDs, metric event IDs, and compact counts from shell
  event logs.
- Add Berkeley SPICE app-deck shell event summaries for Mosaic startup
  dashboards and gates. `BerkeleyAppDeck::app_shell_event_summary()`,
  `run_app_shell_event_summary()`, and their JSON helpers derive compact
  event-kind, severity, diagnostic, repaired-state, and capability counts from
  shell event logs.
- Add Berkeley SPICE app-deck shell event logs for Mosaic product-shell startup
  streams. `BerkeleyAppDeck::app_shell_event_log()`,
  `run_app_shell_event_log()`, and their JSON helpers derive stable status,
  route, primary-action, diagnostic, repaired-state, and capability events from
  shell handoffs.
- Add Berkeley SPICE app-deck shell telemetry for Mosaic startup metrics.
  `BerkeleyAppDeck::app_shell_telemetry()`, `run_app_shell_telemetry()`, and
  their JSON helpers derive compact route, entry-action, availability,
  diagnostic, repaired-state, and capability counts from the shell handoff.
- Add Berkeley SPICE app-deck shell statuses for Mosaic startup chrome and
  telemetry. `BerkeleyAppDeck::app_shell_status()`,
  `run_app_shell_status()`, and their JSON helpers derive a compact route,
  severity, message, entry action, and diagnostic counts from the shell handoff.
- Add Berkeley SPICE app-deck shell handoffs for Mosaic WebAssembly and
  product-shell startup. `BerkeleyAppDeck::app_shell_handoff()`,
  `run_app_shell_handoff()`, and their JSON helpers combine the package
  manifest, startup summary, launch plan, and readiness report into one compact
  startup envelope.
- Add Berkeley SPICE app-deck readiness reports for Mosaic product-shell
  telemetry and startup gates. `BerkeleyAppDeck::app_readiness_report()`,
  `run_app_readiness_report()`, and their JSON helpers summarize launch route,
  panel/action availability, diagnostic severity counts, repaired state, and
  blocking reasons from bootstrap snapshots.
- Add Berkeley SPICE app-deck launch plans for Mosaic product-shell startup
  routing. `BerkeleyAppDeck::app_launch_plan()`, `run_app_launch_plan()`, and
  their JSON helpers derive ready/blocked entry panels, route targets, and panel
  action descriptors from bootstrap snapshots.
- Add Berkeley SPICE app-deck persisted editor-state snapshots for Mosaic host
  restoration. `BerkeleyAppDeck::editor_state_snapshot()` and
  `run_editor_state_snapshot()` now resolve saved selected-card and
  active-command IDs against the current deck, including stale-state repair
  flags.
- Add Berkeley SPICE app-deck editor command plans for Mosaic host wiring.
  `BerkeleyAppDeck::editor_command_plan()` and `run_editor_command_plan()` now
  expose stable per-analysis command IDs, action kinds, targets, enabled states,
  and disabled reasons derived from editor controls.
- Add Berkeley SPICE app-deck editor controls for Mosaic-facing Rust UI
  substrates. `BerkeleyAppDeck::editor_controls()` and `run_editor_controls()`
  now expose stable per-analysis select/run/table/waveform actions, enabled
  states, and disabled reasons derived from the app session state.
- Add Berkeley SPICE app-deck session snapshots for Mosaic-facing Rust UI
  substrates. `BerkeleyAppDeck::session_state()` and `run_session_state()` now
  expose deterministic source fingerprints, selected-analysis state,
  run/blocked status, diagnostics, table columns, output probes, and selected
  waveform availability without requiring UI hosts to own simulator internals.
- Add Berkeley SPICE app-deck waveform inspection series for Mosaic-facing
  Rust UI substrates. Card-indexed analysis artifacts now expose numeric
  plot-ready series derived from stable result tables, including selected-card
  waveform access and probe-grouped AC magnitude/phase series.
- Add Berkeley SPICE app-deck result artifacts for Mosaic-facing Rust UI
  substrates. `BerkeleyAppDeck::run_artifacts()` now exposes normalized source,
  syntax-card-indexed result tables, output-plan artifacts, run-artifact
  summaries, and rawfile / wrdata artifact metadata backed by the engine deck
  execution layer.
- Route `parse_netlist` through the Berkeley SPICE logical-card syntax facade,
  so the default Rust parser consumes normalized cards, supports leading `+`
  continuations, and reports stable syntax diagnostics before semantic
  lowering.
- Add a Berkeley SPICE logical-card syntax facade for Rust/Mosaic app
  substrates. The new surface exposes grammar metadata, normalized logical
  cards, leading `+` continuation handling, source spans, grammar-token names,
  stable syntax diagnostics, analysis inventory, and an app-deck wrapper that
  can run source-order or selected runnable analyses through the existing
  parser.
- Parse `.save`, scoped or global `.probe`, and `.measure` / `.meas` cards,
  and expose `select_outputs()` / `measure_results()` helpers plus matching
  `ParsedNetlist` methods for analysis-plan results.
- Add a deck execution layer with `build_analysis_plan()`, `run_analysis_plan()`,
  `run_netlist()`, plus matching `ParsedNetlist` methods for runnable `.op`,
  `.dc`, `.ac dec` / `.ac log`, and `.tran` cards.

## 0.3.0 — 2026-06-05

- Resolve `.temp` cards into Kelvin engine-call temperatures and let explicit
  `.noise temp=<kelvin>` overrides win over deck-level operating temperatures.
- Route selected `.options` keys into engine-call helpers:
  `dc_op_options()` for DC Newton options and `adaptive_transient_options()`
  for adaptive transient options.
- Parse SPICE `.four <frequency> <V(node)|I(source)>...` Fourier-analysis
  cards.
- Parse SPICE `.print <analysis> <V(node)|I(source)>...` and
  `.plot <analysis> <V(node)|I(source)>...` output cards.
- Parse SPICE `.temp <celsius> [celsius ...]` operating-temperature cards.
- Parse MOS Level-1 capacitance parameters with `.model ... NMOS|PMOS(... CGSO=<c>
  CGDO=<c> CGBO=<c> CBS=<c> CBD=<c>)`.
- Parse diode model-card emission coefficients with `.model ... D(... N=<n>)`
  and pass them into Rust `Diode` elements.
- Parse diode model-card reverse-breakdown parameters with
  `.model ... D(... BV=<v> IBV=<i>)`.
- Parse diode model-card junction capacitance with
  `.model ... D(... CJO=<c>)` / `.model ... D(... CJ0=<c>)`.
- Parse diode model-card transit time with `.model ... D(... TT=<time>)`.
- Parse BJT model-card capacitances with `.model ... NPN|PNP(... CJE=<c>
  CJC=<c>)` and pass them into Rust `Bjt` elements.
- Parse BJT model-card forward transit time with
  `.model ... NPN|PNP(... TF=<time>)`.
- Parse BJT model-card reverse transit time with
  `.model ... NPN|PNP(... TR=<time>)`.
- Parse and validate transient integration methods from
  `.tran ... method=<euler|trap|gear2>`, and expose fallback routing from
  `.options method=<...>`.
- Parse conservative SPICE `T` transmission-line cards of the form
  `Tname n1 n2 n3 n4 Z0=<ohms> TD=<seconds>`, including subcircuit node
  remapping and validation for unsupported, missing, non-finite, and
  non-positive parameters.
- Reject SPICE `K` mutual-inductor cards that reference missing inductors or
  use non-finite coupling coefficients.
- Parse SPICE `K` mutual-inductor cards into `MutualInductor` elements,
  including subcircuit-local inductor reference remapping.
- Parse SPICE `J` JFET elements via `.model <name> NJF(...)` and
  `.model <name> PJF(...)` cards with `BETA` / `B`, `VTO`, and `LAMBDA`
  parameters, including subcircuit drain/gate/source remapping.
- Parse capacitor `IC=<voltage>` initial-voltage parameters.
- Parse inductor `IC=<current>` initial-current parameters.
- Parse SPICE `.tf V(output_node) input_source` transfer-function analysis
  cards.
- Parse SPICE `.sens V(output_node)` DC sensitivity analysis cards.
- Parse SPICE `.mc V(output_node) n_trials [tolerance] [distribution] [seed]`
  Monte Carlo DC analysis cards.
- Parse SPICE `.noise V(output_node) input_source [freq ...] [temp=<kelvin>]`
  AC noise analysis cards.
- Parse SPICE `.options key=value ...` simulator-options cards.

## 0.1.7

- Add independent-source `AC <magnitude> [phase]` parsing, including combined
  `DC <bias> AC <magnitude> [phase]` forms for AC analysis with separate DC
  bias and small-signal excitation.

## 0.1.6

- Add SPICE `M` MOSFET element parsing via `.model <name> NMOS|PMOS(...)`
  Level-1 cards, per-instance parameter overrides such as `W=...` and `L=...`,
  and subcircuit drain/gate/source/body terminal remapping.

## 0.1.5

- Add SPICE `Q` BJT element parsing via `.model <name> NPN|PNP(...)` cards
  with `IS`, `BF` / `BETA_F`, and `VT` parameters, including subcircuit
  terminal remapping.

## 0.1.4

- Add SPICE `D` diode element parsing via `.model <name> D(...)` cards with
  `IS` and `VT` parameters, including subcircuit terminal remapping.

## 0.1.3

- Add SPICE `H` / CCVS controlled-source parsing, including subcircuit
  controlling-source name remapping for expanded CCVS elements.

## 0.1.2

- Add SPICE `F` / CCCS controlled-source parsing, including subcircuit
  controlling-source name remapping for expanded CCCS elements.

## 0.1.1

- Add SPICE `E` / VCVS controlled-source parsing, including subcircuit node
  remapping for expanded VCVS elements.

## 0.1.0

- Add a first SPICE3 netlist parser slice for linear R/C/L circuits,
  independent V/I sources, VCCS elements, PWL/PULSE/SIN/EXP source waveforms,
  and `.op`, `.tran`, `.dc`, and `.ac` analysis cards.
- Add first `.subckt` / `X` instance expansion for hierarchical netlists made
  from supported primitive elements.
