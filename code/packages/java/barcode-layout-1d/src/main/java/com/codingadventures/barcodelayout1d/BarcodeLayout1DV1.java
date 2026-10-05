package com.codingadventures.barcodelayout1d;

import com.codingadventures.paintinstructions.PaintInstruction;
import com.codingadventures.paintinstructions.PaintScene;

import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import java.util.Objects;
import java.util.Set;

/** Pure, bounded integer geometry for the language-neutral barcode-layout-1d v1 contract. */
public final class BarcodeLayout1DV1 {
    private static final int MAX_PATTERN = 65_567;
    private static final int MAX_RUNS = 40_979;
    private static final int MAX_CONTENT = 65_567;
    private static final int MAX_QUIET = 4_096;
    private static final Set<String> ROLES = Set.of("data", "start", "stop", "guard", "check", "inter-character-gap");

    private BarcodeLayout1DV1() {}

    public record Run(String color, int modules, String sourceLabel, long sourceIndex, String role) {}
    public record Symbol(String label, int modules, long sourceIndex, String role) {}
    public record SymbolLayout(String label, int startModule, int endModule, int sourceIndex, String role) {}
    public record Layout(int leftQuietZoneModules, int rightQuietZoneModules, int contentModules,
                         int totalModules, List<SymbolLayout> symbolLayouts) {}
    public record SceneOptions(int moduleWidth, int barHeight, String foreground, String background,
                               String label, Map<String, String> metadata, String humanReadableText,
                               boolean includeHumanReadableText, List<Symbol> symbols) {
        public static SceneOptions defaults() {
            return new SceneOptions(4, 120, "#000000", "#ffffff", "1D barcode",
                    Map.of(), null, false, null);
        }
    }
    public static final class V1Error extends IllegalArgumentException {
        private final String errorId;
        public V1Error(String errorId) { super(errorId); this.errorId = errorId; }
        public String errorId() { return errorId; }
    }
    private static V1Error fail(String id) { return new V1Error(id); }

    private static int scalars(String value, String error) {
        if (value == null) throw fail(error);
        for (int i = 0; i < value.length(); i++) {
            char ch = value.charAt(i);
            if (Character.isHighSurrogate(ch)) {
                if (++i >= value.length() || !Character.isLowSurrogate(value.charAt(i))) throw fail(error);
            } else if (Character.isLowSurrogate(ch)) throw fail(error);
        }
        return value.codePointCount(0, value.length());
    }

    private static void source(String label, long index, String role) {
        if (scalars(label, "invalid-source-attribution") > 4_096 ||
                index < Integer.MIN_VALUE || index > Integer.MAX_VALUE || !ROLES.contains(role)) {
            throw fail("invalid-source-attribution");
        }
    }

    public static List<Run> expandBinary(String pattern, String sourceLabel, long sourceIndex, String role) {
        int length = scalars(pattern, "invalid-binary-token");
        if (length > MAX_PATTERN) throw fail("pattern-too-long");
        if (length == 0) throw fail("empty-pattern");
        for (int i = 0; i < pattern.length(); i++) {
            if (pattern.charAt(i) != '0' && pattern.charAt(i) != '1') throw fail("invalid-binary-token");
        }
        source(sourceLabel, sourceIndex, role);
        var result = new ArrayList<Run>();
        char current = pattern.charAt(0);
        int count = 1;
        for (int i = 1; i < pattern.length(); i++) {
            char token = pattern.charAt(i);
            if (token == current) { count++; continue; }
            if (result.size() >= MAX_RUNS) throw fail("too-many-runs");
            result.add(new Run(current == '1' ? "bar" : "space", count, sourceLabel, sourceIndex, role));
            current = token;
            count = 1;
        }
        if (result.size() >= MAX_RUNS) throw fail("too-many-runs");
        result.add(new Run(current == '1' ? "bar" : "space", count, sourceLabel, sourceIndex, role));
        return List.copyOf(result);
    }

