import XCTest
@testable import Cowsay
import CliBuilder
import Foundation
import PaintInstructions
import PaintVmAscii

func writeCow(_ dir: String, _ name: String, _ contents: String) throws {
    try contents.write(toFile: dir + "/\(name).cow", atomically: true, encoding: .utf8)
}

func resolveRepoRoot() -> String {
    findRepoRoot(FileManager.default.currentDirectoryPath)
}

func makeTempDir() -> String {
    let dir = NSTemporaryDirectory() + "/cowsay_test_\(UUID().uuidString)"
    try! FileManager.default.createDirectory(atPath: dir, withIntermediateDirectories: true)
    return dir
}

func baseInvocation(
    message: String = "hi",
    eyes: String = "oo",
    tongue: String = "  ",
    activeModes: [String] = [],
    noWrap: Bool = false,
    width: Int = 40,
    think: Bool = false,
    cowFile: String = "default"
) -> CowsayInvocation {
    CowsayInvocation(
        message: message, eyes: eyes, tongue: tongue, activeModes: activeModes,
        noWrap: noWrap, width: width, think: think, cowFile: cowFile
    )
}

final class CowsayTests: XCTestCase {
    var tempDir = ""
    var tempOutsideDir = ""

    override func setUp() {
        tempDir = makeTempDir()
        tempOutsideDir = makeTempDir()
    }

    override func tearDown() {
        try? FileManager.default.removeItem(atPath: tempDir)
        try? FileManager.default.removeItem(atPath: tempOutsideDir)
    }

    // -------------------------------------------------------------------
    // wrapText
    // -------------------------------------------------------------------

    func testDoesNotWrapShortText() {
        XCTAssertEqual(wrapText("hello", 40), ["hello"])
    }

    func testWrapsLongTextAtWordBoundaries() {
        XCTAssertEqual(wrapText("the quick brown fox jumps over", 10), ["the quick", "brown fox", "jumps over"])
    }

    func testReturnsEmptyLineForEmptyText() {
        XCTAssertEqual(wrapText("", 40), [""])
    }

    func testKeepsSingleWordLongerThanWidthWhole() {
        XCTAssertEqual(wrapText("supercalifragilisticexpialidocious", 5), ["supercalifragilisticexpialidocious"])
    }

    // -------------------------------------------------------------------
    // formatBubble
    // -------------------------------------------------------------------

    func testReturnsEmptyStringForNoLines() {
        XCTAssertEqual(formatBubble([], false), "")
    }

    func testDrawsSingleLineSpeechBubble() {
        XCTAssertEqual(formatBubble(["hi"], false), " ____\n< hi >\n ----")
    }

    func testDrawsSingleLineThoughtBubble() {
        XCTAssertEqual(formatBubble(["hi"], true), " ____\n( hi )\n ----")
    }

    func testDrawsMultiLineSpeechBubble() {
        XCTAssertEqual(
            formatBubble(["one", "two", "three"], false),
            " _______\n/ one   \\\n| two   |\n\\ three /\n -------"
        )
    }

    func testDrawsMultiLineThoughtBubble() {
        XCTAssertEqual(formatBubble(["one", "two"], true), " _____\n( one )\n( two )\n -----")
    }

    // -------------------------------------------------------------------
    // normalizeTwoChars
    // -------------------------------------------------------------------

    func testPadsOneCharacterValue() {
        XCTAssertEqual(normalizeTwoChars("o"), "o ")
    }

    func testPadsEmptyValue() {
        XCTAssertEqual(normalizeTwoChars(""), "  ")
    }

    func testLeavesTwoCharacterValueUnchanged() {
        XCTAssertEqual(normalizeTwoChars("oo"), "oo")
    }

    func testTruncatesLongerValue() {
        XCTAssertEqual(normalizeTwoChars("ooo"), "oo")
    }

    // -------------------------------------------------------------------
    // resolveEyesAndTongue
    // -------------------------------------------------------------------

    func testKeepsBaseValuesWhenNoModesActive() {
        let result = resolveEyesAndTongue("oo", "  ", [])
        XCTAssertEqual(result.eyes, "oo")
        XCTAssertEqual(result.tongue, "  ")
    }

