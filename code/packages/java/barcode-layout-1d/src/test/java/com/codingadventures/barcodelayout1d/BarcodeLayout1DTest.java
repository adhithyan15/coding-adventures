package com.codingadventures.barcodelayout1d;

import com.codingadventures.paintinstructions.PaintInstruction;
import com.codingadventures.paintinstructions.PaintScene;
import org.junit.jupiter.api.Test;

import java.util.ArrayList;
import java.util.HashMap;
import java.util.List;
import java.util.Map;

import static org.junit.jupiter.api.Assertions.*;

/** Small native examples beside the complete, data-driven v1 corpus. */
class BarcodeLayout1DTest {
    @Test
    void binaryExpansionCoalescesAdjacentModules() {
        var runs = BarcodeLayout1DV1.expandBinary("110100", "*", -1, "start");
        assertEquals(List.of(2, 1, 1, 2), runs.stream().map(BarcodeLayout1DV1.Run::modules).toList());
        assertEquals(List.of("bar", "space", "bar", "space"),
                runs.stream().map(BarcodeLayout1DV1.Run::color).toList());
        assertThrows(UnsupportedOperationException.class, () -> runs.add(runs.get(0)));
    }

    @Test
    void layoutAssignsGapToPrecedingSymbol() {
        var runs = List.of(
                new BarcodeLayout1DV1.Run("bar", 1, "*", -1, "start"),
                new BarcodeLayout1DV1.Run("space", 1, "", -1, "inter-character-gap"),
                new BarcodeLayout1DV1.Run("bar", 2, "A", 0, "data"));
        var layout = BarcodeLayout1DV1.computeLayout(runs, 10, null);
        assertEquals(24, layout.totalModules());
        assertEquals(2, layout.symbolLayouts().get(0).endModule());
        assertEquals(2, layout.symbolLayouts().get(1).startModule());
    }

    @Test
    void sceneContainsBarsOnlyAndOwnsInputCollections() {
        var runs = new ArrayList<>(List.of(
                new BarcodeLayout1DV1.Run("bar", 1, "A", 0, "data"),
                new BarcodeLayout1DV1.Run("space", 1, "A", 0, "data")));
        var metadata = new HashMap<>(Map.of("caller", "original", "totalModules", "spoof"));
        var options = new BarcodeLayout1DV1.SceneOptions(
                4, 120, "#000000", "#ffffff", "Demo", metadata, null, false, null);
        PaintScene first = BarcodeLayout1DV1.projectScene(runs, 2, options);
        metadata.put("caller", "changed");
        runs.clear();
        assertEquals(1, first.instructions.size());
        assertEquals(24, first.width);
        assertEquals("original", first.metadata.get("caller"));
        assertEquals("6", first.metadata.get("totalModules"));
        assertEquals(8, ((PaintInstruction.PaintRect) first.instructions.get(0)).x);
        assertThrows(UnsupportedOperationException.class,
                () -> first.metadata.put("caller", "tampered"));
        assertThrows(UnsupportedOperationException.class,
                () -> first.instructions.clear());
        PaintScene second = BarcodeLayout1DV1.projectScene(
                List.of(new BarcodeLayout1DV1.Run("bar", 1, "A", 0, "data")),
                2, BarcodeLayout1DV1.SceneOptions.defaults());
        assertEquals("A", ((PaintInstruction.PaintRect) second.instructions.get(0))
                .metadata.get("sourceLabel"));
    }

    @Test
    void textValueFailsBeforeNativeResolution() {
        var invalidRuns = List.of(new BarcodeLayout1DV1.Run("bar", 0, "A", 0, "data"));
        var options = new BarcodeLayout1DV1.SceneOptions(
                0, 0, "#000000", "#ffffff", "Demo", Map.of(), "123", false, null);
        var error = assertThrows(BarcodeLayout1DV1.V1Error.class,
                () -> BarcodeLayout1DV1.projectScene(invalidRuns, 0, options));
        assertEquals("human-readable-text-unsupported", error.errorId());
    }

    @Test
    void textEnabledFailsBeforeNativeResolution() {
        var invalidRuns = List.of(new BarcodeLayout1DV1.Run("bar", 0, "A", 0, "data"));
        var options = new BarcodeLayout1DV1.SceneOptions(
                0, 0, "#000000", "#ffffff", "Demo", Map.of(), null, true, null);
        var error = assertThrows(BarcodeLayout1DV1.V1Error.class,
                () -> BarcodeLayout1DV1.projectScene(invalidRuns, 0, options));
        assertEquals("human-readable-text-unsupported", error.errorId());
    }
}