    public static List<Run> expandWidth(String pattern, String narrowMarker, String wideMarker,
                                        int narrowModules, int wideModules, String startingColor,
                                        String sourceLabel, long sourceIndex, String role) {
        int length = scalars(pattern, "invalid-width-token");
        if (length > MAX_PATTERN) throw fail("pattern-too-long");
        if (length == 0) throw fail("empty-pattern");
        if (scalars(narrowMarker, "invalid-marker-configuration") != 1 ||
                scalars(wideMarker, "invalid-marker-configuration") != 1 ||
                narrowMarker.equals(wideMarker)) throw fail("invalid-marker-configuration");
        int narrow = narrowMarker.codePointAt(0);
        int wide = wideMarker.codePointAt(0);
        int[] tokens = pattern.codePoints().toArray();
        for (int token : tokens) if (token != narrow && token != wide) throw fail("invalid-width-token");
        source(sourceLabel, sourceIndex, role);
        if (narrowModules <= 0 || wideModules <= 0) throw fail("invalid-module-count");
        if (!startingColor.equals("bar") && !startingColor.equals("space")) throw fail("invalid-marker-configuration");
        if (length > MAX_RUNS) throw fail("too-many-runs");
        var result = new ArrayList<Run>(length);
        int content = 0;
        for (int i = 0; i < tokens.length; i++) {
            int modules = tokens[i] == wide ? wideModules : narrowModules;
            if (modules > MAX_CONTENT - content) throw fail("content-too-wide");
            content += modules;
            String color = (i & 1) == 0 ? startingColor : (startingColor.equals("bar") ? "space" : "bar");
            result.add(new Run(color, modules, sourceLabel, sourceIndex, role));
        }
        return List.copyOf(result);
    }

    public static Layout computeLayout(List<Run> runs, int quietZoneModules, List<Symbol> symbols) {
        Objects.requireNonNull(runs);
        if (runs.size() > MAX_RUNS) throw fail("too-many-runs");
        int content = 0;
        String previous = null;
        for (Run run : runs) {
            if (run == null || !("bar".equals(run.color()) || "space".equals(run.color())) ||
                    !ROLES.contains(run.role())) throw fail("invalid-source-attribution");
            if (run.modules() <= 0) throw fail("invalid-module-count");
            source(run.sourceLabel(), run.sourceIndex(), run.role());
            if (run.modules() > MAX_CONTENT - content) throw fail("content-too-wide");
            content += run.modules();
            if (run.color().equals(previous)) throw fail("non-alternating-runs");
            previous = run.color();
        }
        if (quietZoneModules < 1 || quietZoneModules > MAX_QUIET) throw fail("invalid-quiet-zone");
        var layouts = new ArrayList<SymbolLayout>();
        if (symbols != null) {
            if (symbols.size() > MAX_RUNS) throw fail("too-many-symbols");
            int cursor = 0;
            for (Symbol symbol : symbols) {
                if (symbol == null || symbol.modules() <= 0) throw fail("invalid-module-count");
                source(symbol.label(), symbol.sourceIndex(), symbol.role());
                if (symbol.role().equals("inter-character-gap")) throw fail("invalid-source-attribution");
                if (symbol.modules() > MAX_CONTENT - cursor) throw fail("symbol-width-mismatch");
                int end = cursor + symbol.modules();
                layouts.add(new SymbolLayout(symbol.label(), cursor, end, (int) symbol.sourceIndex(), symbol.role()));
                cursor = end;
            }
            if (cursor != content) throw fail("symbol-width-mismatch");
        } else {
            int cursor = 0;
            Run active = null;
            int start = 0;
            for (Run run : runs) {
                if (!run.role().equals("inter-character-gap")) {
                    boolean changed = active == null || !active.sourceLabel().equals(run.sourceLabel()) ||
                            active.sourceIndex() != run.sourceIndex() || !active.role().equals(run.role());
                    if (changed) {
                        if (active != null) layouts.add(new SymbolLayout(active.sourceLabel(), start,
                                cursor, (int) active.sourceIndex(), active.role()));
                        active = run;
                        start = cursor;
                    }
                }
                cursor += run.modules();
            }
            if (active != null) layouts.add(new SymbolLayout(active.sourceLabel(), start, cursor,
                    (int) active.sourceIndex(), active.role()));
            if (layouts.size() > MAX_RUNS) throw fail("too-many-symbols");
        }
        return new Layout(quietZoneModules, quietZoneModules, content,
                quietZoneModules + content + quietZoneModules, List.copyOf(layouts));
    }