    func testBorgOverridesEyesOnly() {
        let result = resolveEyesAndTongue("oo", "  ", ["borg"])
        XCTAssertEqual(result.eyes, "==")
        XCTAssertEqual(result.tongue, "  ")
    }

    func testDeadOverridesEyesAndTongue() {
        let result = resolveEyesAndTongue("oo", "  ", ["dead"])
        XCTAssertEqual(result.eyes, "XX")
        XCTAssertEqual(result.tongue, "U ")
    }

    func testStonedOverridesEyesAndTongue() {
        let result = resolveEyesAndTongue("oo", "  ", ["stoned"])
        XCTAssertEqual(result.eyes, "xx")
        XCTAssertEqual(result.tongue, "U ")
    }

    func testIgnoresUnknownMode() {
        let result = resolveEyesAndTongue("oo", "  ", ["not-a-real-mode"])
        XCTAssertEqual(result.eyes, "oo")
        XCTAssertEqual(result.tongue, "  ")
    }

    // -------------------------------------------------------------------
    // loadCow
    // -------------------------------------------------------------------

    func testLoadsBodyBetweenHeredocMarkers() throws {
        try writeCow(tempDir, "default", "$the_cow = <<EOC;\n  $thoughts   ^__^\n   ($eyes)\nEOC\n")
        XCTAssertEqual(try loadCow("default", tempDir), "  $thoughts   ^__^\n   ($eyes)\n")
    }

    func testFallsBackToDefaultCowWhenNamedCowMissing() throws {
        try writeCow(tempDir, "default", "$the_cow = <<EOC;\nfallback\nEOC\n")
        XCTAssertEqual(try loadCow("does-not-exist", tempDir), "fallback\n")
    }

    func testFallsBackToDefaultInsteadOfEscapingViaTraversal() throws {
        try writeCow(tempDir, "default", "$the_cow = <<EOC;\nfallback\nEOC\n")
        try writeCow(tempOutsideDir, "secret", "$the_cow = <<EOC;\nSECRET\nEOC\n")
        try writeCow(tempOutsideDir, "outside", "$the_cow = <<EOC;\nSECRET\nEOC\n")
        for malicious in ["../../../../../../etc/passwd", "..\\..\\..\\secret", "../outside"] {
            XCTAssertEqual(try loadCow(malicious, tempDir), "fallback\n", "for input: \(malicious)")
        }
    }

    func testFallsBackToDefaultInsteadOfFollowingRootedOverride() throws {
        try writeCow(tempDir, "default", "$the_cow = <<EOC;\nfallback\nEOC\n")
        try "$the_cow = <<EOC;\nSECRET\nEOC\n".write(toFile: tempOutsideDir + "/win.cow", atomically: true, encoding: .utf8)
        let rootedName = tempOutsideDir + "/win"
        XCTAssertEqual(try loadCow(rootedName, tempDir), "fallback\n")
    }

    // -------------------------------------------------------------------
    // composeContent
    // -------------------------------------------------------------------

    func testComposesBubbleAndCowWithSubstitutions() throws {
        try writeCow(tempDir, "default", "$the_cow = <<EOC;\n$thoughts $eyes $tongue\nEOC\n")
        XCTAssertEqual(try composeContent(baseInvocation(), tempDir), " ____\n< hi >\n ----\n\\ oo   \n")
    }

    func testThinkModeUsesOForThoughtsAndParenBubble() throws {
        try writeCow(tempDir, "default", "$the_cow = <<EOC;\n$thoughts $eyes $tongue\nEOC\n")
        XCTAssertEqual(try composeContent(baseInvocation(think: true), tempDir), " ____\n( hi )\n ----\no oo   \n")
    }

    func testModeFlagOverridesEyesAndTongueInTemplate() throws {
        try writeCow(tempDir, "default", "$the_cow = <<EOC;\n$thoughts $eyes $tongue\nEOC\n")
        XCTAssertEqual(
            try composeContent(baseInvocation(activeModes: ["dead"]), tempDir),
            " ____\n< hi >\n ----\n\\ XX U \n"
        )
    }

