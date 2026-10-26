import androidx.compose.ui.geometry.Size
import androidx.compose.ui.semantics.SemanticsActions
import androidx.compose.ui.semantics.SemanticsProperties
import androidx.compose.ui.semantics.getOrNull
import androidx.compose.ui.test.ComposeUiTest
import androidx.compose.ui.test.ExperimentalTestApi
import androidx.compose.ui.test.SemanticsMatcher
import androidx.compose.ui.test.assert
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.assertIsNotDisplayed
import androidx.compose.ui.test.hasAnyDescendant
import androidx.compose.ui.test.hasTestTag
import androidx.compose.ui.test.hasText
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performSemanticsAction
import androidx.compose.ui.test.performTextInput
import androidx.compose.ui.test.runSkikoComposeUiTest
import kotlin.test.assertEquals
import org.junit.Test

// Journal's pane scrolls, so an entry below the fold can be reached (JO-5).
//
// A native window scrolls nothing unless the layout says so. Until the pane
// was wrapped in `HostScroll [ timeline-scroll ]`, Journal's declared 1100x760
// window showed about two timeline rows under the pane's controls, and every
// older entry was in the semantics tree but nowhere on screen: no scrollbar,
// no wheel, no keyboard. A test that only asks whether a node EXISTS cannot
// see that, which is why this one asks whether it is DISPLAYED.
//
// The test:
//
//   1. writes ENTRIES entries, more than the window can show;
//   2. takes the bottom-most timeline row and requires that it is NOT
//      displayed, so the test really crosses the fold rather than passing on
//      a window that happens to be tall enough;
//   3. scrolls the pane with the `ScrollBy` semantics action, a bounded number
//      of steps, until that row IS displayed;
//   4. opens it, and requires that the editor now holds that entry.
//
// Why `ScrollBy` and not `performScrollTo()`: the latter hangs against the
// emitted scroll container (EM-11, the same reason TaskAppUiTest avoids it).
// `ScrollBy` is the action a scroll wheel or a screen reader's "scroll down"
// performs, so it is also the honest model of how a person reaches the row.
//
// It runs on its own state file, apart from JournalUiTest's two-launch
// journal, because it leaves eight entries behind.

private val ACCEPTANCE_VIEWPORT = Size(1100f, 760f) // journal-app's [app] window
private const val ENTRIES = 8
private const val SCROLL_STEP_PX = 200f
private const val MAX_SCROLL_STEPS = 40

// Timeline rows are the toolkit RecordList's buttons, tagged
// `record-list-title` (`-selected` for the open entry, `-m2` for a later
// mount), so rows are matched by tag PREFIX, as in JournalUiTest.
private val timelineRowButton: SemanticsMatcher =
    SemanticsMatcher("a timeline row button") { node ->
        node.config.getOrNull(SemanticsProperties.TestTag)?.startsWith("record-list-title") == true
    }

// The pane's scroll container: the node with a vertical scroll range that
// holds the pane's first control.
private val paneScroll: SemanticsMatcher =
    SemanticsMatcher.keyIsDefined(SemanticsProperties.VerticalScrollAxisRange) and
        hasAnyDescendant(hasTestTag("new-entry"))

@OptIn(ExperimentalTestApi::class)
private fun ComposeUiTest.write(title: String, body: String) {
    onNodeWithTag("new-entry").performClick()
    waitForIdle()
    onNodeWithTag("draft-editor-title").performTextInput(title)
    waitForIdle()
    onNodeWithTag("draft-editor-body").performTextInput(body)
    waitForIdle()
    onNodeWithTag("draft-editor-save").performClick()
    waitForIdle()
}

@OptIn(ExperimentalTestApi::class)
private fun ComposeUiTest.isDisplayed(matcher: SemanticsMatcher): Boolean =
    runCatching { onNode(matcher).assertIsDisplayed() }.isSuccess

class JournalScrollUiTest {
    @OptIn(ExperimentalTestApi::class)
    @Test
    fun anEntryBelowTheFoldCanBeScrolledToAndOpened() = runSkikoComposeUiTest(
        size = ACCEPTANCE_VIEWPORT,
    ) {
        val host = checkNotNull(MosaicRuntimeHost.load()) {
            "standard Compose binding did not load the Journal Rust runtime"
        }
        val initialResponse = checkNotNull(host.props()) {
            "standard Compose binding returned no Journal startup props"
        }
        setContent { MosaicApp(host, initialResponse) }
        waitForIdle()

        // A fresh journal: this test owns its state file.
        onNodeWithText("No entries yet").assertIsDisplayed()
        for (n in 1..ENTRIES) write("Reach $n", "Entry $n of $ENTRIES.")

        // 2. The bottom-most row: the last in the semantics tree, which is the
        //    order the pane lays its rows out in. (Not by `boundsInRoot`: a
        //    scroll container clips its children's bounds to its viewport, so
        //    every row below the fold reports the same clipped edge.)
        val rows = onAllNodes(timelineRowButton).fetchSemanticsNodes()
        assertEquals(ENTRIES, rows.size, "one timeline row per entry")
        val bottom = rows.last()
        val title = bottom.config[SemanticsProperties.Text].joinToString("") { it.text }
        val lastRow = timelineRowButton and hasText(title)
        onNode(lastRow).assertIsNotDisplayed()

        // 3. Scroll the pane until the row is on screen.
        var steps = 0
        while (!isDisplayed(lastRow)) {
            check(steps++ < MAX_SCROLL_STEPS) {
                "\"$title\" was still not displayed after $MAX_SCROLL_STEPS scrolls of " +
                    "${SCROLL_STEP_PX}px: the pane does not scroll it into view"
            }
            onNode(paneScroll).performSemanticsAction(SemanticsActions.ScrollBy) {
                it(0f, SCROLL_STEP_PX)
            }
            waitForIdle()
        }

        // 4. Open it: the editor holds that entry.
        onNode(lastRow).assertIsDisplayed().performClick()
        waitForIdle()
        onNodeWithTag("draft-editor-title").assert(hasText(title))
    }
}
