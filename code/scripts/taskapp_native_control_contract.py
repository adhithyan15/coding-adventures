#!/usr/bin/env python3
"""Validate that generated native TaskApp controls remain wired to Rust events.

The live conformance programs prove the host boundary, scheduling projections,
error atomicity, persistence, and recoverable startup. This source contract closes the remaining
gap for backends where hosted-runner accessibility APIs are not dependable: it
requires the emitted input/button controls, event payloads, row projections,
and host-to-view update subscription to remain connected.
"""

from __future__ import annotations

import argparse
import json
from collections import Counter
from pathlib import Path

# Issue #16182: native-complete cannot reject every style drop yet without
# making all five shipping TaskApp gates fail.  Ratchet the measured debt
# instead: an emitter fix may reduce these counts, but a new property or an
# increase fails the same shared contract every native lane already runs.
STYLE_DROP_BASELINES: dict[str, dict[str, int]] = {
    "xaml": {
        "border": 1,
        "border-left-style": 1,
        "border-style": 4,
        "border-top-left-radius": 1,
        "border-top-right-radius": 1,
        "box-shadow": 1,
        "box-sizing": 1,
        "cursor": 2,
        "display": 1,
        "flex-direction": 1,
        "flex-shrink": 9,
        "flex-wrap": 1,
        "font": 1,
        "overflow": 1,
        "text-overflow": 1,
        "transform": 2,
        "white-space": 1,
        "width": 8,
    },
    "swiftui": {
        "border-left-style": 1,
        "border-style": 4,
        "border-top-left-radius": 1,
        "border-top-right-radius": 1,
        "box-shadow": 11,
        "box-sizing": 1,
        "cursor": 2,
        "display": 1,
        "elevation": 10,
        "flex-direction": 1,
        # Compact TaskApp scroll/name wrappers need flexible sizing in Flutter.
        # SwiftUI currently preserves the authored structure but reports those
        # two occurrences as explicit style debt (#16949).
        "flex-grow": 8,
        "flex-shrink": 9,
        "flex-wrap": 1,
        "font": 1,
        "gap": 4,
        "left": 6,
        "margin-bottom": 1,
        "margin-left": 1,
        "min-width": 2,
        "overflow": 1,
        "position": 6,
        "text-overflow": 1,
        "top": 6,
        "transform": 2,
        "white-space": 1,
    },
    "compose": {
        "border": 1,
        "border-left-style": 1,
        "border-top-left-radius": 1,
        "border-top-right-radius": 1,
        "box-shadow": 11,
        "box-sizing": 1,
        "cursor": 2,
        "display": 1,
        "flex-direction": 1,
        "flex-shrink": 2,
        "font": 1,
        "left": 6,
        "margin-bottom": 1,
        "margin-left": 1,
        "min-width": 2,
        "overflow": 1,
        "position": 6,
        "text-overflow": 1,
        "top": 6,
        "transform": 2,
        "white-space": 1,
    },
    "qt": {
        # Recording Qt's existing elevation lowering makes TaskApp's two
        # HostDraggable board-card parts visible to the reporter. Their 22
        # pre-existing container-style omissions are tracked by #17128.
        "background": 2,
        "border-bottom-color": 3,
        "border-bottom-width": 3,
        "border-color": 3,
        "border-left-color": 4,
        "border-left-style": 4,
        "border-left-width": 4,
        "border-radius": 2,
        "border-right-color": 6,
        "border-right-style": 3,
        "border-right-width": 6,
        "border-style": 4,
        "border-top-color": 3,
        "border-top-left-radius": 1,
        "border-top-right-radius": 1,
        "border-top-style": 1,
        "border-top-width": 3,
        "border-width": 3,
        "box-shadow": 11,
        "box-sizing": 1,
        "color": 2,
        "cursor": 2,
        # The compact task identity uses flexible name sizing in Flutter. Qt
        # keeps the portable row structure and records that task-name flex
        # occurrence until the emitter grows an equivalent lowering (#16949).
        "flex-grow": 6,
        "flex-shrink": 9,
        "flex-wrap": 1,
        "font": 1,
        "font-size": 2,
        "gap": 2,
        "left": 6,
        "margin-bottom": 1,
        "margin-left": 1,
        "min-height": 1,
        "outline": 2,
        "padding": 2,
        "position": 6,
        "text-align": 6,
        "top": 6,
        "transform": 2,
        "width": 18,
    },
    "flutter": {
        "background": 10,
        "border-color": 1,
        "border-left-color": 2,
        "border-left-style": 3,
        "border-left-width": 2,
        "border-radius": 15,
        "border-style": 4,
        "border-top-left-radius": 1,
        "border-top-right-radius": 1,
        "border-width": 1,
        "box-shadow": 11,
        "box-sizing": 1,
        "color": 3,
        "cursor": 2,
        "display": 1,
        "flex-direction": 1,
        "flex-shrink": 9,
        "flex-wrap": 1,
        "font-family": 1,
        "gap": 4,
        "height": 3,
        "left": 6,
        "margin-bottom": 1,
        "margin-left": 1,
        "min-height": 3,
        "min-width": 2,
        "overflow": 1,
        "position": 6,
        "text-align": 14,
        "text-overflow": 1,
        "top": 6,
        "transform": 2,
        "white-space": 1,
        "width": 23,
    },
}


