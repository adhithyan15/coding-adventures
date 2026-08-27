// cowsay — routed through PaintVmAscii (Swift port).
//
// Tenth language in the cowsay-through-paint-vm-ascii rollout (after
// csharp, fsharp, perl, haskell, java, kotlin, dart). Everything up
// through composing the bubble+cow text block is ordinary string
// formatting, ported unchanged from the reference implementation at
// `code/programs/go/cowsay/main.go`. The one thing that's different from
// that reference: instead of printing the composed text directly,
// `buildScene` converts it into a `PaintScene` of `glyph_run` instructions
// (one glyph placement per non-space character, positioned on an 8x16
// character grid), and `render` turns that scene back into the terminal
// string we print. This is also the PR that built Swift's `PaintVmAscii`
// from scratch, implementing the full P2D02 contract — see that
// package's own CHANGELOG.
import CliBuilder
import Foundation
import PaintInstructions
import PaintVmAscii

/// paint-vm-ascii's documented default scale factors (`P2D02-paint-vm-ascii.md`).
let scaleX: Double = 8.0
let scaleY: Double = 16.0

/// The resolved set of inputs needed to render one cowsay invocation.
struct CowsayInvocation {
    let message: String
    let eyes: String
    let tongue: String
    let activeModes: [String]
    let noWrap: Bool
    let width: Int
    let think: Bool
    let cowFile: String
}

struct EyesAndTongue {
    let eyes: String
    let tongue: String
}

private struct ModeOverride {
    let eyes: String
    let tongue: String?
}

private let modeOverrides: [String: ModeOverride] = [
    "borg": ModeOverride(eyes: "==", tongue: nil),
    "dead": ModeOverride(eyes: "XX", tongue: "U "),
    "greedy": ModeOverride(eyes: "$$", tongue: nil),
    "paranoid": ModeOverride(eyes: "@@", tongue: nil),
    "stoned": ModeOverride(eyes: "xx", tongue: "U "),
    "tired": ModeOverride(eyes: "--", tongue: nil),
    "wired": ModeOverride(eyes: "OO", tongue: nil),
    "youthful": ModeOverride(eyes: "..", tongue: nil),
]

/// Order matches `code/specs/cowsay.json`'s "modes" mutually-exclusive group.
let modeFlagIds = ["borg", "dead", "greedy", "paranoid", "stoned", "tired", "wired", "youthful"]

// MARK: - Rendering core (ported from code/programs/go/cowsay/main.go)

/// Splits text into lines no longer than `width`, breaking on word
/// boundaries. A single word longer than the width is kept whole (never
/// split mid-word).
func wrapText(_ text: String, _ width: Int) -> [String] {
    if text.count <= width { return [text] }

    let words = text.split(separator: " ", omittingEmptySubsequences: true).map(String.init)
    if words.isEmpty { return [""] }

    var lines: [String] = []
    var current = ""
    for word in words {
        if current.count + word.count + 1 <= width {
            if !current.isEmpty { current += " " }
            current += word
        } else {
            if !current.isEmpty { lines.append(current) }
            current = word
        }
    }
    if !current.isEmpty { lines.append(current) }
    return lines
}

/// Draws the speech/thought bubble around the given lines. A single line
/// gets `"< ... >"` (or `"( ... )"` for a thought bubble); multiple lines
/// get `"/ ... \"`, `"| ... |"`, `"\ ... /"` (or `"( ... )"` on every line
/// for a thought bubble).
func formatBubble(_ lines: [String], _ isThink: Bool) -> String {
    if lines.isEmpty { return "" }

    let maxLen = lines.map(\.count).max() ?? 0
    let borderTop = " " + String(repeating: "_", count: maxLen + 2)
    let borderBottom = " " + String(repeating: "-", count: maxLen + 2)

    var body: [String] = []
    if lines.count == 1 {
        let start = isThink ? "(" : "<"
        let end = isThink ? ")" : ">"
        body = ["\(start) \(lines[0].padding(toLength: maxLen, withPad: " ", startingAt: 0)) \(end)"]
    } else {
        let n = lines.count
        for (i, line) in lines.enumerated() {
            let start: String
            let end: String
            if isThink {
                start = "("
                end = ")"
            } else if i == 0 {
                start = "/"
                end = "\\"
            } else if i == n - 1 {
                start = "\\"
                end = "/"
            } else {
                start = "|"
                end = "|"
            }
            body.append("\(start) \(line.padding(toLength: maxLen, withPad: " ", startingAt: 0)) \(end)")
        }
    }

    return ([borderTop] + body + [borderBottom]).joined(separator: "\n")
}

