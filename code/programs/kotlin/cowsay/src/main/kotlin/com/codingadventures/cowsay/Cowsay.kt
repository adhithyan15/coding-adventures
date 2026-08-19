/**
 * cowsay — routed through paint-vm-ascii (Kotlin port).
 *
 * Sixth language in the cowsay-through-paint-vm-ascii rollout (after
 * csharp, fsharp, perl, haskell, java). Everything up through composing
 * the bubble+cow text block is ordinary string formatting, ported
 * unchanged from the reference implementation at
 * `code/programs/go/cowsay/main.go`. The one thing that's different from
 * that reference: instead of printing the composed text directly,
 * [buildScene] converts it into a `PaintScene` of `glyph_run` instructions
 * (one glyph placement per non-space character, positioned on an 8x16
 * character grid), and [render] turns that scene back into the terminal
 * string we print. This is also the PR that built `kotlin/paint-vm-ascii`
 * from scratch, implementing the full P2D02 contract — see that package's
 * own CHANGELOG.
 */
package com.codingadventures.cowsay

import com.codingadventures.paintinstructions.PaintGlyphPlacement
import com.codingadventures.paintinstructions.PaintInstruction
import com.codingadventures.paintinstructions.PaintScene
import com.codingadventures.paintinstructions.paintGlyphRun
import com.codingadventures.paintvmascii.AsciiOptions
import com.codingadventures.paintvmascii.PaintVmAsciiResult
import com.codingadventures.paintvmascii.render as renderAscii
import java.io.IOException
import java.nio.file.Files
import java.nio.file.InvalidPathException
import java.nio.file.Path
import kotlin.math.max

/** paint-vm-ascii's documented default scale factors (`P2D02-paint-vm-ascii.md`). */
const val SCALE_X = 8.0
const val SCALE_Y = 16.0

/** The resolved set of inputs needed to render one cowsay invocation. */
data class CowsayInvocation(
    val message: String,
    val eyes: String,
    val tongue: String,
    val activeModes: List<String>,
    val noWrap: Boolean,
    val width: Int,
    val think: Boolean,
    val cowFile: String,
)

data class EyesAndTongue(val eyes: String, val tongue: String)

private data class ModeOverride(val eyes: String, val tongue: String?)

private val MODE_OVERRIDES: Map<String, ModeOverride> = linkedMapOf(
    "borg" to ModeOverride("==", null),
    "dead" to ModeOverride("XX", "U "),
    "greedy" to ModeOverride("$$", null),
    "paranoid" to ModeOverride("@@", null),
    "stoned" to ModeOverride("xx", "U "),
    "tired" to ModeOverride("--", null),
    "wired" to ModeOverride("OO", null),
    "youthful" to ModeOverride("..", null),
)

val MODE_FLAG_IDS: List<String> = MODE_OVERRIDES.keys.toList()

// ---------------------------------------------------------------------------
// Rendering core (ported from code/programs/go/cowsay/main.go)
// ---------------------------------------------------------------------------

/**
 * Splits text into lines no longer than [width], breaking on word
 * boundaries. A single word longer than the width is kept whole (never
 * split mid-word).
 */
fun wrapText(text: String, width: Int): List<String> {
    if (text.length <= width) return listOf(text)

    val words = text.split(" ").filter { it.isNotEmpty() }
    if (words.isEmpty()) return listOf("")

    val lines = mutableListOf<String>()
    var current = StringBuilder()
    for (word in words) {
        if (current.length + word.length + 1 <= width) {
            if (current.isNotEmpty()) current.append(' ')
            current.append(word)
        } else {
            if (current.isNotEmpty()) lines.add(current.toString())
            current = StringBuilder(word)
        }
    }
    if (current.isNotEmpty()) lines.add(current.toString())
    return lines
}

/**
 * Draws the speech/thought bubble around the given lines. A single line
 * gets `"< ... >"` (or `"( ... )"` for a thought bubble); multiple lines
 * get `"/ ... \"`, `"| ... |"`, `"\ ... /"` (or `"( ... )"` on every line
 * for a thought bubble).
 */
fun formatBubble(lines: List<String>, isThink: Boolean): String {
    if (lines.isEmpty()) return ""

    val maxLen = lines.maxOf { it.length }
    val borderTop = " " + "_".repeat(maxLen + 2)
    val borderBottom = " " + "-".repeat(maxLen + 2)

    val body = if (lines.size == 1) {
        val (start, end) = if (isThink) "(" to ")" else "<" to ">"
        listOf("$start ${lines[0].padEnd(maxLen)} $end")
    } else {
        val n = lines.size
        lines.mapIndexed { i, line ->
            val (start, end) = when {
                isThink -> "(" to ")"
                i == 0 -> "/" to "\\"
                i == n - 1 -> "\\" to "/"
                else -> "|" to "|"
            }
            "$start ${line.padEnd(maxLen)} $end"
        }
    }

    return (listOf(borderTop) + body + listOf(borderBottom)).joinToString("\n")
}

