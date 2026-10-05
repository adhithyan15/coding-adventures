package com.codingadventures.barcodelayout1d

import com.codingadventures.paintinstructions.PaintInstruction
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertThrows
import org.junit.jupiter.api.Test

/** Native examples supplementing the full v1 data-driven conformance suite. */
class BarcodeLayout1DTest {
    @Test
    fun binaryExpansionCoalescesAdjacentModules() {
        val runs = BarcodeLayout1DV1.expandBinary("110100", "*", -1, "start")
        assertEquals(listOf(2, 1, 1, 2), runs.map { it.modules })
        assertEquals(listOf("bar", "space", "bar", "space"), runs.map { it.color })
    }

    @Test
    fun layoutAssignsGapToPrecedingSymbol() {
        val runs = listOf(
            BarcodeLayout1DV1.Run("bar", 1, "*", -1, "start"),
            BarcodeLayout1DV1.Run("space", 1, "", -1, "inter-character-gap"),
            BarcodeLayout1DV1.Run("bar", 2, "A", 0, "data"),
        )
        val layout = BarcodeLayout1DV1.computeLayout(runs, 10)
        assertEquals(24, layout.totalModules)
        assertEquals(2, layout.symbolLayouts[0].endModule)
        assertEquals(2, layout.symbolLayouts[1].startModule)
    }

    @Test
    fun sceneContainsBarsOnlyAndOwnsInputCollections() {
        val runs = mutableListOf(
            BarcodeLayout1DV1.Run("bar", 1, "A", 0, "data"),
            BarcodeLayout1DV1.Run("space", 1, "A", 0, "data"),
        )
        val metadata = mutableMapOf("caller" to "original", "totalModules" to "spoof")
        val options = BarcodeLayout1DV1.SceneOptions(label = "Demo", metadata = metadata)
        val first = BarcodeLayout1DV1.projectScene(runs, 2, options)
        metadata["caller"] = "changed"
        runs.clear()
        assertEquals(1, first.instructions.size)
        assertEquals(24, first.width)
        assertEquals("original", first.metadata["caller"])
        assertEquals("6", first.metadata["totalModules"])
        assertEquals(8, (first.instructions[0] as PaintInstruction.PaintRect).x)
        val second = BarcodeLayout1DV1.projectScene(
            listOf(BarcodeLayout1DV1.Run("bar", 1, "A", 0, "data")),
            2,
        )
        assertEquals("A", (second.instructions[0] as PaintInstruction.PaintRect).metadata["sourceLabel"])
    }

    @Test
    fun textValueFailsBeforeNativeResolution() {
        val invalidRuns = listOf(BarcodeLayout1DV1.Run("bar", 0, "A", 0, "data"))
        val error = assertThrows(BarcodeLayout1DV1.V1Error::class.java) {
            BarcodeLayout1DV1.projectScene(
                invalidRuns, 0,
                BarcodeLayout1DV1.SceneOptions(moduleWidth = 0, humanReadableText = "123"),
            )
        }
        assertEquals("human-readable-text-unsupported", error.errorId)
    }

    @Test
    fun textEnabledFailsBeforeNativeResolution() {
        val invalidRuns = listOf(BarcodeLayout1DV1.Run("bar", 0, "A", 0, "data"))
        val error = assertThrows(BarcodeLayout1DV1.V1Error::class.java) {
            BarcodeLayout1DV1.projectScene(
                invalidRuns, 0,
                BarcodeLayout1DV1.SceneOptions(moduleWidth = 0, includeHumanReadableText = true),
            )
        }
        assertEquals("human-readable-text-unsupported", error.errorId)
    }
}
