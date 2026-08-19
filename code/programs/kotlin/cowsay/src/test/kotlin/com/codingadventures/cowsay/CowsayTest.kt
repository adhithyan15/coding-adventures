package com.codingadventures.cowsay

import com.codingadventures.clibuilder.ParseOutcome
import com.codingadventures.clibuilder.ParseResult
import com.codingadventures.clibuilder.Parser
import com.codingadventures.paintinstructions.PaintInstruction
import com.codingadventures.paintvmascii.PaintVmAsciiResult
import org.junit.jupiter.api.Test
import org.junit.jupiter.api.io.TempDir
import java.nio.file.Files
import java.nio.file.Path
import kotlin.test.assertEquals
import kotlin.test.assertIs
import kotlin.test.assertTrue

class CowsayTest {

    private fun writeCow(dir: Path, name: String, contents: String) {
        Files.writeString(dir.resolve("$name.cow"), contents)
    }

    private fun resolveRepoRoot(): Path = findRepoRoot(Path.of("").toAbsolutePath())

    // -------------------------------------------------------------------
    // wrapText
    // -------------------------------------------------------------------

    @Test
    fun `does not wrap short text`() {
        assertEquals(listOf("hello"), wrapText("hello", 40))
    }

    @Test
    fun `wraps long text at word boundaries`() {
        assertEquals(listOf("the quick", "brown fox", "jumps over"), wrapText("the quick brown fox jumps over", 10))
    }

    @Test
    fun `returns an empty line for empty text`() {
        assertEquals(listOf(""), wrapText("", 40))
    }

    @Test
    fun `keeps a single word longer than the width whole`() {
        assertEquals(listOf("supercalifragilisticexpialidocious"), wrapText("supercalifragilisticexpialidocious", 5))
    }

    // -------------------------------------------------------------------
    // formatBubble
    // -------------------------------------------------------------------

    @Test
    fun `returns empty string for no lines`() {
        assertEquals("", formatBubble(emptyList(), false))
    }

    @Test
    fun `draws a single-line speech bubble`() {
        assertEquals(" ____\n< hi >\n ----", formatBubble(listOf("hi"), false))
    }

    @Test
    fun `draws a single-line thought bubble`() {
        assertEquals(" ____\n( hi )\n ----", formatBubble(listOf("hi"), true))
    }

    @Test
    fun `draws a multi-line speech bubble with slash pipe backslash borders`() {
        assertEquals(
            " _______\n/ one   \\\n| two   |\n\\ three /\n -------",
            formatBubble(listOf("one", "two", "three"), false),
        )
    }

    @Test
    fun `draws a multi-line thought bubble with parens on every line`() {
        assertEquals(" _____\n( one )\n( two )\n -----", formatBubble(listOf("one", "two"), true))
    }

    // -------------------------------------------------------------------
    // normalizeTwoChars
    // -------------------------------------------------------------------

    @Test
    fun `pads a one-character value`() {
        assertEquals("o ", normalizeTwoChars("o"))
    }

    @Test
    fun `pads an empty value`() {
        assertEquals("  ", normalizeTwoChars(""))
    }

    @Test
    fun `leaves a two-character value unchanged`() {
        assertEquals("oo", normalizeTwoChars("oo"))
    }

    @Test
    fun `truncates a longer value`() {
        assertEquals("oo", normalizeTwoChars("ooo"))
    }

    // -------------------------------------------------------------------
    // resolveEyesAndTongue
    // -------------------------------------------------------------------

    @Test
    fun `keeps base values when no modes are active`() {
        val result = resolveEyesAndTongue("oo", "  ", emptyList())
        assertEquals("oo", result.eyes)
        assertEquals("  ", result.tongue)
    }

    @Test
    fun `borg overrides eyes only`() {
        val result = resolveEyesAndTongue("oo", "  ", listOf("borg"))
        assertEquals("==", result.eyes)
        assertEquals("  ", result.tongue)
    }

