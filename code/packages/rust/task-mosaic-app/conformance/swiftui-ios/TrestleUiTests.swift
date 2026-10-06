import XCTest

// Trestle on iPadOS, driven through its generated controls (UI89 §4.1, §4.4).
//
// The simulator gate (§2.3) proves the app starts, persists and restores
// without being touched. This proves a person can use it: a task typed into
// the composer reaches the Rust scheduler, its completion toggle and Delete
// button act on it, and a saved task is still there after the app is
// terminated and launched again. It is the desktop TaskAppUiTest
// (conformance/compose/TaskAppUiTest.kt) on an iPad, and the same steps as
// TrestleAndroidUiTest:
//
//   launch 1   empty Inbox -> add a task -> complete it, reopen it -> delete
//              it -> add the task that must survive
//   launch 2   (terminate, launch: a new process) the survivor is back ->
//              delete it -> the Inbox is empty again
//
// CI uninstalls the app first, and `xcodebuild test` installs it fresh, so
// launch 1 starts empty and launch 2 reads what launch 1 wrote to the app's
// own container.
//
// An iPad in landscape, as JournalUiTests runs: the composer (name, due,
// Add) and a task row (toggle, name, chips, Edit, Delete) are each one row,
// and an XCUITest touches only what is on screen.
//
// Controls are found by part name, the `.accessibilityIdentifier` every
// generated control carries (TaskApp.mll). Two layout facts:
//
//   * After a successful add the engine sets `new-task-name-focus`, and the
//     composer swaps `name-input` for `name-input-corrected` so the field
//     keeps keyboard focus. Either is "the task name field".
//   * Each task row's buttons are `toggle`, `task-name`, `edit-btn` and
//     `del-btn`. The test keeps at most one task, so a part names one
//     element. The toggle's accessibility label is the engine's
//     action-oriented name ("Complete task: …"), so reading it back is a
//     round trip through the scheduler.

private let firstTask = "Scheduled on iPadOS"
private let keptTask = "Kept across an iPadOS relaunch"
private let due = "2026-01-09"
private let emptyTitle = "Your Inbox is ready"

// The engine is loaded before the first frame, and the CI simulator is
// slow on a cold boot.
private let startupTimeout: TimeInterval = 60
private let settleTimeout: TimeInterval = 10

final class TrestleUiTests: XCTestCase {
    private var app: XCUIApplication!

    override func setUpWithError() throws {
        continueAfterFailure = false
        app = XCUIApplication()
    }

    private func element(_ identifier: String) -> XCUIElement {
        app.descendants(matching: .any).matching(identifier: identifier).firstMatch
    }

    private func anyOf(_ identifiers: [String]) -> XCUIElement {
        app.descendants(matching: .any).matching(
            NSPredicate(format: "identifier IN %@", identifiers)
        ).firstMatch
    }

    private var taskNameField: XCUIElement { anyOf(["name-input", "name-input-corrected"]) }
    private var dueField: XCUIElement { anyOf(["due-input", "due-input-corrected"]) }

    private func launch() {
        app.launch()
        XCTAssertTrue(
            taskNameField.waitForExistence(timeout: startupTimeout),
            "the generated composer never appeared"
        )
        XCTAssertFalse(element("mosaic-startup-failure").exists, "the app showed its startup failure")
    }

    /// The software keyboard covers the bottom of a landscape iPad. Hide it
    /// when it is up; with a hardware keyboard connected there is none, and
    /// this does nothing.
    private func hideKeyboard() {
        let hide = app.keyboards.buttons["Hide keyboard"]
        if hide.exists && hide.isHittable {
            hide.tap()
        }
    }

    private func tap(_ target: XCUIElement, _ what: String) {
        XCTAssertTrue(target.waitForExistence(timeout: settleTimeout), "\(what) is missing")
        target.tap()
    }

    private func add(_ name: String) {
        tap(taskNameField, "the task name field")
        taskNameField.typeText(name)
        tap(dueField, "the due field")
        dueField.typeText(due)
        hideKeyboard()
        tap(element("add-btn"), "Add task")
        hideKeyboard()
    }

    private func waitForAbsence(_ target: XCUIElement, _ what: String) {
        let gone = expectation(for: NSPredicate(format: "exists == false"), evaluatedWith: target)
        XCTAssertEqual(XCTWaiter.wait(for: [gone], timeout: settleTimeout), .completed, "\(what) is still there")
    }

    private func waitForLabel(_ target: XCUIElement, _ label: String) {
        let named = expectation(for: NSPredicate(format: "label == %@", label), evaluatedWith: target)
        XCTAssertEqual(
            XCTWaiter.wait(for: [named], timeout: settleTimeout),
            .completed,
            "expected \"\(label)\", found \"\(target.label)\""
        )
    }

    private func taskName(_ name: String) -> XCUIElement {
        app.buttons.matching(
            NSPredicate(format: "identifier == %@ AND label == %@", "task-name", name)
        ).firstMatch
    }

    func testGeneratedControlsAddCompleteDeleteAndRestoreTasks() throws {
        XCUIDevice.shared.orientation = .landscapeLeft

        // ---- Launch 1: an empty Inbox, a task through its lifecycle. -------
        launch()
        XCTAssertTrue(app.staticTexts[emptyTitle].waitForExistence(timeout: settleTimeout))
        XCTAssertFalse(element("toggle").exists, "a fresh Inbox has no task rows")

        add(firstTask)
        XCTAssertTrue(taskName(firstTask).waitForExistence(timeout: settleTimeout), "the first task's row")
        XCTAssertTrue(app.staticTexts["due \(due)"].exists)
        waitForAbsence(app.staticTexts[emptyTitle], "the empty state")

        let toggle = element("toggle")
        waitForLabel(toggle, "Complete task: \(firstTask)")
        tap(toggle, "the completion toggle")
        waitForLabel(toggle, "Reopen task: \(firstTask)")
        tap(toggle, "the completion toggle")
        waitForLabel(toggle, "Complete task: \(firstTask)")

        tap(element("del-btn"), "Delete")
        waitForAbsence(taskName(firstTask), "the deleted task's row")
        XCTAssertTrue(app.staticTexts[emptyTitle].waitForExistence(timeout: settleTimeout))

        add(keptTask)
        XCTAssertTrue(taskName(keptTask).waitForExistence(timeout: settleTimeout), "the kept task's row")

        // ---- Launch 2: a new process restores the survivor. ----------------
        app.terminate()
        launch()
        XCTAssertTrue(taskName(keptTask).waitForExistence(timeout: settleTimeout), "the task was not restored")
        XCTAssertTrue(app.staticTexts["due \(due)"].exists)
        XCTAssertFalse(app.staticTexts[emptyTitle].exists)
        hideKeyboard()
        tap(element("del-btn"), "Delete")
        waitForAbsence(taskName(keptTask), "the restored task's row")
        XCTAssertTrue(app.staticTexts[emptyTitle].waitForExistence(timeout: settleTimeout))
    }
}
