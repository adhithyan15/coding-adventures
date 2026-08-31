import androidx.compose.ui.test.assertCountEquals
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.assertTextContains
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onAllNodesWithText
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performTextInput
import kotlin.test.assertEquals
import kotlin.test.assertFails
import org.junit.Rule
import org.junit.Test

private const val UI_TASK_NAME = "Native acceptance task"
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

        compose.onNodeWithTag("toggle").performClick()
        compose.waitForIdle()
        compose.onNodeWithTag("toggle").assertTextContains("✓")
        // The progress node carries the updated Rust value, but issue #13565
        // tracks making it visible in the default desktop viewport.
        compose.onAllNodesWithText("100%").assertCountEquals(1)
        compose.onNodeWithTag("toggle").performClick()
        compose.waitForIdle()
        compose.onNodeWithTag("toggle").assertTextContains("○")

        compose.onNodeWithTag("del-btn").performClick()
        compose.waitForIdle()
        compose.onAllNodesWithText(UI_TASK_NAME).assertCountEquals(0)

        compose.onNodeWithTag("name-input").performTextInput(UI_PERSISTED_TASK_NAME)
        compose.waitForIdle()
        compose.onNodeWithTag("due-input").performTextInput(UI_DUE)
        compose.waitForIdle()
        compose.onNodeWithTag("add-btn").performClick()
        compose.waitForIdle()
        compose.onNodeWithText(UI_PERSISTED_TASK_NAME).assertIsDisplayed()
        compose.onNodeWithText(UI_SUMMARY).assertIsDisplayed()
    }
}
