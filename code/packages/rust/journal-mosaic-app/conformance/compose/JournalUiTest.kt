import androidx.compose.ui.geometry.Size
import androidx.compose.ui.semantics.SemanticsActions
import androidx.compose.ui.semantics.SemanticsProperties
import androidx.compose.ui.semantics.getOrNull
import androidx.compose.ui.test.ComposeUiTest
import androidx.compose.ui.test.ExperimentalTestApi
import androidx.compose.ui.test.SemanticsMatcher
import androidx.compose.ui.test.SemanticsNodeInteraction
import androidx.compose.ui.test.assertCountEquals
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.hasAnyDescendant
import androidx.compose.ui.test.hasContentDescription
import androidx.compose.ui.test.hasTestTag
import androidx.compose.ui.test.hasText
import androidx.compose.ui.test.onAllNodesWithTag
import androidx.compose.ui.test.onAllNodesWithText
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performSemanticsAction
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
// Timeline rows are mosaic-pkg-card Cards, each opened by the "Open" button
// beside it, tagged `timeline-entry-open` (or `-selected` for the entry open
// in the editor), so rows are matched by tag PREFIX. The button's visible
// text is "Open" on every row; its accessible name, the Compose
// `contentDescription`, is "Open {title}" (the app's open-label row field),
// so rows are matched by tag AND that name. (They were the toolkit
// RecordList's `record-list-title` buttons, matched by their text, before
// the timeline became Cards.)
//
// Rows are scrolled into view before they are asserted. A Card row with its
// Open button is about 220px tall in the declared 1100x760 window, and the
// pane's controls end about 500px down, so the second entry starts below the
// fold (misfit M7 in journal-component-demand-audit-v1.md). The pane scrolls
// (JO-5, `HostScroll [ timeline-scroll ]`, Compose's `verticalScroll`), and
// `inPane` below reaches a pane node the way a person would: back to the top,
// then down a bounded number of steps, each one the `ScrollBy` semantics
// action a scroll wheel or a screen reader's "scroll down" performs. Every
// row is still required to be DISPLAYED, not merely to exist: a row that is
// in the semantics tree but nowhere on screen is the bug JO-5 fixed.
// `performScrollTo()` is not used because it hangs against the emitted
// scroll container (EM-11), as in JournalScrollUiTest and TaskAppUiTest.

private const val UI_DRAFT_TITLE = "Drafted on Compose"
private const val UI_KEPT_TITLE = "Kept across a relaunch"
private const val UI_KEPT_BODY = "Saved by JournalUiTest; the restored launch must find it."
private const val UI_EMPTY = "No entries yet"

// journal-app's declared `[app]` window, as in JournalScreenshots.
private val ACCEPTANCE_VIEWPORT = Size(1100f, 760f)

private fun timelineRow(title: String): SemanticsMatcher =
    SemanticsMatcher("a timeline entry's Open button") { node ->
        node.config.getOrNull(SemanticsProperties.TestTag)?.startsWith("timeline-entry-open") == true
    } and hasContentDescription("Open $title")

// The pane's scroll container: the node with a vertical scroll range that
// holds the pane's first control (as in JournalScrollUiTest).
private val paneScroll: SemanticsMatcher =
    SemanticsMatcher.keyIsDefined(SemanticsProperties.VerticalScrollAxisRange) and
        hasAnyDescendant(hasTestTag("new-entry"))

private const val SCROLL_STEP_PX = 200f
private const val MAX_SCROLL_STEPS = 40

@OptIn(ExperimentalTestApi::class)
private fun ComposeUiTest.isDisplayed(matcher: SemanticsMatcher): Boolean =
    runCatching { onNode(matcher).assertIsDisplayed() }.isSuccess

@OptIn(ExperimentalTestApi::class)
private fun ComposeUiTest.paneScrollRange() =
    onNode(paneScroll).fetchSemanticsNode().config[SemanticsProperties.VerticalScrollAxisRange]

@OptIn(ExperimentalTestApi::class)
private fun ComposeUiTest.scrollPaneBy(px: Float) {
    onNode(paneScroll).performSemanticsAction(SemanticsActions.ScrollBy) { it(0f, px) }
    waitForIdle()
}

// Scroll the pane until the node [matcher] names is on screen, and return it,
// asserted displayed.
//
//   1. the node must exist at all (so a missing row fails as "not found", not
//      as a scroll that never ends);
//   2. scroll back to the top, so where the last step left the pane does not
//      matter, and a row ABOVE the viewport is reachable too;
//   3. scroll down, one `ScrollBy` step at a time, until the node is
//      displayed. Reaching the end of the scroll range first, or running out
//      of steps, fails with the node's name.
@OptIn(ExperimentalTestApi::class)
private fun ComposeUiTest.inPane(matcher: SemanticsMatcher, what: String): SemanticsNodeInteraction {
    onNode(matcher).assertExists("$what is not in the semantics tree")
    var steps = 0
    while (paneScrollRange().value() > 0f) {
        check(steps++ < MAX_SCROLL_STEPS) { "the pane did not scroll back to its top" }
        scrollPaneBy(-SCROLL_STEP_PX)
    }
    steps = 0
    while (!isDisplayed(matcher)) {
        val range = paneScrollRange()
        check(range.value() < range.maxValue()) {
            "$what is not displayed with the pane scrolled to its end"
        }
        check(steps++ < MAX_SCROLL_STEPS) {
            "$what was still not displayed after $MAX_SCROLL_STEPS scrolls of ${SCROLL_STEP_PX}px"
        }
        scrollPaneBy(SCROLL_STEP_PX)
    }
    return onNode(matcher).assertIsDisplayed()
}

@OptIn(ExperimentalTestApi::class)
private fun ComposeUiTest.row(title: String): SemanticsNodeInteraction =
    inPane(timelineRow(title), "the timeline row \"Open $title\"")

@OptIn(ExperimentalTestApi::class)
private fun ComposeUiTest.emptyState(): SemanticsNodeInteraction =
    inPane(hasText(UI_EMPTY), "the empty state \"$UI_EMPTY\"")

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
            row(UI_KEPT_TITLE)
            onAllNodesWithText(UI_EMPTY).assertCountEquals(0)
            row(UI_KEPT_TITLE).performClick()
            waitForIdle()
            onNodeWithTag("draft-editor-delete").performClick()
            waitForIdle()
            onAllNodes(timelineRow(UI_KEPT_TITLE)).assertCountEquals(0)
            emptyState()
            return@runSkikoComposeUiTest
        }

        emptyState()
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
        row(UI_DRAFT_TITLE)
        onAllNodesWithText(UI_EMPTY).assertCountEquals(0)
        // Saved, so the editor now offers Delete.
        onNodeWithTag("draft-editor-delete").assertIsDisplayed()

        inPane(hasTestTag("new-entry"), "the New entry button").performClick()
        waitForIdle()
        onAllNodesWithTag("draft-editor-delete").assertCountEquals(0)
        write(UI_KEPT_TITLE, UI_KEPT_BODY)
        // Both rows are displayed, each once the pane is scrolled to it: with
        // Cards the second (older) one starts below the fold.
        row(UI_KEPT_TITLE)
        row(UI_DRAFT_TITLE)

        // Open the first entry from the timeline and delete it.
        row(UI_DRAFT_TITLE).performClick()
        waitForIdle()
        onNodeWithTag("draft-editor-delete").performClick()
        waitForIdle()
        onAllNodes(timelineRow(UI_DRAFT_TITLE)).assertCountEquals(0)
        row(UI_KEPT_TITLE)
    }
}
