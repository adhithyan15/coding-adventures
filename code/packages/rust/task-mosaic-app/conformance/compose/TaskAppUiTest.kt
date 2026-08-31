import androidx.compose.ui.test.assertCountEquals
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.assertTextContains
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onAllNodesWithText
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performTextInput
import androidx.compose.ui.test.performTextReplacement
import kotlin.test.assertEquals
import kotlin.test.assertFails
import org.junit.Rule
import org.junit.Test

private const val UI_TASK_NAME = "Native acceptance task"
private const val UI_EDITED_TASK_NAME = "Edited native task"
private const val UI_EDITED_DUE = "2026-01-12"
private const val UI_PERSISTED_TASK_NAME = "Persisted native task"
private const val UI_DUE = "2026-01-09"
private const val UI_SUMMARY = "1 task(s) · 0 done · projected finish 2026-01-05"

class TaskAppUiTest {
    @get:Rule
    val compose = createComposeRule()

    @Test
    fun generatedControlsDriveRustSchedulingLifecycle() {
        val restoredOnLaunch = System.getenv("MOSAIC_EXPECT_RESTORED") == "1"
        val host = checkNotNull(MosaicRuntimeHost.load()) {
            "standard Compose binding did not load the TaskApp Rust runtime"
        }
        compose.setContent { MosaicApp(host) }
        compose.waitForIdle()

        if (restoredOnLaunch) {
            compose.onNodeWithText(UI_PERSISTED_TASK_NAME).assertIsDisplayed()
            compose.onNodeWithText("due $UI_DUE").assertIsDisplayed()
            compose.onNodeWithText(UI_SUMMARY).assertIsDisplayed()
            compose.onNodeWithTag("del-btn").performClick()
            compose.waitForIdle()
            compose.onAllNodesWithText(UI_PERSISTED_TASK_NAME).assertCountEquals(0)
            return
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

        compose.onNodeWithTag("toggle").performClick()
        compose.waitForIdle()
        compose.onNodeWithTag("toggle").assertTextContains("✓")
        // The Rust-owned completion value must be visible, not merely present
        // in the semantics tree beyond the measured desktop viewport.
        compose.onNodeWithText("100%").assertIsDisplayed()
        compose.onNodeWithTag("toggle").performClick()
        compose.waitForIdle()
        compose.onNodeWithTag("toggle").assertTextContains("○")

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
