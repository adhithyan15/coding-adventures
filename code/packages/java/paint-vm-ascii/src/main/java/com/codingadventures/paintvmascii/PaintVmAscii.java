package com.codingadventures.paintvmascii;

import com.codingadventures.paintinstructions.PaintGlyphPlacement;
import com.codingadventures.paintinstructions.PaintInstruction;
import com.codingadventures.paintinstructions.PaintScene;
import com.codingadventures.paintinstructions.Transform2D;

import java.util.HashMap;
import java.util.List;
import java.util.Map;
import java.util.Optional;

/**
 * A small, pure terminal backend for {@link PaintScene} values.
 *
 * <p>Implements the full {@code P2D02-paint-vm-ascii.md} contract:
 * filled/stroked rectangles, lines, glyph runs, and plain (untransformed,
 * unfiltered, fully opaque) groups/clips/layers. Scene coordinates are
 * divided by a configurable horizontal and vertical scale to obtain
 * character-cell coordinates.
 *
 * <p>The buffer is a {@code Map} from {@code (row, col)} to a {@link Cell},
 * rather than a mutable 2D array — scenes rendered by this backend are
 * small (terminal-sized, capped by {@link #MAX_AXIS_CELLS}), so the
 * simplicity of a sparse map outweighs any performance concern, and it
 * keeps the box-drawing merge logic (two strokes sharing a corner combine
 * into one character) expressible without a pre-sized grid.
 */
public final class PaintVmAscii {

    /** Package version, shared with the other language implementations. */
    public static final String VERSION = "0.1.0";

    private PaintVmAscii() {}

    // -------------------------------------------------------------------
    // Directional/fill tag bits — merge so the right box-drawing glyph
    // (corner, tee, cross) is chosen regardless of draw order.
    // -------------------------------------------------------------------

    private static final int FLAG_UP = 1;
    private static final int FLAG_RIGHT = 2;
    private static final int FLAG_DOWN = 4;
    private static final int FLAG_LEFT = 8;
    private static final int FLAG_FILL = 16;

    private static final Map<Integer, Character> BOX_CHARACTERS = Map.ofEntries(
            Map.entry(FLAG_LEFT | FLAG_RIGHT, '─'),
            Map.entry(FLAG_UP | FLAG_DOWN, '│'),
            Map.entry(FLAG_DOWN | FLAG_RIGHT, '┌'),
            Map.entry(FLAG_DOWN | FLAG_LEFT, '┐'),
            Map.entry(FLAG_UP | FLAG_RIGHT, '└'),
            Map.entry(FLAG_UP | FLAG_LEFT, '┘'),
            Map.entry(FLAG_LEFT | FLAG_RIGHT | FLAG_DOWN, '┬'),
            Map.entry(FLAG_LEFT | FLAG_RIGHT | FLAG_UP, '┴'),
            Map.entry(FLAG_UP | FLAG_DOWN | FLAG_RIGHT, '├'),
            Map.entry(FLAG_UP | FLAG_DOWN | FLAG_LEFT, '┤'),
            Map.entry(FLAG_UP | FLAG_DOWN | FLAG_LEFT | FLAG_RIGHT, '┼'),
            Map.entry(FLAG_RIGHT, '─'),
            Map.entry(FLAG_LEFT, '─'),
            Map.entry(FLAG_UP, '│'),
            Map.entry(FLAG_DOWN, '│'));

    private static final char FILL_CHAR = '█';

    // -------------------------------------------------------------------
    // Buffer
    // -------------------------------------------------------------------

    /** One character cell. {@code CellText} always wins over {@code CellTag} — literal text is never overwritten. */
    private sealed interface Cell {
        record CellTag(int flags) implements Cell {}

        record CellText(char ch) implements Cell {}
    }

    private record Point(int row, int col) {}

    private record ClipBounds(int minCol, int minRow, int maxCol, int maxRow) {
        boolean inside(int row, int col) {
            return row >= minRow && row < maxRow && col >= minCol && col < maxCol;
        }