    // -------------------------------------------------------------------
    // buildScene
    // -------------------------------------------------------------------

    func testCreatesOneGlyphRunPerNonBlankLine() {
        let scene = buildScene("hi\n\nyo")
        let runs = scene.instructions.compactMap { instruction -> PaintGlyphRunInstruction? in
            guard case .glyphRun(let run) = instruction else { return nil }
            return run
        }
        XCTAssertEqual(runs.count, 2)
        XCTAssertEqual(runs[0].glyphs.map(\.glyphId), [Int(Character("h").asciiValue!), Int(Character("i").asciiValue!)])
        XCTAssertEqual(runs[0].glyphs.map(\.x), [0.0, scaleX])
        XCTAssertEqual(runs[1].glyphs.map(\.glyphId), [Int(Character("y").asciiValue!), Int(Character("o").asciiValue!)])
        XCTAssertEqual(runs[1].glyphs.map(\.y), [2 * scaleY, 2 * scaleY])
    }

    func testSkipsSpacesRatherThanPlacingThem() {
        let scene = buildScene("a b")
        let runs = scene.instructions.compactMap { instruction -> PaintGlyphRunInstruction? in
            guard case .glyphRun(let run) = instruction else { return nil }
            return run
        }
        XCTAssertEqual(runs.count, 1)
        XCTAssertEqual(runs[0].glyphs.count, 2)
    }

    func testCoversAllLinesInSceneDimensions() {
        let scene = buildScene("abc\nde")
        XCTAssertEqual(scene.width, Int(3 * scaleX))
        XCTAssertEqual(scene.height, Int(2 * scaleY))
    }

    // -------------------------------------------------------------------
    // render round trip
    // -------------------------------------------------------------------

    func testRoundTripsSimpleSingleLineText() throws {
        let scene = buildScene("hi")
        let text = try render(scene, AsciiOptions(scaleX: Int(scaleX), scaleY: Int(scaleY)))
        XCTAssertEqual(text, "hi")
    }

    func testRoundTripsBubbleAndCowBlockTrimmingTrailingBlankLine() throws {
        let content = " ____\n< hi >\n ----\n\\   ^__^\n"
        let scene = buildScene(content)
        let text = try render(scene, AsciiOptions(scaleX: Int(scaleX), scaleY: Int(scaleY)))
        XCTAssertEqual(text, " ____\n< hi >\n ----\n\\   ^__^")
    }

    // -------------------------------------------------------------------
    // CLI glue
    // -------------------------------------------------------------------

    func testIsListRequestedTrueWhenFlagPresent() {
        XCTAssertTrue(isListRequested(["list": true]))
    }

    func testIsListRequestedFalseWhenFlagAbsent() {
        XCTAssertFalse(isListRequested([:]))
    }

    func testResolveMessageFromArgumentsJoinsPositionalWords() {
        XCTAssertEqual(resolveMessageFromArguments(["message": ["hello", "there"]]), "hello there")
    }

    func testResolveMessageFromArgumentsReturnsNilWhenArgumentsEmpty() {
        XCTAssertNil(resolveMessageFromArguments([:]))
    }

    func testResolveMessageFromArgumentsReturnsNilWhenMessageListEmpty() {
        XCTAssertNil(resolveMessageFromArguments(["message": []]))
    }

    func testBuildInvocationUsesDefaultsWhenNoFlagsSet() {
        let invocation = buildInvocation("hi", [:])
        XCTAssertEqual(invocation.message, "hi")
        XCTAssertEqual(invocation.eyes, "oo")
        XCTAssertEqual(invocation.tongue, "  ")
        XCTAssertEqual(invocation.cowFile, "default")
        XCTAssertFalse(invocation.noWrap)
        XCTAssertFalse(invocation.think)
        XCTAssertEqual(invocation.width, 40)
        XCTAssertTrue(invocation.activeModes.isEmpty)
    }

