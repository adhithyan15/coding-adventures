import androidx.compose.material.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.test.assertIsEnabled
import androidx.compose.ui.test.assertIsNotEnabled
import androidx.compose.ui.test.assertTextEquals
import androidx.compose.ui.test.click
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performImeAction
import androidx.compose.ui.test.performMouseInput
import androidx.compose.ui.test.performTextReplacement
import kotlin.test.assertEquals
import org.junit.Rule
import org.junit.Test

private class RecordingMosaicHost(
    private val navigationDisabled: Boolean,
) : MosaicComposeHost {
    val events = mutableListOf<Map<String, Any?>>()
    private val contentSurface: @Composable () -> Unit = { Text("Compose host surface") }

    override fun props(): Map<String, Any?> = response("Ready")

    override fun handleEvent(event: Map<String, Any?>): Map<String, Any?>? {
        events += event.toMap()
        return if (event["event"] == "onNavigate") {
            response("Navigated through MosaicHost")
        } else {
            null
        }
    }

    private fun response(status: String): Map<String, Any?> = mapOf(
        "props" to mapOf(
            "address" to "http://venture.test/start",
            "page-title" to "Venture Compose Acceptance",
            "status-text" to status,
            "back-disabled" to navigationDisabled,
            "forward-disabled" to navigationDisabled,
            "navigation-disabled" to navigationDisabled,
            "content-surface" to contentSurface,
        ),
    )
}

class VentureChromeInteractionTest {
    @get:Rule
    val rule = createComposeRule()

    @Test
    fun disabledNativeControlsSuppressMosaicDispatch() {
        val host = RecordingMosaicHost(navigationDisabled = true)
        rule.setContent { MosaicApp(host) }
        rule.waitForIdle()

        rule.onNodeWithText("Venture Compose Acceptance").assertExists()
        rule.onNodeWithText("Compose host surface").assertExists()
        for (tag in listOf("back-button", "forward-button", "reload-button", "go-button")) {
            rule.onNodeWithTag(tag).assertIsNotEnabled().performMouseInput { click() }
        }
        rule.onNodeWithTag("address-input").assertIsNotEnabled()
        rule.waitForIdle()
        assertEquals(emptyList(), host.events)
    }

    @Test
    fun addressReturnAndGoCrossTheMosaicHostSeam() {
        val host = RecordingMosaicHost(navigationDisabled = false)
        rule.setContent { MosaicApp(host) }
        rule.waitForIdle()

        val input = rule.onNodeWithTag("address-input").assertIsEnabled()
        input.performTextReplacement("http://venture.test/next")
        rule.waitForIdle()
        assertEquals(
            mapOf(
                "event" to "onAddressChange",
                "value" to "http://venture.test/next",
            ),
            host.events.last(),
        )

        input.performImeAction()
        rule.waitForIdle()
        assertEquals("onNavigate", host.events.last()["event"])
        rule.onNodeWithText("Navigated through MosaicHost").assertExists()

        rule.onNodeWithTag("go-button").assertIsEnabled().performClick()
        rule.waitForIdle()
        assertEquals(2, host.events.count { it["event"] == "onNavigate" })
        rule.onNodeWithTag("address-input").assertTextEquals("http://venture.test/start")
    }
}
