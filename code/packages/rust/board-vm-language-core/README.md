# board-vm-language-core

`board-vm-language-core` is the Rust-owned boundary for high-level Board VM
frontends such as Ruby, Python, Lua, Java, and future REPLs.

Language packages should treat themselves as syntax sugar over this crate. The
binary protocol details stay in Rust:

- BVM module construction for common programs such as onboard LED blink
- request ids and request frame construction
- COBS stream framing and CRC handling
- program upload/run frame construction
- raw response frame decoding and payload offset reporting
- target upload plans for Arduino CLI, ESP ROM serial, and Pico UF2 adapters

This crate exports a normal Rust API for the repo's language bridge packages.
Ruby bindings should be built on `ruby-bridge`, Python bindings on
`python-bridge`, and other runtimes should follow the same pattern with their
own bridge crates. Those bridge packages provide the language-native surface;
this crate provides the shared Board VM bytes underneath them.

The small C ABI-friendly surface is intentionally secondary. It is useful for
tools, experiments, or runtimes that do not yet have a first-class bridge crate,
but it should not become a parallel protocol implementation path for Ruby,
Python, or any other supported language.

The crate deliberately does not open serial ports or USB devices yet. Language
frontends may own ergonomic transport discovery in the short term, but the bytes
they send and decode should come from this Rust core.

