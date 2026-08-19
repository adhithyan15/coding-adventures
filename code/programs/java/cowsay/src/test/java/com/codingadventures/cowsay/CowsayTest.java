package com.codingadventures.cowsay;

import com.codingadventures.clibuilder.ParseOutcome;
import com.codingadventures.clibuilder.ParseResult;
import com.codingadventures.clibuilder.Parser;
import com.codingadventures.paintinstructions.PaintInstruction;
import com.codingadventures.paintinstructions.PaintScene;
import com.codingadventures.paintvmascii.AsciiOptions;
import com.codingadventures.paintvmascii.PaintVmAscii;
import com.codingadventures.paintvmascii.PaintVmAsciiResult;
import org.junit.jupiter.api.DisplayName;
import org.junit.jupiter.api.Nested;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;

import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;

import static org.junit.jupiter.api.Assertions.*;

class CowsayTest {

    private static void writeCow(Path dir, String name, String contents) throws IOException {
        Files.writeString(dir.resolve(name + ".cow"), contents);
    }

    private static Path resolveRepoRoot() {
        return Cowsay.findRepoRoot(Path.of("").toAbsolutePath());
    }

    @Nested
    @DisplayName("wrapText")
    class WrapTextTests {
        @Test
        @DisplayName("does not wrap short text")
        void doesNotWrapShortText() {
            assertEquals(List.of("hello"), Cowsay.wrapText("hello", 40));
        }

        @Test
        @DisplayName("wraps long text at word boundaries")
        void wrapsAtWordBoundaries() {
            assertEquals(
                    List.of("the quick", "brown fox", "jumps over"),
                    Cowsay.wrapText("the quick brown fox jumps over", 10));
        }

        @Test
        @DisplayName("returns an empty line for empty text")
        void emptyText() {
            assertEquals(List.of(""), Cowsay.wrapText("", 40));
        }

        @Test
        @DisplayName("keeps a single word longer than the width whole")
        void keepsLongWordWhole() {
            assertEquals(
                    List.of("supercalifragilisticexpialidocious"),
                    Cowsay.wrapText("supercalifragilisticexpialidocious", 5));
        }
    }

    @Nested
    @DisplayName("formatBubble")
    class FormatBubbleTests {
        @Test
        @DisplayName("returns empty string for no lines")
        void noLines() {
            assertEquals("", Cowsay.formatBubble(List.of(), false));
        }

        @Test
        @DisplayName("draws a single-line speech bubble")
        void singleLineSpeech() {
            assertEquals(" ____\n< hi >\n ----", Cowsay.formatBubble(List.of("hi"), false));
        }

        @Test
        @DisplayName("draws a single-line thought bubble")
        void singleLineThought() {
            assertEquals(" ____\n( hi )\n ----", Cowsay.formatBubble(List.of("hi"), true));
        }

        @Test
        @DisplayName("draws a multi-line speech bubble with slash/pipe/backslash borders")
        void multiLineSpeech() {
            assertEquals(
                    " _______\n/ one   \\\n| two   |\n\\ three /\n -------",
                    Cowsay.formatBubble(List.of("one", "two", "three"), false));
        }

        @Test
        @DisplayName("draws a multi-line thought bubble with parens on every line")
        void multiLineThought() {
            assertEquals(
                    " _____\n( one )\n( two )\n -----",
                    Cowsay.formatBubble(List.of("one", "two"), true));
        }
    }

    @Nested
    @DisplayName("normalizeTwoChars")
    class NormalizeTwoCharsTests {
        @Test
        @DisplayName("pads a one-character value")
        void padsOneChar() {
            assertEquals("o ", Cowsay.normalizeTwoChars("o"));
        }

        @Test
        @DisplayName("pads an empty value")
        void padsEmpty() {
            assertEquals("  ", Cowsay.normalizeTwoChars(""));
        }

        @Test
        @DisplayName("leaves a two-character value unchanged")
        void leavesUnchanged() {
            assertEquals("oo", Cowsay.normalizeTwoChars("oo"));
        }

        @Test
        @DisplayName("truncates a longer value")
        void truncatesLonger() {
            assertEquals("oo", Cowsay.normalizeTwoChars("ooo"));
        }
    }

