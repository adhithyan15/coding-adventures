package dev.codingadventures.journalapp.uitest

import androidx.compose.ui.semantics.SemanticsActions
import androidx.compose.ui.semantics.SemanticsProperties
import androidx.compose.ui.semantics.getOrNull
import androidx.compose.ui.test.SemanticsMatcher
import androidx.compose.ui.test.SemanticsNodeInteraction
import androidx.compose.ui.test.assertCountEquals
import androidx.compose.ui.test.hasTestTag
import androidx.compose.ui.test.hasText
import androidx.compose.ui.test.junit4.createAndroidComposeRule
import androidx.compose.ui.test.performSemanticsAction
import androidx.compose.ui.test.performTextInput
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import mosaic.android.MosaicActivity
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

// Journal on Android, driven through its generated controls (UI89 §4.1, §4.2).
//
// The emulator gate (§3.7) proves the app starts, persists and restores
// without being touched. This proves a person can use it: text typed into
// the generated editor reaches the Rust runtime, a saved entry appears in
// the timeline, and it is still there after a cold relaunch. It is the
// desktop JournalUiTest (conformance/compose/JournalUiTest.kt) on a phone,
// in two cold launches against the app's own storage:
//
//   launch 1   empty journal -> write and save two entries -> delete one
//   launch 2   (a new process) the survivor is back -> delete it -> empty
//
// The launch number comes from `am instrument -e mosaicLaunch N`
// (code/scripts/mosaic-android-ui-test.sh), the Android form of the desktop
// test's MOSAIC_EXPECT_RESTORED. The script clears the app before launch 1.
//
// The real MosaicActivity runs (createAndroidComposeRule), so this is the
// shipped app: real filesDir, real JNA load of libmosaic_app.so, real host.
//
// Why semantics actions rather than touches. On a phone the app shell's
// navigation split collapses (`collapse: auto`), so whether the timeline and
// the editor share the screen depends on the device's window size class. A
// touch lands wherever the node is drawn, or misses it when it is clipped.
// A semantics action (OnClick, text input) runs the control's own handler
// wherever it is drawn. The test therefore proves the controls reach the
// engine and the screen is rebuilt from what the engine answered, and it
// reads that screen through the semantics tree (assertExists). Asserting
// what is visible on a compact screen is the compact layout's own test.
//
// Timeline rows are the toolkit RecordList's buttons, tagged
// `record-list-title`. RecordList is mounted more than once ("On this day"
// sits above the timeline) and a later mount's parts are suffixed `-m2`, so
// rows are matched by tag PREFIX and by text, as on the desktop.

private const val DRAFT_TITLE = "Drafted on Android"
private const val KEPT_TITLE = "Kept across an Android relaunch"
private const val KEPT_BODY = "Saved by JournalAndroidUiTest; launch 2 must find it."
private const val EMPTY = "No entries yet"

// The engine loads off the main thread (MosaicStartup), and the emulator is
// software-rendered: allow for a slow first frame.
private const val STARTUP_TIMEOUT_MS = 60_000L

private fun timelineRow(title: String): SemanticsMatcher =
    SemanticsMatcher("a RecordList row button") { node ->
        node.config.getOrNull(SemanticsProperties.TestTag)?.startsWith("record-list-title") == true
    } and hasText(title)

private fun SemanticsNodeInteraction.activate(): SemanticsNodeInteraction =
    performSemanticsAction(SemanticsActions.OnClick)

@RunWith(AndroidJUnit4::class)
class JournalAndroidUiTest {
    @get:Rule
    val compose = createAndroidComposeRule<MosaicActivity>()

    private fun awaitEditor() {
        compose.waitUntil(STARTUP_TIMEOUT_MS) {
            compose.onAllNodes(hasTestTag("mosaic-startup-failure")).fetchSemanticsNodes().isNotEmpty() ||
                compose.onAllNodes(hasTestTag("draft-editor-title")).fetchSemanticsNodes().isNotEmpty()
        }
        compose.onAllNodes(hasTestTag("mosaic-startup-failure")).assertCountEquals(0)
        compose.waitForIdle()
    }

    private fun write(title: String, body: String) {
        compose.onNode(hasTestTag("draft-editor-title")).performTextInput(title)
        compose.waitForIdle()
        compose.onNode(hasTestTag("draft-editor-body")).performTextInput(body)
        compose.waitForIdle()
        compose.onNode(hasTestTag("draft-editor-save")).activate()
        compose.waitForIdle()
    }

    @Test
    fun generatedControlsWriteSaveAndRestoreEntries() {
        val launch = InstrumentationRegistry.getArguments().getString("mosaicLaunch")
        check(launch == "1" || launch == "2") {
            "run with -e mosaicLaunch 1 or 2 (code/scripts/mosaic-android-ui-test.sh), got $launch"
        }
        awaitEditor()

        if (launch == "2") {
            // Restored from storage by a new process, not re-typed.
            compose.onNode(timelineRow(KEPT_TITLE)).assertExists()
            compose.onAllNodes(hasText(EMPTY)).assertCountEquals(0)
            compose.onNode(timelineRow(KEPT_TITLE)).activate()
            compose.waitForIdle()
            compose.onNode(hasTestTag("draft-editor-delete")).activate()
            compose.waitForIdle()
            compose.onAllNodes(timelineRow(KEPT_TITLE)).assertCountEquals(0)
            compose.onNode(hasText(EMPTY)).assertExists()
            return
        }

        compose.onNode(hasText(EMPTY)).assertExists()
        // A draft that has never been saved has nothing to delete.
        compose.onAllNodes(hasTestTag("draft-editor-delete")).assertCountEquals(0)

        write(DRAFT_TITLE, "The first entry, deleted before the relaunch.")
        compose.onNode(timelineRow(DRAFT_TITLE)).assertExists()
        compose.onAllNodes(hasText(EMPTY)).assertCountEquals(0)
        // Saved, so the editor now offers Delete.
        compose.onNode(hasTestTag("draft-editor-delete")).assertExists()

        compose.onNode(hasTestTag("new-entry")).activate()
        compose.waitForIdle()
        compose.onAllNodes(hasTestTag("draft-editor-delete")).assertCountEquals(0)
        write(KEPT_TITLE, KEPT_BODY)
        compose.onNode(timelineRow(KEPT_TITLE)).assertExists()
        compose.onNode(timelineRow(DRAFT_TITLE)).assertExists()

        // Open the first entry from the timeline and delete it.
        compose.onNode(timelineRow(DRAFT_TITLE)).activate()
        compose.waitForIdle()
        compose.onNode(hasTestTag("draft-editor-delete")).activate()
        compose.waitForIdle()
        compose.onAllNodes(timelineRow(DRAFT_TITLE)).assertCountEquals(0)
        compose.onNode(timelineRow(KEPT_TITLE)).assertExists()
    }
}