    func testBuildInvocationHonorsExplicitFlags() {
        let flags: [String: Any] = [
            "eyes": "^^", "tongue": "vv", "cowfile": "dragon",
            "nowrap": true, "think": true, "width": 20, "borg": true,
        ]
        let invocation = buildInvocation("hi", flags)
        XCTAssertEqual(invocation.eyes, "^^")
        XCTAssertEqual(invocation.tongue, "vv")
        XCTAssertEqual(invocation.cowFile, "dragon")
        XCTAssertTrue(invocation.noWrap)
        XCTAssertTrue(invocation.think)
        XCTAssertEqual(invocation.width, 20)
        XCTAssertEqual(invocation.activeModes, ["borg"])
    }

    func testBuildInvocationRejectsNegativeWidth() {
        XCTAssertEqual(buildInvocation("hi", ["width": -5]).width, 1)
    }

    func testListCowFilesReturnsSortedBasenames() throws {
        try writeCow(tempDir, "tux", "")
        try writeCow(tempDir, "default", "")
        try writeCow(tempDir, "dragon", "")
        XCTAssertEqual(try listCowFiles(tempDir), ["default", "dragon", "tux"])
    }

    // -------------------------------------------------------------------
    // CliBuilder argv convention
    // -------------------------------------------------------------------

    // Regression test: unlike Kotlin/Java/Dart, Swift's CommandLine.arguments
    // ALREADY includes the executable path at index 0 (matching CliBuilder's
    // C/Go-style expectation directly) -- no placeholder needs to be
    // prepended. This test drives the real Parser with a hand-built argv
    // that mimics that shape, confirming the first real token isn't dropped.
    func testDoesNotDropFirstTokenWhenProgramNamePlaceholderIsPrepended() throws {
        let repoRoot = resolveRepoRoot()
        let specPath = repoRoot + "/code/specs/cowsay.json"

        let outcome1 = try Parser(specPath: specPath, argv: ["cowsay", "hello"]).parse()
        guard case .parsed(let result1) = outcome1 else { return XCTFail("expected .parsed") }
        XCTAssertEqual(resolveMessageFromArguments(argumentsAsStringArrays(result1.arguments)), "hello")

        let outcome2 = try Parser(specPath: specPath, argv: ["cowsay", "hello", "world"]).parse()
        guard case .parsed(let result2) = outcome2 else { return XCTFail("expected .parsed") }
        XCTAssertEqual(resolveMessageFromArguments(argumentsAsStringArrays(result2.arguments)), "hello world")
    }

    // -------------------------------------------------------------------
    // end-to-end golden output
    // -------------------------------------------------------------------

    func testResolvesRealCowsDirectory() throws {
        let repoRoot = resolveRepoRoot()
        let cowsDir = repoRoot + "/code/specs/cows"
        XCTAssertTrue(try listCowFiles(cowsDir).contains("default"))
    }

    func testDefaultCowSpeakingHelloWorld() throws {
        let repoRoot = resolveRepoRoot()
        let cowsDir = repoRoot + "/code/specs/cows"
        let invocation = baseInvocation(message: "Hello, World!")
        let text = try renderCowsay(invocation, cowsDir)
        XCTAssertEqual(
            text,
            [
                " _______________",
                "< Hello, World! >",
                " ---------------",
                "        \\   ^__^",
                "         \\  (oo)\\_______",
                "            (__)\\       )\\/\\",
                "                ||----w |",
                "                ||     ||",
            ].joined(separator: "\n")
        )
    }

    func testBorgModeThinkingWithDefaultCow() throws {
        let repoRoot = resolveRepoRoot()
        let cowsDir = repoRoot + "/code/specs/cows"
        let invocation = baseInvocation(message: "beep", activeModes: ["borg"], think: true)
        let text = try renderCowsay(invocation, cowsDir)
        XCTAssertEqual(
            text,
            [
                " ______",
                "( beep )",
                " ------",
                "        o   ^__^",
                "         o  (==)\\_______",
                "            (__)\\       )\\/\\",
                "                ||----w |",
                "                ||     ||",
            ].joined(separator: "\n")
        )
    }
}