    @Test
    fun `dead overrides both eyes and tongue`() {
        val result = resolveEyesAndTongue("oo", "  ", listOf("dead"))
        assertEquals("XX", result.eyes)
        assertEquals("U ", result.tongue)
    }

    @Test
    fun `stoned overrides both eyes and tongue`() {
        val result = resolveEyesAndTongue("oo", "  ", listOf("stoned"))
        assertEquals("xx", result.eyes)
        assertEquals("U ", result.tongue)
    }

    @Test
    fun `ignores an unknown mode`() {
        val result = resolveEyesAndTongue("oo", "  ", listOf("not-a-real-mode"))
        assertEquals("oo", result.eyes)
        assertEquals("  ", result.tongue)
    }

    // -------------------------------------------------------------------
    // loadCow
    // -------------------------------------------------------------------

    @Test
    fun `loads the body between heredoc markers`(@TempDir dir: Path) {
        writeCow(dir, "default", "\$the_cow = <<EOC;\n  \$thoughts   ^__^\n   (\$eyes)\nEOC\n")
        assertEquals("  \$thoughts   ^__^\n   (\$eyes)\n", loadCow("default", dir))
    }

    @Test
    fun `falls back to default cow when the named cow is missing`(@TempDir dir: Path) {
        writeCow(dir, "default", "\$the_cow = <<EOC;\nfallback\nEOC\n")
        assertEquals("fallback\n", loadCow("does-not-exist", dir))
    }

    @Test
    fun `falls back to default cow instead of escaping via traversal`(@TempDir dir: Path, @TempDir outsideDir: Path) {
        writeCow(dir, "default", "\$the_cow = <<EOC;\nfallback\nEOC\n")
        writeCow(outsideDir, "secret", "\$the_cow = <<EOC;\nSECRET\nEOC\n")
        writeCow(outsideDir, "outside", "\$the_cow = <<EOC;\nSECRET\nEOC\n")
        for (malicious in listOf("../../../../../../etc/passwd", "..\\..\\..\\secret", "../outside")) {
            assertEquals("fallback\n", loadCow(malicious, dir), "for input: $malicious")
        }
    }

    @Test
    fun `falls back to default cow instead of following a rooted path override`(@TempDir dir: Path, @TempDir outsideDir: Path) {
        writeCow(dir, "default", "\$the_cow = <<EOC;\nfallback\nEOC\n")
        val rootedTarget = outsideDir.resolve("win.cow")
        Files.writeString(rootedTarget, "\$the_cow = <<EOC;\nSECRET\nEOC\n")
        val rootedName = outsideDir.resolve("win").toString()
        assertEquals("fallback\n", loadCow(rootedName, dir))
    }

    // -------------------------------------------------------------------
    // composeContent
    // -------------------------------------------------------------------

    private fun baseInvocation() = CowsayInvocation("hi", "oo", "  ", emptyList(), false, 40, false, "default")

    @Test
    fun `composes bubble and cow with substitutions`(@TempDir dir: Path) {
        writeCow(dir, "default", "\$the_cow = <<EOC;\n\$thoughts \$eyes \$tongue\nEOC\n")
        assertEquals(" ____\n< hi >\n ----\n\\ oo   \n", composeContent(baseInvocation(), dir))
    }

    @Test
    fun `think mode uses o for thoughts and a paren bubble`(@TempDir dir: Path) {
        writeCow(dir, "default", "\$the_cow = <<EOC;\n\$thoughts \$eyes \$tongue\nEOC\n")
        val invocation = baseInvocation().copy(think = true)
        assertEquals(" ____\n( hi )\n ----\no oo   \n", composeContent(invocation, dir))
    }

    @Test
    fun `a mode flag overrides eyes and tongue in the cow template`(@TempDir dir: Path) {
        writeCow(dir, "default", "\$the_cow = <<EOC;\n\$thoughts \$eyes \$tongue\nEOC\n")
        val invocation = baseInvocation().copy(activeModes = listOf("dead"))
        assertEquals(" ____\n< hi >\n ----\n\\ XX U \n", composeContent(invocation, dir))
    }

    // -------------------------------------------------------------------
    // buildScene
    // -------------------------------------------------------------------

