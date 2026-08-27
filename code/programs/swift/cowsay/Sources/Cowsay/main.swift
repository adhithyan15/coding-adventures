// cowsay (Swift) — entry point.
//
// Thin CLI wiring: load and parse `code/specs/cowsay.json` via CliBuilder,
// resolve the parsed flags/arguments into a `CowsayInvocation`, and hand
// off to `renderCowsay` for the actual formatting + paint-vm-ascii render.
// See `code/specs/cowsay-paintvm-pipeline.md` for the design.
//
// CliBuilder's `Parser.parse()` follows the C/Go argv convention where
// index 0 is the program name. Unlike every other port in this rollout,
// Swift's own `CommandLine.arguments` ALREADY includes the executable
// path at index 0 (the same convention C's argv/Go's os.Args use) — no
// placeholder needs to be prepended here, unlike Kotlin's `args`/Dart's
// `args`/Java's `args`, which all exclude the program name and need one
// synthesized (see lessons.md's "C#"/"Haskell"/"Java"/"Kotlin" sections
// for that pitfall in the languages where it actually applies).
//
// Output is written via `FileHandle.write(Data)` rather than `print()` —
// a raw byte write is never subject to platform newline translation, so
// output is always LF-only without needing the explicit workaround the
// JVM/Dart ports needed for the same guarantee.
import CliBuilder
import Foundation
import PaintInstructions
import PaintVmAscii

#if canImport(Glibc)
import Glibc
#elseif canImport(Darwin)
import Darwin
#elseif canImport(WinSDK)
import WinSDK
#endif

/// Whether stdin is an interactive terminal (as opposed to a pipe or
/// redirected file) — used to decide whether a missing message argument
/// should fall back to reading stdin or simply produce no output. The
/// POSIX `isatty`/`fileno` names are deprecated on Windows in favor of
/// `_isatty`/`_fileno`; both spellings wrap the same underlying check.
func stdinIsTerminal() -> Bool {
    #if os(Windows)
    return _isatty(_fileno(stdin)) != 0
    #else
    return isatty(fileno(stdin)) != 0
    #endif
}

func writeOut(_ text: String) {
    FileHandle.standardOutput.write(Data(text.utf8))
}

func writeErr(_ text: String) {
    FileHandle.standardError.write(Data(text.utf8))
}

func run(_ result: ParseResult, _ cowsDir: String) throws {
    let flags = flagsAsAny(result.flags)
    let arguments = argumentsAsStringArrays(result.arguments)

    if isListRequested(flags) {
        for name in try listCowFiles(cowsDir) {
            writeOut(name + "\n")
        }
        return
    }

    var message = resolveMessageFromArguments(arguments)
    if message == nil {
        if stdinIsTerminal() { return }
        let data = FileHandle.standardInput.readDataToEndOfFile()
        message = (String(data: data, encoding: .utf8) ?? "")
            .trimmingCharacters(in: .whitespacesAndNewlines)
    }

    guard let message, !message.isEmpty else { return }

    let invocation = buildInvocation(message, flags)
    let text = try renderCowsay(invocation, cowsDir)
    writeOut(text + "\n")
}

let repoRoot = findRepoRoot(FileManager.default.currentDirectoryPath)
let specPath = repoRoot + "/code/specs/cowsay.json"
let cowsDir = repoRoot + "/code/specs/cows"

do {
    let outcome = try Parser(specPath: specPath, argv: CommandLine.arguments).parse()
    switch outcome {
    case .help(let help):
        writeOut(help.text + "\n")
    case .version(let versionResult):
        writeOut(versionResult.version + "\n")
    case .parsed(let result):
        try run(result, cowsDir)
    }
} catch let error as SpecError {
    writeErr(error.description + "\n")
    exit(1)
} catch let error as ParseErrors {
    writeErr(error.description + "\n")
    exit(1)
} catch let error as ParseIssue {
    writeErr(error.message + "\n")
    exit(1)
} catch {
    // Any other error (a missing/unreadable CLI spec or .cow template, or
    // a PaintVmAsciiError from a malformed scene) is reported the same
    // way rather than letting an uncaught throw terminate the process
    // with a raw runtime-error message.
    writeErr("\(error)\n")
    exit(1)
}
