import XCTest

// Journal on iPadOS, driven through its generated controls (UI89 §4.1, §4.3).
//
// The simulator gate (§2.3) proves the app starts, persists and restores
// without being touched. This proves a person can use it: text typed into
// the generated editor reaches the Rust runtime, a saved entry appears in
// the timeline, and it is still there after the app is terminated and
// launched again. It is the desktop JournalUiTest
// (conformance/compose/JournalUiTest.kt) on an iPad:
//
//   launch 1   empty journal -> write and save two entries -> delete one
//   launch 2   (terminate, launch: a new process) the survivor is back ->
//              delete it -> the journal is empty again
//
// CI uninstalls the app first (code/scripts/mosaic-ios-simulator-gate.sh
// leaves state behind), and `xcodebuild test` installs it fresh, so launch 1
// starts empty and launch 2 reads what launch 1 wrote to Application
// Support, where users' state goes.
//
// Why an iPad in landscape. The app shell is a NavigationSplitView with
// `collapse: auto`; on an iPhone it collapses to a stack, and the editor is
// a navigation push away from the timeline. An XCUITest touches only what is
// on screen, so it runs where both columns are: an iPad, turned to
// landscape before the first launch.
//
// Controls are found by part name, the `.accessibilityIdentifier` every
// generated control carries, as the Android test finds them by test tag.
// Timeline rows are the toolkit RecordList's buttons, identified
// `record-list-title` (or `-selected`). RecordList is mounted more than once
// ("On this day" sits above the timeline), and a later mount's parts are
// suffixed `-m2`, so rows are matched by identifier PREFIX and by label.

private let draftTitle = "Drafted on iPadOS"
private let keptTitle = "Kept across an iPadOS relaunch"
private let keptBody = "Saved by JournalUiTests; launch 2 must find it."
private let emptyTitle = "No entries yet"

// The engine is loaded before the first frame, and the CI simulator is
// slow on a cold boot.
private let startupTimeout: TimeInterval = 60
private let settleTimeout: TimeInterval = 10

final class JournalUiTests: XCTestCase {
    private var app: XCUIApplication!

    override func setUpWithError() throws {
        continueAfterFailure = false
        app = XCUIApplication()
    }

    private func element(_ identifier: String) -> XCUIElement {
        app.descendants(matching: .any).matching(identifier: identifier).firstMatch
    }

    private func row(_ title: String) -> XCUIElement {
        app.buttons.matching(
            NSPredicate(
                format: "identifier BEGINSWITH %@ AND label CONTAINS %@",
                "record-list-title",
                title
            )
        ).firstMatch
    }

    private func launch() {
        app.launch()
        XCTAssertTrue(
            element("draft-editor-title").waitForExistence(timeout: startupTimeout),
            "the generated editor never appeared"
        )
        XCTAssertFalse(element("mosaic-startup-failure").exists, "the app showed its startup failure")
    }

    /// The software keyboard covers the bottom of a landscape iPad, where
    /// Save may be drawn. Hide it when it is up; with a hardware keyboard
    /// connected there is none, and this does nothing.
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

    private func write(_ title: String, _ body: String) {
        tap(element("draft-editor-title"), "the title field")
        element("draft-editor-title").typeText(title)
        tap(element("draft-editor-body"), "the body field")
        element("draft-editor-body").typeText(body)
        hideKeyboard()
        tap(element("draft-editor-save"), "Save")
    }

    private func waitForAbsence(_ target: XCUIElement, _ what: String) {
        let gone = expectation(for: NSPredicate(format: "exists == false"), evaluatedWith: target)
        XCTAssertEqual(XCTWaiter.wait(for: [gone], timeout: settleTimeout), .completed, "\(what) is still there")
    }

    func testGeneratedControlsWriteSaveAndRestoreEntries() throws {
        XCUIDevice.shared.orientation = .landscapeLeft

        // ---- Launch 1: an empty journal, two entries, one deleted. --------
        launch()
        XCTAssertTrue(app.staticTexts[emptyTitle].waitForExistence(timeout: settleTimeout))
        // A draft that has never been saved has nothing to delete.
        XCTAssertFalse(element("draft-editor-delete").exists)

        write(draftTitle, "The first entry, deleted before the relaunch.")
        XCTAssertTrue(row(draftTitle).waitForExistence(timeout: settleTimeout), "the first entry's row")
        waitForAbsence(app.staticTexts[emptyTitle], "the empty state")
        // Saved, so the editor now offers Delete.
        XCTAssertTrue(element("draft-editor-delete").waitForExistence(timeout: settleTimeout))

        tap(element("new-entry"), "New entry")
        waitForAbsence(element("draft-editor-delete"), "Delete on a new draft")
        write(keptTitle, keptBody)
        XCTAssertTrue(row(keptTitle).waitForExistence(timeout: settleTimeout), "the second entry's row")
        XCTAssertTrue(row(draftTitle).exists)

        // Open the first entry from the timeline and delete it.
        tap(row(draftTitle), "the first entry's row")
        tap(element("draft-editor-delete"), "Delete")
        waitForAbsence(row(draftTitle), "the deleted entry's row")
        XCTAssertTrue(row(keptTitle).exists)

        // ---- Launch 2: a new process restores the survivor. ----------------
        app.terminate()
        launch()
        XCTAssertTrue(row(keptTitle).waitForExistence(timeout: settleTimeout), "the entry was not restored")
        XCTAssertFalse(app.staticTexts[emptyTitle].exists)
        tap(row(keptTitle), "the restored entry's row")
        tap(element("draft-editor-delete"), "Delete")
        waitForAbsence(row(keptTitle), "the restored entry's row")
        XCTAssertTrue(app.staticTexts[emptyTitle].waitForExistence(timeout: settleTimeout))
    }
}
