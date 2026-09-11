import androidx.compose.ui.test.assertCountEquals
import androidx.compose.ui.test.assertContentDescriptionEquals
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.assertTextContains
import androidx.compose.ui.test.ExperimentalTestApi
import androidx.compose.ui.test.onAllNodesWithText
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performTextInput
import androidx.compose.ui.test.performTextReplacement
import androidx.compose.ui.test.runSkikoComposeUiTest
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.semantics.ScrollAxisRange
import androidx.compose.ui.semantics.SemanticsNode
import androidx.compose.ui.test.ComposeUiTest
import androidx.compose.ui.test.onRoot
import kotlin.test.assertEquals
import kotlin.test.assertFails
import org.junit.Test

private const val UI_TASK_NAME = "Native acceptance task"
private const val UI_EDITED_TASK_NAME = "Edited native task"
private const val UI_EDITED_DUE = "2026-01-12"
private const val UI_PERSISTED_TASK_NAME = "Persisted native task"
private const val UI_DUE = "2026-01-09"
private const val UI_SUMMARY = "1 task(s) · 0 done · projected finish 2026-01-05"

// The acceptance viewport is DECLARED, not inherited (#14771).
//
// This test previously used `createComposeRule()`, whose surface is whatever
// the framework defaults to -- 1024 x 768. Nobody chose that number, and the
// gate below ("must be DISPLAYED, not merely present in the semantics tree")
// silently meant "must fit in 1024 x 768". TaskApp's layout scrolls, so the
// first change that made it a few pixels taller pushed the freshly added task
// row to y = 768.0 and failed an assertion that was never about height.
//
// Width, not height, was the real constraint: at 1024 the composer and list
// wrap and the content grows to ~1012 px tall; at 1280 it reflows to ~538 px.
//
// So state the window instead of inheriting it. Every `assertIsDisplayed()`
// below keeps its full strength -- this is a real desktop window, and content
// still has to be on screen in it without scrolling.
//
// Two things this deliberately does NOT paper over, both filed separately:
//   * the generated app's own `Window` takes Compose's 800 x 600 default,
//     which is smaller than this and smaller than its content needs; and
//   * `performScrollTo()` hangs against the emitted scroll container, so
//     scrolling is not currently an option for reaching content below a fold.
private val ACCEPTANCE_VIEWPORT = Size(1280f, 900f)

/**
 * Asserts the app fits [ACCEPTANCE_VIEWPORT] vertically, with nothing pushed
 * below a fold.
 *
 * This exists because the gate it replaces was accidental. Every
 * `assertIsDisplayed()` below implies "fits the window", but only for the one
 * node it names, and only by luck of where that node happens to land. When
 * directional padding made the layout a few pixels taller, what failed was an
 * assertion about a task row -- reported as "is not displayed", with nothing
 * pointing at height.
 *
 * A scrollable that can scroll reports `maxValue > 0`: that IS the overflow, in
 * pixels. Asserting it is zero states the invariant directly, so the next
 * change that grows the layout fails saying so, at the stage where it happened.
 *
 * TaskApp is legitimately scrollable with enough tasks. The claim here is
 * narrow and about this lifecycle only: one or two tasks must not need
 * scrolling on a real desktop window.
 */
@OptIn(ExperimentalTestApi::class)
private fun ComposeUiTest.assertFitsViewport(stage: String) {
    var overflow = 0f
    fun walk(node: SemanticsNode) {
        for (entry in node.config) {
            if (entry.key.name == "VerticalScrollAxisRange") {
                val range = entry.value as? ScrollAxisRange ?: continue
                overflow = maxOf(overflow, range.maxValue())
            }
        }
        node.children.forEach(::walk)
    }
    walk(onRoot().fetchSemanticsNode())
    assertEquals(
        0f,
        overflow,
        "TaskApp overflows the ${ACCEPTANCE_VIEWPORT.width.toInt()} x " +
            "${ACCEPTANCE_VIEWPORT.height.toInt()} acceptance viewport by " +
            "${overflow}px at stage '$stage' -- content below the fold is not " +
            "reachable, because performScrollTo() hangs against the emitted " +
            "scroll container.",
    )
}

