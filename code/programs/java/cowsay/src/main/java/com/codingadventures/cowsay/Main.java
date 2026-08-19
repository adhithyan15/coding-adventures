package com.codingadventures.cowsay;

import com.codingadventures.clibuilder.CliBuilderError;
import com.codingadventures.clibuilder.HelpResult;
import com.codingadventures.clibuilder.ParseOutcome;
import com.codingadventures.clibuilder.ParseResult;
import com.codingadventures.clibuilder.Parser;
import com.codingadventures.clibuilder.VersionResult;
import com.codingadventures.paintvmascii.PaintVmAsciiResult;

import java.io.IOException;
import java.io.InputStreamReader;
import java.io.PrintStream;
import java.nio.charset.StandardCharsets;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.List;
import java.util.Map;

/**
 * cowsay (Java) — entry point.
 *
 * <p>Thin CLI wiring: load and parse {@code code/specs/cowsay.json} via
 * CliBuilder, resolve the parsed flags/arguments into a {@link
 * Cowsay.CowsayInvocation}, and hand off to {@link Cowsay#render} for the
 * actual formatting + paint-vm-ascii render. See {@code
 * code/specs/cowsay-paintvm-pipeline.md} for the design.
 *
 * <p>CliBuilder's {@link Parser#parse()} follows the C/Go argv convention
 * where index 0 is the program name ({@code argv.getFirst()} in {@code
 * Parser.parse()}); Java's {@code String[] args} passed to {@code main}
 * does NOT include the program name — passing it straight through would
 * silently drop the first real CLI token, the same pitfall documented for
 * every other port (see lessons.md, "C#" and "Haskell" sections).
 *
 * <p>Explicitly forces UTF-8 encoding and LF-only line endings on stdout
 * and stderr, rather than relying on JVM/platform defaults — matching the
 * fix applied to the Haskell port after its own default-encoding/newline
 * surprises on Windows (see lessons.md, "Haskell" section).
 */
public final class Main {

    private Main() {}

    public static void main(String[] args) {
        PrintStream out = new PrintStream(System.out, true, StandardCharsets.UTF_8);
        PrintStream err = new PrintStream(System.err, true, StandardCharsets.UTF_8);

        Path repoRoot = Cowsay.findRepoRoot(Path.of("").toAbsolutePath());
        Path specPath = repoRoot.resolve("code").resolve("specs").resolve("cowsay.json");
        Path cowsDir = repoRoot.resolve("code").resolve("specs").resolve("cows");

        List<String> argv = new ArrayList<>();
        argv.add("cowsay");
        argv.addAll(List.of(args));

        try {
            ParseOutcome outcome = new Parser(specPath, argv).parse();
            switch (outcome) {
                case HelpResult help -> out.print(help.text() + "\n");
                case VersionResult version -> out.print(version.version() + "\n");
                case ParseResult parseResult -> run(parseResult, cowsDir, out, err);
                default -> throw new AssertionError("unreachable: unknown ParseOutcome " + outcome);
            }
        } catch (CliBuilderError error) {
            err.println(error.getMessage());
            System.exit(1);
        } catch (IOException error) {
            // Reading the CLI spec, a .cow template, or the cows directory
            // listing can all fail with an IOException (missing file,
            // permissions, a broken repo-root discovery). Report it the
            // same way a CliBuilderError is reported, rather than letting a
            // raw stack trace reach the user.
            err.println(error.getMessage());
            System.exit(1);
        }
    }

    private static void run(ParseResult result, Path cowsDir, PrintStream out, PrintStream err) throws IOException {
        Map<String, Object> flags = result.flags();
        Map<String, Object> arguments = result.arguments();

        if (Cowsay.isListRequested(flags)) {
            for (String name : Cowsay.listCowFiles(cowsDir)) {
                out.print(name + "\n");
            }
            return;
        }

        String message = Cowsay.resolveMessageFromArguments(arguments).orElse(null);
        if (message == null) {
            if (System.console() != null) {
                return;
            }
            message = readAll(new InputStreamReader(System.in, StandardCharsets.UTF_8)).strip();
        }

        if (message.isEmpty()) {
            return;
        }

        Cowsay.CowsayInvocation invocation = Cowsay.buildInvocation(message, flags);
        PaintVmAsciiResult renderResult = Cowsay.render(invocation, cowsDir);
        switch (renderResult) {
            case PaintVmAsciiResult.Ok(String text) -> out.print(text + "\n");
            case PaintVmAsciiResult.Err(var renderError) -> {
                err.println(renderError);
                System.exit(1);
            }
        }
    }

    private static String readAll(InputStreamReader reader) throws IOException {
        StringBuilder builder = new StringBuilder();
        char[] buffer = new char[4096];
        int read;
        while ((read = reader.read(buffer)) != -1) {
            builder.append(buffer, 0, read);
        }
        return builder.toString();
    }
}
