package com.codingadventures.cowsay;

import com.codingadventures.paintinstructions.PaintGlyphPlacement;
import com.codingadventures.paintinstructions.PaintInstruction;
import com.codingadventures.paintinstructions.PaintInstructions;
import com.codingadventures.paintinstructions.PaintScene;
import com.codingadventures.paintvmascii.AsciiOptions;
import com.codingadventures.paintvmascii.PaintVmAscii;
import com.codingadventures.paintvmascii.PaintVmAsciiResult;

import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.InvalidPathException;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.Optional;
import java.util.regex.Matcher;
import java.util.regex.Pattern;
import java.util.stream.Collectors;

/**
 * cowsay — routed through paint-vm-ascii (Java port).
 *
 * <p>Fifth language in the cowsay-through-paint-vm-ascii rollout (after
 * csharp, fsharp, perl, haskell). Everything up through composing the
 * bubble+cow text block is ordinary string formatting, ported unchanged
 * from the reference implementation at {@code code/programs/go/cowsay/main.go}.
 * The one thing that's different from that reference: instead of printing
 * the composed text directly, {@link #buildScene} converts it into a {@link
 * PaintScene} of {@code glyph_run} instructions (one glyph placement per
 * non-space character, positioned on an 8x16 character grid), and {@link
 * PaintVmAscii#render} turns that scene back into the terminal string we
 * print. This is also the PR that built {@code java/paint-vm-ascii} from
 * scratch, implementing the full P2D02 contract — see that package's own
 * CHANGELOG.
 */
public final class Cowsay {

    private Cowsay() {}

    /** paint-vm-ascii's documented default scale factors ({@code P2D02-paint-vm-ascii.md}). */
    public static final double SCALE_X = 8.0;
    public static final double SCALE_Y = 16.0;

    /** The resolved set of inputs needed to render one cowsay invocation. */
    public record CowsayInvocation(
            String message,
            String eyes,
            String tongue,
            List<String> activeModes,
            boolean noWrap,
            int width,
            boolean think,
            String cowFile) {}

    private record ModeOverride(String eyes, String tongue) {}

    private static final Map<String, ModeOverride> MODE_OVERRIDES = new LinkedHashMap<>();

    static {
        MODE_OVERRIDES.put("borg", new ModeOverride("==", null));
        MODE_OVERRIDES.put("dead", new ModeOverride("XX", "U "));
        MODE_OVERRIDES.put("greedy", new ModeOverride("$$", null));
        MODE_OVERRIDES.put("paranoid", new ModeOverride("@@", null));
        MODE_OVERRIDES.put("stoned", new ModeOverride("xx", "U "));
        MODE_OVERRIDES.put("tired", new ModeOverride("--", null));
        MODE_OVERRIDES.put("wired", new ModeOverride("OO", null));
        MODE_OVERRIDES.put("youthful", new ModeOverride("..", null));
    }

    public static final List<String> MODE_FLAG_IDS = List.copyOf(MODE_OVERRIDES.keySet());

    // -------------------------------------------------------------------
    // Rendering core (ported from code/programs/go/cowsay/main.go)
    // -------------------------------------------------------------------

    /**
     * Splits text into lines no longer than {@code width}, breaking on word
     * boundaries. A single word longer than the width is kept whole (never
     * split mid-word).
     */
    public static List<String> wrapText(String text, int width) {
        if (text.length() <= width) {
            return List.of(text);
        }

        String[] words = text.split(" +");
        List<String> nonEmptyWords = new ArrayList<>();
        for (String word : words) {
            if (!word.isEmpty()) {
                nonEmptyWords.add(word);
            }
        }
        if (nonEmptyWords.isEmpty()) {
            return List.of("");
        }

        List<String> lines = new ArrayList<>();
        StringBuilder current = new StringBuilder();
        for (String word : nonEmptyWords) {
            if (current.length() + word.length() + 1 <= width) {
                if (current.length() > 0) {
                    current.append(' ');
                }
                current.append(word);
            } else {
                if (current.length() > 0) {
                    lines.add(current.toString());
                }
                current = new StringBuilder(word);
            }
        }
        if (current.length() > 0) {
            lines.add(current.toString());
        }
        return lines;
    }