/// Pads or truncates a mode string (eyes/tongue) to exactly two characters,
/// matching cowsay's convention that eyes/tongue are always a 2-char glyph.
func normalizeTwoChars(_ value: String) -> String {
    if value.count < 2 { return value.padding(toLength: 2, withPad: " ", startingAt: 0) }
    if value.count > 2 { return String(value.prefix(2)) }
    return value
}

/// Applies mode shortcuts (--borg, --dead, etc.) on top of the base
/// eyes/tongue flag values, then normalizes both to two characters. Modes
/// are mutually exclusive per cowsay.json, but this accepts any set for
/// robustness.
func resolveEyesAndTongue(_ baseEyes: String, _ baseTongue: String, _ activeModes: [String]) -> EyesAndTongue {
    var eyes = baseEyes
    var tongue = baseTongue
    for mode in activeModes {
        guard let override = modeOverrides[mode] else { continue }
        eyes = override.eyes
        if let overrideTongue = override.tongue { tongue = overrideTongue }
    }
    return EyesAndTongue(eyes: normalizeTwoChars(eyes), tongue: normalizeTwoChars(tongue))
}

/// Walks up from `startDir` looking for CLAUDE.md, the repo-root sentinel
/// file. CLAUDE.md (not code/specs/cowsay.json itself) is used
/// deliberately — it's a more robust marker than reaching for the very
/// file being located, and this exact fix was called out as a lesson from
/// a prior, reverted cowsay Lua port's CI pathing problems (PR #1535).
func findRepoRoot(_ startDir: String) -> String {
    var dir = normalizeAbsolutePath(startDir)
    for _ in 0..<24 {
        if FileManager.default.fileExists(atPath: dir + "/CLAUDE.md") { return dir }
        let parent = (dir as NSString).deletingLastPathComponent
        if parent.isEmpty || parent == dir { return normalizeAbsolutePath(startDir) }
        dir = parent
    }
    return normalizeAbsolutePath(startDir)
}

func normalizeAbsolutePath(_ path: String) -> String {
    URL(fileURLWithPath: path).standardizedFileURL.path
}

/// The last path segment of `value`, treating both `/` and `\` as
/// separators regardless of host platform. Deliberately NOT
/// `URL(fileURLWithPath:).lastPathComponent`: that resolves a relative
/// input against the current working directory before taking the last
/// component, so a literal `".."` input would silently resolve to the
/// CWD's *actual parent directory name* instead of the literal two-dot
/// string — non-deterministic and dependent on the caller's working
/// directory, not the "harmless literal filename" behavior every other
/// port's `loadCow` relies on. This mirrors `Path.fileName` in the JVM
/// ports instead: directory components (including any number of `..`)
/// are discarded entirely via plain string splitting, leaving only the
/// final segment, with no filesystem or CWD interaction at all.
func basenameOf(_ value: String) -> String {
    let normalized = value.replacingOccurrences(of: "\\", with: "/")
    let segments = normalized.split(separator: "/", omittingEmptySubsequences: true)
    return segments.last.map(String.init) ?? ""
}

/// Whether `value` is rooted: a POSIX absolute path (`/...`), a
/// Windows-style rooted path (`\...`), or a Windows drive-qualified path
/// (`C:\...`, `C:/...`).
func looksRooted(_ value: String) -> Bool {
    if value.isEmpty { return false }
    if value.hasPrefix("/") || value.hasPrefix("\\") { return true }
    let chars = Array(value)
    if chars.count >= 2 && chars[1] == ":" && chars[0].isLetter { return true }
    return false
}

private let cowBodyPattern = try! NSRegularExpression(pattern: "<<EOC;\\n(.*?)EOC", options: [.dotMatchesLineSeparators])

