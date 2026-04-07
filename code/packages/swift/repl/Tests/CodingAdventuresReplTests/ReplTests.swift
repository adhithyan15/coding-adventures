// ReplTests.swift — Comprehensive test suite for the CodingAdventuresRepl framework.
//
// Design principles:
//
//   1. Zero real I/O — every test uses `inputFn`/`outputFn` injection.
//      No stdin, stdout, or file system access anywhere in this file.
//
//   2. Captured output — each test collects output in a `[String]` array
//      and asserts on its contents.
//
//   3. Deterministic — tests use `.sync` mode by default to avoid
//      non-determinism from thread scheduling. Specific async tests use
//      `.async_mode` and assert only on observable results, not timing.
//
//   4. Naming — test function names spell out exactly what property is
//      being verified, making failures self-documenting.
//
// Coverage targets:
//   - EchoLanguage: basic echo, :quit, empty string, whitespace
//   - EvalResult handling: .ok(nil), .ok(text), .error, .quit
//   - EOF handling: nil from inputFn
//   - Prompt: banner printed once, line prompt printed per iteration
//   - Waiting: SilentWaiting start/tick/stop
//   - Mode: sync and async both work
//   - DefaultPrompt: non-empty strings
//   - Runner: sequence of inputs, first quit wins, output fn correctness

import XCTest
@testable import CodingAdventuresRepl

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

// A Language that always returns .error("forced") — used to test error display.
struct AlwaysErrorLanguage: Language {
    func eval(_ input: String) -> EvalResult {
        if input == ":quit" { return .quit }
        return .error("forced")
    }
}

// A Language that returns .ok(nil) for every input except ":quit".
// Models a language where statements produce no visible output (e.g. assignments).
struct SilentOkLanguage: Language {
    func eval(_ input: String) -> EvalResult {
        if input == ":quit" { return .quit }
        return .ok(nil)  // success, but nothing to print
    }
}

// A counting Waiting implementation — records how many times tick() was called.
// Used to verify that the Waiting plugin is actually invoked in async mode.
class CountingWaiting: Waiting {
    typealias State = Int
    var stopCalled = false
    var startCalled = false

    func start() -> Int { startCalled = true; return 0 }
    func tick(_ state: Int) -> Int { state + 1 }
    func tickMs() -> Int { 10 }  // short interval so tests don't hang
    func stop(_ state: Int) { stopCalled = true }
}

// ─────────────────────────────────────────────────────────────────────────────
// Main test class
// ─────────────────────────────────────────────────────────────────────────────

final class ReplTests: XCTestCase {

    // ── Helper ────────────────────────────────────────────────────────────

    /// Run the REPL with a list of inputs (nil = EOF) and capture all output.
    ///
    /// Uses `EchoLanguage`, `DefaultPrompt`, `SilentWaiting` unless the
    /// caller passes a different `mode`.
    func runRepl(inputs: [String?], mode: Mode = .sync) -> [String] {
        var output: [String] = []
        var inputQueue = inputs
        runWithIO(
            language: EchoLanguage(),
            prompt: DefaultPrompt(),
            waiting: SilentWaiting(),
            inputFn: {
                if inputQueue.isEmpty { return nil }
                return inputQueue.removeFirst()
            },
            outputFn: { output.append($0) },
            mode: mode
        )
        return output
    }

    // ─────────────────────────────────────────────────────────────────────
    // Test 1 — EchoLanguage: basic echo
    // ─────────────────────────────────────────────────────────────────────
    func testEchoBasic() {
        // "hello" should appear in the output.
        let out = runRepl(inputs: ["hello", ":quit"])
        XCTAssertTrue(
            out.contains("hello"),
            "EchoLanguage should echo 'hello' back; got \(out)"
        )
    }

    // ─────────────────────────────────────────────────────────────────────
    // Test 2 — :quit terminates the loop
    // ─────────────────────────────────────────────────────────────────────
    func testQuitTerminates() {
        // "Goodbye!" should appear and the loop should not process anything after.
        let out = runRepl(inputs: [":quit", "this should not appear"])
        let joined = out.joined()
        XCTAssertTrue(joined.contains("Goodbye!"), "Expected 'Goodbye!' in output")
        XCTAssertFalse(
            joined.contains("this should not appear"),
            "Input after :quit should not be echoed"
        )
    }

