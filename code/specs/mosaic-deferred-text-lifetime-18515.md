# Deferred XAML text binding lifetime

The opt-in deferred TaskApp Board transition unloads an editor. WinUI clears
its generated binding reference, then invokes a retained LostFocus callback
that reads that null reference. The captured stack identifies
UpdateTwoWay_170_Text writing NewTaskName, not UnloadObject itself.

For experimental deferred emission, slot-backed HostInput Text bindings will
request UpdateSourceTrigger=PropertyChanged. This updates the owner while the
editor is live and avoids the generated LostFocus source-update listener.
The setting applies consistently to slot-backed inputs in that experimental
component, including template inputs. Ordinary eager output remains unchanged.
Expression-backed OneWay inputs and authored event wiring remain unchanged.
The experiment remains default-off. This changes source-update timing for
opt-in callers and must be documented explicitly.

Validate default/experimental output, the reproduced full-package Board crash,
task lifecycle and persisted restart, plus repeated view switching and editing.
Do not claim general IME, caret or accessibility acceptance from smoke alone.
Never patch generated C# or suppress exceptions as the repair.

## Local validation (2026-11-03)

The full package compiler generated TaskApp from this repair with the deferred
option enabled for the probe; production defaults were restored to false.
Release x64 built successfully. The DPI-aware native smoke passed at 144 DPI:
wide/narrow/wide pane changes, Board/List transition, task creation, scheduling,
completion, reopen, deletion, and persisted restart. The generated listener for
NewTaskName uses TextProperty registration instead of LostFocus.

The probe used the retained mosaic_app.dll from the earlier eager artifact;
this validates the emitted host repair, not a newly packaged runtime or release.
The full emitter suite passed (397 unit tests plus integration/doc tests).
Repeated editing across views, IME/caret behavior, and general accessibility
remain acceptance work; the smoke result does not establish those properties.