    /**
     * Draws the speech/thought bubble around the given lines. A single line
     * gets {@code "< ... >"} (or {@code "( ... )"} for a thought bubble);
     * multiple lines get {@code "/ ... \"}, {@code "| ... |"},
     * {@code "\ ... /"} (or {@code "( ... )"} on every line for a thought
     * bubble).
     */
    public static String formatBubble(List<String> lines, boolean isThink) {
        if (lines.isEmpty()) {
            return "";
        }

        int maxLen = 0;
        for (String line : lines) {
            maxLen = Math.max(maxLen, line.length());
        }

        String borderTop = " " + "_".repeat(maxLen + 2);
        String borderBottom = " " + "-".repeat(maxLen + 2);

        List<String> body = new ArrayList<>();
        if (lines.size() == 1) {
            String start = isThink ? "(" : "<";
            String end = isThink ? ")" : ">";
            body.add(start + " " + pad(lines.get(0), maxLen) + " " + end);
        } else {
            int n = lines.size();
            for (int i = 0; i < n; i++) {
                String start;
                String end;
                if (isThink) {
                    start = "(";
                    end = ")";
                } else if (i == 0) {
                    start = "/";
                    end = "\\";
                } else if (i == n - 1) {
                    start = "\\";
                    end = "/";
                } else {
                    start = "|";
                    end = "|";
                }
                body.add(start + " " + pad(lines.get(i), maxLen) + " " + end);
            }
        }

        List<String> result = new ArrayList<>();
        result.add(borderTop);
        result.addAll(body);
        result.add(borderBottom);
        return String.join("\n", result);
    }

    private static String pad(String value, int width) {
        if (value.length() >= width) {
            return value;
        }
        return value + " ".repeat(width - value.length());
    }

    /**
     * Pads or truncates a mode string (eyes/tongue) to exactly two
     * characters, matching cowsay's convention that eyes/tongue are always
     * a 2-char glyph.
     */
    public static String normalizeTwoChars(String value) {
        if (value.length() < 2) {
            return value + " ".repeat(2 - value.length());
        }
        return value.length() > 2 ? value.substring(0, 2) : value;
    }

    /**
     * Applies mode shortcuts (--borg, --dead, etc.) on top of the base
     * eyes/tongue flag values, then normalizes both to two characters.
     * Modes are mutually exclusive per cowsay.json, but this accepts any
     * set for robustness.
     */
    public static EyesAndTongue resolveEyesAndTongue(String baseEyes, String baseTongue, List<String> activeModes) {
        String eyes = baseEyes;
        String tongue = baseTongue;
        for (String mode : activeModes) {
            ModeOverride override = MODE_OVERRIDES.get(mode);
            if (override == null) {
                continue;
            }
            eyes = override.eyes();
            if (override.tongue() != null) {
                tongue = override.tongue();
            }
        }
        return new EyesAndTongue(normalizeTwoChars(eyes), normalizeTwoChars(tongue));
    }

    public record EyesAndTongue(String eyes, String tongue) {}

    /**
     * Walks up from {@code startDir} looking for CLAUDE.md, the repo-root
     * sentinel file. CLAUDE.md (not code/specs/cowsay.json itself) is used
     * deliberately — it's a more robust marker than reaching for the very
     * file being located, and this exact fix was called out as a lesson
     * from a prior, reverted cowsay Lua port's CI pathing problems (PR
     * #1535).
     */
    public static Path findRepoRoot(Path startDir) {
        Path dir = startDir;
        for (int i = 0; i < 24; i++) {
            if (Files.exists(dir.resolve("CLAUDE.md"))) {
                return dir;
            }
            Path parent = dir.getParent();
            if (parent == null) {
                return startDir;
            }
            dir = parent;
        }
        return startDir;
    }

    private static final Pattern COW_BODY_PATTERN = Pattern.compile("<<EOC;\\n(.*?)EOC", Pattern.DOTALL);

