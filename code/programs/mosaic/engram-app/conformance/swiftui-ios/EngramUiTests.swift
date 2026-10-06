import XCTest

// Engram on iPadOS: Anki export and import through the document picker,
// answered by the build's fake (UI89 §4.4).
//
// iOS draws its document picker in another process, with a layout that
// changes from one iOS release to the next, so this test does not drive it.
// The app is launched with `-MosaicUITestPicker` instead. In a build made
// with `mosaic-compile pkg --ios-ui-test`, and only in its Debug
// configuration, that argument swaps the platform library's picker for
// `MosaicUITestPicker.swift`'s fake. The fake saves to and opens one fixed
// file in the app's own temporary directory. Everything past the picker is
// the shipped app: Engram's handler, the platform library's router (one file
// operation at a time, the size limit, the background read and write), and
// the engine that writes and merges the package.
//
//   launch 1   a fresh collection (no deck called "Default" anywhere) ->
//              Export: the fake saves the package -> Import: the fake opens
//              that same file, and the package's "Default" deck appears in
//              the deck list
//   launch 2   (terminate, launch: a new process) the imported deck is still
//              listed: the merge was saved, not only drawn
//
// The engine's side of this round trip is pinned in engram-mosaic-app
// (a_fresh_collection_exports_a_zip_whose_import_lists_the_default_deck),
// which also checks that the export is a zip. Here the import succeeding is
// the proof the export wrote a whole package, because the engine merges
// nothing else.
//
// CI uninstalls the app first, and `xcodebuild test` installs it fresh, so
// launch 1 starts from an empty collection and an empty temporary directory.

private let pickerArgument = "-MosaicUITestPicker"
private let importedDeck = "Default"

// The engine is loaded before the first frame, and the CI simulator is
// slow on a cold boot.
private let startupTimeout: TimeInterval = 60
private let settleTimeout: TimeInterval = 10

final class EngramUiTests: XCTestCase {
    private var app: XCUIApplication!

    override func setUpWithError() throws {
        continueAfterFailure = false
        app = XCUIApplication()
        app.launchArguments = [pickerArgument]
    }

    private func element(_ identifier: String) -> XCUIElement {
        app.descendants(matching: .any).matching(identifier: identifier).firstMatch
    }

    // The deck list's row for the imported deck: DeckStatsPanel draws each
    // deck as a `deck-option-button` labelled with its name.
    private var deck: XCUIElement {
        app.buttons.matching(
            NSPredicate(format: "identifier == %@ AND label == %@", "deck-option-button", importedDeck)
        ).firstMatch
    }

    private func launch() {
        app.launch()
        XCTAssertTrue(
            element("export-button").waitForExistence(timeout: startupTimeout),
            "the Decks screen never appeared"
        )
        XCTAssertFalse(element("mosaic-startup-failure").exists, "the app showed its startup failure")
    }

    private func tap(_ target: XCUIElement, _ what: String) {
        XCTAssertTrue(target.waitForExistence(timeout: settleTimeout), "\(what) is missing")
        target.tap()
    }

    func testAnkiExportAndImportRoundTripThroughThePicker() throws {
        XCUIDevice.shared.orientation = .landscapeLeft

        // ---- Launch 1: export, then import what was exported. ---------------
        launch()
        // Nothing says "Default" before the import, so seeing it after is
        // the import's doing.
        XCTAssertFalse(
            app.descendants(matching: .any).matching(NSPredicate(format: "label == %@", importedDeck)).firstMatch.exists,
            "a fresh collection already shows \"\(importedDeck)\""
        )

        tap(element("export-button"), "Export")
        // The router allows one file operation at a time, and the export is
        // written off the main thread, with nothing on screen saying when it
        // is done. An Import tapped before then is refused, as it would be for
        // a person. So Import is tapped again, at most three times in all,
        // until the deck appears.
        var imported = false
        for _ in 1...3 where !imported {
            tap(element("import-button"), "Import")
            imported = deck.waitForExistence(timeout: settleTimeout)
        }
        XCTAssertTrue(imported, "the imported package's \"\(importedDeck)\" deck never appeared")

        // ---- Launch 2: a new process restores the imported deck. ----------
        app.terminate()
        launch()
        XCTAssertTrue(deck.waitForExistence(timeout: settleTimeout), "the imported deck was not restored")
    }
}
