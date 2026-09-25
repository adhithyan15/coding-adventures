// Behaviour of the Compose platform library (UI87 §7) without a display: the
// dialogs are replaced by a fake that returns a chosen file (or cancels), so
// the real open/save logic, limits and routing run as they would in an app.
//
// Copied into a generated Compose project's src/test/kotlin and run with
//   gradle test --tests MosaicPlatformEffectsTest
// (CI does this in the Journal Compose lane).

import java.io.File
import java.nio.file.Files
import java.util.Base64
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNull
import kotlin.test.assertTrue
import org.junit.Test

private class FakeDialogs(private val choice: File?) : MosaicFileDialogs {
    var lastExtensions: List<String> = emptyList()
    var lastSuggestedName: String? = null
    override fun chooseFileToOpen(extensions: List<String>): File? {
        lastExtensions = extensions
        return choice
    }
    override fun chooseFileToSave(suggestedName: String, extensions: List<String>): File? {
        lastExtensions = extensions
        lastSuggestedName = suggestedName
        return choice
    }
}

private fun encoded(text: String): String = Base64.getEncoder().encodeToString(text.toByteArray())

class MosaicPlatformEffectsTest {
    private val directory: File = Files.createTempDirectory("mosaic-platform-effects").toFile()

    @Test
    fun routesByKind() {
        // The app claims a kind: it goes to the app, even a standard one.
        assertEquals(false, mosaicRoutesToPlatform("files.save", setOf("files.save")))
        // A standard kind nobody claimed: the platform library.
        assertEquals(true, mosaicRoutesToPlatform("files.open", setOf("importAnki")))
        assertEquals(true, mosaicRoutesToPlatform("files.save", null))
        // A non-standard kind: the app when it claimed nothing (the original
        // meaning), nobody when it named its kinds and this is not one.
        assertEquals(false, mosaicRoutesToPlatform("importAnki", null))
        assertNull(mosaicRoutesToPlatform("somethingElse", setOf("importAnki")))
    }

    @Test
    fun savesTheBytesUnderTheChosenName() {
        val target = File(directory, "journal-2026-09-25.json")
        val dialogs = FakeDialogs(target)
        val outcome = mosaicRunFilesSave(
            mapOf(
                "suggestedName" to "journal-2026-09-25.json",
                "accept" to listOf("application/json"),
                "bytes" to encoded("""{"version":1}"""),
            ),
            dialogs,
        )
        assertEquals(mapOf("ok" to mapOf("name" to "journal-2026-09-25.json")), outcome)
        assertEquals("""{"version":1}""", target.readText())
        assertEquals(listOf("json"), dialogs.lastExtensions)
        // Written beside the target and moved into place: no temporary left.
        assertTrue(directory.listFiles()!!.none { it.name.endsWith(".tmp") })
    }

    @Test
    fun aCancelledDialogIsNotAFailure() {
        val outcome = mosaicRunFilesSave(
            mapOf("suggestedName" to "a.json", "bytes" to encoded("x")),
            FakeDialogs(null),
        )
        assertEquals(mapOf("cancelled" to emptyMap<String, Any?>()), outcome)
    }

    @Test
    fun refusesANameThatIsAPath() {
        for (name in listOf("../escape.json", "dir/a.json", "a\\b.json", "", ".", "..", "a\u0000b")) {
            assertFalse(mosaicIsPlainFileName(name), name)
            val outcome = mosaicRunFilesSave(mapOf("suggestedName" to name, "bytes" to encoded("x")), FakeDialogs(null))
            assertTrue(outcome.containsKey("failed"), name)
        }
        assertTrue(mosaicIsPlainFileName("journal.json"))
    }

    @Test
    fun refusesBytesThatAreNotBase64OrTooLarge() {
        val bad = mosaicRunFilesSave(mapOf("suggestedName" to "a.bin", "bytes" to "not base64!"), FakeDialogs(null))
        assertTrue(bad.containsKey("failed"))
        val huge = "A".repeat(((MOSAIC_MAX_SAVE_BYTES / 3) + 2) * 4)
        val tooLarge = mosaicRunFilesSave(mapOf("suggestedName" to "a.bin", "bytes" to huge), FakeDialogs(null))
        assertTrue(tooLarge.containsKey("failed"))
    }

    @Test
    fun opensAFileAndReportsItsNameTypeAndBytes() {
        val file = File(directory, "photo.png")
        file.writeBytes(byteArrayOf(1, 2, 3))
        val outcome = mosaicRunFilesOpen(mapOf("accept" to listOf("image/png")), FakeDialogs(file))
        @Suppress("UNCHECKED_CAST")
        val ok = outcome["ok"] as Map<String, Any?>
        assertEquals("photo.png", ok["name"])
        assertEquals("image/png", ok["mimeType"])
        assertEquals(Base64.getEncoder().encodeToString(byteArrayOf(1, 2, 3)), ok["bytes"])
    }

    @Test
    fun openingSomethingThatIsNotAFileFails() {
        val outcome = mosaicRunFilesOpen(emptyMap<String, Any?>(), FakeDialogs(directory))
        assertTrue(outcome.containsKey("failed"))
        assertEquals(
            mapOf("cancelled" to emptyMap<String, Any?>()),
            mosaicRunFilesOpen(emptyMap<String, Any?>(), FakeDialogs(null)),
        )
    }
}