        /**
         * Clamp a cell coordinate into this clip's bounds. Used before
         * building any range that iterates between two cell coordinates
         * (rect fill/stroke, line endpoints), so a caller-supplied geometry
         * with a huge (but valid) extent can't force iteration/recursion
         * far beyond the actual clipped surface — bounded by the clip's own
         * size instead of by caller input.
         */
        int clampCol(int value) {
            return Math.max(minCol, Math.min(value, maxCol - 1));
        }

        int clampRow(int value) {
            return Math.max(minRow, Math.min(value, maxRow - 1));
        }

        ClipBounds intersect(ClipBounds child) {
            return new ClipBounds(
                    Math.max(minCol, child.minCol),
                    Math.max(minRow, child.minRow),
                    Math.min(maxCol, child.maxCol),
                    Math.min(maxRow, child.maxRow));
        }
    }

    private static void writeTag(ClipBounds clip, int row, int col, int flags, Map<Point, Cell> buffer) {
        if (!clip.inside(row, col)) {
            return;
        }
        Point p = new Point(row, col);
        Cell existing = buffer.get(p);
        if (existing instanceof Cell.CellText) {
            return;
        }
        if (existing instanceof Cell.CellTag(int existingFlags)) {
            buffer.put(p, new Cell.CellTag(existingFlags | flags));
        } else {
            buffer.put(p, new Cell.CellTag(flags));
        }
    }

    private static void writeChar(ClipBounds clip, int row, int col, char ch, Map<Point, Cell> buffer) {
        if (!clip.inside(row, col)) {
            return;
        }
        buffer.put(new Point(row, col), new Cell.CellText(ch));
    }

    private static char resolveCell(Cell cell) {
        if (cell instanceof Cell.CellText(char ch)) {
            return ch;
        }
        Cell.CellTag tag = (Cell.CellTag) cell;
        int directions = tag.flags() & (FLAG_UP | FLAG_RIGHT | FLAG_DOWN | FLAG_LEFT);
        if (directions != 0) {
            Character boxChar = BOX_CHARACTERS.get(directions);
            if (boxChar != null) {
                return boxChar;
            }
        }
        if ((tag.flags() & FLAG_FILL) != 0) {
            return FILL_CHAR;
        }
        return '+';
    }

    private static String bufferToText(int rows, int columns, Map<Point, Cell> buffer) {
        List<String> lines = new java.util.ArrayList<>();
        for (int row = 0; row < rows; row++) {
            StringBuilder line = new StringBuilder(columns);
            for (int col = 0; col < columns; col++) {
                Cell cell = buffer.get(new Point(row, col));
                line.append(cell == null ? ' ' : resolveCell(cell));
            }
            int end = line.length();
            while (end > 0 && line.charAt(end - 1) == ' ') {
                end--;
            }
            lines.add(line.substring(0, end));
        }
        int lastNonBlank = lines.size();
        while (lastNonBlank > 0 && lines.get(lastNonBlank - 1).isEmpty()) {
            lastNonBlank--;
        }
        return String.join("\n", lines.subList(0, lastNonBlank));
    }

    // -------------------------------------------------------------------
    // Coordinate conversion
    // -------------------------------------------------------------------

    /**
     * Cell-coordinate values are saturated to this bound (rather than left
     * as a raw rounded result) so a large-but-ordinary finite {@code double}
     * can never land on exactly {@code Integer.MIN_VALUE}/{@code MAX_VALUE}.
     * Without this, a clip extent rounding to an extreme value could defeat
     * {@link ClipBounds#clampCol}/{@link ClipBounds#clampRow} downstream via
     * integer overflow in the {@code maxCol - 1} they compute, un-clamping
     * any shape nested in that clip and reopening the unbounded-iteration
     * DoS the clip clamping exists to prevent. A billion cells in either
     * direction is far beyond any real rendered scene (scenes are
     * additionally capped at {@link #MAX_AXIS_CELLS} per axis) while
     * leaving enormous headroom below {@code int}'s actual bounds for
     * {@code clampCol}/{@code clampRow}'s arithmetic to stay overflow-free.
     */
    private static final int CELL_BOUND = 1_000_000_000;