/// Loads a .cow template's body from `cowsDir`, falling back to
/// default.cow when the requested file doesn't exist. The template is a
/// Perl heredoc (`$the_cow = <<EOC; ... EOC`); only the body between the
/// heredoc markers is returned.
///
/// `cowName` comes from the user-supplied -f/--file flag, so it is
/// treated as untrusted: only a bare filename (no directory separators,
/// no rooted/absolute path) is accepted, and the resolved path is
/// verified to stay inside `cowsDir` before it's read — otherwise this
/// falls back to default.cow instead of reading an arbitrary file the
/// caller pointed at via `".."`, a rooted override, or similar (mirrors
/// the fix applied to every other port's loadCow after `/security-review`).
/// The `.cow` suffix is appended in the same string as the extracted
/// basename (not via a separate path-join step), so even a literal `".."`
/// basename becomes the harmless single filename segment `"...cow"`
/// rather than a parent-directory reference.
func loadCow(_ cowName: String, _ cowsDir: String) throws -> String {
    var cowsRoot = normalizeAbsolutePath(cowsDir)
    if cowsRoot.hasSuffix("/") { cowsRoot.removeLast() }

    let safeName = basenameOf(cowName)
    let rooted = looksRooted(cowName)

    var candidate: String? = nil
    if !safeName.isEmpty && !rooted {
        candidate = normalizeAbsolutePath(cowsRoot + "/" + safeName + ".cow")
    }

    var withinCowsDir = false
    var candidateExists = false
    if let candidate {
        withinCowsDir = candidate == cowsRoot || candidate.hasPrefix(cowsRoot + "/")
        candidateExists = withinCowsDir && FileManager.default.fileExists(atPath: candidate)
    }

    let cowPath = (candidate != nil && withinCowsDir && candidateExists) ? candidate! : cowsRoot + "/default.cow"

    let contents = try String(contentsOfFile: cowPath, encoding: .utf8)
    let fullRange = NSRange(contents.startIndex..<contents.endIndex, in: contents)
    if let match = cowBodyPattern.firstMatch(in: contents, options: [], range: fullRange),
       let bodyRange = Range(match.range(at: 1), in: contents) {
        return String(contents[bodyRange])
    }
    return contents
}

/// Composes the full bubble+cow text block for one invocation —
/// everything up to (but not including) the paint-vm-ascii render step.
func composeContent(_ invocation: CowsayInvocation, _ cowsDir: String) throws -> String {
    let eyesAndTongue = resolveEyesAndTongue(invocation.eyes, invocation.tongue, invocation.activeModes)

    var lines: [String] = []
    for rawLine in invocation.message.components(separatedBy: "\n") {
        if rawLine.isEmpty {
            lines.append("")
        } else if invocation.noWrap {
            lines.append(rawLine)
        } else {
            lines.append(contentsOf: wrapText(rawLine, invocation.width))
        }
    }

    let thoughts = invocation.think ? "o" : "\\"
    let bubble = formatBubble(lines, invocation.think)

    let cowTemplate = try loadCow(invocation.cowFile, cowsDir)
    let cow = cowTemplate
        .replacingOccurrences(of: "$eyes", with: eyesAndTongue.eyes)
        .replacingOccurrences(of: "$tongue", with: eyesAndTongue.tongue)
        .replacingOccurrences(of: "$thoughts", with: thoughts)
        .replacingOccurrences(of: "\\\\", with: "\\")

    return "\(bubble)\n\(cow)"
}

/// Converts a composed text block into a `PaintScene`: one `glyph_run`
/// instruction per line, one glyph placement per non-space character. See
/// `code/specs/cowsay-paintvm-pipeline.md` §3 for the full contract,
/// including why glyphId is a literal Unicode code point here (an
/// ASCII-backend-only relaxation of the general PaintGlyphRun contract).
func buildScene(_ text: String) -> PaintScene {
    let normalized = text.replacingOccurrences(of: "\r\n", with: "\n")
    let lines = normalized.components(separatedBy: "\n")

    var maxWidth = 0
    for line in lines {
        if line.count > maxWidth { maxWidth = line.count }
    }

    var instructions: [PaintInstruction] = []
    for (row, line) in lines.enumerated() {
        var glyphs: [PaintGlyphPlacement] = []
        for (col, scalar) in line.unicodeScalars.enumerated() {
            if scalar == " " { continue }
            glyphs.append(PaintGlyphPlacement(glyphId: Int(scalar.value), x: Double(col) * scaleX, y: Double(row) * scaleY))
        }
        if !glyphs.isEmpty {
            instructions.append(paintGlyphRun(glyphs: glyphs, fontRef: "terminal-mono", fontSize: scaleY, fill: "#000000"))
        }
    }

    let width = Int((Double(max(1, maxWidth)) * scaleX).rounded())
    let height = Int((Double(max(1, lines.count)) * scaleY).rounded())
    return PaintScene(width: width, height: height, instructions: instructions, background: "transparent")
}