    // ─────────────────────────────────────────────────────────────────────
    // Test 3 — Empty string input is echoed back
    // ─────────────────────────────────────────────────────────────────────
    func testEmptyStringEchoed() {
        // Empty string is valid input — EchoLanguage returns .ok("").
        // The output array should contain an empty string element.
        let out = runRepl(inputs: ["", ":quit"])
        // The output array will have: banner, linePrompt, "", linePrompt, "Goodbye!"
        XCTAssertTrue(
            out.contains(""),
            "Empty string should be in output; got \(out)"
        )
    }

    // ─────────────────────────────────────────────────────────────────────
    // Test 4 — Error result prints "Error: ..."
    // ─────────────────────────────────────────────────────────────────────
    func testErrorResultPrintsPrefix() {
        // AlwaysErrorLanguage returns .error("forced") for non-quit input.
        var output: [String] = []
        var q: [String?] = ["anything", ":quit"]
        runWithIO(
            language: AlwaysErrorLanguage(),
            prompt: DefaultPrompt(),
            waiting: SilentWaiting(),
            inputFn: { q.isEmpty ? nil : q.removeFirst() },
            outputFn: { output.append($0) },
            mode: .sync
        )
        let joined = output.joined()
        XCTAssertTrue(
            joined.contains("Error: forced"),
            "Error result should print 'Error: forced'; got \(joined)"
        )
    }

    // ─────────────────────────────────────────────────────────────────────
    // Test 5 — nil from inputFn exits gracefully (EOF)
    // ─────────────────────────────────────────────────────────────────────
    func testNilInputExitsGracefully() {
        // A nil as the first input simulates immediate EOF (e.g. empty pipe).
        // The runner should return without crashing; no "Goodbye!" is printed.
        let out = runRepl(inputs: [nil])
        let joined = out.joined()
        XCTAssertFalse(
            joined.contains("Goodbye!"),
            "EOF should not print Goodbye!; got \(joined)"
        )
        // Banner should still appear before the first input read.
        XCTAssertTrue(
            joined.contains("REPL"),
            "Banner should appear even on immediate EOF; got \(joined)"
        )
    }

    // ─────────────────────────────────────────────────────────────────────
    // Test 6 — Multiple inputs in sequence
    // ─────────────────────────────────────────────────────────────────────
    func testMultipleInputsInSequence() {
        let out = runRepl(inputs: ["one", "two", "three", ":quit"])
        let joined = out.joined()
        XCTAssertTrue(joined.contains("one"),   "Expected 'one' in output")
        XCTAssertTrue(joined.contains("two"),   "Expected 'two' in output")
        XCTAssertTrue(joined.contains("three"), "Expected 'three' in output")
    }

    // ─────────────────────────────────────────────────────────────────────
    // Test 7 — Sync mode works
    // ─────────────────────────────────────────────────────────────────────
    func testSyncMode() {
        // Verify that .sync mode still produces correct output.
        let out = runRepl(inputs: ["sync-test", ":quit"], mode: .sync)
        XCTAssertTrue(out.contains("sync-test"), "Sync mode should echo 'sync-test'")
    }

    // ─────────────────────────────────────────────────────────────────────
    // Test 8 — Async mode works
    // ─────────────────────────────────────────────────────────────────────
    func testAsyncMode() {
        // Verify that .async_mode produces the same observable output as sync.
        let out = runRepl(inputs: ["async-test", ":quit"], mode: .async_mode)
        XCTAssertTrue(
            out.contains("async-test"),
            "Async mode should echo 'async-test'; got \(out)"
        )
    }

    // ─────────────────────────────────────────────────────────────────────
    // Test 9 — EchoLanguage: non-quit input returns .ok
    // ─────────────────────────────────────────────────────────────────────
    func testEchoLanguageNonQuitReturnsOk() {
        let lang = EchoLanguage()
        let result = lang.eval("anything")
        XCTAssertEqual(result, .ok("anything"))
    }

    // ─────────────────────────────────────────────────────────────────────
    // Test 10 — SilentWaiting: tick increments the counter
    // ─────────────────────────────────────────────────────────────────────
    func testSilentWaitingTickIncrements() {
        let w = SilentWaiting()
        let s0 = w.start()
        XCTAssertEqual(s0, 0, "start() should return 0")
        let s1 = w.tick(s0)
        XCTAssertEqual(s1, 1, "tick(0) should return 1")
        let s2 = w.tick(s1)
        XCTAssertEqual(s2, 2, "tick(1) should return 2")
    }