    private static int toCell(double coordinate, int scale) {
        double scaled = coordinate / scale;
        if (Double.isNaN(scaled)) {
            return 0;
        }
        if (scaled >= CELL_BOUND) {
            return CELL_BOUND;
        }
        if (scaled <= -CELL_BOUND) {
            return -CELL_BOUND;
        }
        return (int) Math.round(scaled);
    }

    /**
     * Converts an {@code int}-valued coordinate ({@link
     * PaintInstruction.PaintRect} uses {@code int}, not {@code double}) via
     * the same saturating {@link #toCell(double, int)} path. Callers that
     * need to sum two {@code int}s first (e.g. {@code x + width}) must do
     * that addition in {@code long} before calling this, so two large
     * {@code int}s summing can't silently wrap through {@code int} overflow
     * before ever reaching the saturation check.
     */
    private static int toCellFromLong(long coordinate, int scale) {
        return toCell((double) coordinate, scale);
    }

    // -------------------------------------------------------------------
    // Validation
    // -------------------------------------------------------------------

    private static boolean isFinite(double value) {
        return !Double.isNaN(value) && !Double.isInfinite(value);
    }

    private static boolean validRectangle(PaintInstruction.PaintRect r) {
        return r.width >= 0 && r.height >= 0;
    }

    private static boolean validLine(PaintInstruction.PaintLine l) {
        return isFinite(l.x1) && isFinite(l.y1) && isFinite(l.x2) && isFinite(l.y2);
    }

    /**
     * Validates the individual fields *and* the {@code x+width}/
     * {@code y+height} extents used by {@link #clipBoundsOf} — two
     * individually-finite values near {@code Double.MAX_VALUE} can still
     * sum to +Infinity under IEEE-754 arithmetic, so checking the fields
     * alone isn't sufficient to guarantee {@link #toCell} never sees a
     * non-finite input.
     */
    private static boolean validClip(PaintInstruction.PaintClip c) {
        return isFinite(c.x) && isFinite(c.y) && isFinite(c.width) && isFinite(c.height)
                && c.width >= 0 && c.height >= 0
                && isFinite(c.x + c.width) && isFinite(c.y + c.height);
    }

    private static boolean isIdentityTransform(Optional<Transform2D> transform) {
        return transform.isEmpty() || transform.get().isIdentity();
    }

    private static Optional<PaintVmAsciiError> assertPlainGroup(PaintInstruction.PaintGroup group) {
        if (!isIdentityTransform(group.transform)) {
            return Optional.of(new PaintVmAsciiError.UnsupportedInstruction("group with a non-identity transform"));
        }
        if (group.opacity.isPresent() && group.opacity.get() != 1.0) {
            return Optional.of(new PaintVmAsciiError.UnsupportedInstruction("group with non-default opacity"));
        }
        return Optional.empty();
    }

    private static Optional<PaintVmAsciiError> assertPlainLayer(PaintInstruction.PaintLayer layer) {
        if (!isIdentityTransform(layer.transform)) {
            return Optional.of(new PaintVmAsciiError.UnsupportedInstruction("layer with a non-identity transform"));
        }
        if (layer.opacity.isPresent() && layer.opacity.get() != 1.0) {
            return Optional.of(new PaintVmAsciiError.UnsupportedInstruction("layer with non-default opacity"));
        }
        if (layer.hasFilters) {
            return Optional.of(new PaintVmAsciiError.UnsupportedInstruction("layer with filters"));
        }
        if (layer.blendMode.isPresent() && !layer.blendMode.get().equals("normal")) {
            return Optional.of(new PaintVmAsciiError.UnsupportedInstruction("layer with a non-normal blend mode"));
        }
        return Optional.empty();
    }

    private static boolean visiblePaint(String paint) {
        String trimmed = paint.trim();
        return !trimmed.isEmpty() && !trimmed.equals("transparent") && !trimmed.equals("none");
    }

    // -------------------------------------------------------------------
    // Top-level render
    // -------------------------------------------------------------------