/// End-to-end: compose the bubble+cow text, build a `PaintScene` from it,
/// and render that scene through paint-vm-ascii.
func renderCowsay(_ invocation: CowsayInvocation, _ cowsDir: String) throws -> String {
    let content = try composeContent(invocation, cowsDir)
    let scene = buildScene(content)
    return try render(scene, AsciiOptions(scaleX: Int(scaleX), scaleY: Int(scaleY)))
}

// MARK: - CLI glue — the bridge between CliBuilder's flags/arguments maps and
// this module's typed invocation. Kept in this file (rather than main.swift)
// so it's directly unit-testable without spawning a process or driving a
// real Parser.

func isListRequested(_ flags: [String: Any]) -> Bool {
    (flags["list"] as? Bool) == true
}

/// Cow file basenames under `cowsDir`, sorted ordinally.
func listCowFiles(_ cowsDir: String) throws -> [String] {
    let entries = try FileManager.default.contentsOfDirectory(atPath: cowsDir)
    return entries
        .filter { $0.hasSuffix(".cow") }
        .map { String($0.dropLast(".cow".count)) }
        .sorted()
}

/// Resolves the message from the parsed "message" positional argument.
/// Returns nil when no message was given on argv — the caller should fall
/// back to stdin.
func resolveMessageFromArguments(_ arguments: [String: [String]]) -> String? {
    guard let parts = arguments["message"], !parts.isEmpty else { return nil }
    return parts.joined(separator: " ")
}

/// Builds a `CowsayInvocation` from a resolved message and the parsed
/// flags map, applying cowsay.json's documented defaults for any flag
/// that wasn't explicitly set.
func buildInvocation(_ message: String, _ flags: [String: Any]) -> CowsayInvocation {
    let eyes = (flags["eyes"] as? String) ?? "oo"
    let tongue = (flags["tongue"] as? String) ?? "  "
    let cowFile = (flags["cowfile"] as? String) ?? "default"
    let noWrap = (flags["nowrap"] as? Bool) == true
    let think = (flags["think"] as? Bool) == true

    let width = (flags["width"] as? Int).map(clampWidth) ?? 40

    let activeModes = modeFlagIds.filter { (flags[$0] as? Bool) == true }

    return CowsayInvocation(
        message: message, eyes: eyes, tongue: tongue, activeModes: activeModes,
        noWrap: noWrap, width: width, think: think, cowFile: cowFile
    )
}

private func clampWidth(_ value: Int) -> Int {
    value < 1 ? 1 : value
}

// MARK: - CliValue conversion

/// Unwraps a `CliValue` to its native Swift type. Kept in this file (not
/// main.swift) alongside the rest of the CLI glue so it's directly
/// unit-testable.
func unwrap(_ value: CliValue) -> Any? {
    switch value {
    case .null: return nil
    case .bool(let b): return b
    case .int(let i): return i
    case .double(let d): return d
    case .string(let s): return s
    case .array(let arr): return arr
    }
}

func flagsAsAny(_ flags: [String: CliValue]) -> [String: Any] {
    var result: [String: Any] = [:]
    for (key, value) in flags {
        if let unwrapped = unwrap(value) { result[key] = unwrapped }
    }
    return result
}

func argumentsAsStringArrays(_ arguments: [String: CliValue]) -> [String: [String]] {
    var result: [String: [String]] = [:]
    for (key, value) in arguments {
        if case .array(let items) = value {
            result[key] = items.map(\.description)
        }
    }
    return result
}
