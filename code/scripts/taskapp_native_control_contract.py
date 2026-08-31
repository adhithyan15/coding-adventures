#!/usr/bin/env python3
"""Validate that generated native TaskApp controls remain wired to Rust events.

The live conformance programs prove the host boundary, scheduling projections,
error atomicity, and persistence. This source contract closes the remaining
gap for backends where hosted-runner accessibility APIs are not dependable: it
requires the emitted input/button controls, event payloads, row projections,
and host-to-view update subscription to remain connected.
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path


CONTRACTS: dict[str, dict[str, tuple[str, ...]]] = {
    "qt": {
        "TaskApp.qml": (
            'objectName: "name-input"',
            "Component.onCompleted: forceActiveFocus()",
            "onTextChanged: newTaskNameChange(text)",
            "onAccepted: addTask()",
            'objectName: "due-input"',
            "onTextChanged: newTaskDueChange(text)",
            'objectName: "add-btn"',
            "onClicked: addTask()",
            'objectName: "complexity-toggle"',
            "onClicked: toggleProjectComplexity()",
            'objectName: "toggle"',
            "onClicked: toggleTask(i)",
            'objectName: "del-btn"',
            "onClicked: deleteTask(i)",
            "text: ( row [ 2 ] )",
            "text: ( row [ 3 ] )",
            "onMosaicEvent: function(event) { applyMosaicResponse(mosaicHost.handleRequiredEvent(event)) }",
            'text: "Your Inbox is ready"',
        ),
    },
    "flutter": {
        "lib/TaskApp.dart": (
            "autofocus: true,",
            "onChanged: (value) => dispatch(TaskAppEventNewTaskNameChange(value: value))",
            "onChanged: (value) => dispatch(TaskAppEventNewTaskDueChange(value: value))",
            "onPressed: () => dispatch(TaskAppEventAddTask())",
            "onPressed: () => dispatch(TaskAppEventToggleProjectComplexity())",
            "onPressed: () => dispatch(TaskAppEventToggleTask(index: i))",
            "onPressed: () => dispatch(TaskAppEventDeleteTask(index: i))",
            "Text(( row [ 2 ] ))",
            "Text(( row [ 3 ] ))",
            'Text("Your Inbox is ready")',
        ),
        "lib/main.dart": (
            "_mosaicHost.setPropsChangedHandler",
            "setState(() {",
            "_hostProps = nextProps",
            "_mosaicHost.handleEvent(event.mosaicEnvelope)",
        ),
    },
    "compose": {
        "src/main/kotlin/TaskApp.kt": (
            "private fun _MosaicAutoFocus(content: @Composable (Modifier) -> Unit)",
            "_MosaicAutoFocus { _mosaicAutoFocusModifier ->",
            '.testTag("name-input")',
            "onValueChange = { v -> dispatch(TaskAppEvent.NewTaskNameChange(v)) }",
            '.testTag("due-input")',
            "onValueChange = { v -> dispatch(TaskAppEvent.NewTaskDueChange(v)) }",
            '.testTag("add-btn")',
            "onClick = { dispatch(TaskAppEvent.AddTask) }",
            '.testTag("complexity-toggle")',
            "onClick = { dispatch(TaskAppEvent.ToggleProjectComplexity) }",
            '.testTag("toggle")',
            "onClick = { dispatch(TaskAppEvent.ToggleTask(i)) }",
            '.testTag("del-btn")',
            "onClick = { dispatch(TaskAppEvent.DeleteTask(i)) }",
            "Text(( row [ 2 ] )",
            "Text(( row [ 3 ] )",
            'Text("Your Inbox is ready"',
        ),
        "src/main/kotlin/Main.kt": (
            "mosaicHost.setPropsChangedHandler",
            "hostProps = nextProps",
            "mosaicHost.handleEvent(event.mosaicEnvelope)",
        ),
    },
    "swiftui": {
        "Sources/App/TaskApp.swift": (
            "_MosaicFocusState(autoFocus: true, content: _mosaicFocusContent)",
            '.accessibilityIdentifier("name-input")',
            "dispatch(.newTaskNameChange(value: $0))",
            '.accessibilityIdentifier("due-input")',
            "dispatch(.newTaskDueChange(value: $0))",
            '.accessibilityIdentifier("add-btn")',
            "dispatch(.addTask)",
            '.accessibilityIdentifier("complexity-toggle")',
            "dispatch(.toggleProjectComplexity)",
            '.accessibilityIdentifier("toggle")',
            "dispatch(.toggleTask(index: i))",
            '.accessibilityIdentifier("del-btn")',
            "dispatch(.deleteTask(index: i))",
            "_mosaicText(( row [ 2 ] ))",
            "_mosaicText(( row [ 3 ] ))",
            'Text("Your Inbox is ready")',
        ),
        "Sources/App/App.swift": (
            "bridge.setPropsChangedHandler?",
            "self?.refreshProps()",
            "applyHostResponse(bridge.handleEvent(event.mosaicEnvelope as NSDictionary, name: event.mosaicName as NSString)",
            "applyHostResponse(bridge.applyProps()",
        ),
    },
    "xaml": {
        "TaskApp.xaml": (
            'AutomationProperties.AutomationId="name-input"',
            'Loaded="NameInput_Loaded"',
            "TextChanged=\"NameInput_TextChanged\"",
            'AutomationProperties.AutomationId="due-input"',
            "TextChanged=\"DueInput_TextChanged\"",
            'AutomationProperties.AutomationId="add-btn"',
            "Click=\"AddBtn_Click\"",
            'AutomationProperties.AutomationId="complexity-toggle"',
            "Click=\"ComplexityToggle_Click\"",
            'AutomationProperties.AutomationId="toggle"',
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
            "?.Tag is TaskApp_Row2Vm row",
        ),
    },
}


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