    /**
     * Upper bound on the number of character cells a rendered scene may
     * occupy, both in total and per axis. Scene dimensions are otherwise
     * only checked for being non-negative, so without this a
     * caller-supplied width/height of e.g. one billion would force
     * {@link #bufferToText} to iterate on an enormous number of cells even
     * with zero drawing instructions — a denial-of-service unrelated to
     * (and not fixed by) the per-instruction clip clamping. The per-axis
     * bound is required in addition to the product bound: a zero-width,
     * huge-height scene has a product of zero (passing a product-only
     * check) while still forcing an unbounded traversal along the
     * surviving axis.
     */
    private static final long MAX_AXIS_CELLS = 2000;

    private static final long MAX_BUFFER_CELLS = MAX_AXIS_CELLS * MAX_AXIS_CELLS;

    /** Render with {@link AsciiOptions#DEFAULT}. */
    public static PaintVmAsciiResult renderDefault(PaintScene scene) {
        return render(scene, AsciiOptions.DEFAULT);
    }

    /** Render a scene as terminal-friendly text. */
    public static PaintVmAsciiResult render(PaintScene scene, AsciiOptions options) {
        if (options.scaleX <= 0) {
            return new PaintVmAsciiResult.Err(new PaintVmAsciiError.InvalidScaleX(options.scaleX));
        }
        if (options.scaleY <= 0) {
            return new PaintVmAsciiResult.Err(new PaintVmAsciiError.InvalidScaleY(options.scaleY));
        }
        if (scene.width < 0 || scene.height < 0) {
            return new PaintVmAsciiResult.Err(new PaintVmAsciiError.InvalidSceneDimensions(scene.width, scene.height));
        }

        long columns = ceilDiv(scene.width, options.scaleX);
        long rows = ceilDiv(scene.height, options.scaleY);
        if (columns > MAX_AXIS_CELLS || rows > MAX_AXIS_CELLS || columns * rows > MAX_BUFFER_CELLS) {
            return new PaintVmAsciiResult.Err(new PaintVmAsciiError.SceneTooLarge(scene.width, scene.height));
        }

        ClipBounds clip = new ClipBounds(0, 0, (int) columns, (int) rows);
        Map<Point, Cell> buffer = new HashMap<>();
        for (PaintInstruction instruction : scene.instructions) {
            Optional<PaintVmAsciiError> error = dispatch(options, clip, buffer, instruction);
            if (error.isPresent()) {
                return new PaintVmAsciiResult.Err(error.get());
            }
        }
        return new PaintVmAsciiResult.Ok(bufferToText((int) rows, (int) columns, buffer));
    }

    private static long ceilDiv(long numerator, long denominator) {
        return (numerator + denominator - 1) / denominator;
    }

    /**
     * Render one instruction (recursing into group/clip/layer children),
     * mutating {@code buffer} in place and failing loudly on anything not
     * in the P2D02 contract.
     */
    private static Optional<PaintVmAsciiError> dispatch(
            AsciiOptions options, ClipBounds clip, Map<Point, Cell> buffer, PaintInstruction instruction) {
        return switch (instruction) {
            case PaintInstruction.PaintRect r -> {
                if (!validRectangle(r)) {
                    yield Optional.of(new PaintVmAsciiError.InvalidRectangleGeometry(r.x, r.y, r.width, r.height));
                }
                renderRectangle(options, clip, r, buffer);
                yield Optional.empty();
            }
            case PaintInstruction.PaintLine l -> {
                if (!validLine(l)) {
                    yield Optional.of(new PaintVmAsciiError.InvalidLineGeometry(l.x1, l.y1, l.x2, l.y2));
                }
                renderLine(options, clip, l, buffer);
                yield Optional.empty();
            }
            case PaintInstruction.PaintGlyphRun g -> {
                renderGlyphRun(options, clip, g, buffer);
                yield Optional.empty();
            }
            case PaintInstruction.PaintGroup group -> {
                Optional<PaintVmAsciiError> plainCheck = assertPlainGroup(group);
                if (plainCheck.isPresent()) {
                    yield plainCheck;
                }
                yield dispatchChildren(options, clip, buffer, group.children);
            }
            case PaintInstruction.PaintClip c -> {
                if (!validClip(c)) {
                    yield Optional.of(new PaintVmAsciiError.InvalidClipGeometry(c.x, c.y, c.width, c.height));
                }
                ClipBounds nextClip = clip.intersect(clipBoundsOf(options, c));
                yield dispatchChildren(options, nextClip, buffer, c.children);
            }
            case PaintInstruction.PaintLayer layer -> {
                Optional<PaintVmAsciiError> plainCheck = assertPlainLayer(layer);
                if (plainCheck.isPresent()) {
                    yield plainCheck;
                }
                yield dispatchChildren(options, clip, buffer, layer.children);
            }
            case PaintInstruction.PaintPath ignored -> Optional.of(new PaintVmAsciiError.UnsupportedInstruction("path"));
        };
    }

