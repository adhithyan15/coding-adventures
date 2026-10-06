package dev.codingadventures.engramapp.uitest

import android.app.Activity
import android.app.Instrumentation.ActivityResult
import android.content.Intent
import android.net.Uri
import androidx.compose.ui.semantics.SemanticsActions
import androidx.compose.ui.test.ComposeTimeoutException
import androidx.compose.ui.test.SemanticsNodeInteraction
import androidx.compose.ui.test.assertCountEquals
import androidx.compose.ui.test.hasTestTag
import androidx.compose.ui.test.hasText
import androidx.compose.ui.test.junit4.createAndroidComposeRule
import androidx.compose.ui.test.performSemanticsAction
import androidx.test.espresso.intent.Intents
import androidx.test.espresso.intent.Intents.intended
import androidx.test.espresso.intent.Intents.intending
import androidx.test.espresso.intent.matcher.IntentMatchers.hasAction
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import java.io.File
import mosaic.android.MosaicActivity
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

// Engram on Android: Anki export and import through the document picker,
// answered by the test (UI89 §4.4).
//
// The system draws the picker (DocumentsUI), in another app, with a layout
// that changes from one Android release to the next, so this test does not
// drive it. Espresso-Intents answers it instead. The Activity Result API
// starts the picker with startActivityForResult, which Espresso-Intents
// intercepts, and the test answers with a document of its own. Everything
// past the picker is the shipped app: Engram's handler, the platform
// library's router (one file operation at a time, the size limit, the
// background read and write), and the engine that writes and merges the
// package.
//
// The document is a file:// URI in the app's own cache. The app and this
// test share one process, so the test can write it and read it, and the
// router opens it through ContentResolver like any document. No provider is
// declared, in the app or in the test APK. The router never asks a chosen
// document's name to pass a check: a file:// URI has no DISPLAY_NAME, so
// the app is told "document", and Engram does not use the name.
//
//   launch 1   a fresh collection (no deck called "Default" anywhere) ->
//              Export: the picker is asked to CREATE a document, and the file
//              becomes a whole zip (local header first, end record last) ->
//              Import that same file: the picker is asked to OPEN one, and
//              the package's "Default" deck appears in the deck list
//   launch 2   (a new process) the imported deck is still listed: the
//              merge was saved, not only drawn
//
// The engine's side of this round trip is pinned in engram-mosaic-app
// (a_fresh_collection_exports_a_zip_whose_import_lists_the_default_deck).
// The launch number comes from `am instrument -e mosaicLaunch N`
// (code/scripts/mosaic-android-ui-test.sh), which clears the app first.

private const val IMPORTED_DECK = "Default"

// The engine loads off the main thread, and the emulator is
// software-rendered: allow for a slow first frame.
private const val STARTUP_TIMEOUT_MS = 60_000L
private const val TRANSFER_TIMEOUT_MS = 20_000L
private const val IMPORT_TIMEOUT_MS = 10_000L

// A zip's first four bytes (a local file header) and the start of its end of
// central directory record. Engram writes no archive comment, so that record
// is the file's last 22 bytes.
private val LOCAL_HEADER = byteArrayOf(0x50, 0x4B, 0x03, 0x04)
private val END_RECORD = byteArrayOf(0x50, 0x4B, 0x05, 0x06)

private fun SemanticsNodeInteraction.activate(): SemanticsNodeInteraction =
    performSemanticsAction(SemanticsActions.OnClick)

/** True once [file] holds a whole zip: it starts and ends like one. */
private fun isWholeZip(file: File): Boolean {
    val bytes = if (file.isFile) file.readBytes() else return false
    return bytes.size >= 26 &&
        bytes.copyOfRange(0, 4).contentEquals(LOCAL_HEADER) &&
        bytes.copyOfRange(bytes.size - 22, bytes.size - 18).contentEquals(END_RECORD)
}

@RunWith(AndroidJUnit4::class)
class EngramAndroidUiTest {
    @get:Rule
    val compose = createAndroidComposeRule<MosaicActivity>()

    private fun awaitDecks() {
        compose.waitUntil(STARTUP_TIMEOUT_MS) {
            compose.onAllNodes(hasTestTag("mosaic-startup-failure")).fetchSemanticsNodes().isNotEmpty() ||
                compose.onAllNodes(hasTestTag("export-button")).fetchSemanticsNodes().isNotEmpty()
        }
        compose.onAllNodes(hasTestTag("mosaic-startup-failure")).assertCountEquals(0)
        compose.waitForIdle()
    }

    // The deck list's row for the imported deck: DeckStatsPanel draws each
    // deck as a `deck-option-button` labelled with its name.
    private fun deckListed(): Boolean =
        compose.onAllNodes(hasTestTag("deck-option-button") and hasText(IMPORTED_DECK))
            .fetchSemanticsNodes().isNotEmpty()

    @Test
    fun ankiExportAndImportRoundTripThroughThePicker() {
        val launch = InstrumentationRegistry.getArguments().getString("mosaicLaunch")
        check(launch == "1" || launch == "2") {
            "run with -e mosaicLaunch 1 or 2 (code/scripts/mosaic-android-ui-test.sh), got $launch"
        }
        awaitDecks()

        if (launch == "2") {
            // Restored from storage by a new process, not re-imported.
            check(deckListed()) { "the imported deck was not restored" }
            return
        }

        // Nothing says "Default" before the import, so seeing it after is
        // the import's doing.
        compose.onAllNodes(hasText(IMPORTED_DECK)).assertCountEquals(0)
        val context = InstrumentationRegistry.getInstrumentation().targetContext
        val document = File(context.cacheDir, "engram-ui-test.apkg")
        document.delete()
        val chosen = ActivityResult(Activity.RESULT_OK, Intent().setData(Uri.fromFile(document)))

        Intents.init()
        try {
            // Export: the picker is asked where to save, and "chooses" the
            // test's file. The router writes the package there off the main
            // thread, so the test waits for the whole zip.
            intending(hasAction(Intent.ACTION_CREATE_DOCUMENT)).respondWith(chosen)
            compose.onNode(hasTestTag("export-button")).activate()
            compose.waitUntil(TRANSFER_TIMEOUT_MS) { isWholeZip(document) }
            intended(hasAction(Intent.ACTION_CREATE_DOCUMENT))

            // Import: the picker is asked for a document, and "chooses" the
            // file the export wrote. The router allows one file operation at
            // a time, and the export's answer reaches the main thread just
            // after its last byte, so an Import tapped in between is refused
            // (as it would be for a person). So it is tapped again, at most
            // three times in all, until the deck appears.
            intending(hasAction(Intent.ACTION_OPEN_DOCUMENT)).respondWith(chosen)
            var imported = false
            for (attempt in 1..3) {
                compose.onNode(hasTestTag("import-button")).activate()
                imported = try {
                    compose.waitUntil(IMPORT_TIMEOUT_MS) { deckListed() }
                    true
                } catch (timeout: ComposeTimeoutException) {
                    false
                }
                if (imported) break
            }
            check(imported) { "the imported package's \"$IMPORTED_DECK\" deck never appeared" }
            intended(hasAction(Intent.ACTION_OPEN_DOCUMENT))
        } finally {
            Intents.release()
        }
    }
}