    @Test
    fun `creates one glyph_run per non-blank line with correct placements`() {
        val scene = buildScene("hi\n\nyo")
        val runs = scene.instructions.filterIsInstance<PaintInstruction.PaintGlyphRun>()
        assertEquals(2, runs.size)
        assertEquals(listOf('h'.code, 'i'.code), runs[0].glyphs.map { it.glyphId })
        assertEquals(listOf(0.0, SCALE_X), runs[0].glyphs.map { it.x })
        assertEquals(listOf('y'.code, 'o'.code), runs[1].glyphs.map { it.glyphId })
        assertEquals(listOf(2 * SCALE_Y, 2 * SCALE_Y), runs[1].glyphs.map { it.y })
    }

    @Test
    fun `skips spaces rather than placing them`() {
        val scene = buildScene("a b")
        val runs = scene.instructions.filterIsInstance<PaintInstruction.PaintGlyphRun>()
        assertEquals(1, runs.size)
        assertEquals(2, runs[0].glyphs.size)
    }

    @Test
    fun `covers all lines in the scene dimensions`() {
        val scene = buildScene("abc\nde")
        assertEquals((3 * SCALE_X).toInt(), scene.width)
        assertEquals((2 * SCALE_Y).toInt(), scene.height)
    }

    // -------------------------------------------------------------------
    // render round trip
    // -------------------------------------------------------------------

    @Test
    fun `round-trips simple single-line text`() {
        val scene = buildScene("hi")
        val result = com.codingadventures.paintvmascii.render(scene, com.codingadventures.paintvmascii.AsciiOptions(SCALE_X.toInt(), SCALE_Y.toInt()))
        val ok = assertIs<PaintVmAsciiResult.Ok>(result)
        assertEquals("hi", ok.text)
    }

    @Test
    fun `round-trips a bubble and cow block trimming the trailing blank line`() {
        val content = " ____\n< hi >\n ----\n\\   ^__^\n"
        val scene = buildScene(content)
        val result = com.codingadventures.paintvmascii.render(scene, com.codingadventures.paintvmascii.AsciiOptions(SCALE_X.toInt(), SCALE_Y.toInt()))
        val ok = assertIs<PaintVmAsciiResult.Ok>(result)
        assertEquals(" ____\n< hi >\n ----\n\\   ^__^", ok.text)
    }

    // -------------------------------------------------------------------
    // CLI glue
    // -------------------------------------------------------------------

    @Test
    fun `isListRequested is true when the flag is present`() {
        assertTrue(isListRequested(mapOf("list" to true)))
    }

    @Test
    fun `isListRequested is false when the flag is absent`() {
        assertEquals(false, isListRequested(emptyMap()))
    }

    @Test
    fun `resolveMessageFromArguments joins positional words`() {
        assertEquals("hello there", resolveMessageFromArguments(mapOf("message" to listOf("hello", "there"))))
    }

    @Test
    fun `resolveMessageFromArguments returns null when arguments is empty`() {
        assertEquals(null, resolveMessageFromArguments(emptyMap()))
    }

    @Test
    fun `resolveMessageFromArguments returns null when the message list is empty`() {
        assertEquals(null, resolveMessageFromArguments(mapOf("message" to emptyList<Any?>())))
    }

    @Test
    fun `buildInvocation uses defaults when no flags are set`() {
        val invocation = buildInvocation("hi", emptyMap())
        assertEquals("hi", invocation.message)
        assertEquals("oo", invocation.eyes)
        assertEquals("  ", invocation.tongue)
        assertEquals("default", invocation.cowFile)
        assertEquals(false, invocation.noWrap)
        assertEquals(false, invocation.think)
        assertEquals(40, invocation.width)
        assertTrue(invocation.activeModes.isEmpty())
    }

