package dev.codingadventures.trestle.uitest

import androidx.compose.ui.semantics.SemanticsActions
import androidx.compose.ui.test.SemanticsMatcher
import androidx.compose.ui.test.SemanticsNodeInteraction
import androidx.compose.ui.test.assertContentDescriptionEquals
import androidx.compose.ui.test.assertCountEquals
import androidx.compose.ui.test.assertTextContains
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

// Trestle on Android, driven through its generated controls (UI89 §4.1, §4.4).
//
// The emulator gate (§3.7) proves the app starts, persists and restores
// without being touched. This proves a person can use it: a task typed into
// the composer reaches the Rust scheduler, its completion toggle and Delete
// button act on it, and a saved task is still there after a cold relaunch.
// It is the desktop TaskAppUiTest (conformance/compose/TaskAppUiTest.kt) on
// a phone, in two cold launches against the app's own storage:
//
//   launch 1   empty Inbox -> add a task -> complete it, reopen it -> delete
//              it -> add the task that must survive
//   launch 2   (a new process) the survivor is back -> delete it -> empty
//
// The launch number comes from `am instrument -e mosaicLaunch N`
// (code/scripts/mosaic-android-ui-test.sh), which clears the app before
// launch 1. Semantics actions rather than touches, for the reason
// JournalAndroidUiTest gives: on a phone the composer row and the task row
// are wider than the screen, and an action runs the control's handler
// wherever the layout put it.
//
// Two layout facts the matchers follow (TaskApp.mll):
//
//   * After a successful add the engine sets `new-task-name-focus`, and the
//     composer swaps `name-input` for `name-input-corrected` so the field
//     keeps keyboard focus. Either is "the task name field".
//   * Each task row's buttons are tagged `toggle`, `edit-btn` and `del-btn`.
//     The test keeps at most one task, so a tag names one node.

private const val FIRST_TASK = "Scheduled on Android"
private const val KEPT_TASK = "Kept across an Android relaunch"
private const val DUE = "2026-01-09"
private const val EMPTY = "Your Inbox is ready"

// The engine loads off the main thread (MosaicStartup), and the emulator is
// software-rendered: allow for a slow first frame.
private const val STARTUP_TIMEOUT_MS = 60_000L

private fun hasAnyTag(vararg tags: String): SemanticsMatcher =
    tags.map(::hasTestTag).reduce { either, tag -> either or tag }

private val taskNameField = hasAnyTag("name-input", "name-input-corrected")
private val dueField = hasAnyTag("due-input", "due-input-corrected")

private fun SemanticsNodeInteraction.activate(): SemanticsNodeInteraction =
    performSemanticsAction(SemanticsActions.OnClick)

@RunWith(AndroidJUnit4::class)
class TrestleAndroidUiTest {
    @get:Rule
    val compose = createAndroidComposeRule<MosaicActivity>()

    private fun awaitComposer() {
        compose.waitUntil(STARTUP_TIMEOUT_MS) {
            compose.onAllNodes(hasTestTag("mosaic-startup-failure")).fetchSemanticsNodes().isNotEmpty() ||
                compose.onAllNodes(taskNameField).fetchSemanticsNodes().isNotEmpty()
        }
        compose.onAllNodes(hasTestTag("mosaic-startup-failure")).assertCountEquals(0)
        compose.waitForIdle()
    }

    private fun add(name: String) {
        compose.onNode(taskNameField).performTextInput(name)
        compose.waitForIdle()
        compose.onNode(dueField).performTextInput(DUE)
        compose.waitForIdle()
        compose.onNode(hasTestTag("add-btn")).activate()
        compose.waitForIdle()
    }

    private fun deleteTheTask() {
        compose.onNode(hasTestTag("del-btn")).activate()
        compose.waitForIdle()
    }

    @Test
    fun generatedControlsAddCompleteDeleteAndRestoreTasks() {
        val launch = InstrumentationRegistry.getArguments().getString("mosaicLaunch")
        check(launch == "1" || launch == "2") {
            "run with -e mosaicLaunch 1 or 2 (code/scripts/mosaic-android-ui-test.sh), got $launch"
        }
        awaitComposer()

        if (launch == "2") {
            // Restored from storage by a new process, not re-typed.
            compose.onNode(hasText(KEPT_TASK)).assertExists()
            compose.onNode(hasText("due $DUE")).assertExists()
            compose.onAllNodes(hasText(EMPTY)).assertCountEquals(0)
            deleteTheTask()
            compose.onAllNodes(hasText(KEPT_TASK)).assertCountEquals(0)
            compose.onNode(hasText(EMPTY)).assertExists()
            return
        }

        compose.onNode(hasText(EMPTY)).assertExists()
        compose.onAllNodes(hasTestTag("toggle")).assertCountEquals(0)

        add(FIRST_TASK)
        compose.onNode(hasText(FIRST_TASK)).assertExists()
        compose.onNode(hasText("due $DUE")).assertExists()
        compose.onAllNodes(hasText(EMPTY)).assertCountEquals(0)

        // The completion toggle: its glyph and its action-oriented name both
        // come from the engine's row (cells 0 and 16), so each tap is a round
        // trip through the scheduler.
        val toggle = compose.onNode(hasTestTag("toggle"))
        toggle.assertContentDescriptionEquals("Complete task: $FIRST_TASK")
        toggle.activate()
        compose.waitForIdle()
        toggle.assertTextContains("✓")
        toggle.assertContentDescriptionEquals("Reopen task: $FIRST_TASK")
        toggle.activate()
        compose.waitForIdle()
        toggle.assertTextContains("○")
        toggle.assertContentDescriptionEquals("Complete task: $FIRST_TASK")

        deleteTheTask()
        compose.onAllNodes(hasText(FIRST_TASK)).assertCountEquals(0)
        compose.onNode(hasText(EMPTY)).assertExists()

        // The task launch 2 must find.
        add(KEPT_TASK)
        compose.onNode(hasText(KEPT_TASK)).assertExists()
        compose.onNode(hasText("due $DUE")).assertExists()
    }
}