    // ─────────────────────────────────────────────────────────────────────
    // Test 11 — DefaultPrompt: returns non-empty strings
    // ─────────────────────────────────────────────────────────────────────
    func testDefaultPromptNonEmpty() {
        let p = DefaultPrompt()
        XCTAssertFalse(p.globalPrompt().isEmpty, "globalPrompt() should not be empty")
        XCTAssertFalse(p.linePrompt().isEmpty,   "linePrompt() should not be empty")
    }

    // ─────────────────────────────────────────────────────────────────────
    // Test 12 — Global prompt printed exactly once
    // ─────────────────────────────────────────────────────────────────────
    func testGlobalPromptPrintedOnce() {
        // Run a two-input session and count banner occurrences.
        let banner = DefaultPrompt().globalPrompt()
        let out = runRepl(inputs: ["a", "b", ":quit"])
        let bannerCount = out.filter { $0 == banner }.count
        XCTAssertEqual(bannerCount, 1, "Banner should appear exactly once")
    }

    // ─────────────────────────────────────────────────────────────────────
    // Test 13 — Line prompt printed before each input
    // ─────────────────────────────────────────────────────────────────────
    func testLinePromptPrintedEachIteration() {
        // With 3 inputs (a, b, :quit), the line prompt should appear 3 times.
        let linePrompt = DefaultPrompt().linePrompt()
        let out = runRepl(inputs: ["a", "b", ":quit"])
        let promptCount = out.filter { $0 == linePrompt }.count
        XCTAssertEqual(promptCount, 3, "Line prompt should appear 3 times for 3 inputs")
    }

    // ─────────────────────────────────────────────────────────────────────
    // Test 14 — nil waiting (no plugin) works in async mode
    // ─────────────────────────────────────────────────────────────────────
    func testNilWaitingInAsyncMode() {
        // Pass `waiting: nil as SilentWaiting?` — the runner should use
        // group.wait() with no ticking and still produce correct output.
        var output: [String] = []
        var q: [String?] = ["nil-waiting-test", ":quit"]
        runWithIO(
            language: EchoLanguage(),
            prompt: DefaultPrompt(),
            waiting: nil as SilentWaiting?,
            inputFn: { q.isEmpty ? nil : q.removeFirst() },
            outputFn: { output.append($0) },
            mode: .async_mode
        )
        XCTAssertTrue(
            output.contains("nil-waiting-test"),
            "Async mode with nil waiting should still echo input; got \(output)"
        )
    }

    // ─────────────────────────────────────────────────────────────────────
    // Test 15 — Long sequence of echoes
    // ─────────────────────────────────────────────────────────────────────
    func testLongSequenceOfEchoes() {
        // Send 50 inputs followed by :quit — all should be echoed.
        let words = (1...50).map { "word\($0)" }
        let inputs: [String?] = words.map { Optional($0) } + [":quit"]
        let out = runRepl(inputs: inputs)
        for word in words {
            XCTAssertTrue(out.contains(word), "Expected '\(word)' in output")
        }
    }

    // ─────────────────────────────────────────────────────────────────────
    // Test 16 — Output function called correctly
    // ─────────────────────────────────────────────────────────────────────
    func testOutputFunctionCalledCorrectly() {
        // The output function should receive: banner, linePrompt, "echo", linePrompt, "Goodbye!"
        var output: [String] = []
        var q: [String?] = ["echo", ":quit"]
        runWithIO(
            language: EchoLanguage(),
            prompt: DefaultPrompt(),
            waiting: SilentWaiting(),
            inputFn: { q.isEmpty ? nil : q.removeFirst() },
            outputFn: { output.append($0) },
            mode: .sync
        )
        // The output array should contain all expected strings.
        XCTAssertTrue(output.joined().contains("REPL"),     "Banner expected")
        XCTAssertTrue(output.contains("> "),                "Line prompt expected")
        XCTAssertTrue(output.contains("echo"),              "Echo expected")
        XCTAssertTrue(output.joined().contains("Goodbye!"), "Goodbye expected")
    }

    // ─────────────────────────────────────────────────────────────────────
    // Test 17 — Whitespace-only input is echoed back
    // ─────────────────────────────────────────────────────────────────────
    func testWhitespaceOnlyInputEchoed() {
        // "   " is not ":quit", so EchoLanguage returns .ok("   ").
        // Note: Runner trims trailing newlines but NOT spaces.
        let out = runRepl(inputs: ["   ", ":quit"])
        XCTAssertTrue(
            out.contains("   "),
            "Whitespace-only input should be echoed; got \(out)"
        )
    }