    /**
     * Loads a .cow template's body from {@code cowsDir}, falling back to
     * default.cow when the requested file doesn't exist. The template is a
     * Perl heredoc ({@code $the_cow = <<EOC; ... EOC}); only the body
     * between the heredoc markers is returned.
     *
     * <p>{@code cowName} comes from the user-supplied -f/--file flag, so it
     * is treated as untrusted: only a bare filename (no directory
     * separators, no rooted/absolute path) is accepted, and the resolved
     * path is verified to stay inside {@code cowsDir} before it's read —
     * otherwise this falls back to default.cow instead of reading an
     * arbitrary file the caller pointed at via {@code ".."}, a rooted
     * override, or similar (mirrors the fix applied to every other port's
     * loadCow after {@code /security-review}). A malformed {@code cowName}
     * that {@link Path#of} cannot even parse (e.g. an embedded NUL byte) is
     * treated the same as a rooted path: rejected outright.
     */
    public static String loadCow(String cowName, Path cowsDir) throws IOException {
        Path cowsRoot = cowsDir.toAbsolutePath().normalize();

        String safeName;
        boolean rooted;
        try {
            Path parsed = Path.of(cowName);
            Path fileName = parsed.getFileName();
            safeName = fileName == null ? "" : fileName.toString();
            rooted = parsed.isAbsolute();
        } catch (InvalidPathException e) {
            safeName = "";
            rooted = true;
        }

        Path candidate = null;
        if (!safeName.isEmpty() && !rooted) {
            try {
                candidate = cowsRoot.resolve(safeName + ".cow").toAbsolutePath().normalize();
            } catch (InvalidPathException e) {
                candidate = null;
            }
        }

        boolean withinCowsDir = candidate != null && candidate.startsWith(cowsRoot);
        Path cowPath = (candidate != null && withinCowsDir && Files.exists(candidate))
                ? candidate
                : cowsRoot.resolve("default.cow");

        String contents = Files.readString(cowPath, StandardCharsets.UTF_8);
        Matcher matcher = COW_BODY_PATTERN.matcher(contents);
        return matcher.find() ? matcher.group(1) : contents;
    }

    /**
     * Composes the full bubble+cow text block for one invocation —
     * everything up to (but not including) the paint-vm-ascii render step.
     */
    public static String composeContent(CowsayInvocation invocation, Path cowsDir) throws IOException {
        EyesAndTongue eyesAndTongue = resolveEyesAndTongue(invocation.eyes(), invocation.tongue(), invocation.activeModes());

        List<String> lines = new ArrayList<>();
        for (String rawLine : invocation.message().split("\n", -1)) {
            if (rawLine.isEmpty()) {
                lines.add("");
            } else if (invocation.noWrap()) {
                lines.add(rawLine);
            } else {
                lines.addAll(wrapText(rawLine, invocation.width()));
            }
        }

        String thoughts = invocation.think() ? "o" : "\\";
        String bubble = formatBubble(lines, invocation.think());

        String cowTemplate = loadCow(invocation.cowFile(), cowsDir);
        String cow = cowTemplate
                .replace("$eyes", eyesAndTongue.eyes())
                .replace("$tongue", eyesAndTongue.tongue())
                .replace("$thoughts", thoughts)
                .replace("\\\\", "\\");

        return bubble + "\n" + cow;
    }

    /**
     * Converts a composed text block into a {@link PaintScene}: one
     * {@code glyph_run} instruction per line, one glyph placement per
     * non-space character. See {@code code/specs/cowsay-paintvm-pipeline.md}
     * &sect;3 for the full contract, including why glyphId is a literal
     * Unicode code point here (an ASCII-backend-only relaxation of the
     * general PaintGlyphRun contract).
     */
    public static PaintScene buildScene(String text) {
        String normalized = text.replace("\r\n", "\n");
        String[] lines = normalized.split("\n", -1);

        int maxWidth = 0;
        for (String line : lines) {
            maxWidth = Math.max(maxWidth, line.length());
        }

        List<PaintInstruction> instructions = new ArrayList<>();
        for (int row = 0; row < lines.length; row++) {
            String line = lines[row];
            List<PaintGlyphPlacement> glyphs = new ArrayList<>();
            for (int col = 0; col < line.length(); col++) {
                char ch = line.charAt(col);
                if (ch == ' ') {
                    continue;
                }
                glyphs.add(new PaintGlyphPlacement(ch, col * SCALE_X, row * SCALE_Y));
            }
            if (!glyphs.isEmpty()) {
                instructions.add(PaintInstructions.paintGlyphRun(glyphs, "terminal-mono", SCALE_Y, "#000000"));
            }
        }

        int width = (int) (Math.max(1, maxWidth) * SCALE_X);
        int height = (int) (Math.max(1, lines.length) * SCALE_Y);
        return new PaintScene(width, height, "transparent", instructions);
    }

