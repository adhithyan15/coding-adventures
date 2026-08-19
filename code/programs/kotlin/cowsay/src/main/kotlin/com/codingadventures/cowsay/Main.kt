/**
 * cowsay (Kotlin) — entry point.
 *
 * Thin CLI wiring: load and parse `code/specs/cowsay.json` via
 * CliBuilder, resolve the parsed flags/arguments into a
 * [CowsayInvocation], and hand off to [render] for the actual formatting
 * + paint-vm-ascii render. See `code/specs/cowsay-paintvm-pipeline.md`
 * for the design.
 *
 * CliBuilder's `Parser.parse()` follows the C/Go argv convention where
 * index 0 is the program name (`argv.first()` in `Parser.parse()`);
 * Kotlin's `args: Array<String>` passed to `main` does NOT include the
 * program name — passing it straight through would silently drop the
 * first real CLI token, the same pitfall documented for every other port
 * (see lessons.md, "C#"/"Haskell" sections).
 *
 * Explicitly forces UTF-8 encoding and LF-only line endings on stdout and
 * stderr, rather than relying on JVM/platform defaults — matching the fix
 * applied to the Haskell and Java ports after their own
 * default-encoding/newline surprises on Windows (see lessons.md,
 * "Haskell" section).
 */
package com.codingadventures.cowsay

import com.codingadventures.clibuilder.CliBuilderError
import com.codingadventures.clibuilder.HelpResult
import com.codingadventures.clibuilder.ParseOutcome
import com.codingadventures.clibuilder.ParseResult
import com.codingadventures.clibuilder.Parser
import com.codingadventures.clibuilder.VersionResult
import com.codingadventures.paintvmascii.PaintVmAsciiResult
import java.io.IOException
import java.io.PrintStream
import java.io.UncheckedIOException
import java.nio.charset.StandardCharsets
import java.nio.file.Path
import kotlin.system.exitProcess

fun main(args: Array<String>) {
    val out = PrintStream(System.out, true, StandardCharsets.UTF_8)
    val err = PrintStream(System.err, true, StandardCharsets.UTF_8)

    val repoRoot = findRepoRoot(Path.of("").toAbsolutePath())
    val specPath = repoRoot.resolve("code").resolve("specs").resolve("cowsay.json")
    val cowsDir = repoRoot.resolve("code").resolve("specs").resolve("cows")

    val argv = listOf("cowsay") + args.toList()

    try {
        when (val outcome: ParseOutcome = Parser(specPath.toString(), argv).parse()) {
            is HelpResult -> out.print(outcome.text + "\n")
            is VersionResult -> out.print(outcome.version + "\n")
            is ParseResult -> run(outcome, cowsDir, out, err)
        }
    } catch (error: CliBuilderError) {
        err.println(error.message)
        exitProcess(1)
    } catch (error: IOException) {
        // Reading the CLI spec, a .cow template, or the cows directory
        // listing can all fail with an IOException (missing file,
        // permissions, a broken repo-root discovery). Report it the same
        // way a CliBuilderError is reported, rather than letting a raw
        // stack trace reach the user.
        err.println(error.message)
        exitProcess(1)
    } catch (error: UncheckedIOException) {
        // Files.list()'s Stream (used by listCowFiles) can throw this
        // during iteration instead of a plain IOException — not a subtype
        // of IOException, so it needs its own catch to get the same clean
        // error reporting rather than a raw stack trace.
        err.println(error.message)
        exitProcess(1)
    }
}

private fun run(result: ParseResult, cowsDir: Path, out: PrintStream, err: PrintStream) {
    val flags = result.flags
    val arguments = result.arguments

    if (isListRequested(flags)) {
        for (name in listCowFiles(cowsDir)) {
            out.print(name + "\n")
        }
        return
    }

    var message = resolveMessageFromArguments(arguments)
    if (message == null) {
        if (System.console() != null) return
        message = System.`in`.readBytes().toString(StandardCharsets.UTF_8).trim()
    }

    if (message.isEmpty()) return

    val invocation = buildInvocation(message, flags)
    when (val renderResult = render(invocation, cowsDir)) {
        is PaintVmAsciiResult.Ok -> out.print(renderResult.text + "\n")
        is PaintVmAsciiResult.Err -> {
            err.println(renderResult.error)
            exitProcess(1)
        }
    }
}