    @Nested
    @DisplayName("resolveEyesAndTongue")
    class ResolveEyesAndTongueTests {
        @Test
        @DisplayName("keeps base values when no modes are active")
        void noModes() {
            var result = Cowsay.resolveEyesAndTongue("oo", "  ", List.of());
            assertEquals("oo", result.eyes());
            assertEquals("  ", result.tongue());
        }

        @Test
        @DisplayName("borg overrides eyes only")
        void borgOverridesEyesOnly() {
            var result = Cowsay.resolveEyesAndTongue("oo", "  ", List.of("borg"));
            assertEquals("==", result.eyes());
            assertEquals("  ", result.tongue());
        }

        @Test
        @DisplayName("dead overrides both eyes and tongue")
        void deadOverridesBoth() {
            var result = Cowsay.resolveEyesAndTongue("oo", "  ", List.of("dead"));
            assertEquals("XX", result.eyes());
            assertEquals("U ", result.tongue());
        }

        @Test
        @DisplayName("stoned overrides both eyes and tongue")
        void stonedOverridesBoth() {
            var result = Cowsay.resolveEyesAndTongue("oo", "  ", List.of("stoned"));
            assertEquals("xx", result.eyes());
            assertEquals("U ", result.tongue());
        }

        @Test
        @DisplayName("ignores an unknown mode")
        void ignoresUnknownMode() {
            var result = Cowsay.resolveEyesAndTongue("oo", "  ", List.of("not-a-real-mode"));
            assertEquals("oo", result.eyes());
            assertEquals("  ", result.tongue());
        }
    }

    @Nested
    @DisplayName("loadCow")
    class LoadCowTests {
        @Test
        @DisplayName("loads the body between heredoc markers")
        void loadsBody(@TempDir Path dir) throws IOException {
            writeCow(dir, "default", "$the_cow = <<EOC;\n  $thoughts   ^__^\n   ($eyes)\nEOC\n");
            assertEquals("  $thoughts   ^__^\n   ($eyes)\n", Cowsay.loadCow("default", dir));
        }

        @Test
        @DisplayName("falls back to default.cow when the named cow is missing")
        void fallsBackWhenMissing(@TempDir Path dir) throws IOException {
            writeCow(dir, "default", "$the_cow = <<EOC;\nfallback\nEOC\n");
            assertEquals("fallback\n", Cowsay.loadCow("does-not-exist", dir));
        }

        @Test
        @DisplayName("falls back to default.cow instead of escaping via traversal")
        void fallsBackInsteadOfTraversal(@TempDir Path dir, @TempDir Path outsideDir) throws IOException {
            writeCow(dir, "default", "$the_cow = <<EOC;\nfallback\nEOC\n");
            writeCow(outsideDir, "secret", "$the_cow = <<EOC;\nSECRET\nEOC\n");
            writeCow(outsideDir, "outside", "$the_cow = <<EOC;\nSECRET\nEOC\n");
            for (String malicious : List.of(
                    "../../../../../../etc/passwd",
                    "..\\..\\..\\secret",
                    "../outside")) {
                assertEquals("fallback\n", Cowsay.loadCow(malicious, dir), "for input: " + malicious);
            }
        }

        @Test
        @DisplayName("falls back to default.cow instead of following a rooted path override")
        void fallsBackInsteadOfRootedOverride(@TempDir Path dir, @TempDir Path outsideDir) throws IOException {
            writeCow(dir, "default", "$the_cow = <<EOC;\nfallback\nEOC\n");
            Path rootedTarget = outsideDir.resolve("win.cow");
            Files.writeString(rootedTarget, "$the_cow = <<EOC;\nSECRET\nEOC\n");
            String rootedName = outsideDir.resolve("win").toString();
            assertEquals("fallback\n", Cowsay.loadCow(rootedName, dir));
        }
    }

    @Nested
    @DisplayName("composeContent")
    class ComposeContentTests {
        private static Cowsay.CowsayInvocation baseInvocation() {
            return new Cowsay.CowsayInvocation("hi", "oo", "  ", List.of(), false, 40, false, "default");
        }

