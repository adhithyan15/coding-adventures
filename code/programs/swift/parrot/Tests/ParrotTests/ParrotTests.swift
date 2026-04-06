// ParrotTests.swift — Tests for the Parrot REPL program.
//
// These tests verify that the Parrot program composes the CodingAdventuresRepl
// framework correctly and that the ParrotPrompt type behaves as specified.
//
// Design:
//   - No real stdin/stdout. All tests use injected inputFn/outputFn.
//   - Tests check observable behaviour (output strings), not implementation
//     details (function call counts, internal state).
//   - Both sync and async modes are exercised to confirm correct behaviour
//     regardless of dispatch strategy.

import XCTest
import CodingAdventuresRepl
@testable import Parrot

final class ParrotTests: XCTestCase {

    // ── Helper ─────────────────────────────────────────────────────────────

    /// Run the Parrot REPL with a list of inputs and capture all output.
    ///
    /// Uses the same plugin configuration as `main.swift`:
    /// `EchoLanguage`, `ParrotPrompt`, `SilentWaiting`.
    func run(_ inputs: [String?], mode: Mode = .async_mode) -> [String] {
        var out: [String] = []
        var q = inputs
        runWithIO(
            language: EchoLanguage(),
            prompt: ParrotPrompt(),
            waiting: SilentWaiting(),
            inputFn: { q.isEmpty ? nil : q.removeFirst() },
            outputFn: { out.append($0) },
            mode: mode
        )
        return out
    }

    // ── Test 1 — Banner contains "Parrot" ─────────────────────────────────
    func testBannerContainsParrot() {
        // The globalPrompt banner must mention "Parrot" so users know what
        // program they are running.
        let out = run([":quit"])
        XCTAssertTrue(
            out.joined().contains("Parrot"),
            "Banner must contain 'Parrot'; got \(out)"
        )
    }

    // ── Test 2 — Echoes user input ─────────────────────────────────────────
    func testEchoesInput() {
        // "squawk" should appear in the output after being entered.
        let out = run(["squawk", ":quit"])
        XCTAssertTrue(
            out.contains("squawk"),
            "Parrot should echo 'squawk'; got \(out)"
        )
    }

    // ── Test 3 — :quit ends the session with "Goodbye!" ───────────────────
    func testQuitEndsSession() {
        let out = run([":quit"])
        XCTAssertTrue(
            out.joined().contains("Goodbye"),
            "':quit' should produce 'Goodbye'; got \(out)"
        )
    }

    // ── Test 4 — Multiple echoes ───────────────────────────────────────────
    func testMultipleEchoes() {
        let out = run(["hello", "world", ":quit"])
        XCTAssertTrue(out.contains("hello"), "Expected 'hello' in output")
        XCTAssertTrue(out.contains("world"), "Expected 'world' in output")
    }

    // ── Test 5 — Sync mode works ───────────────────────────────────────────
    func testSyncMode() {
        let out = run(["sync test", ":quit"], mode: .sync)
        XCTAssertTrue(
            out.contains("sync test"),
            "Sync mode should echo 'sync test'; got \(out)"
        )
    }

    // ── Test 6 — Line prompt contains 🦜 ──────────────────────────────────
    func testLinePromptContainsParrot() {
        let prompt = ParrotPrompt()
        XCTAssertTrue(
            prompt.linePrompt().contains("🦜"),
            "linePrompt() must contain the parrot emoji 🦜"
        )
    }

    // ── Test 7 — Global prompt mentions :quit ─────────────────────────────
    func testGlobalPromptMentionsQuit() {
        let prompt = ParrotPrompt()
        XCTAssertTrue(
            prompt.globalPrompt().contains(":quit"),
            "globalPrompt() must mention ':quit' so users know how to exit"
        )
    }

    // ── Test 8 — EOF exits gracefully ─────────────────────────────────────
    func testEofExitsGracefully() {
        // nil input = EOF — should not crash, and the banner should still print.
        let out = run([nil])
        XCTAssertTrue(
            out.joined().contains("Parrot"),
            "Banner should appear even on immediate EOF; got \(out)"
        )
    }

    // ── Test 9 — Line prompt printed before each input ────────────────────
    func testLinePromptPrintedEachInput() {
        // With inputs ["a", "b", ":quit"], linePrompt() should appear 3 times.
        let linePrompt = ParrotPrompt().linePrompt()
        let out = run(["a", "b", ":quit"], mode: .sync)
        let count = out.filter { $0 == linePrompt }.count
        XCTAssertEqual(count, 3, "Line prompt should appear 3 times for 3 inputs")
    }

    // ── Test 10 — Banner printed once ─────────────────────────────────────
    func testBannerPrintedOnce() {
        let banner = ParrotPrompt().globalPrompt()
        let out = run(["one", "two", ":quit"], mode: .sync)
        let count = out.filter { $0 == banner }.count
        XCTAssertEqual(count, 1, "Banner should be printed exactly once")
    }

    // ── Test 11 — ParrotPrompt linePrompt is non-empty ────────────────────
    func testParrotPromptLinePromptNonEmpty() {
        let prompt = ParrotPrompt()
        XCTAssertFalse(prompt.linePrompt().isEmpty, "linePrompt() must not be empty")
    }

    // ── Test 12 — ParrotPrompt globalPrompt is non-empty ──────────────────
    func testParrotPromptGlobalPromptNonEmpty() {
        let prompt = ParrotPrompt()
        XCTAssertFalse(prompt.globalPrompt().isEmpty, "globalPrompt() must not be empty")
    }

    // ── Test 13 — No output after :quit ───────────────────────────────────
    func testNoOutputAfterQuit() {
        let out = run([":quit", "should not appear"], mode: .sync)
        XCTAssertFalse(
            out.joined().contains("should not appear"),
            "Nothing after :quit should appear"
        )
    }

    // ── Test 14 — Async mode produces same output as sync ─────────────────
    func testAsyncAndSyncGiveSameOutput() {
        let syncOut  = run(["parrot", ":quit"], mode: .sync).joined()
        let asyncOut = run(["parrot", ":quit"], mode: .async_mode).joined()
        // Both should contain the echo.
        XCTAssertTrue(syncOut.contains("parrot"),  "sync mode should echo 'parrot'")
        XCTAssertTrue(asyncOut.contains("parrot"), "async mode should echo 'parrot'")
    }

    // ── Test 15 — Empty string is echoed ──────────────────────────────────
    func testEmptyStringEchoed() {
        // Empty string is valid input — parrot should echo it back.
        let out = run(["", ":quit"], mode: .sync)
        // The output array will have an empty string element from the echo.
        XCTAssertTrue(
            out.contains(""),
            "Empty string should appear in output; got \(out)"
        )
    }
}
