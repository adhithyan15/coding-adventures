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
import androidx.compose.material.Text
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.semantics.ScrollAxisRange
import androidx.compose.ui.semantics.SemanticsNode
import androidx.compose.ui.test.ComposeUiTest
import androidx.compose.ui.test.onRoot
import kotlin.test.assertEquals
import kotlin.test.assertFails
import kotlin.test.assertTrue
import org.junit.Test
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicInteger

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
// So state the window instead of inheriting it. This value is the TaskApp
// manifest's `[app]` initial window size, and the package contract test pins
// the two declarations together. Every `assertIsDisplayed()` below keeps its
// full strength -- this is the real desktop window the generated app opens,
// and content still has to be on screen in it without scrolling.
//
// Two things this deliberately does NOT paper over, both filed separately:
//   * `performScrollTo()` hangs against the emitted scroll container, so
//     scrolling is not currently an option for reaching content below a fold.
private val ACCEPTANCE_VIEWPORT = Size(1280f, 900f)

private class StartupRecoveryHost : MosaicComposeHost {
    val propsCalls = AtomicInteger(0)

    override fun props(): Map<String, Any?> {
        propsCalls.incrementAndGet()
        return mapOf("props" to emptyMap<String, Any?>())
    }

    override fun handleEvent(event: Map<String, Any?>): Map<String, Any?> =
        mapOf("props" to emptyMap<String, Any?>())
}

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

/**
 * Asserts nothing in the topbar has been squeezed to nothing (#14847, #14815).
 *
 * The vertical companion to [assertFitsViewport], and it exists for the same
 * reason: a HORIZONTAL overflow has no scroll range to read, so the only
 * evidence is a child that measured zero.
 *
 * The `On track` chip measured `0 x 168` at 1280 for as long as the topbar
 * carried the view switcher, and every existence and semantics assertion in
 * this file passed the whole time -- a node squeezed to zero width is still
 * present, still named, and still "displayed". That is exactly why this
 * asserts a WIDTH rather than presence.
 *
 * A width, not the sum of the row's children: the sum is not observable from
 * the semantics tree, and the endpoint of a right-aligned row tracks the
 * VIEWPORT rather than its content -- reading that endpoint as a content
 * demand is how #14847 first got its number wrong by 2.4x.
 */
@OptIn(ExperimentalTestApi::class)
private fun ComposeUiTest.assertTopbarIsNotStarved(stage: String) {
    var starved: String? = null
    fun walk(node: SemanticsNode) {
        val text = node.config
            .firstOrNull { it.key.name == "Text" }
            ?.value
            ?.let { (it as? List<*>)?.joinToString("") { part -> part.toString() } }
        // Only the topbar band. A zero-width node further down the page is a
        // different question and should fail in its own test, saying so.
        if (text != null && text.isNotEmpty() && node.positionInRoot.y < 120f && node.size.width == 0) {
            starved = text
        }
        node.children.forEach(::walk)
    }
    walk(onRoot().fetchSemanticsNode())
    assertEquals(
        null,
        starved,
        "a topbar element measured ZERO width at stage '$stage' in the " +
            "${ACCEPTANCE_VIEWPORT.width.toInt()}px acceptance viewport. It is " +
            "present and named, so every other assertion here still passes -- " +
            "the row is over-subscribed and something has to leave it.",
    )
}

/**
 * #15263: the status sentence and recovery path must occupy distinct lines.
 * Presence assertions passed while the old Row painted both glyph runs over
 * each other, so compare the rendered bounds directly.
 */
@OptIn(ExperimentalTestApi::class)
private fun ComposeUiTest.assertStorageSummaryIsLegible() {
    val status = onNodeWithText("Saved locally on this device")
        .fetchSemanticsNode()
        .boundsInRoot
    val location = onNodeWithText("Local only", substring = true)
        .fetchSemanticsNode()
        .boundsInRoot
    assertTrue(status.width > 0f && location.width > 0f, "storage summary text was starved")
    assertTrue(
        status.bottom <= location.top,
        "storage summary lines overlap: status=$status location=$location",
    )
}

class TaskAppUiTest {
    @OptIn(ExperimentalTestApi::class)
    @Test
    fun generatedStartupFailureIsVisibleAndRetryRerunsInitialization() = runSkikoComposeUiTest {
        val releaseFirstAttempt = CountDownLatch(1)
        val attempts = AtomicInteger(0)
        val recoveredHost = StartupRecoveryHost()

        setContent {
            MosaicStartup(
                loadHost = {
                    if (attempts.incrementAndGet() == 1) {
                        check(releaseFirstAttempt.await(5, TimeUnit.SECONDS)) {
                            "startup acceptance did not release the first attempt"
                        }
                        error("fixture initialization failed")
                    }
                    recoveredHost
                },
                content = { _, _ -> Text("Recovered TaskApp") },
            )
        }

        onNodeWithTag("mosaic-startup-loading").assertIsDisplayed()
        onNodeWithText("Starting TaskApp…").assertIsDisplayed()

        releaseFirstAttempt.countDown()
        waitUntil(timeoutMillis = 5_000) {
            onAllNodesWithText("TaskApp could not start").fetchSemanticsNodes().isNotEmpty()
        }
        onNodeWithTag("mosaic-startup-failure").assertIsDisplayed()
        onNodeWithText("TaskApp could not start").assertIsDisplayed()
        onNodeWithText("Your saved tasks have not been changed. Retrying is safe.")
            .assertIsDisplayed()
        onNodeWithText("fixture initialization failed").assertIsDisplayed()

        onNodeWithText("Try again").performClick()
        // Wait for what the user sees, not for the second attempt to START:
        // `attempts` reaches 2 as soon as loadHost is called, before the
        // recovered host is returned and composed, so asserting the content
        // right after that wait raced the recomposition (a CI flake).
        waitUntil(timeoutMillis = 5_000) {
            onAllNodesWithText("Recovered TaskApp").fetchSemanticsNodes().isNotEmpty()
        }
        onNodeWithText("Recovered TaskApp").assertIsDisplayed()
        assertEquals(2, attempts.get())
        assertEquals(1, recoveredHost.propsCalls.get())
    }

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
        val initialResponse = checkNotNull(host.props()) {
            "standard Compose binding returned no TaskApp startup props"
        }
        compose.setContent { MosaicApp(host, initialResponse) }
        compose.waitForIdle()
        compose.assertStorageSummaryIsLegible()

        if (restoredOnLaunch) {
            compose.onNodeWithText(UI_PERSISTED_TASK_NAME).assertIsDisplayed()
            compose.onNodeWithText("due $UI_DUE").assertIsDisplayed()
            assertFitsViewport("restored on launch")
            assertTopbarIsNotStarved("restored on launch")
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
            assertTopbarIsNotStarved("after first add")
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
            assertTopbarIsNotStarved("after completion")
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