        @Test
        @DisplayName("composes bubble and cow with substitutions")
        void composesWithSubstitutions(@TempDir Path dir) throws IOException {
            writeCow(dir, "default", "$the_cow = <<EOC;\n$thoughts $eyes $tongue\nEOC\n");
            String content = Cowsay.composeContent(baseInvocation(), dir);
            assertEquals(" ____\n< hi >\n ----\n\\ oo   \n", content);
        }

        @Test
        @DisplayName("think mode uses o for thoughts and a paren bubble")
        void thinkMode(@TempDir Path dir) throws IOException {
            writeCow(dir, "default", "$the_cow = <<EOC;\n$thoughts $eyes $tongue\nEOC\n");
            var invocation = new Cowsay.CowsayInvocation("hi", "oo", "  ", List.of(), false, 40, true, "default");
            String content = Cowsay.composeContent(invocation, dir);
            assertEquals(" ____\n( hi )\n ----\no oo   \n", content);
        }

        @Test
        @DisplayName("a mode flag overrides eyes (and tongue) in the cow template")
        void modeFlagOverrides(@TempDir Path dir) throws IOException {
            writeCow(dir, "default", "$the_cow = <<EOC;\n$thoughts $eyes $tongue\nEOC\n");
            var invocation = new Cowsay.CowsayInvocation("hi", "oo", "  ", List.of("dead"), false, 40, false, "default");
            String content = Cowsay.composeContent(invocation, dir);
            assertEquals(" ____\n< hi >\n ----\n\\ XX U \n", content);
        }
    }

    @Nested
    @DisplayName("buildScene")
    class BuildSceneTests {
        @Test
        @DisplayName("creates one glyph_run per non-blank line with correct placements")
        void oneGlyphRunPerLine() {
            PaintScene scene = Cowsay.buildScene("hi\n\nyo");
            List<PaintInstruction.PaintGlyphRun> runs = scene.instructions.stream()
                    .filter(PaintInstruction.PaintGlyphRun.class::isInstance)
                    .map(PaintInstruction.PaintGlyphRun.class::cast)
                    .toList();
            assertEquals(2, runs.size());
            assertEquals(List.of((int) 'h', (int) 'i'), runs.get(0).glyphs.stream().map(g -> g.glyphId).toList());
            assertEquals(List.of(0.0, Cowsay.SCALE_X), runs.get(0).glyphs.stream().map(g -> g.x).toList());
            assertEquals(List.of((int) 'y', (int) 'o'), runs.get(1).glyphs.stream().map(g -> g.glyphId).toList());
            assertEquals(
                    List.of(2 * Cowsay.SCALE_Y, 2 * Cowsay.SCALE_Y),
                    runs.get(1).glyphs.stream().map(g -> g.y).toList());
        }

        @Test
        @DisplayName("skips spaces rather than placing them")
        void skipsSpaces() {
            PaintScene scene = Cowsay.buildScene("a b");
            List<PaintInstruction.PaintGlyphRun> runs = scene.instructions.stream()
                    .filter(PaintInstruction.PaintGlyphRun.class::isInstance)
                    .map(PaintInstruction.PaintGlyphRun.class::cast)
                    .toList();
            assertEquals(1, runs.size());
            assertEquals(2, runs.get(0).glyphs.size());
        }

        @Test
        @DisplayName("covers all lines in the scene dimensions")
        void coversAllLines() {
            PaintScene scene = Cowsay.buildScene("abc\nde");
            assertEquals((int) (3 * Cowsay.SCALE_X), scene.width);
            assertEquals((int) (2 * Cowsay.SCALE_Y), scene.height);
        }
    }

    @Nested
    @DisplayName("render round trip")
    class RenderRoundTripTests {
        @Test
        @DisplayName("round-trips simple single-line text")
        void roundTripsSingleLine() {
            PaintScene scene = Cowsay.buildScene("hi");
            PaintVmAsciiResult result = PaintVmAscii.render(scene, new AsciiOptions((int) Cowsay.SCALE_X, (int) Cowsay.SCALE_Y));
            assertInstanceOf(PaintVmAsciiResult.Ok.class, result);
            assertEquals("hi", ((PaintVmAsciiResult.Ok) result).text());
        }