    private static Optional<PaintVmAsciiError> dispatchChildren(
            AsciiOptions options, ClipBounds clip, Map<Point, Cell> buffer, List<PaintInstruction> children) {
        for (PaintInstruction child : children) {
            Optional<PaintVmAsciiError> error = dispatch(options, clip, buffer, child);
            if (error.isPresent()) {
                return error;
            }
        }
        return Optional.empty();
    }

    private static ClipBounds clipBoundsOf(AsciiOptions options, PaintInstruction.PaintClip c) {
        return new ClipBounds(
                toCell(c.x, options.scaleX),
                toCell(c.y, options.scaleY),
                toCell(c.x + c.width, options.scaleX),
                toCell(c.y + c.height, options.scaleY));
    }

    // -------------------------------------------------------------------
    // Rect
    // -------------------------------------------------------------------

    private static void renderRectangle(
            AsciiOptions options, ClipBounds clip, PaintInstruction.PaintRect r, Map<Point, Cell> buffer) {
        int c1 = clip.clampCol(toCellFromLong(r.x, options.scaleX));
        int r1 = clip.clampRow(toCellFromLong(r.y, options.scaleY));
        int c2 = clip.clampCol(toCellFromLong((long) r.x + r.width, options.scaleX));
        int r2 = clip.clampRow(toCellFromLong((long) r.y + r.height, options.scaleY));

        if (visiblePaint(r.fill)) {
            for (int row = r1; row <= r2; row++) {
                for (int col = c1; col <= c2; col++) {
                    writeTag(clip, row, col, FLAG_FILL, buffer);
                }
            }
        }

        if (!r.stroke.trim().isEmpty()) {
            writeTag(clip, r1, c1, FLAG_DOWN | FLAG_RIGHT, buffer);
            writeTag(clip, r1, c2, FLAG_DOWN | FLAG_LEFT, buffer);
            writeTag(clip, r2, c1, FLAG_UP | FLAG_RIGHT, buffer);
            writeTag(clip, r2, c2, FLAG_UP | FLAG_LEFT, buffer);
            for (int col = c1 + 1; col < c2; col++) {
                writeTag(clip, r1, col, FLAG_LEFT | FLAG_RIGHT, buffer);
                writeTag(clip, r2, col, FLAG_LEFT | FLAG_RIGHT, buffer);
            }
            for (int row = r1 + 1; row < r2; row++) {
                writeTag(clip, row, c1, FLAG_UP | FLAG_DOWN, buffer);
                writeTag(clip, row, c2, FLAG_UP | FLAG_DOWN, buffer);
            }
        }
    }

    // -------------------------------------------------------------------
    // Line (horizontal/vertical fast paths + Bresenham for the diagonal case)
    // -------------------------------------------------------------------