    @Test
    fun `buildInvocation honors explicit flags`() {
        val flags = mapOf(
            "eyes" to "^^",
            "tongue" to "vv",
            "cowfile" to "dragon",
            "nowrap" to true,
            "think" to true,
            "width" to 20L,
            "borg" to true,
        )
        val invocation = buildInvocation("hi", flags)
        assertEquals("^^", invocation.eyes)
        assertEquals("vv", invocation.tongue)
        assertEquals("dragon", invocation.cowFile)
        assertTrue(invocation.noWrap)
        assertTrue(invocation.think)
        assertEquals(20, invocation.width)
        assertEquals(listOf("borg"), invocation.activeModes)
    }

    @Test
    fun `buildInvocation clamps a very large width and rejects a negative width`() {
        assertEquals(Int.MAX_VALUE, buildInvocation("hi", mapOf("width" to 99_999_999_999L)).width)
        assertEquals(1, buildInvocation("hi", mapOf("width" to -5L)).width)
    }

    @Test
    fun `listCowFiles returns sorted basenames`(@TempDir dir: Path) {
        writeCow(dir, "tux", "")
        writeCow(dir, "default", "")
        writeCow(dir, "dragon", "")
        assertEquals(listOf("default", "dragon", "tux"), listCowFiles(dir))
    }

    // -------------------------------------------------------------------
    // CliBuilder argv convention
    // -------------------------------------------------------------------

    // Regression test: this Kotlin CliBuilder's Parser.parse() DOES expect
    // a leading program-name placeholder (it reads argv.first() as the
    // program name and errors if argv is empty), matching the C/Go
    // convention. Verified against the real Parser, not just hand-built
    // flags/arguments maps.
    @Test
    fun `does not drop the first token when a program-name placeholder is prepended`() {
        val repoRoot = resolveRepoRoot()
        val specPath = repoRoot.resolve("code").resolve("specs").resolve("cowsay.json")

        val outcome1: ParseOutcome = Parser(specPath.toString(), listOf("cowsay", "hello")).parse()
        val result1 = assertIs<ParseResult>(outcome1)
        assertEquals("hello", resolveMessageFromArguments(result1.arguments))

        val outcome2: ParseOutcome = Parser(specPath.toString(), listOf("cowsay", "hello", "world")).parse()
        val result2 = assertIs<ParseResult>(outcome2)
        assertEquals("hello world", resolveMessageFromArguments(result2.arguments))
    }

    // -------------------------------------------------------------------
    // end-to-end golden output
    // -------------------------------------------------------------------

    @Test
    fun `resolves the real cows directory`() {
        val repoRoot = resolveRepoRoot()
        val cowsDir = repoRoot.resolve("code").resolve("specs").resolve("cows")
        assertTrue(listCowFiles(cowsDir).contains("default"))
    }

    @Test
    fun `default cow speaking Hello World`() {
        val repoRoot = resolveRepoRoot()
        val cowsDir = repoRoot.resolve("code").resolve("specs").resolve("cows")
        val invocation = CowsayInvocation("Hello, World!", "oo", "  ", emptyList(), false, 40, false, "default")
        val result = render(invocation, cowsDir)
        val ok = assertIs<PaintVmAsciiResult.Ok>(result)
        assertEquals(
            listOf(
                " _______________",
                "< Hello, World! >",
                " ---------------",
                "        \\   ^__^",
                "         \\  (oo)\\_______",
                "            (__)\\       )\\/\\",
                "                ||----w |",
                "                ||     ||",
            ).joinToString("\n"),
            ok.text,
        )
    }

    @Test
    fun `borg mode thinking with the default cow`() {
        val repoRoot = resolveRepoRoot()
        val cowsDir = repoRoot.resolve("code").resolve("specs").resolve("cows")
        val invocation = CowsayInvocation("beep", "oo", "  ", listOf("borg"), false, 40, true, "default")
        val result = render(invocation, cowsDir)
        val ok = assertIs<PaintVmAsciiResult.Ok>(result)
        assertEquals(
            listOf(
                " ______",
                "( beep )",
                " ------",
                "        o   ^__^",
                "         o  (==)\\_______",
                "            (__)\\       )\\/\\",
                "                ||----w |",
                "                ||     ||",
            ).joinToString("\n"),
            ok.text,
        )
    }
}