        @Test
        @DisplayName("round-trips multi-line text")
        void roundTripsMultiLine() {
            PaintScene scene = Cowsay.buildScene("hello\nworld");
            PaintVmAsciiResult result = PaintVmAscii.render(scene, new AsciiOptions((int) Cowsay.SCALE_X, (int) Cowsay.SCALE_Y));
            assertInstanceOf(PaintVmAsciiResult.Ok.class, result);
            assertEquals("hello\nworld", ((PaintVmAsciiResult.Ok) result).text());
        }

        @Test
        @DisplayName("round-trips a bubble+cow block, trimming the trailing blank line")
        void roundTripsBubbleAndCow() {
            String content = " ____\n< hi >\n ----\n\\   ^__^\n";
            PaintScene scene = Cowsay.buildScene(content);
            PaintVmAsciiResult result = PaintVmAscii.render(scene, new AsciiOptions((int) Cowsay.SCALE_X, (int) Cowsay.SCALE_Y));
            assertInstanceOf(PaintVmAsciiResult.Ok.class, result);
            assertEquals(" ____\n< hi >\n ----\n\\   ^__^", ((PaintVmAsciiResult.Ok) result).text());
        }
    }

    @Nested
    @DisplayName("CLI glue")
    class CliGlueTests {
        @Nested
        @DisplayName("isListRequested")
        class IsListRequestedTests {
            @Test
            @DisplayName("is true when the flag is present")
            void truePresent() {
                assertTrue(Cowsay.isListRequested(Map.of("list", Boolean.TRUE)));
            }

            @Test
            @DisplayName("is false when the flag is absent")
            void falseAbsent() {
                assertFalse(Cowsay.isListRequested(Map.of()));
            }

            @Test
            @DisplayName("is false when the flag is explicitly false")
            void falseExplicit() {
                assertFalse(Cowsay.isListRequested(Map.of("list", Boolean.FALSE)));
            }
        }

        @Nested
        @DisplayName("resolveMessageFromArguments")
        class ResolveMessageFromArgumentsTests {
            @Test
            @DisplayName("joins positional words")
            void joinsWords() {
                var result = Cowsay.resolveMessageFromArguments(Map.of("message", List.of("hello", "there")));
                assertEquals("hello there", result.orElseThrow());
            }

            @Test
            @DisplayName("returns empty when arguments is empty")
            void emptyWhenAbsent() {
                assertTrue(Cowsay.resolveMessageFromArguments(Map.of()).isEmpty());
            }

            @Test
            @DisplayName("returns empty when the message list is empty")
            void emptyWhenListEmpty() {
                assertTrue(Cowsay.resolveMessageFromArguments(Map.of("message", List.of())).isEmpty());
            }
        }

        @Nested
        @DisplayName("buildInvocation")
        class BuildInvocationTests {
            @Test
            @DisplayName("uses defaults when no flags are set")
            void usesDefaults() {
                var invocation = Cowsay.buildInvocation("hi", Map.of());
                assertEquals("hi", invocation.message());
                assertEquals("oo", invocation.eyes());
                assertEquals("  ", invocation.tongue());
                assertEquals("default", invocation.cowFile());
                assertFalse(invocation.noWrap());
                assertFalse(invocation.think());
                assertEquals(40, invocation.width());
                assertTrue(invocation.activeModes().isEmpty());
            }

            @Test
            @DisplayName("honors explicit flags")
            void honorsExplicitFlags() {
                Map<String, Object> flags = new LinkedHashMap<>();
                flags.put("eyes", "^^");
                flags.put("tongue", "vv");
                flags.put("cowfile", "dragon");
                flags.put("nowrap", Boolean.TRUE);
                flags.put("think", Boolean.TRUE);
                flags.put("width", 20L);
                flags.put("borg", Boolean.TRUE);

                var invocation = Cowsay.buildInvocation("hi", flags);
                assertEquals("^^", invocation.eyes());
                assertEquals("vv", invocation.tongue());
                assertEquals("dragon", invocation.cowFile());
                assertTrue(invocation.noWrap());
                assertTrue(invocation.think());
                assertEquals(20, invocation.width());
                assertEquals(List.of("borg"), invocation.activeModes());
            }

