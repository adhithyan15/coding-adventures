package com.codingadventures.paintinstructions;

import java.util.List;
import java.util.Map;


/**
 * paint-instructions — backend-neutral 2D paint scene model.
 *
 * <p>This package defines the lightweight intermediate representation that sits
 * between a high-level drawing abstraction (like a barcode layout engine) and
 * a concrete rendering backend (SVG, Canvas, Metal, terminal, etc.).
 *
 * <h2>Why an intermediate representation?</h2>
 *
 * <p>Without a neutral IR, every barcode encoder would need to know how to draw
 * rectangles in SVG, or hexagons on a terminal, or pixels in Metal.  That
 * coupling explodes the number of combinations: N encoders × M backends.
 *
 * <p>With an IR we only need N encoder → IR adapters and M IR → backend adapters.
 * The encoders and backends never know about each other.
 *
 * <pre>
 * QR encoder  ─┐
 * DataMatrix  ─┼──→  PaintScene  ──→  SVG backend
 * MaxiCode    ─┘                  ──→  Canvas backend
 *                                  ──→  Terminal backend
 * </pre>
 *
 * <h2>Key types</h2>
 *
 * <ul>
 *   <li>{@link PathCommand} — a single drawing command (MoveTo, LineTo, ClosePath).</li>
 *   <li>{@link PaintInstruction} — sealed: {@link PaintInstruction.PaintRect} or
 *       {@link PaintInstruction.PaintPath}.</li>
 *   <li>{@link PaintScene} — top-level container with width, height, background,
 *       and a list of instructions.</li>
 * </ul>
 *
 * <h2>Builder helpers</h2>
 *
 * <p>Static helper methods in this class make construction ergonomic:
 *
 * <pre>
 *   PaintScene scene = PaintInstructions.createScene(
 *       210, 210, "#ffffff",
 *       List.of(
 *           PaintInstructions.paintRect(40, 40, 10, 10, "#000000"),
 *           PaintInstructions.paintRect(50, 40, 10, 10, "#ffffff")
 *       )
 *   );
 * </pre>
 *
 * <p>Spec: P2D00 paint-instructions.
 */
public final class PaintInstructions {

    /** Package version following Semantic Versioning 2.0. */
    public static final String VERSION = "0.1.0";

    // Private constructor — this is a static-only utility class.
    private PaintInstructions() {}

    // =========================================================================
    // paintRect — builder helper
    // =========================================================================

    /**
     * Build a {@link PaintInstruction.PaintRect}.
     *
     * <p>Applies a default fill of {@code "#000000"} (black) when the provided
     * fill string is null or blank.
     *
     * @param x        Left edge in pixels.
     * @param y        Top edge in pixels.
     * @param width    Width in pixels (≥ 0).
     * @param height   Height in pixels (≥ 0).
     * @param fill     CSS colour string.  Defaults to {@code "#000000"} if blank.
     * @param metadata Optional annotations.
     * @return A ready-to-use {@link PaintInstruction.PaintRect}.
     *
     * <p>Example:
     * <pre>
     *   PaintInstruction.PaintRect bg = PaintInstructions.paintRect(0, 0, 210, 210, "#ffffff");
     * </pre>
     */
    public static PaintInstruction.PaintRect paintRect(
            int x, int y, int width, int height,
            String fill, Map<String, String> metadata) {
        String effectiveFill = (fill == null || fill.isBlank()) ? "#000000" : fill;
        Map<String, String> effectiveMeta = (metadata == null) ? Map.of() : metadata;
        return new PaintInstruction.PaintRect(x, y, width, height, effectiveFill, effectiveMeta);
    }

    /**
     * Build a {@link PaintInstruction.PaintRect} with empty metadata.
     *
     * @param x      Left edge in pixels.
     * @param y      Top edge in pixels.
     * @param width  Width in pixels.
     * @param height Height in pixels.
     * @param fill   CSS colour string.
     * @return A ready-to-use {@link PaintInstruction.PaintRect}.
     */
    public static PaintInstruction.PaintRect paintRect(
            int x, int y, int width, int height, String fill) {
        return paintRect(x, y, width, height, fill, Map.of());
    }

    // =========================================================================
    // paintPath — builder helper
    // =========================================================================

    /**
     * Build a {@link PaintInstruction.PaintPath}.
     *
     * <p>Applies a default fill of {@code "#000000"} (black) when the provided
     * fill string is null or blank.
     *
     * @param commands Ordered path commands describing the polygon.
     * @param fill     CSS colour string.  Defaults to {@code "#000000"} if blank.
     * @param metadata Optional annotations.
     * @return A ready-to-use {@link PaintInstruction.PaintPath}.
     *
     * <p>Example — flat-top hexagon:
     * <pre>
     *   List&lt;PathCommand&gt; cmds = buildHexCommands(cx, cy, r);
     *   PaintInstruction.PaintPath hex = PaintInstructions.paintPath(cmds, "#1a1a1a");
     * </pre>
     */
    public static PaintInstruction.PaintPath paintPath(
            List<PathCommand> commands, String fill, Map<String, String> metadata) {
        String effectiveFill = (fill == null || fill.isBlank()) ? "#000000" : fill;
        Map<String, String> effectiveMeta = (metadata == null) ? Map.of() : metadata;
        return new PaintInstruction.PaintPath(commands, effectiveFill, effectiveMeta);
    }

    /**
     * Build a {@link PaintInstruction.PaintPath} with empty metadata.
     *
     * @param commands Ordered path commands.
     * @param fill     CSS colour string.
     * @return A ready-to-use {@link PaintInstruction.PaintPath}.
     */
    public static PaintInstruction.PaintPath paintPath(List<PathCommand> commands, String fill) {
        return paintPath(commands, fill, Map.of());
    }

    // =========================================================================
    // createScene — builder helper
    // =========================================================================

    /**
     * Build a {@link PaintScene}.
     *
     * <p>Applies a default background of {@code "#ffffff"} (white) when the provided
     * background string is null or blank.
     *
     * @param width        Canvas width in pixels.
     * @param height       Canvas height in pixels.
     * @param background   CSS background colour.  Defaults to {@code "#ffffff"} if blank.
     * @param instructions Ordered list of paint instructions.
     * @param metadata     Optional scene-level annotations.
     * @return A ready-to-use {@link PaintScene}.
     *
     * <p>Example:
     * <pre>
     *   PaintScene scene = PaintInstructions.createScene(
     *       210, 210, "#ffffff",
     *       List.of(PaintInstructions.paintRect(0, 0, 210, 210, "#ffffff")),
     *       Map.of()
     *   );
     * </pre>
     */
    public static PaintScene createScene(
            int width, int height, String background,
            List<PaintInstruction> instructions,
            Map<String, String> metadata) {
        String effectiveBg = (background == null || background.isBlank()) ? "#ffffff" : background;
        List<PaintInstruction> effectiveInstr = (instructions == null) ? List.of() : instructions;
        Map<String, String> effectiveMeta = (metadata == null) ? Map.of() : metadata;
        return new PaintScene(width, height, effectiveBg, effectiveInstr, effectiveMeta);
    }

    /**
     * Build a {@link PaintScene} with empty metadata.
     *
     * @param width        Canvas width in pixels.
     * @param height       Canvas height in pixels.
     * @param background   CSS background colour.
     * @param instructions Ordered list of paint instructions.
     * @return A ready-to-use {@link PaintScene}.
     */
    public static PaintScene createScene(
            int width, int height, String background,
            List<PaintInstruction> instructions) {
        return createScene(width, height, background, instructions, Map.of());
    }
}