class TaskAppUiTest {
    @OptIn(ExperimentalTestApi::class)
    @Test
    fun generatedControlsDriveRustSchedulingLifecycle() = runSkikoComposeUiTest(
        size = ACCEPTANCE_VIEWPORT,
    ) {
        // Keeps the rest of the test reading exactly as it did under the rule.
        val compose = this
        val restoredOnLaunch = System.getenv("MOSAIC_EXPECT_RESTORED") == "1"
        val host = checkNotNull(MosaicRuntimeHost.load()) {
            "standard Compose binding did not load the TaskApp Rust runtime"
        }
        compose.setContent { MosaicApp(host) }
        compose.waitForIdle()

        if (restoredOnLaunch) {
            compose.onNodeWithText(UI_PERSISTED_TASK_NAME).assertIsDisplayed()
            compose.onNodeWithText("due $UI_DUE").assertIsDisplayed()
            assertFitsViewport("restored on launch")
            compose.onNodeWithText(UI_SUMMARY).assertIsDisplayed()
            compose.onNodeWithTag("del-btn").performClick()
            compose.waitForIdle()
            compose.onAllNodesWithText(UI_PERSISTED_TASK_NAME).assertCountEquals(0)
            return@runSkikoComposeUiTest
        }

        val before = host.snapshot()
        assertFails {
            host.handleEvent(
                mapOf(
                    "name" to "onNewTaskNameChange",
                    "payload" to mapOf("value" to 7),
                ),
            )
        }
        assertEquals(before, host.snapshot())
        compose.onAllNodesWithText(UI_TASK_NAME).assertCountEquals(0)

        compose.onNodeWithTag("name-input").performTextInput(UI_TASK_NAME)
        compose.waitForIdle()
        compose.onNodeWithTag("due-input").performTextInput(UI_DUE)
        compose.waitForIdle()
        compose.onNodeWithTag("add-btn").performClick()
        compose.waitForIdle()

        assertFitsViewport("after first add")
        compose.onNodeWithText(UI_TASK_NAME).assertIsDisplayed()
        compose.onNodeWithText("due $UI_DUE").assertIsDisplayed()
        // Scheduling is always projected in the Rust-owned summary. Do not
        // assume a complexity toggle selects Timeline: its next mode is Board.
        compose.onNodeWithText(UI_SUMMARY).assertIsDisplayed()

        compose.onNodeWithTag("edit-btn").performClick()
        compose.waitForIdle()
        compose.onNodeWithTag("edit-name-input").performTextReplacement(UI_EDITED_TASK_NAME)
        compose.onNodeWithTag("edit-due-input").performTextReplacement(UI_EDITED_DUE)
        compose.onNodeWithTag("edit-save-btn").performClick()
        compose.waitForIdle()
        compose.onNodeWithText(UI_EDITED_TASK_NAME).assertIsDisplayed()
        compose.onNodeWithText("due $UI_EDITED_DUE").assertIsDisplayed()

        compose.onNodeWithTag("toggle")
            .assertContentDescriptionEquals("Complete task: $UI_EDITED_TASK_NAME")
        compose.onNodeWithTag("toggle").performClick()
        compose.waitForIdle()
        compose.onNodeWithTag("toggle").assertTextContains("✓")
        compose.onNodeWithTag("toggle")
            .assertContentDescriptionEquals("Reopen task: $UI_EDITED_TASK_NAME")
        // The Rust-owned completion value must be visible, not merely present
        // in the semantics tree beyond the measured desktop viewport.
        assertFitsViewport("after completion")
        compose.onNodeWithText("100%").assertIsDisplayed()
        compose.onNodeWithTag("toggle").performClick()
        compose.waitForIdle()
        compose.onNodeWithTag("toggle").assertTextContains("○")
        compose.onNodeWithTag("toggle")
            .assertContentDescriptionEquals("Complete task: $UI_EDITED_TASK_NAME")

        compose.onNodeWithTag("del-btn").performClick()
        compose.waitForIdle()
        compose.onAllNodesWithText(UI_EDITED_TASK_NAME).assertCountEquals(0)

        // A successful add deliberately returns focus to the composer through
        // its focus-preserving branch. Exercise that live branch instead of
        // assuming the initial, never-focused control is still mounted.
        compose.onNodeWithTag("name-input-corrected").performTextInput(UI_PERSISTED_TASK_NAME)
        compose.waitForIdle()
        compose.onNodeWithTag("due-input").performTextInput(UI_DUE)
        compose.waitForIdle()
        compose.onNodeWithTag("add-btn").performClick()
        compose.waitForIdle()
        compose.onNodeWithText(UI_PERSISTED_TASK_NAME).assertIsDisplayed()
        compose.onNodeWithText(UI_SUMMARY).assertIsDisplayed()
    }
}