Firmware flashing follows the same rule: frontends can present native options,
but they should query Rust-owned upload helpers to learn the artifact kind,
transport requirements, reset behavior, port hint, adapter steps, and Arduino
CLI platform/FQBN metadata for each board family. Generic clients can use
`upload_plan_for_target`; Arduino CLI-specific clients can use
`arduino_cli_upload_options_for_target` so native USB, USB-serial bridge, and
external-adapter port selection stay in Rust instead of language-local tables.
`arduino_cli_port_discovery_for_target` adds the matching discovery/reset
metadata, including the 1200-baud native USB bootloader touch convention and
runtime port rediscovery expectation for boards whose Arduino package owns that
reset path. `arduino_cli_upload_invocation_for_target` keeps the final
`arduino-cli upload` flag template Rust-owned too, including the FQBN, port,
input-file, input-directory, upload-property, and verify flags language
frontends should fill with concrete paths.
`arduino_cli_upload_command_for_target` and
`arduino_cli_upload_command_with_options_for_target` go one step further by
building concrete `arduino-cli upload` argv from the selected port, firmware
image, optional upload properties, and verify flag.
`arduino_cli_upload_execution_plan_for_target` layers reset and rediscovery
metadata onto that concrete argv, so language frontends can execute the command
while still asking Rust whether the selected port is native USB, a USB serial
bridge, or an external adapter and whether runtime rediscovery is expected.
`arduino_cli_upload_process_for_target` and
`arduino_cli_upload_process_for_execution_plan` turn that plan into a typed
process-launch contract with executable, argv, stdio capture modes, success
exit codes, and rediscovery hints, leaving only the OS spawn call in the
frontend adapter.
`arduino_cli_upload_result_for_target` and
`arduino_cli_upload_result_for_execution_plan` normalize the `arduino-cli`
process exit code plus stdout/stderr into success, failure kind, retry, port
selection, board-package install, firmware-artifact, and rediscovery hints so
language frontends do not parse CLI diagnostics themselves.
`arduino_cli_upload_result_for_process_output` performs the same classification
directly from the process-launch contract after the frontend captures
stdout/stderr.
`arduino_cli_upload_runtime_handoff_for_execution_plan` and
`arduino_cli_upload_runtime_handoff_for_process_output` derive the Board VM
transport port to open after a successful upload. Native USB boards prefer the
`New upload port:` value reported by Arduino CLI and otherwise carry the
rediscovery requirement with the selected upload port; USB serial bridge and
external-adapter targets reuse the selected port.
`serial_runtime_open_plan_for_target` and
`serial_runtime_open_plan_from_upload_handoff` describe the runtime serial
transport open contract for language adapters: baud rate, timeout, stale-byte
clearing, open-settle delay, endpoint scheme, and Board VM wire protocol. The
actual OS serial open still belongs to the frontend adapter or CLI.
`parse_serial_endpoint`, `parse_tcp_endpoint`, and `parse_bluetooth_endpoint`
keep endpoint scheme and transport metadata in the same Rust-owned boundary so
frontends do not need parallel endpoint tables. `parse_host_endpoint` provides
the transport-classification summary over all supported endpoint forms, letting
CLI consumers and language adapters route serial, TCP, BLE GATT, and RFCOMM
without duplicating scheme dispatch. `parse_host_endpoint_with_error` returns
the same summary with a Rust-owned parse error kind for malformed endpoint
input, so frontends can present native messages without inventing their own
endpoint failure taxonomy. `host_endpoint_connection_label` also keeps the
cross-transport display policy in Rust: serial endpoints carry the runtime baud
rate in host logs, while TCP and Bluetooth endpoints use the endpoint string
alone. `host_endpoint_session_summary` packages the parsed endpoint metadata
with that connection label so CLI consumers and language adapters can open a
session without recombining transport and display policy.
`input_callback_plan_for_target` and
`input_callback_plan_with_options_for_target` provide the same Rust-owned
boundary for interrupt-backed input callbacks. The default button plan uses a
falling edge, pull-up, debounce window, bounded event queue, and cooperative
event-queue dispatch model, while the planner validates target input,
interrupt, pull-mode, queue, and callback budget support before frontends wire
callbacks to buttons or other input events.
`input_callback_event_for_plan` and `input_callback_invocation_for_event`
package the runtime delivery side of that contract: digital input events carry
the board, pin, trigger, level, sequence, and timestamp, and invocation summaries
bind those events back to the callback program, instruction budget, debounce
window, queue policy, interrupt backing, and cooperative dispatch model. That
lets adapters pass button events through without owning their own callback
payload schema.
`input_callback_queue_plan_for_invocation` keeps the next cooperative
event-queue decision in Rust too. Frontends still own queue storage and runtime
scheduling, but Rust decides whether a callback invocation is enqueued, drops
the incoming event, drops the oldest queued event first, and whether cooperative
dispatch should be woken.
`input_callback_session_queue_summary` wraps those enqueue/drop decisions with
the endpoint session, queue action label, depth change, and Rust-owned message
so adapters can present queue admission without rebuilding queue policy text.
`input_callback_dispatch_plan_for_queue_plan` then turns an admitted queue item
into the callback execution handoff: callback program id, instruction budget,
event identity, queue action, and cooperative dispatch reason stay in Rust,
while language adapters only schedule the native callback runner.
`input_callback_session_dispatch_summary` adds the endpoint session, handoff
label, queue action, instruction budget, and dispatch message for adapters that
need to log or display the callback runner handoff.
`input_callback_result_for_dispatch_plan` closes that loop by preserving the
dispatch metadata while normalizing the callback runner status, executed
instruction count, and elapsed time into a Rust-owned completion,
budget-exceeded, incomplete, or failure summary.
`input_callback_transport_result_summary` combines that callback completion
record with the parsed host endpoint session and Rust-owned connection label,
so transport-aware callback logs do not have to rebuild serial, TCP, or
Bluetooth display policy in language adapters.
`input_callback_completion_plan_for_result` maps that result into the
cooperative event-queue follow-up: remove completed callbacks, keep incomplete
callbacks scheduled, and drop budget-exhausted or failed callbacks with a
Rust-owned action and queue-depth update.
`input_callback_session_completion_summary` packages the endpoint session,
callback result label, completion action, and queue follow-up into one
transport-aware record for language adapters to log or present without
rebuilding callback lifecycle policy.
`input_callback_session_lifecycle_summary` stitches queue admission, optional
dispatch, optional runner result, and completion follow-up into one
endpoint-aware Rust-owned record, so adapters can render the callback lifecycle
without reimplementing queue, dispatch, or completion branching.
`input_callback_transport_action_summary` reduces that lifecycle into a
transport-facing action name, label, queue depth, terminal/retry flags, and
message, so adapters can route callback dispatch, completion, retry, and drop
handling without duplicating lifecycle branching.
`input_callback_transport_effect_summary` expands the action into concrete
transport effects: whether to dispatch a callback, emit a drop notice, emit a
result, remove a queue item, keep cooperative dispatch scheduled, and what the
queue depth is after the effect.
`input_callback_transport_report_summary` turns those effects into stable
transport report kinds, names, labels, queue-depth metadata, and messages, so
adapters can log or emit callback dispatch, drop, completion, running,
budget-exhausted, and failure reports without owning that branching.
`input_callback_transport_event_summary` maps those reports into adapter-facing
event names, labels, queue-depth metadata, and messages, so language frontends
can emit dispatch, dropped-callback, completed-callback, running-callback,
budget-exceeded, and failed-callback events from Rust-owned transport policy.
`input_callback_transport_delivery_summary` turns those events into a
Rust-owned delivery route, publication flag, queue-depth metadata, and message,
so adapters know whether to hand the callback to the runner or publish a native
event without carrying transport delivery logic.
`input_callback_plan_diagnostic`, `input_callback_event_diagnostic`, and
`input_callback_queue_plan_diagnostic` turn those planner, event, and queue
errors into stable Rust-owned kind names, labels, and messages, so frontends
can report callback lifecycle failures without carrying a parallel diagnostic
taxonomy.
`input_callback_session_plan_diagnostic`,
`input_callback_session_event_diagnostic`, and
`input_callback_session_queue_plan_diagnostic` add endpoint-session context to
those diagnostics, preserving serial/TCP/Bluetooth display policy in Rust for
transport-aware callback failure logs.

Frontends that already have an Arduino CLI platform or FQBN should pass it back
to Rust rather than carrying their own selector table. `detect_target` resolves
unique FQBNs to a concrete board target, and `targets_for_upload_selector`
returns every target for broader or intentionally shared package identities.