CONTRACTS: dict[str, dict[str, tuple[str, ...]]] = {
    "qt": {
        "TaskApp.qml": (
            'objectName: "name-input"',
            'Accessible.name: "Task name"',
            "Component.onCompleted: forceActiveFocus()",
            "onTextChanged: newTaskNameChange(text)",
            "onAccepted: addTask()",
            'objectName: "name-input-error"',
            'objectName: "name-input-corrected"',
            'objectName: "due-input-error"',
            'objectName: "due-input-corrected"',
            "text: newTaskNameError",
            "text: newTaskDueError",
            'objectName: "due-input"',
            "onTextChanged: newTaskDueChange(text)",
            'objectName: "add-btn"',
            "onClicked: addTask()",
            'objectName: "complexity-toggle"',
            "onClicked: toggleProjectComplexity()",
            'objectName: "toggle"',
            "Accessible.name: (( row [ 16 ] )) || text",
            "onClicked: toggleTask(i)",
            'objectName: "del-btn"',
            "onClicked: deleteTask(i)",
            "onTextChanged: editTaskNameChange(text)",
            "onTextChanged: editTaskDueChange(text)",
            "onClicked: editTask(i)",
            "onClicked: saveTaskEdit()",
            "onClicked: cancelTaskEdit()",
            "text: ( row [ 2 ] )",
            "text: ( row [ 3 ] )",
            "onMosaicEvent: function(event) { applyMosaicResponse(mosaicHost.handleRequiredEvent(event)) }",
            'text: "Your Inbox is ready"',
        ),
        "main.cpp": (
            'objectName: "mosaic-startup-root"',
            'objectName: "mosaic-startup-loading"',
            'objectName: "mosaic-startup-failure"',
            'text: "TaskApp could not start"',
            'text: "Your saved data has not been changed. Retrying is safe."',
            'objectName: "mosaic-startup-retry"',
            'text: "Try again"',
            "QTimer::singleShot(50, &view",
            "auto candidate = std::make_unique<MosaicHost>();",
            "activeHost.reset();",
        ),
    },
    "flutter": {
        "lib/TaskApp.dart": (
            "autofocus: true,",
            'label: "Task name"',
            "onChanged: (value) => dispatch(TaskAppEventNewTaskNameChange(value: value))",
            "onChanged: (value) => dispatch(TaskAppEventNewTaskDueChange(value: value))",
            "Text(newTaskNameError, ",
            "Text(newTaskDueError, ",
            "_mosaicTruthy(newTaskNameFocus)",
            "_mosaicTruthy(newTaskDueFocus)",
            "onPressed: () => dispatch(TaskAppEventAddTask())",
            "onPressed: () => dispatch(TaskAppEventToggleProjectComplexity())",
            "Semantics(label: ((( row [ 16 ] )).isEmpty ? (( row [ 0 ] )) : (( row [ 16 ] ))), button: true, enabled:",
            "onPressed: () => dispatch(TaskAppEventToggleTask(index: i))",
            "onPressed: () => dispatch(TaskAppEventDeleteTask(index: i))",
            "onChanged: (value) => dispatch(TaskAppEventEditTaskNameChange(value: value))",
            "onChanged: (value) => dispatch(TaskAppEventEditTaskDueChange(value: value))",
            "onPressed: () => dispatch(TaskAppEventEditTask(index: i))",
            "onPressed: () => dispatch(TaskAppEventSaveTaskEdit())",
            "onPressed: () => dispatch(TaskAppEventCancelTaskEdit())",
            "Text(( row [ 2 ] ))",
            "Text(( row [ 3 ] ), ",
            'Text("Your Inbox is ready", ',
        ),
        "lib/main.dart": (
            "host.setPropsChangedHandler",
            "final host = providedHost ?? widget.mosaicHostLoader()",
            "on Object catch (error)",
            "setState(() {",
            "_hostProps = nextProps",
            "_mosaicHost!.handleEvent(event.mosaicEnvelope)",
            "key: const Key('mosaic-startup-loading')",
            "key: const Key('mosaic-startup-failure')",
            "TaskApp could not start",
            "Your saved tasks have not been changed. Retrying is safe.",
            "key: const Key('mosaic-startup-retry')",
            "onPressed: _retryStartup",
            "previousHost?.dispose()",
        ),
    },
    "compose": {
        "src/main/kotlin/TaskApp.kt": (
            "private fun _MosaicAutoFocus(content: @Composable (Modifier) -> Unit)",
            "_MosaicAutoFocus { _mosaicAutoFocusModifier ->",
            '.testTag("name-input")',
            'contentDescription = "Task name"',
            "onValueChange = { v -> dispatch(TaskAppEvent.NewTaskNameChange(v)) }",
            '.testTag("name-input-error")',
            '.testTag("name-input-corrected")',
            '.testTag("due-input-error")',
            '.testTag("due-input-corrected")',
            "Text(newTaskNameError, ",
            "Text(newTaskDueError, ",
            '.testTag("due-input")',
            "onValueChange = { v -> dispatch(TaskAppEvent.NewTaskDueChange(v)) }",
            '.testTag("add-btn")',
            "onClick = { dispatch(TaskAppEvent.AddTask) }",
            '.testTag("complexity-toggle")',
            "onClick = { dispatch(TaskAppEvent.ToggleProjectComplexity) }",
            '.testTag("toggle")',
            "(( row [ 16 ] )).toString().takeIf { it.isNotEmpty() }?.let { contentDescription = it }",
            "onClick = { dispatch(TaskAppEvent.ToggleTask(i)) }",
            '.testTag("del-btn")',
            "onClick = { dispatch(TaskAppEvent.DeleteTask(i)) }",
            "onValueChange = { v -> dispatch(TaskAppEvent.EditTaskNameChange(v)) }",
            "onValueChange = { v -> dispatch(TaskAppEvent.EditTaskDueChange(v)) }",
            "onClick = { dispatch(TaskAppEvent.EditTask(i)) }",
            "onClick = { dispatch(TaskAppEvent.SaveTaskEdit) }",
            "onClick = { dispatch(TaskAppEvent.CancelTaskEdit) }",
            "Text(( row [ 2 ] ), ",
            "Text(( row [ 3 ] ), ",
            'Text("Your Inbox is ready", ',
        ),
        # The shared app shell (UI89 §3.4): the startup UI and the host wiring
        # that desktop and Android both use.
        "src/main/kotlin/MosaicAppShell.kt": (
            "fun MosaicStartup(",
            'testTag("mosaic-startup-loading")',
            'Text("Starting TaskApp…")',
            'testTag("mosaic-startup-failure")',
            'Text("TaskApp could not start")',
            'Text("Your saved tasks have not been changed. Retrying is safe.")',
            'Button(onClick = { attempt += 1 }) { Text("Try again") }',
            "withContext(Dispatchers.IO)",
            "mosaicHost.setPropsChangedHandler",
            "hostProps = nextProps",
            "mosaicHost.handleEvent(event.mosaicEnvelope)",
        ),
    },
    "swiftui": {
        "Sources/App/TaskApp.swift": (
            "_MosaicFocusState(autoFocus: true, content: _mosaicFocusContent)",
            '.accessibilityIdentifier("name-input")',
            '.accessibilityLabel(Text(verbatim: "Task name"))',
            "dispatch(.newTaskNameChange(value: $0))",
            '.accessibilityIdentifier("name-input-error")',
            '.accessibilityIdentifier("name-input-corrected")',
            '.accessibilityIdentifier("due-input-error")',
            '.accessibilityIdentifier("due-input-corrected")',
            "_mosaicText(newTaskNameError)",
            "_mosaicText(newTaskDueError)",
            '.accessibilityIdentifier("due-input")',
            "dispatch(.newTaskDueChange(value: $0))",
            '.accessibilityIdentifier("add-btn")',
            "dispatch(.addTask)",
            '.accessibilityIdentifier("complexity-toggle")',
            "dispatch(.toggleProjectComplexity)",
            '.accessibilityIdentifier("toggle")',
            ".accessibilityLabel(_mosaicA11yName(( row [ 16 ] ), fallback: ( row [ 0 ] )))",
            "dispatch(.toggleTask(index: i))",
            '.accessibilityIdentifier("del-btn")',
            "dispatch(.deleteTask(index: i))",
            "dispatch(.editTaskNameChange(value: $0))",
            "dispatch(.editTaskDueChange(value: $0))",
            "dispatch(.editTask(index: i))",
            "dispatch(.saveTaskEdit)",
            "dispatch(.cancelTaskEdit)",
            "_mosaicText(( row [ 2 ] ))",
            "_mosaicText(( row [ 3 ] ))",
            'Text("Your Inbox is ready")',
        ),
        "Sources/App/App.swift": (
            "MosaicStartupView(host: host)",
            "MosaicRuntimeHost.loadRecoverable",
            "candidate.setPropsChangedHandler",
            "self?.refreshProps()",
            'applyHostResponse(bridge?.handleEvent(["payload": event.mosaicPayload] as NSDictionary, name: event.mosaicName as NSString)',
            "applyHostResponse(bridge?.applyProps()",
            '.accessibilityIdentifier("mosaic-startup-loading")',
            'Text("TaskApp could not start")',
            '.accessibilityIdentifier("mosaic-startup-failure")',
            'Text("Your saved tasks have not been changed. Retrying is safe.")',
            'Button("Try again") { host.retryStartup() }',
            '.accessibilityIdentifier("mosaic-startup-retry")',
            "candidate.close()",
            "bridge?.close?()",
        ),
    },
    "xaml": {
        "MainWindow.xaml": (
            'AutomationProperties.AutomationId="mosaic-startup-loading"',
            'AutomationProperties.AutomationId="mosaic-startup-failure"',
            'Text="TaskApp could not start"',
            'Text="Your saved tasks have not been changed. Retrying is safe."',
            'AutomationProperties.AutomationId="mosaic-startup-retry"',
        ),
        "MainWindow.xaml.cs": (
            "DispatcherQueue.TryEnqueue(StartRuntime)",
            "RetryStartup_Click",
            "MosaicRuntimeHost.Close()",
            "ShowStartupFailure",
        ),
        "TaskApp.xaml": (
            'AutomationProperties.AutomationId="name-input"',
            'AutomationProperties.Name="Task name"',
            'Loaded="NameInput_Loaded"',
            "TextChanged=\"NameInput_TextChanged\"",
            'AutomationProperties.AutomationId="name-input-error"',
            'AutomationProperties.AutomationId="name-input-corrected"',
            'AutomationProperties.AutomationId="due-input-error"',
            'AutomationProperties.AutomationId="due-input-corrected"',
            'Text="{x:Bind NewTaskNameError, Mode=OneWay}"',
            'Text="{x:Bind NewTaskDueError, Mode=OneWay}"',
            'AutomationProperties.AutomationId="due-input"',
            "TextChanged=\"DueInput_TextChanged\"",
            'AutomationProperties.AutomationId="add-btn"',
            "Click=\"AddBtn_Click\"",
            'AutomationProperties.AutomationId="complexity-toggle"',
            "Click=\"ComplexityToggle_Click\"",
            'AutomationProperties.AutomationId="toggle"',
            'AutomationProperties.AutomationId="toggle" AutomationProperties.Name="{x:Bind Expr_',
            'Tag="{x:Bind}"',
            "Click=\"Toggle_Click\"",
            'AutomationProperties.AutomationId="del-btn"',
            "Click=\"DelBtn_Click\"",
            "Mode=OneWay",
            'Text="Your Inbox is ready"',
        ),
        "TaskApp.xaml.cs": (
            "FocusManager.GetFocusedElement(tb.XamlRoot) is null",
            "new TaskAppEvent.NewTaskNameChange(tb.Text)",
            "new TaskAppEvent.NewTaskDueChange(tb.Text)",
            "new TaskAppEvent.AddTask()",
            "new TaskAppEvent.ToggleProjectComplexity()",
            "new TaskAppEvent.ToggleTask(",
            "new TaskAppEvent.DeleteTask(",
            "new TaskAppEvent.EditTaskNameChange(",
            "new TaskAppEvent.EditTaskDueChange(",
            "new TaskAppEvent.EditTask(",
            "new TaskAppEvent.SaveTaskEdit()",
            "new TaskAppEvent.CancelTaskEdit()",
            "?.Tag is TaskApp_Row2Vm row",
        ),
    },
}