/**
 * Pads or truncates a mode string (eyes/tongue) to exactly two characters,
 * matching cowsay's convention that eyes/tongue are always a 2-char glyph.
 */
fun normalizeTwoChars(value: String): String = when {
    value.length < 2 -> value + " ".repeat(2 - value.length)
    value.length > 2 -> value.substring(0, 2)
    else -> value
}

/**
 * Applies mode shortcuts (--borg, --dead, etc.) on top of the base
 * eyes/tongue flag values, then normalizes both to two characters. Modes
 * are mutually exclusive per cowsay.json, but this accepts any set for
 * robustness.
 */
fun resolveEyesAndTongue(baseEyes: String, baseTongue: String, activeModes: List<String>): EyesAndTongue {
    var eyes = baseEyes
    var tongue = baseTongue
    for (mode in activeModes) {
        val override = MODE_OVERRIDES[mode] ?: continue
        eyes = override.eyes
        if (override.tongue != null) tongue = override.tongue
    }
    return EyesAndTongue(normalizeTwoChars(eyes), normalizeTwoChars(tongue))
}

/**
 * Walks up from [startDir] looking for CLAUDE.md, the repo-root sentinel
 * file. CLAUDE.md (not code/specs/cowsay.json itself) is used
 * deliberately — it's a more robust marker than reaching for the very
 * file being located, and this exact fix was called out as a lesson from
 * a prior, reverted cowsay Lua port's CI pathing problems (PR #1535).
 */
fun findRepoRoot(startDir: Path): Path {
    var dir = startDir
    repeat(24) {
        if (Files.exists(dir.resolve("CLAUDE.md"))) return dir
        dir = dir.parent ?: return startDir
    }
    return startDir
}

private val COW_BODY_PATTERN = Regex("<<EOC;\\n(.*?)EOC", RegexOption.DOT_MATCHES_ALL)

/**
 * Loads a .cow template's body from [cowsDir], falling back to
 * default.cow when the requested file doesn't exist. The template is a
 * Perl heredoc (`$the_cow = <<EOC; ... EOC`); only the body between the
 * heredoc markers is returned.
 *
 * [cowName] comes from the user-supplied -f/--file flag, so it is treated
 * as untrusted: only a bare filename (no directory separators, no
 * rooted/absolute path) is accepted, and the resolved path is verified to
 * stay inside [cowsDir] before it's read — otherwise this falls back to
 * default.cow instead of reading an arbitrary file the caller pointed at
 * via `".."`, a rooted override, or similar (mirrors the fix applied to
 * every other port's loadCow after `/security-review`). A malformed
 * [cowName] that [Path.of] cannot even parse (e.g. an embedded NUL byte)
 * is treated the same as a rooted path: rejected outright.
 */
fun loadCow(cowName: String, cowsDir: Path): String {
    val cowsRoot = cowsDir.toAbsolutePath().normalize()

    val (safeName, rooted) = try {
        val parsed = Path.of(cowName)
        (parsed.fileName?.toString() ?: "") to parsed.isAbsolute
    } catch (e: InvalidPathException) {
        "" to true
    }

    val candidate = if (safeName.isNotEmpty() && !rooted) {
        try {
            cowsRoot.resolve("$safeName.cow").toAbsolutePath().normalize()
        } catch (e: InvalidPathException) {
            null
        }
    } else {
        null
    }

    val withinCowsDir = candidate != null && candidate.startsWith(cowsRoot)
    val cowPath = if (candidate != null && withinCowsDir && Files.exists(candidate)) {
        candidate
    } else {
        cowsRoot.resolve("default.cow")
    }

    val contents = Files.readString(cowPath)
    val match = COW_BODY_PATTERN.find(contents)
    return match?.groupValues?.get(1) ?: contents
}

/**
 * Composes the full bubble+cow text block for one invocation —
 * everything up to (but not including) the paint-vm-ascii render step.
 */