    public static PaintScene projectScene(List<Run> runs, int quietZoneModules, SceneOptions options) {
        SceneOptions config = options == null ? SceneOptions.defaults() : options;
        if (config.includeHumanReadableText() || config.humanReadableText() != null) {
            throw fail("human-readable-text-unsupported");
        }
        if (config.moduleWidth() < 1 || config.moduleWidth() > 8_192 ||
                config.barHeight() < 1 || config.barHeight() > 8_192 ||
                scalars(config.foreground(), "invalid-render-config") > 128 ||
                scalars(config.background(), "invalid-render-config") > 128) throw fail("invalid-render-config");
        Layout layout = computeLayout(runs, quietZoneModules, config.symbols());
        Map<String, String> caller = config.metadata();
        if (caller == null || caller.size() > 64) throw fail("metadata-too-large");
        var metadata = new HashMap<String, String>();
        int bytes = 0;
        for (var entry : caller.entrySet()) {
            if (scalars(entry.getKey(), "metadata-too-large") > 128 ||
                    scalars(entry.getValue(), "metadata-too-large") > 4_096) throw fail("metadata-too-large");
            bytes += entry.getKey().getBytes(StandardCharsets.UTF_8).length +
                    entry.getValue().getBytes(StandardCharsets.UTF_8).length;
            if (bytes > 65_536) throw fail("metadata-too-large");
            metadata.put(entry.getKey(), entry.getValue());
        }
        if (scalars(config.label(), "metadata-too-large") > 4_096) throw fail("metadata-too-large");
        var rectangles = new ArrayList<PaintInstruction>();
        int cursor = quietZoneModules;
        for (Run run : runs) {
            int end = cursor + run.modules();
            if (run.color().equals("bar")) {
                rectangles.add(new PaintInstruction.PaintRect(cursor * config.moduleWidth(), 0,
                        run.modules() * config.moduleWidth(), config.barHeight(), config.foreground(),
                        Map.of("sourceLabel", run.sourceLabel(), "sourceIndex", Long.toString(run.sourceIndex()),
                                "role", run.role(), "moduleStart", Integer.toString(cursor),
                                "moduleEnd", Integer.toString(end))));
            }
            cursor = end;
        }
        int sceneWidth = layout.totalModules() * config.moduleWidth();
        metadata.put("label", config.label());
        metadata.put("leftQuietZoneModules", Integer.toString(layout.leftQuietZoneModules()));
        metadata.put("rightQuietZoneModules", Integer.toString(layout.rightQuietZoneModules()));
        metadata.put("contentModules", Integer.toString(layout.contentModules()));
        metadata.put("totalModules", Integer.toString(layout.totalModules()));
        metadata.put("moduleWidthPx", Integer.toString(config.moduleWidth()));
        metadata.put("barHeightPx", Integer.toString(config.barHeight()));
        metadata.put("sceneWidthPx", Integer.toString(sceneWidth));
        metadata.put("sceneHeightPx", Integer.toString(config.barHeight()));
        metadata.put("symbolCount", Integer.toString(layout.symbolLayouts().size()));
        return new PaintScene(sceneWidth, config.barHeight(), config.background(),
                List.copyOf(rectangles), Map.copyOf(metadata));
    }
}