def validate_style_degradations(
    backend: str, report: object, report_path: Path | str
) -> list[str]:
    """Reject style-drop growth while allowing existing debt to shrink."""

    errors: list[str] = []
    label = str(report_path)
    if not isinstance(report, dict):
        return [f"{label}: degradation report is not an object"]

    entries = report.get("styleDegradations")
    if not isinstance(entries, list):
        return [f"{label}: styleDegradations is missing or is not an array"]

    counts: Counter[str] = Counter()
    for index, entry in enumerate(entries):
        entry_label = f"{label}: styleDegradations[{index}]"
        if not isinstance(entry, dict):
            errors.append(f"{entry_label} is not an object")
            continue
        if entry.get("code") != "style.property-dropped":
            errors.append(f"{entry_label} has an invalid code")
        if entry.get("backend") != backend:
            errors.append(
                f"{entry_label} has backend {entry.get('backend')!r}, expected {backend!r}"
            )
        for field in ("component", "layoutPath", "reason"):
            if not isinstance(entry.get(field), str) or not entry[field]:
                errors.append(f"{entry_label}.{field} must be a non-empty string")
        primitive = entry.get("primitive")
        if not isinstance(primitive, str) or not primitive:
            errors.append(f"{entry_label}.primitive must be a non-empty string")
            continue
        counts[primitive] += 1

    baseline = STYLE_DROP_BASELINES[backend]
    for primitive, count in sorted(counts.items()):
        maximum = baseline.get(primitive)
        if maximum is None:
            errors.append(
                f"{label}: {backend} introduced unbaselined style drop {primitive!r} "
                f"({count} occurrence(s))"
            )
        elif count > maximum:
            errors.append(
                f"{label}: {backend} style drop {primitive!r} increased "
                f"from at most {maximum} to {count} occurrence(s)"
            )
    return errors


def validate(backend: str, generated_dir: Path) -> list[str]:
    errors: list[str] = []
    report_path = generated_dir / "mosaic-degradations.json"
    try:
        report = json.loads(report_path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        errors.append(f"{report_path}: cannot read degradation report: {error}")
    else:
        if report.get("nativeComplete") is not True:
            errors.append(f"{report_path}: nativeComplete is not true")
        if report.get("degradations") != []:
            errors.append(f"{report_path}: degradations are not empty")
        errors.extend(validate_style_degradations(backend, report, report_path))

    for relative_path, markers in CONTRACTS[backend].items():
        path = generated_dir / relative_path
        try:
            source = path.read_text(encoding="utf-8")
        except OSError as error:
            errors.append(f"{path}: cannot read generated source: {error}")
            continue
        for marker in markers:
            if marker not in source:
                errors.append(f"{path}: missing control-contract marker {marker!r}")
    return errors


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--backend", required=True, choices=sorted(CONTRACTS))
    parser.add_argument("--generated-dir", required=True, type=Path)
    args = parser.parse_args()
    errors = validate(args.backend, args.generated_dir)
    if errors:
        for error in errors:
            print(error)
        return 1
    print(f"TaskApp {args.backend} emitted-control contract passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