fun composeContent(invocation: CowsayInvocation, cowsDir: Path): String {
    val eyesAndTongue = resolveEyesAndTongue(invocation.eyes, invocation.tongue, invocation.activeModes)

    val lines = mutableListOf<String>()
    for (rawLine in invocation.message.split("\n")) {
        when {
            rawLine.isEmpty() -> lines.add("")
            invocation.noWrap -> lines.add(rawLine)
            else -> lines.addAll(wrapText(rawLine, invocation.width))
        }
    }

    val thoughts = if (invocation.think) "o" else "\\"
    val bubble = formatBubble(lines, invocation.think)

    val cowTemplate = loadCow(invocation.cowFile, cowsDir)
    val cow = cowTemplate
        .replace("\$eyes", eyesAndTongue.eyes)
        .replace("\$tongue", eyesAndTongue.tongue)
        .replace("\$thoughts", thoughts)
        .replace("\\\\", "\\")

    return "$bubble\n$cow"
}

/**
 * Converts a composed text block into a [PaintScene]: one `glyph_run`
 * instruction per line, one glyph placement per non-space character. See
 * `code/specs/cowsay-paintvm-pipeline.md` §3 for the full contract,
 * including why glyphId is a literal Unicode code point here (an
 * ASCII-backend-only relaxation of the general PaintGlyphRun contract).
 */
fun buildScene(text: String): PaintScene {
    val normalized = text.replace("\r\n", "\n")
    val lines = normalized.split("\n")

    val maxWidth = lines.maxOfOrNull { it.length } ?: 0

    val instructions = mutableListOf<PaintInstruction>()
    for ((row, line) in lines.withIndex()) {
        val glyphs = line.mapIndexedNotNull { col, ch ->
            if (ch == ' ') null else PaintGlyphPlacement(ch.code, col * SCALE_X, row * SCALE_Y)
        }
        if (glyphs.isNotEmpty()) {
            instructions.add(paintGlyphRun(glyphs, "terminal-mono", SCALE_Y, "#000000"))
        }
    }

    val width = (max(1, maxWidth) * SCALE_X).toInt()
    val height = (max(1, lines.size) * SCALE_Y).toInt()
    return PaintScene(width, height, "transparent", instructions)
}

/**
 * End-to-end: compose the bubble+cow text, build a [PaintScene] from it,
 * and render that scene through paint-vm-ascii.
 */
fun render(invocation: CowsayInvocation, cowsDir: Path): PaintVmAsciiResult {
    val content = composeContent(invocation, cowsDir)
    val scene = buildScene(content)
    return renderAscii(scene, AsciiOptions(SCALE_X.toInt(), SCALE_Y.toInt()))
}

// ---------------------------------------------------------------------------
// CLI glue — the bridge between CliBuilder's flags/arguments maps and this
// module's typed invocation. Kept in this file (rather than Main.kt) so
// it's directly unit-testable without spawning a process or driving a real
// Parser.
// ---------------------------------------------------------------------------

fun isListRequested(flags: Map<String, Any?>): Boolean = flags["list"] == true

/** Cow file basenames under [cowsDir], sorted ordinally. */
fun listCowFiles(cowsDir: Path): List<String> =
    Files.list(cowsDir).use { entries ->
        entries
            .filter { it.fileName.toString().endsWith(".cow") }
            .map { it.fileName.toString().removeSuffix(".cow") }
            .sorted()
            .toList()
    }

/**
 * Resolves the message from the parsed "message" positional argument.
 * Returns null when no message was given on argv — the caller should fall
 * back to stdin.
 */
fun resolveMessageFromArguments(arguments: Map<String, Any?>): String? {
    val parts = arguments["message"] as? List<*>
    if (parts.isNullOrEmpty()) return null
    return parts.joinToString(" ") { it.toString() }
}

/**
 * Builds a [CowsayInvocation] from a resolved message and the parsed
 * flags map, applying cowsay.json's documented defaults for any flag that
 * wasn't explicitly set.
 */
fun buildInvocation(message: String, flags: Map<String, Any?>): CowsayInvocation {
    val eyes = flags["eyes"] as? String ?: "oo"
    val tongue = flags["tongue"] as? String ?: "  "
    val cowFile = flags["cowfile"] as? String ?: "default"
    val noWrap = flags["nowrap"] == true
    val think = flags["think"] == true

    val width = (flags["width"] as? Number)?.toLong()?.let(::clampWidth) ?: 40

    val activeModes = MODE_FLAG_IDS.filter { flags[it] == true }

    return CowsayInvocation(message, eyes, tongue, activeModes, noWrap, width, think, cowFile)
}

private fun clampWidth(value: Long): Int = when {
    value < 1 -> 1
    value > Int.MAX_VALUE -> Int.MAX_VALUE
    else -> value.toInt()
}
