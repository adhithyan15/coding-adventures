import androidx.compose.ui.geometry.Size
import androidx.compose.ui.semantics.SemanticsActions
import androidx.compose.ui.semantics.SemanticsProperties
import androidx.compose.ui.test.ComposeUiTest
import androidx.compose.ui.test.ExperimentalTestApi
import androidx.compose.ui.test.SemanticsMatcher
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.assertIsNotDisplayed
import androidx.compose.ui.test.hasAnyDescendant
import androidx.compose.ui.test.hasTestTag
import androidx.compose.ui.test.hasText
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performSemanticsAction
import androidx.compose.ui.test.runSkikoComposeUiTest
import kotlin.test.assertEquals
import kotlin.test.assertTrue
import org.junit.Test

// Engram's screens scroll, so a control below the fold can be reached (EN-6).
//
// A native window scrolls nothing unless the layout says so. Until the screens
// were wrapped in `HostScroll [ content-scroll ]`, everything below the bottom
// edge of Engram's window was in the semantics tree but nowhere on screen: no
// scrollbar, no wheel, no keyboard. The Options screen is the plainest case.
// Its deck options alone fill an 800x600 window, so the note type editor under
// them could not be reached at all. A test that only asks whether a node
// EXISTS cannot see that, which is why this one asks whether it is DISPLAYED.
//
// The test:
//
//   1. opens Options in the window Engram really opens in (no `[app]` window
//      is declared, so Compose Desktop's default, 800x600);
//   2. requires that the note type editor's New button is NOT displayed, so
//      the test really crosses the fold rather than passing on a window that
//      happens to be tall enough;
//   3. scrolls the screen with the `ScrollBy` semantics action, a bounded
//      number of steps, until that button IS displayed;
//   4. clicks it, and requires that the engine started a new note type: its
//      minted `note-type-...` id appears, where before there was none;
//   5. requires that the screen switcher is still displayed. The header sits
//      outside the scroll on purpose, so scrolling a screen never scrolls the
//      way to the other screens off the top.
//
// Why `ScrollBy` and not `performScrollTo()`: the latter hangs against the
// emitted scroll container (EM-11, the same reason TaskAppUiTest and
// JournalScrollUiTest avoid it). `ScrollBy` is the action a scroll wheel or a
// screen reader's "scroll down" performs, so it is also the honest model of
// how a person reaches the button.
//
// Run, against a generated Compose project (CI's Compose lane does this):
//
//   cp EngramScrollUiTest.kt <generated>/compose/src/test/kotlin/
//   MOSAIC_APP_STATE_PATH=<fresh file> gradle -p <generated>/compose \
//     test --tests EngramScrollUiTest

private val DEFAULT_WINDOW = Size(800f, 600f) // Compose Desktop's bare Window()
private const val SCROLL_STEP_PX = 200f
private const val MAX_SCROLL_STEPS = 40

// The last row of controls on the Options screen: the note type editor's
// New / Save / Delete / Cancel. New is the one whose effect can be seen.
private val newNoteType: SemanticsMatcher = hasTestTag("note-type-new-button")

// The screen's scroll container: the node with a vertical scroll range that
// holds the button.
private val screenScroll: SemanticsMatcher =
    SemanticsMatcher.keyIsDefined(SemanticsProperties.VerticalScrollAxisRange) and
        hasAnyDescendant(newNoteType)

// A note type the editor has started. `start_new` mints an id of this shape,
// and the editor shows it as the note type id's value.
private val mintedNoteTypeId: SemanticsMatcher = hasText("note-type-", substring = true)

@OptIn(ExperimentalTestApi::class)
private fun ComposeUiTest.isDisplayed(matcher: SemanticsMatcher): Boolean =
    runCatching { onNode(matcher).assertIsDisplayed() }.isSuccess

class EngramScrollUiTest {
    @OptIn(ExperimentalTestApi::class)
    @Test
    fun aControlBelowTheFoldCanBeScrolledToAndUsed() = runSkikoComposeUiTest(
        size = DEFAULT_WINDOW,
    ) {
        // Clicking New changes the collection the runtime snapshots, so this
        // must never run against a real one: without MOSAIC_APP_STATE_PATH the
        // host falls back to the app's per-user data file. Require a fresh,
        // isolated one.
        val statePath = checkNotNull(System.getenv("MOSAIC_APP_STATE_PATH")?.takeIf { it.isNotBlank() }) {
            "set MOSAIC_APP_STATE_PATH to a fresh temporary file; this test changes the collection"
        }
        val stateFile = java.io.File(statePath)
        check(!stateFile.exists() || stateFile.length() == 0L) {
            "MOSAIC_APP_STATE_PATH ($statePath) already holds state; point it at a fresh file"
        }
        val host = checkNotNull(MosaicRuntimeHost.load()) {
            "standard Compose binding did not load the Engram Rust runtime"
        }
        val initialResponse = checkNotNull(host.props()) {
            "standard Compose binding returned no Engram startup props"
        }
        setContent { MosaicApp(host, initialResponse) }
        waitForIdle()

        // 1. Options, through the nav's own control (as EngramScreenshots
        //    does: "Options" is matched by the switcher's tag and its label).
        val optionsTab = hasTestTag("segmented-option") and hasText("Options")
        onNode(optionsTab).performClick()
        waitForIdle()

        // 2. The button exists, but is below the fold.
        assertEquals(1, onAllNodes(newNoteType).fetchSemanticsNodes().size, "one New note type button")
        onNode(newNoteType).assertIsNotDisplayed()
        assertTrue(
            onAllNodes(mintedNoteTypeId).fetchSemanticsNodes().isEmpty(),
            "a fresh collection has no note type in the editor yet",
        )

        // 3. Scroll the screen until the button is on screen.
        var steps = 0
        while (!isDisplayed(newNoteType)) {
            check(steps++ < MAX_SCROLL_STEPS) {
                "New note type was still not displayed after $MAX_SCROLL_STEPS scrolls of " +
                    "${SCROLL_STEP_PX}px: the screen does not scroll it into view"
            }
            onNode(screenScroll).performSemanticsAction(SemanticsActions.ScrollBy) {
                it(0f, SCROLL_STEP_PX)
            }
            waitForIdle()
        }

        // 4. Use it: the engine starts a new note type.
        onNode(newNoteType).assertIsDisplayed().performClick()
        waitForIdle()
        assertTrue(
            onAllNodes(mintedNoteTypeId).fetchSemanticsNodes().isNotEmpty(),
            "clicking New note type puts a new note type in the editor",
        )

        // 5. The switcher stayed put. Options is the selected segment now,
        //    which the toolkit tags `segmented-option-selected`.
        onNode(hasTestTag("segmented-option-selected") and hasText("Options")).assertIsDisplayed()
    }
}