    /**
     * End-to-end: compose the bubble+cow text, build a {@link PaintScene}
     * from it, and render that scene through paint-vm-ascii.
     */
    public static PaintVmAsciiResult render(CowsayInvocation invocation, Path cowsDir) throws IOException {
        String content = composeContent(invocation, cowsDir);
        PaintScene scene = buildScene(content);
        return PaintVmAscii.render(scene, new AsciiOptions((int) SCALE_X, (int) SCALE_Y));
    }

    // -------------------------------------------------------------------
    // CLI glue — the bridge between CliBuilder's flags/arguments maps and
    // the typed invocation this class renders. Kept in this class (rather
    // than Main.java) so it's directly unit-testable without spawning a
    // process or driving a real Parser.
    // -------------------------------------------------------------------

    public static boolean isListRequested(Map<String, Object> flags) {
        return Boolean.TRUE.equals(flags.get("list"));
    }

    /** Cow file basenames under {@code cowsDir}, sorted ordinally. */
    public static List<String> listCowFiles(Path cowsDir) throws IOException {
        try (var entries = Files.list(cowsDir)) {
            return entries
                    .filter(p -> p.getFileName().toString().endsWith(".cow"))
                    .map(p -> {
                        String name = p.getFileName().toString();
                        return name.substring(0, name.length() - ".cow".length());
                    })
                    .sorted()
                    .collect(Collectors.toList());
        }
    }

    /**
     * Resolves the message from the parsed "message" positional argument.
     * Returns empty when no message was given on argv — the caller should
     * fall back to stdin.
     */
    @SuppressWarnings("unchecked")
    public static Optional<String> resolveMessageFromArguments(Map<String, Object> arguments) {
        Object messageValue = arguments.get("message");
        if (messageValue instanceof List<?> parts && !parts.isEmpty()) {
            return Optional.of(parts.stream().map(String::valueOf).collect(Collectors.joining(" ")));
        }
        return Optional.empty();
    }

    /**
     * Builds a {@link CowsayInvocation} from a resolved message and the
     * parsed flags map, applying cowsay.json's documented defaults for any
     * flag that wasn't explicitly set.
     */
    public static CowsayInvocation buildInvocation(String message, Map<String, Object> flags) {
        String eyes = flags.get("eyes") instanceof String s ? s : "oo";
        String tongue = flags.get("tongue") instanceof String s ? s : "  ";
        String cowFile = flags.get("cowfile") instanceof String s ? s : "default";
        boolean noWrap = Boolean.TRUE.equals(flags.get("nowrap"));
        boolean think = Boolean.TRUE.equals(flags.get("think"));

        int width = 40;
        Object widthValue = flags.get("width");
        if (widthValue instanceof Number number) {
            width = clampWidth(number.longValue());
        }

        List<String> activeModes = new ArrayList<>();
        for (String mode : MODE_FLAG_IDS) {
            if (Boolean.TRUE.equals(flags.get(mode))) {
                activeModes.add(mode);
            }
        }

        return new CowsayInvocation(message, eyes, tongue, activeModes, noWrap, width, think, cowFile);
    }

    private static int clampWidth(long value) {
        if (value < 1) {
            return 1;
        }
        if (value > Integer.MAX_VALUE) {
            return Integer.MAX_VALUE;
        }
        return (int) value;
    }
}
