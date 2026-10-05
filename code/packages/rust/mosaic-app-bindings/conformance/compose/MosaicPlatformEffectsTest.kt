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
        for (name in listOf(
            "../escape.json", "dir/a.json", "a\\b.json", "", ".", "..", "a\u0000b",
            "D:report.json", "notes.json:stream", "invoice\u202Efdp.exe", "trailing.", "trailing ",
            // Tightened after the SwiftUI library's security review (UI87 §7):
            ".zshrc", " leading.json", "nbsp\u00A0", "\u3000ideographic.json",
            "line\u2028break.json", "para\u2029break.json",
            "tag\uDB40\uDC01.json", // U+E0001 LANGUAGE TAG, a format character outside the BMP
            "x".repeat(256), "Invoice.pdf      .command",
            ".\u0301zshrc", "Invoice.pdf\u2800\u2800.txt",
            "a\uD800.json", "a\uE000.json",
            "Invoice.pdf" + " \uFE00".repeat(30) + " x.html", "\uFE00.zshrc", "notes.txt\uFE00.",
        )) {
            assertFalse(mosaicIsPlainFileName(name), name)
            val outcome = mosaicRunFilesSave(mapOf("suggestedName" to name, "bytes" to encoded("x")), FakeDialogs(null))
            assertTrue(outcome.containsKey("failed"), name)
        }
        assertTrue(mosaicIsPlainFileName("journal.json"))
        assertTrue(mosaicIsPlainFileName("caf\u00E9 menu.json"))
        assertTrue(mosaicIsPlainFileName("\u2764\uFE0F list.txt"))
    }

    @Test
    fun withNoAcceptedTypeAnExecutableExtensionIsRefused() {
        for (name in listOf(
            "run.command", "open.terminal", "site.webloc", "setup.EXE", "go.desktop", "a.ps1",
            "a.j\u017F", "img.iso", "clip.scf", "app.AppImage", "run\u0D4E.terminal",
        )) {
            assertTrue(mosaicHasExecutableExtension(name), name)
            val outcome = mosaicRunFilesSave(
                mapOf("suggestedName" to name, "bytes" to encoded("x")),
                FakeDialogs(File(directory, name)),
            )
            assertEquals(
                mapOf("failed" to mapOf("message" to "suggestedName must not end in an executable extension")),
                outcome,
                name,
            )
            assertFalse(File(directory, name).exists(), name)
        }
        // An ordinary document with no accepted type still saves.
        assertFalse(mosaicHasExecutableExtension("notes.txt"))
        assertFalse(mosaicHasExecutableExtension("README"))
    }

    @Test
    fun windowsDeviceNamesAreNeverPlainNames() {
        // UI87 §3.1: on Windows these are the console, the null device or a
        // port, never a file; refused on every host so a name saves alike.
        for (name in listOf("CON", "con.txt", "Nul.json", "COM1.json", "lpt9", "COM\u00B9.json", "CON .txt", "CONIN\$.log", "aux.tar.gz")) {
            assertFalse(mosaicIsPlainFileName(name), name)
        }
        for (name in listOf("console.txt", "CONFIG.json", "aux-notes.txt", "COM10.json", "my.CON", "nul report.json", "CON\u0131N\$.txt")) {
            assertTrue(mosaicIsPlainFileName(name), name)
        }
    }

    @Test
    fun activeContentAndNonAsciiExtensionsCountAsExecutable() {
        // Web pages, SVG, web archives and shortcuts, macro documents and
        // Python scripts run code when opened; a non-ASCII extension can be
        // a lookalike of `.exe` (Cyrillic) or `.exe` with a combining mark.
        for (name in listOf("page.html", "page.HTM", "card.svg", "archive.mht", "shortcut.website", "report.xlsm", "deck.pptm", "tool.py", "invoice.\u0435x\u0435", "setup.exe\u0301", "macros.xlsb", "addin.xla", "link.iqy", "sheet.slk", "remote.rdp", "app.pyzw", "cache.pyc")) {
            assertTrue(mosaicHasExecutableExtension(name), name)
        }
        for (name in listOf("notes.txt", "data.xlsx", "report.docx", "photo.png")) {
            assertFalse(mosaicHasExecutableExtension(name), name)
        }
    }

    @Test
    fun theNameMustMatchTheAcceptedType() {
        val outcome = mosaicRunFilesSave(
            mapOf("suggestedName" to "notes.exe", "accept" to listOf("application/json"), "bytes" to encoded("{}")),
            FakeDialogs(File(directory, "notes.exe")),
        )
        assertTrue(outcome.containsKey("failed"))
    }

    @Test
    fun savingOverAFileKeepsItsPermissions() {
        val target = File(directory, "private.json")
        target.writeText("old")
        val path = target.toPath()
        val supportsPosix = try {
            Files.setPosixFilePermissions(path, java.nio.file.attribute.PosixFilePermissions.fromString("rw-------"))
            true
        } catch (unsupported: UnsupportedOperationException) {
            false
        }
        val outcome = mosaicRunFilesSave(
            mapOf("suggestedName" to "private.json", "accept" to listOf("application/json"), "bytes" to encoded("new")),
            FakeDialogs(target),
        )
        assertTrue(outcome.containsKey("ok"))
        assertEquals("new", target.readText())
        if (supportsPosix) {
            assertEquals(
                "rw-------",
                java.nio.file.attribute.PosixFilePermissions.toString(Files.getPosixFilePermissions(path)),
            )
        }
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
    fun onlyAnImageOnlyOpenStartsInPictures() {
        // UI59 §2: a "pictures only" open starts in the Pictures folder.
        assertTrue(mosaicOnlyImages(listOf("png", "jpg", "jpeg")), "images start in Pictures")
        assertTrue(mosaicOnlyImages(listOf("svg")), "svg is an image")
        assertFalse(mosaicOnlyImages(listOf("png", "txt")), "mixed types keep the default start")
        assertFalse(mosaicOnlyImages(emptyList()), "any file keeps the default start")
        assertFalse(mosaicOnlyImages(listOf("pdf")), "a document keeps the default start")
    }

    // ---- The asynchronous path (UI89 §3.8): a picker that answers later ----

    /** A picker that holds each request until the test answers it. */
    private class LaterPicker : MosaicDocumentPicker {
        var opens = 0
        var creates = 0
        var lastAccept: MosaicAccept? = null
        var lastSuggestedName: String? = null
        var lastBytes: ByteArray? = null
        var openDone: ((MosaicOpenedDocument?) -> Unit)? = null
        var createDone: ((MosaicSaveTarget?) -> Unit)? = null
        override fun open(accept: MosaicAccept, done: (MosaicOpenedDocument?) -> Unit) {
            opens += 1
            lastAccept = accept
            openDone = done
        }
        override fun create(request: MosaicSaveRequest, done: (MosaicSaveTarget?) -> Unit) {
            creates += 1
            lastAccept = request.accept
            lastSuggestedName = request.suggestedName
            lastBytes = request.bytes
            createDone = done
        }
    }

    private class Document(
        override val name: String,
        override val mimeType: String?,
        private val bytes: ByteArray,
        private val failure: Exception? = null,
    ) : MosaicOpenedDocument {
        override fun read(limit: Long): ByteArray? {
            failure?.let { throw it }
            return mosaicReadBounded(bytes.inputStream(), limit)
        }
    }

    private class Target(override val name: String, private val failure: Exception? = null) : MosaicSaveTarget {
        var written: ByteArray? = null
        override fun write(bytes: ByteArray) {
            failure?.let { throw it }
            written = bytes
        }
    }

    /** Work handed to "the background", run when the test says so. */
    private class Background {
        val queued = mutableListOf<() -> Unit>()
        val run: (() -> Unit) -> Unit = { queued += it }
        fun drain() {
            while (queued.isNotEmpty()) queued.removeAt(0)()
        }
    }

    @Test
    fun anOpenAnsweredLaterIsReadInTheBackgroundAndAnsweredOnce() {
        val picker = LaterPicker()
        val background = Background()
        val outcomes = mutableListOf<Map<String, Any?>>()
        mosaicAnswerFilesOpen(mapOf("accept" to listOf("application/json", "x/unknown")), picker, background.run) { outcomes += it }
        // Nothing yet: the picker has not answered.
        assertEquals(0, outcomes.size)
        assertEquals(listOf("application/json"), picker.lastAccept!!.mimeTypes)
        assertEquals(listOf("json"), picker.lastAccept!!.extensions)
        val done = picker.openDone!!
        done(Document("notes.json", "application/json", "{}".toByteArray()))
        // Answered, but read off the UI thread: still nothing until it runs.
        assertEquals(0, outcomes.size)
        // A picker that answers again is not heard.
        done(null)
        background.drain()
        assertEquals<List<Map<String, Any?>>>(
            listOf(mapOf("ok" to mapOf("name" to "notes.json", "mimeType" to "application/json", "bytes" to encoded("{}")))),
            outcomes,
        )
    }

    @Test
    fun aDocumentWithNoTypeTakesItsNamesAndACancelIsACancel() {
        val picker = LaterPicker()
        val outcomes = mutableListOf<Map<String, Any?>>()
        mosaicAnswerFilesOpen(emptyMap<String, Any?>(), picker, { it() }) { outcomes += it }
        assertEquals(emptyList(), picker.lastAccept!!.mimeTypes)
        picker.openDone!!(Document("photo.PNG", null, byteArrayOf(1)))
        @Suppress("UNCHECKED_CAST")
        assertEquals("image/png", (outcomes.single()["ok"] as Map<String, Any?>)["mimeType"])

        val cancelled = mutableListOf<Map<String, Any?>>()
        mosaicAnswerFilesOpen(emptyMap<String, Any?>(), picker, { it() }) { cancelled += it }
        picker.openDone!!(null)
        assertEquals<List<Map<String, Any?>>>(listOf(mapOf("cancelled" to emptyMap<String, Any?>())), cancelled)
    }

    @Test
    fun aReadIsBoundedAndItsOwnErrorTextNeverReachesTheApp() {
        val picker = LaterPicker()
        val outcomes = mutableListOf<Map<String, Any?>>()
        mosaicAnswerFilesOpen(emptyMap<String, Any?>(), picker, { it() }) { outcomes += it }
        // A provider that lies about nothing but sends too much.
        picker.openDone!!(object : MosaicOpenedDocument {
            override val name = "big.bin"
            override val mimeType: String? = null
            override fun read(limit: Long) = mosaicReadBounded(
                object : java.io.InputStream() {
                    var left = limit + 1
                    override fun read(): Int = if (left-- > 0) 0 else -1
                },
                limit,
            )
        })
        assertEquals(
            mapOf("failed" to mapOf("message" to "the selected file is larger than $MOSAIC_MAX_OPEN_BYTES bytes")),
            outcomes.single(),
        )
        outcomes.clear()
        mosaicAnswerFilesOpen(emptyMap<String, Any?>(), picker, { it() }) { outcomes += it }
        picker.openDone!!(Document("a.txt", null, byteArrayOf(), java.io.IOException("/home/person/secret/a.txt: denied")))
        assertEquals(mapOf("failed" to mapOf("message" to "couldn't read the selected file")), outcomes.single())
        outcomes.clear()
        mosaicAnswerFilesOpen(emptyMap<String, Any?>(), picker, { it() }) { outcomes += it }
        picker.openDone!!(Document("a.txt", null, byteArrayOf(), MosaicFileFailure("that is not a regular file")))
        assertEquals(mapOf("failed" to mapOf("message" to "that is not a regular file")), outcomes.single())
    }

    @Test
    fun aPickerThatCannotBeShownFailsTheRequest() {
        val throwing = object : MosaicDocumentPicker {
            override fun open(accept: MosaicAccept, done: (MosaicOpenedDocument?) -> Unit) =
                throw MosaicFileFailure("there is no window to show the file picker in")
            override fun create(request: MosaicSaveRequest, done: (MosaicSaveTarget?) -> Unit) =
                throw IllegalStateException("/private/path")
        }
        val outcomes = mutableListOf<Map<String, Any?>>()
        mosaicAnswerFilesOpen(emptyMap<String, Any?>(), throwing, { it() }) { outcomes += it }
        mosaicAnswerFilesSave(mapOf("suggestedName" to "a.json", "bytes" to encoded("x")), throwing, { it() }) { outcomes += it }
        assertEquals<List<Map<String, Any?>>>(
            listOf(
                mapOf("failed" to mapOf("message" to "there is no window to show the file picker in")),
                mapOf("failed" to mapOf("message" to "the file dialog failed")),
            ),
            outcomes,
        )
    }

    @Test
    fun aRefusedSaveShowsNoPicker() {
        val picker = LaterPicker()
        val outcomes = mutableListOf<Map<String, Any?>>()
        for (payload in listOf(
            mapOf("suggestedName" to "../x.json", "bytes" to encoded("x")),
            mapOf("suggestedName" to "CON.json", "bytes" to encoded("x")),
            mapOf("suggestedName" to "run.command", "bytes" to encoded("x")),
            mapOf("suggestedName" to "a.txt", "accept" to listOf("application/json"), "bytes" to encoded("x")),
            mapOf("suggestedName" to "a.json", "bytes" to "not base64!"),
        )) {
            mosaicAnswerFilesSave(payload, picker, { it() }) { outcomes += it }
        }
        assertEquals(0, picker.creates)
        assertEquals(5, outcomes.size)
        assertTrue(outcomes.all { it.containsKey("failed") }, "$outcomes")
    }

    @Test
    fun aSaveAnsweredLaterReportsTheNameTheProviderChose() {
        val picker = LaterPicker()
        val background = Background()
        val outcomes = mutableListOf<Map<String, Any?>>()
        mosaicAnswerFilesSave(
            mapOf("suggestedName" to "journal.json", "accept" to listOf("application/json"), "bytes" to encoded("{}")),
            picker,
            background.run,
        ) { outcomes += it }
        assertEquals("journal.json", picker.lastSuggestedName)
        assertEquals(listOf("application/json"), picker.lastAccept!!.mimeTypes)
        // The provider kept the old file and named the new one itself.
        val target = Target("journal (1).json")
        picker.createDone!!(target)
        assertEquals(0, outcomes.size)
        background.drain()
        assertEquals<List<Map<String, Any?>>>(listOf(mapOf("ok" to mapOf("name" to "journal (1).json"))), outcomes)
        assertEquals("{}", String(target.written!!))
    }

    @Test
    fun aFailedWriteOrAFailedHandOffIsAFailureNotACrash() {
        val picker = LaterPicker()
        val outcomes = mutableListOf<Map<String, Any?>>()
        val save = mapOf("suggestedName" to "a.json", "bytes" to encoded("x"))
        mosaicAnswerFilesSave(save, picker, { it() }) { outcomes += it }
        picker.createDone!!(Target("a.json", java.io.IOException("content://provider/secret: gone")))
        mosaicAnswerFilesSave(save, picker, { throw java.util.concurrent.RejectedExecutionException() }) { outcomes += it }
        picker.createDone!!(Target("a.json"))
        mosaicAnswerFilesSave(save, picker, { it() }) { outcomes += it }
        picker.createDone!!(null)
        assertEquals<List<Map<String, Any?>>>(
            listOf(
                mapOf("failed" to mapOf("message" to "couldn't save the file")),
                mapOf("failed" to mapOf("message" to "couldn't save the file")),
                mapOf("cancelled" to emptyMap<String, Any?>()),
            ),
            outcomes,
        )
    }

    /** A host with no runtime: it records deferrals and answers. */
    private class FakeHost : MosaicPlatformEffectHost {
        override var effectHandler: ((Long, String, Any?, String) -> Unit)? = null
        var waitingOn = mutableSetOf<Long>()
        val deferred = mutableListOf<Long>()
        val answers = mutableMapOf<Long, Map<String, Any?>>()
        override fun deferEffect(id: Long): Boolean {
            if (id !in waitingOn) return false
            deferred += id
            return true
        }
        override fun completeEffect(id: Long, result: Map<String, Any?>): Map<String, Any?> {
            answers[id] = result
            return emptyMap()
        }
    }

    @Test
    fun theRouterDefersThenAnswersOnceThroughThePicker() {
        val host = FakeHost()
        val appCalls = mutableListOf<String>()
        host.effectHandler = { _, kind, _, _ -> appCalls += kind }
        val ui = mutableListOf<() -> Unit>()
        val picker = LaterPicker()
        installMosaicPlatformEffects(host, setOf("importAnki"), runOnUi = { ui += it })
        // Idempotent: the handler is a router now, and is not wrapped again.
        val router = host.effectHandler
        installMosaicPlatformEffects(host, null)
        assertTrue(host.effectHandler === router)
        // A router of its own, on this picker.
        val routed = FakeHost()
        routed.effectHandler = { _, kind, _, _ -> appCalls += kind }
        routed.effectHandler = MosaicPlatformRouter(routed, routed.effectHandler, setOf("importAnki"), picker, { ui += it })
        routed.waitingOn += setOf(1L, 2L)
        routed.effectHandler!!(1, "files.open", emptyMap<String, Any?>(), "await")
        assertEquals(listOf(1L), routed.deferred)
        assertEquals(1, ui.size)
        // One file operation at a time.
        routed.effectHandler!!(2, "files.open", emptyMap<String, Any?>(), "await")
        assertEquals(mapOf("failed" to mapOf("message" to "another file operation is in progress")), routed.answers[2])
        ui.removeAt(0)()
        assertEquals(null, routed.answers[1])
        picker.openDone!!(null)
        assertEquals(mapOf("cancelled" to emptyMap<String, Any?>()), routed.answers[1])
        // The app's kind reaches the app; an unclaimed custom kind reaches
        // nobody; a notify needs no answer; an id nobody awaits opens nothing.
        routed.effectHandler!!(3, "importAnki", null, "await")
        routed.effectHandler!!(4, "somethingElse", null, "await")
        routed.effectHandler!!(5, "files.open", null, "notify")
        routed.effectHandler!!(6, "files.open", null, "await")
        assertEquals(listOf("importAnki"), appCalls)
        assertTrue(ui.isEmpty() && routed.answers.keys == setOf(1L, 2L))
        // The router is free again after a refused deferral.
        routed.waitingOn += 7L
        routed.effectHandler!!(7, "files.open", null, "await")
        assertEquals(1, ui.size)
    }

    /** UI89 §3.11: an app's own kinds answered through the library's picker. */
    @Test
    fun anAppsOwnKindsAreAnsweredThroughThePicker() {
        assertEquals(null, mosaicPlatformRouter(FakeHost()), "no router before the library is installed")

        val host = FakeHost()
        val appCalls = mutableListOf<String>()
        host.effectHandler = { _, kind, _, _ -> appCalls += kind }
        val ui = mutableListOf<() -> Unit>()
        val background = Background()
        val picker = LaterPicker()
        host.effectHandler =
            MosaicPlatformRouter(host, host.effectHandler, setOf("importThing"), picker, { ui += it }, background.run)
        val router = mosaicPlatformRouter(host)!!
        // The app's kinds still reach the app first.
        host.effectHandler!!(9, "importThing", null, "await")
        assertEquals(listOf("importThing"), appCalls)
        assertTrue(host.deferred.isEmpty())

        // Open: the app's extensions reach the picker; the app's ok builds the answer.
        host.waitingOn += 1L
        router.openForApp(1, MosaicAccept(emptyList(), listOf("apkg", "colpkg")), 4) { name, bytes ->
            mapOf("apkg" to Base64.getEncoder().encodeToString(bytes), "from" to name)
        }
        assertEquals(listOf(1L), host.deferred)
        assertEquals(1, ui.size)
        ui.removeAt(0)()
        assertEquals(listOf("apkg", "colpkg"), picker.lastAccept!!.extensions)
        // One file operation at a time.
        host.waitingOn += 2L
        router.saveForApp(2, "out.apkg", "PK".toByteArray(), MosaicAccept(emptyList(), listOf("apkg"))) { emptyMap() }
        assertEquals(mapOf("failed" to mapOf("message" to "another file operation is in progress")), host.answers[2])
        picker.openDone!!(Document("deck.apkg", null, "PK34".toByteArray()))
        background.drain()
        ui.removeAt(0)()
        assertEquals(
            mapOf("ok" to mapOf("apkg" to Base64.getEncoder().encodeToString("PK34".toByteArray()), "from" to "deck.apkg")),
            host.answers[1],
        )

        // The app's limit, not files.open's, bounds the read.
        host.waitingOn += 3L
        router.openForApp(3, MosaicAccept(emptyList(), emptyList()), 4) { _, _ -> emptyMap() }
        ui.removeAt(0)()
        picker.openDone!!(Document("big.apkg", null, ByteArray(5)))
        background.drain()
        ui.removeAt(0)()
        assertEquals(mapOf("failed" to mapOf("message" to "the selected file is larger than 4 bytes")), host.answers[3])

        // A refused name shows no picker and defers nothing; an executable
        // extension is refused even when the app accepts it; an upper-case
        // extension in the app's list still matches.
        val creates = picker.creates
        host.waitingOn += setOf(4L, 5L, 6L)
        router.saveForApp(4, "../deck.apkg", "PK".toByteArray(), MosaicAccept(emptyList(), listOf("apkg"))) { emptyMap() }
        router.saveForApp(5, "deck.exe", "PK".toByteArray(), MosaicAccept(emptyList(), listOf("apkg"))) { emptyMap() }
        router.saveForApp(6, "run.command", "x".toByteArray(), MosaicAccept(emptyList(), listOf("command"))) { emptyMap() }
        assertEquals(creates, picker.creates)
        assertTrue(ui.isEmpty())
        assertEquals(mapOf("failed" to mapOf("message" to "suggestedName must be a plain file name")), host.answers[4])
        assertEquals(
            mapOf("failed" to mapOf("message" to "suggestedName must end in an extension of an accepted type")),
            host.answers[5],
        )
        assertEquals(
            mapOf("failed" to mapOf("message" to "suggestedName must not end in an executable extension")),
            host.answers[6],
        )
        assertTrue(host.deferred.none { it in setOf(4L, 5L, 6L) })

        // Save: the app's name and bytes reach the picker; the app's ok answers.
        host.waitingOn += 7L
        router.saveForApp(7, "engram.apkg", "PK\u0003\u0004".toByteArray(), MosaicAccept(emptyList(), listOf("APKG"))) {
            name -> mapOf("savedAs" to name)
        }
        ui.removeAt(0)()
        assertEquals("engram.apkg", picker.lastSuggestedName)
        assertTrue("PK\u0003\u0004".toByteArray().contentEquals(picker.lastBytes))
        val target = Target("engram (1).apkg")
        picker.createDone!!(target)
        background.drain()
        ui.removeAt(0)()
        assertEquals(mapOf("ok" to mapOf("savedAs" to "engram (1).apkg")), host.answers[7])
        assertTrue("PK\u0003\u0004".toByteArray().contentEquals(target.written))

        // failPending still reaches an app request in flight.
        host.waitingOn += 8L
        router.openForApp(8, MosaicAccept(emptyList(), emptyList()), 4) { _, _ -> emptyMap() }
        ui.removeAt(0)()
        assertTrue(router.failPending("the activity went away"))
        assertEquals(mapOf("failed" to mapOf("message" to "the activity went away")), host.answers[8])
    }

    @Test
    fun withABackgroundTheOutcomeComesBackThroughTheUiThread() {
        val host = FakeHost()
        val ui = mutableListOf<() -> Unit>()
        val background = Background()
        val picker = LaterPicker()
        val router = MosaicPlatformRouter(host, null, null, picker, { ui += it }, background.run)
        host.effectHandler = router
        host.waitingOn += 1L
        router(1, "files.save", mapOf("suggestedName" to "a.json", "bytes" to encoded("{}")), "await")
        ui.removeAt(0)()
        val target = Target("a.json")
        picker.createDone!!(target)
        assertTrue(background.queued.size == 1 && host.answers.isEmpty())
        background.drain()
        assertTrue(ui.size == 1 && host.answers.isEmpty(), "handed back, not completed off the UI thread")
        ui.removeAt(0)()
        assertEquals(mapOf("ok" to mapOf("name" to "a.json")), host.answers[1])
    }

    @Test
    fun aPickerWhoseAnswerCannotArriveIsFailedOnce() {
        val host = FakeHost()
        val picker = LaterPicker()
        val router = MosaicPlatformRouter(host, null, null, picker, { it() })
        assertFalse(router.failPending("the window closed"))
        host.waitingOn += setOf(1L, 2L)
        router(1, "files.open", null, "await")
        assertTrue(router.failPending("the window closed"))
        assertEquals(mapOf("failed" to mapOf("message" to "the window closed")), host.answers[1])
        // A late answer from the picker is not heard, and the router is free.
        picker.openDone!!(Document("late.txt", null, byteArrayOf()))
        assertEquals(mapOf("failed" to mapOf("message" to "the window closed")), host.answers[1])
        assertFalse(router.failPending("again"))
        router(2, "files.open", null, "await")
        assertEquals(2, picker.opens)
    }

    @Test
    fun aThrowInTheBackgroundIsStillAnAnswer() {
        // An OutOfMemoryError while encoding, or a provider whose name
        // lookup throws: it happens where no caller can catch it, so the
        // library must answer anyway.
        val picker = LaterPicker()
        val background = Background()
        val outcomes = mutableListOf<Map<String, Any?>>()
        mosaicAnswerFilesOpen(emptyMap<String, Any?>(), picker, background.run) { outcomes += it }
        picker.openDone!!(object : MosaicOpenedDocument {
            override val name: String get() = throw OutOfMemoryError("Java heap space")
            override val mimeType: String? = null
            override fun read(limit: Long) = byteArrayOf(1)
        })
        background.drain()
        assertEquals<List<Map<String, Any?>>>(listOf(mapOf("failed" to mapOf("message" to "couldn't read the selected file"))), outcomes)
        outcomes.clear()
        mosaicAnswerFilesSave(mapOf("suggestedName" to "a.json", "bytes" to encoded("x")), picker, background.run) { outcomes += it }
        picker.createDone!!(object : MosaicSaveTarget {
            override val name: String get() = throw StackOverflowError()
            override fun write(bytes: ByteArray) {}
        })
        background.drain()
        assertEquals<List<Map<String, Any?>>>(listOf(mapOf("failed" to mapOf("message" to "couldn't save the file"))), outcomes)
    }

    @Test
    fun anAnswerTheHostCannotTakeIsAnsweredAgainSmall() {
        val host = object : MosaicPlatformEffectHost {
            override var effectHandler: ((Long, String, Any?, String) -> Unit)? = null
            val answers = mutableListOf<Map<String, Any?>>()
            override fun deferEffect(id: Long) = true
            override fun completeEffect(id: Long, result: Map<String, Any?>): Map<String, Any?> {
                if (result.containsKey("ok")) throw OutOfMemoryError("Java heap space")
                answers += result
                return emptyMap()
            }
        }
        val picker = LaterPicker()
        val router = MosaicPlatformRouter(host, null, null, picker, { it() })
        router(1, "files.open", null, "await")
        picker.openDone!!(Document("big.bin", null, byteArrayOf(1, 2, 3)))
        assertEquals<List<Map<String, Any?>>>(listOf(mapOf("failed" to mapOf("message" to "couldn't deliver the file"))), host.answers)
        // And the router is free for the next request.
        router(2, "files.open", null, "await")
        assertEquals(2, picker.opens)
    }

    @Test
    fun aStreamThatOnlyEverReturnsNothingFailsInsteadOfHanging() {
        val idle = object : java.io.InputStream() {
            override fun read(): Int = throw AssertionError("bulk reads only")
            override fun read(b: ByteArray, off: Int, len: Int): Int = 0
        }
        val failure = runCatching { mosaicReadBounded(idle, 10) }.exceptionOrNull()
        assertTrue(failure is MosaicFileFailure, "$failure")
        // A stream that pauses now and then still reads to the end.
        var calls = 0
        val pausing = object : java.io.InputStream() {
            override fun read(): Int = throw AssertionError("bulk reads only")
            override fun read(b: ByteArray, off: Int, len: Int): Int = when (calls++) {
                0, 2 -> 0
                1 -> { b[off] = 7; 1 }
                else -> -1
            }
        }
        assertEquals(listOf<Byte>(7), mosaicReadBounded(pausing, 10)!!.toList())
    }

    /**
     * A stream that blocks until it is closed -- as a provider's pipe does
     * when the provider stops filling it -- and then throws, as Android's
     * file streams do for a thread blocked on a stream another one closed.
     */
    private class StalledStream : java.io.InputStream() {
        private val closed = java.util.concurrent.CountDownLatch(1)
        override fun read(): Int = throw AssertionError("bulk reads only")
        override fun read(b: ByteArray, off: Int, len: Int): Int {
            closed.await()
            throw java.io.IOException("closed under a blocked read")
        }
        override fun close() = closed.countDown()
    }

    @Test
    fun aStalledReadIsStoppedAndSaysSo() {
        // UI89 §3.8: no byte for the allowance, and the watch closes the
        // stream; the blocked read throws, and `stalled` tells the caller why.
        val stream = StalledStream()
        val started = System.nanoTime()
        val error = MosaicStallWatch(stallMillis = 200).use { watch ->
            watch.stopWith { stream.close() }
            val thrown = runCatching { mosaicReadBounded(mosaicWatchedInput(stream, watch), 10) }
                .exceptionOrNull()
            assertTrue(watch.stalled, "the watch fired")
            thrown
        }
        assertTrue(error is java.io.IOException, "$error")
        val waited = java.util.concurrent.TimeUnit.NANOSECONDS.toMillis(System.nanoTime() - started)
        assertTrue(waited in 200..5_000, "stopped after the allowance, not before: $waited ms")
    }

    @Test
    fun aSlowTransferThatKeepsMovingIsNeverStopped() {
        // Progress, not total time: a byte every 50 ms for 600 ms against a
        // 400 ms allowance -- well beyond any gap between bytes, and shorter
        // than the whole read -- reads to the end.
        var sent = 0
        val slow = object : java.io.InputStream() {
            override fun read(): Int = throw AssertionError("bulk reads only")
            override fun read(b: ByteArray, off: Int, len: Int): Int {
                if (sent == 12) return -1
                Thread.sleep(50)
                b[off] = sent++.toByte()
                return 1
            }
        }
        MosaicStallWatch(stallMillis = 400).use { watch ->
            watch.stopWith { throw AssertionError("a moving transfer was stopped") }
            assertEquals(12, mosaicReadBounded(mosaicWatchedInput(slow, watch), 100)!!.size)
            assertFalse(watch.stalled)
        }
    }

    @Test
    fun aWatchThatAlreadyFiredStopsWhatItIsGivenNext() {
        // The open may stall before the stream exists; the stream's stop then
        // runs as soon as it is handed over.
        MosaicStallWatch(stallMillis = 50).use { watch ->
            val deadline = System.nanoTime() + 5_000_000_000L
            while (!watch.stalled && System.nanoTime() < deadline) Thread.sleep(10)
            assertTrue(watch.stalled)
            var stopped = 0
            watch.stopWith { stopped++ }
            assertEquals(1, stopped)
        }
        // A closed watch never fires.
        val closed = MosaicStallWatch(stallMillis = 50)
        var fired = false
        closed.stopWith { fired = true }
        closed.close()
        Thread.sleep(200)
        assertFalse(fired)
        assertFalse(closed.stalled)
    }

    @Test
    fun aWatchedWriteGoesInPiecesAndKeepsTheWatchFed() {
        val bytes = ByteArray(200_000) { it.toByte() }
        val out = java.io.ByteArrayOutputStream()
        val writes = mutableListOf<Int>()
        val counting = object : java.io.OutputStream() {
            override fun write(b: Int) = throw AssertionError("bulk writes only")
            override fun write(b: ByteArray, off: Int, len: Int) {
                writes += len
                out.write(b, off, len)
            }
        }
        MosaicStallWatch(stallMillis = 200).use { watch ->
            mosaicWriteWatched(counting, bytes, watch)
            assertFalse(watch.stalled)
        }
        assertTrue(bytes.contentEquals(out.toByteArray()))
        assertEquals(listOf(65_536, 65_536, 65_536, 3_392), writes)
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
