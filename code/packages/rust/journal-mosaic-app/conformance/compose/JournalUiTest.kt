import androidx.compose.ui.geometry.Size
import androidx.compose.ui.test.ComposeUiTest
import androidx.compose.ui.test.ExperimentalTestApi
import androidx.compose.ui.test.SemanticsMatcher
import androidx.compose.ui.test.assertCountEquals
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.semantics.SemanticsProperties
import androidx.compose.ui.semantics.getOrNull
import androidx.compose.ui.test.hasText
import androidx.compose.ui.test.onAllNodesWithTag
import androidx.compose.ui.test.onAllNodesWithText
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performTextInput
import androidx.compose.ui.test.runSkikoComposeUiTest
import kotlin.test.assertEquals
import kotlin.test.assertFails
import org.junit.Test

// Journal on Compose, driven through its generated controls (J5, #14416).
//
// The Compose lane used to stop at `compileKotlin`: the app built, but
// nothing proved that typing into the generated editor reaches the Rust
// runtime, that a saved entry shows up in the timeline, or that it survives
// a relaunch. This test drives the real generated app over the real Rust
// runtime (`MosaicRuntimeHost.load()`, the same binding the shipped app
// uses), in the window the manifest declares.
//
// It runs twice against one state file, as TaskAppUiTest does:
//
//   first launch    empty journal -> write and save two entries -> delete one
//   restored launch (MOSAIC_EXPECT_RESTORED=1) the survivor is back -> delete
//                   it -> the journal is empty again
//
// Timeline rows are the toolkit RecordList's buttons, tagged
// `record-list-title` (or `-selected` for the entry open in the editor).
// RecordList is mounted more than once (On this day, J4c, sits above the
// timeline), and the resolver suffixes a later mount's parts (`-m2`,
// #15959), so rows are matched by tag PREFIX. The editor's title field holds
// the same text as its row, so rows are matched by tag AND text rather than
// by text alone.

private const val UI_DRAFT_TITLE = "Drafted on Compose"
private const val UI_KEPT_TITLE = "Kept across a relaunch"
private const val UI_KEPT_BODY = "Saved by JournalUiTest; the restored launch must find it."
private const val UI_EMPTY = "No entries yet"

// journal-app's declared `[app]` window, as in JournalScreenshots.
private val ACCEPTANCE_VIEWPORT = Size(1100f, 760f)

private fun timelineRow(title: String): SemanticsMatcher =
    SemanticsMatcher("a RecordList row button") { node ->
        node.config.getOrNull(SemanticsProperties.TestTag)?.startsWith("record-list-title") == true
    } and hasText(title)

@OptIn(ExperimentalTestApi::class)
private fun ComposeUiTest.write(title: String, body: String) {
    onNodeWithTag("draft-editor-title").performTextInput(title)
    waitForIdle()
    onNodeWithTag("draft-editor-body").performTextInput(body)
    waitForIdle()
    onNodeWithTag("draft-editor-save").performClick()
    waitForIdle()
}

class JournalUiTest {
    @OptIn(ExperimentalTestApi::class)
    @Test
    fun generatedControlsWriteSaveAndRestoreEntries() = runSkikoComposeUiTest(
        size = ACCEPTANCE_VIEWPORT,
    ) {
        val restoredOnLaunch = System.getenv("MOSAIC_EXPECT_RESTORED") == "1"
        val host = checkNotNull(MosaicRuntimeHost.load()) {
            "standard Compose binding did not load the Journal Rust runtime"
        }
        val initialResponse = checkNotNull(host.props()) {
            "standard Compose binding returned no Journal startup props"
        }
        setContent { MosaicApp(host, initialResponse) }
        waitForIdle()

        if (restoredOnLaunch) {
            onNode(timelineRow(UI_KEPT_TITLE)).assertIsDisplayed()
            onAllNodesWithText(UI_EMPTY).assertCountEquals(0)
            onNode(timelineRow(UI_KEPT_TITLE)).performClick()
            waitForIdle()
            onNodeWithTag("draft-editor-delete").performClick()
            waitForIdle()
            onAllNodes(timelineRow(UI_KEPT_TITLE)).assertCountEquals(0)
            onNodeWithText(UI_EMPTY).assertIsDisplayed()
            return@runSkikoComposeUiTest
        }

        onNodeWithText(UI_EMPTY).assertIsDisplayed()
        // A draft that has never been saved has nothing to delete.
        onAllNodesWithTag("draft-editor-delete").assertCountEquals(0)

        // A malformed event is refused and changes nothing (the runtime's
        // payload validation, reached through the same binding).
        val before = host.snapshot()
        assertFails {
            host.handleEvent(
                mapOf("name" to "onTitleChange", "payload" to mapOf("value" to 7)),
            )
        }
        assertEquals(before, host.snapshot())

        write(UI_DRAFT_TITLE, "The first entry, deleted before the relaunch.")
        onNode(timelineRow(UI_DRAFT_TITLE)).assertIsDisplayed()
        onAllNodesWithText(UI_EMPTY).assertCountEquals(0)
        // Saved, so the editor now offers Delete.
        onNodeWithTag("draft-editor-delete").assertIsDisplayed()

        onNodeWithTag("new-entry").performClick()
        waitForIdle()
        onAllNodesWithTag("draft-editor-delete").assertCountEquals(0)
        write(UI_KEPT_TITLE, UI_KEPT_BODY)
        onNode(timelineRow(UI_KEPT_TITLE)).assertIsDisplayed()
        onNode(timelineRow(UI_DRAFT_TITLE)).assertIsDisplayed()

        // Open the first entry from the timeline and delete it.
        onNode(timelineRow(UI_DRAFT_TITLE)).performClick()
        waitForIdle()
        onNodeWithTag("draft-editor-delete").performClick()
        waitForIdle()
        onAllNodes(timelineRow(UI_DRAFT_TITLE)).assertCountEquals(0)
        onNode(timelineRow(UI_KEPT_TITLE)).assertIsDisplayed()
    }
}