    private static void renderLine(
            AsciiOptions options, ClipBounds clip, PaintInstruction.PaintLine line, Map<Point, Cell> buffer) {
        // Clamped into the clip's own bounds before use — an out-of-range
        // but otherwise valid (finite) endpoint can't force iteration or
        // Bresenham recursion far beyond the actual clipped surface.
        int c1 = clip.clampCol(toCell(line.x1, options.scaleX));
        int r1 = clip.clampRow(toCell(line.y1, options.scaleY));
        int c2 = clip.clampCol(toCell(line.x2, options.scaleX));
        int r2 = clip.clampRow(toCell(line.y2, options.scaleY));

        if (r1 == r2) {
            int minCol = Math.min(c1, c2);
            int maxCol = Math.max(c1, c2);
            for (int col = minCol; col <= maxCol; col++) {
                int flags;
                if (minCol == maxCol) {
                    flags = FLAG_LEFT | FLAG_RIGHT;
                } else if (col == minCol) {
                    flags = FLAG_RIGHT;
                } else if (col == maxCol) {
                    flags = FLAG_LEFT;
                } else {
                    flags = FLAG_LEFT | FLAG_RIGHT;
                }
                writeTag(clip, r1, col, flags, buffer);
            }
            return;
        }

        if (c1 == c2) {
            int minRow = Math.min(r1, r2);
            int maxRow = Math.max(r1, r2);
            for (int row = minRow; row <= maxRow; row++) {
                int flags;
                if (minRow == maxRow) {
                    flags = FLAG_UP | FLAG_DOWN;
                } else if (row == minRow) {
                    flags = FLAG_DOWN;
                } else if (row == maxRow) {
                    flags = FLAG_UP;
                } else {
                    flags = FLAG_UP | FLAG_DOWN;
                }
                writeTag(clip, row, c1, flags, buffer);
            }
            return;
        }

        int deltaRow = Math.abs(r2 - r1);
        int deltaCol = Math.abs(c2 - c1);
        int stepRow = r1 < r2 ? 1 : -1;
        int stepCol = c1 < c2 ? 1 : -1;
        int diagonalFlags = deltaCol > deltaRow ? FLAG_LEFT | FLAG_RIGHT : FLAG_UP | FLAG_DOWN;

        int row = r1;
        int col = c1;
        int error = 0;
        while (true) {
            writeTag(clip, row, col, diagonalFlags, buffer);
            if (row == r2 && col == c2) {
                break;
            }
            int doubled = 2 * error;
            if (doubled > -deltaRow) {
                error -= deltaRow;
                col += stepCol;
            }
            if (doubled < deltaCol) {
                error += deltaCol;
                row += stepRow;
            }
        }
    }

    // -------------------------------------------------------------------
    // Glyph run
    // -------------------------------------------------------------------

    /**
     * A glyph with a non-finite position is skipped rather than passed to
     * {@link #toCell} — unlike a malformed rect/line/clip, a single bad
     * glyph placement doesn't need to fail the whole render.
     */
    private static void renderGlyphRun(
            AsciiOptions options, ClipBounds clip, PaintInstruction.PaintGlyphRun run, Map<Point, Cell> buffer) {
        for (PaintGlyphPlacement glyph : run.glyphs) {
            if (!isFinite(glyph.x) || !isFinite(glyph.y)) {
                continue;
            }
            int row = toCell(glyph.y, options.scaleY);
            int col = toCell(glyph.x, options.scaleX);
            writeChar(clip, row, col, toSafeTerminalGlyph(glyph.glyphId), buffer);
        }
    }

    /**
     * ASCII-backend-specific relaxation of the general {@code
     * PaintGlyphPlacement} contract: {@code glyphId} is treated as a
     * literal Unicode code point (no font resolution happens in a
     * terminal), per {@code P2D02-paint-vm-ascii.md}. Control characters,
     * bidi-control code points, and UTF-16 surrogate code points are
     * replaced with {@code ?} so a crafted message can't inject terminal
     * escape sequences or ill-formed UTF-16.
     */
    private static char toSafeTerminalGlyph(int codePoint) {
        if (codePoint >= 0 && codePoint <= 0x10FFFF && isSafeTerminalCodePoint(codePoint)) {
            if (Character.charCount(codePoint) == 1) {
                return (char) codePoint;
            }
            return '?';
        }
        return '?';
    }

    private static boolean isSafeTerminalCodePoint(int codePoint) {
        if (codePoint < 0x20) {
            return false;
        }
        if (codePoint >= 0x7f && codePoint <= 0x9f) {
            return false;
        }
        if (codePoint >= 0xD800 && codePoint <= 0xDFFF) {
            return false;
        }
        if (codePoint == 0x200e || codePoint == 0x200f || codePoint == 0x061c) {
            return false;
        }
        if (codePoint >= 0x202a && codePoint <= 0x202e) {
            return false;
        }
        return !(codePoint >= 0x2066 && codePoint <= 0x2069);
    }
}