            @Test
            @DisplayName("clamps a very large width and rejects a negative width")
            void clampsWidth() {
                assertEquals(Integer.MAX_VALUE, Cowsay.buildInvocation("hi", Map.of("width", 99_999_999_999L)).width());
                assertEquals(1, Cowsay.buildInvocation("hi", Map.of("width", -5L)).width());
            }
        }

        @Test
        @DisplayName("listCowFiles returns sorted basenames")
        void listCowFilesSorted(@TempDir Path dir) throws IOException {
            writeCow(dir, "tux", "");
            writeCow(dir, "default", "");
            writeCow(dir, "dragon", "");
            assertEquals(List.of("default", "dragon", "tux"), Cowsay.listCowFiles(dir));
        }
    }

    @Nested
    @DisplayName("CliBuilder argv convention")
    class CliBuilderArgvConventionTests {
        // Regression test: unlike Perl's CliBuilder, this Java CliBuilder's
        // Parser.parse() DOES expect a leading program-name placeholder (it
        // reads argv.getFirst() as the program name and errors if argv is
        // empty), matching the C/Go convention. Verified against the real
        // Parser, not just hand-built flags/arguments maps.
        @Test
        @DisplayName("does not drop the first token when a program-name placeholder is prepended")
        void doesNotDropFirstToken() throws IOException {
            Path repoRoot = resolveRepoRoot();
            Path specPath = repoRoot.resolve("code").resolve("specs").resolve("cowsay.json");

            ParseOutcome outcome = new Parser(specPath, List.of("cowsay", "hello")).parse();
            assertInstanceOf(ParseResult.class, outcome);
            var message = Cowsay.resolveMessageFromArguments(((ParseResult) outcome).arguments());
            assertEquals("hello", message.orElseThrow());

            ParseOutcome outcome2 = new Parser(specPath, List.of("cowsay", "hello", "world")).parse();
            var message2 = Cowsay.resolveMessageFromArguments(((ParseResult) outcome2).arguments());
            assertEquals("hello world", message2.orElseThrow());
        }
    }

    @Nested
    @DisplayName("end-to-end golden output")
    class EndToEndGoldenOutputTests {
        @Test
        @DisplayName("resolves the real cows directory")
        void resolvesRealCowsDirectory() throws IOException {
            Path repoRoot = resolveRepoRoot();
            Path cowsDir = repoRoot.resolve("code").resolve("specs").resolve("cows");
            assertTrue(Cowsay.listCowFiles(cowsDir).contains("default"));
        }

        @Test
        @DisplayName("default cow speaking Hello, World!")
        void defaultCowSpeaking() throws IOException {
            Path repoRoot = resolveRepoRoot();
            Path cowsDir = repoRoot.resolve("code").resolve("specs").resolve("cows");
            var invocation = new Cowsay.CowsayInvocation(
                    "Hello, World!", "oo", "  ", List.of(), false, 40, false, "default");
            PaintVmAsciiResult result = Cowsay.render(invocation, cowsDir);
            assertInstanceOf(PaintVmAsciiResult.Ok.class, result);
            assertEquals(
                    String.join("\n",
                            " _______________",
                            "< Hello, World! >",
                            " ---------------",
                            "        \\   ^__^",
                            "         \\  (oo)\\_______",
                            "            (__)\\       )\\/\\",
                            "                ||----w |",
                            "                ||     ||"),
                    ((PaintVmAsciiResult.Ok) result).text());
        }

        @Test
        @DisplayName("borg mode thinking with the default cow")
        void borgModeThinking() throws IOException {
            Path repoRoot = resolveRepoRoot();
            Path cowsDir = repoRoot.resolve("code").resolve("specs").resolve("cows");
            var invocation = new Cowsay.CowsayInvocation(
                    "beep", "oo", "  ", List.of("borg"), false, 40, true, "default");
            PaintVmAsciiResult result = Cowsay.render(invocation, cowsDir);
            assertInstanceOf(PaintVmAsciiResult.Ok.class, result);
            assertEquals(
                    String.join("\n",
                            " ______",
                            "( beep )",
                            " ------",
                            "        o   ^__^",
                            "         o  (==)\\_______",
                            "            (__)\\       )\\/\\",
                            "                ||----w |",
                            "                ||     ||"),
                    ((PaintVmAsciiResult.Ok) result).text());
        }
    }
}