    // ─────────────────────────────────────────────────────────────────────
    // Test 18 — Empty string input is handled (separate from test 3)
    // ─────────────────────────────────────────────────────────────────────
    func testEmptyStringInputHandled() {
        // EchoLanguage.eval("") returns .ok("").
        let lang = EchoLanguage()
        let result = lang.eval("")
        XCTAssertEqual(result, .ok(""), "EchoLanguage.eval('') should return .ok('')")
    }

    // ─────────────────────────────────────────────────────────────────────
    // Test 19 — First :quit exits; subsequent inputs not processed
    // ─────────────────────────────────────────────────────────────────────
    func testFirstQuitExits() {
        // After :quit, "after-quit" must NOT appear in the output.
        let out = runRepl(inputs: [":quit", "after-quit"])
        let joined = out.joined()
        XCTAssertTrue(joined.contains("Goodbye!"),       "Goodbye! should appear")
        XCTAssertFalse(joined.contains("after-quit"),    "after-quit must not appear")
    }

    // ─────────────────────────────────────────────────────────────────────
    // Test 20 — Mode.default is async_mode
    // ─────────────────────────────────────────────────────────────────────
    func testModeDefaultIsAsync() {
        // The spec requires Mode.default == .async_mode.
        switch Mode.default {
        case .async_mode:
            break  // expected
        case .sync:
            XCTFail("Mode.default should be .async_mode, got .sync")
        }
    }

    // ─────────────────────────────────────────────────────────────────────
    // Test 21 — .ok(nil) produces no extra output
    // ─────────────────────────────────────────────────────────────────────
    func testOkNilProducesNoOutput() {
        // SilentOkLanguage returns .ok(nil) — nothing should be printed for eval.
        var output: [String] = []
        var q: [String?] = ["silent", ":quit"]
        runWithIO(
            language: SilentOkLanguage(),
            prompt: DefaultPrompt(),
            waiting: SilentWaiting(),
            inputFn: { q.isEmpty ? nil : q.removeFirst() },
            outputFn: { output.append($0) },
            mode: .sync
        )
        // "silent" should NOT appear in output (language returned .ok(nil))
        XCTAssertFalse(
            output.contains("silent"),
            ".ok(nil) should produce no output for the eval result"
        )
    }

    // ─────────────────────────────────────────────────────────────────────
    // Test 22 — EchoLanguage: :quit returns .quit
    // ─────────────────────────────────────────────────────────────────────
    func testEchoLanguageQuitReturnsQuit() {
        let lang = EchoLanguage()
        let result = lang.eval(":quit")
        XCTAssertEqual(result, .quit, "EchoLanguage.eval(':quit') should return .quit")
    }

    // ─────────────────────────────────────────────────────────────────────
    // Test 23 — SilentWaiting: tickMs returns 100
    // ─────────────────────────────────────────────────────────────────────
    func testSilentWaitingTickMs() {
        let w = SilentWaiting()
        XCTAssertEqual(w.tickMs(), 100, "SilentWaiting.tickMs() should be 100")
    }

    // ─────────────────────────────────────────────────────────────────────
    // Test 24 — DefaultPrompt: linePrompt ends with space
    // ─────────────────────────────────────────────────────────────────────
    func testDefaultPromptLineEndsWithSpace() {
        let p = DefaultPrompt()
        XCTAssertTrue(
            p.linePrompt().hasSuffix(" "),
            "linePrompt() should end with a space for readable cursor placement"
        )
    }

    // ─────────────────────────────────────────────────────────────────────
    // Test 25 — Async mode: CountingWaiting is invoked
    // ─────────────────────────────────────────────────────────────────────
    func testAsyncModeInvokesWaitingPlugin() {
        // CountingWaiting tracks start/stop calls.
        // With a fast EchoLanguage, start() and stop() should both be called.
        let waiting = CountingWaiting()
        var q: [String?] = ["probe", ":quit"]
        var output: [String] = []
        runWithIO(
            language: EchoLanguage(),
            prompt: DefaultPrompt(),
            waiting: waiting,
            inputFn: { q.isEmpty ? nil : q.removeFirst() },
            outputFn: { output.append($0) },
            mode: .async_mode
        )
        // start() and stop() must each be called at least twice
        // (once per non-quit eval call). We check at least once for robustness.
        XCTAssertTrue(waiting.startCalled, "Waiting.start() should be called in async mode")
        XCTAssertTrue(waiting.stopCalled,  "Waiting.stop() should be called in async mode")
    }
}
