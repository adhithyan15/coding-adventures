import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.toAwtImage
import androidx.compose.ui.test.ExperimentalTestApi
import androidx.compose.ui.test.hasClickAction
import androidx.compose.ui.test.hasText
import androidx.compose.ui.test.onLast
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performTextInput
import androidx.compose.ui.test.performTextReplacement
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.runSkikoComposeUiTest
import java.io.File
import javax.imageio.ImageIO
import org.junit.Test

// Renders Journal to PNGs so its layout can be LOOKED AT (J5, #14416).
//
// Same technique as TaskAppScreenshots (#14798): every semantic gate can pass
// while an app is unreadable, so this renders the real generated Compose app
// over the real Rust runtime and writes images for a person to look at. It
// asserts only that rendering succeeds at the declared window size; it is not
// a pixel-diff gate.
//
// Run:
//   MOSAIC_SHOT_DIR=/some/dir gradle -p <generated>/compose test --tests JournalScreenshots
//
// Skipped unless MOSAIC_SHOT_DIR is set.
private val SHOT_VIEWPORT = Size(1100f, 760f) // journal-app's declared [app] window

class JournalScreenshots {
    @OptIn(ExperimentalTestApi::class)
    @Test
    fun renderTheJournalStates() {
        val target = System.getenv("MOSAIC_SHOT_DIR") ?: return
        val dir = File(target).apply { mkdirs() }

        runSkikoComposeUiTest(size = SHOT_VIEWPORT) {
            val host = checkNotNull(MosaicRuntimeHost.load()) {
                "standard Compose binding did not load the Journal Rust runtime"
            }
            val initialResponse = checkNotNull(host.props()) {
                "standard Compose binding returned no Journal startup props"
            }
            setContent { MosaicApp(host, initialResponse) }
            waitForIdle()

            fun shot(name: String) {
                val image = captureToImage().toAwtImage()
                val file = File(dir, "$name.png")
                ImageIO.write(image, "png", file)
                check(image.width == SHOT_VIEWPORT.width.toInt()) {
                    "rendered ${image.width}px wide, expected ${SHOT_VIEWPORT.width.toInt()}"
                }
                println("SHOT ${file.absolutePath} ${image.width}x${image.height}")
            }

            shot("01-empty-journal")

            // Fields by tag, not position: the pane and editor keep gaining
            // fields (search J4a, tags J4d).
            onNodeWithTag("draft-editor-title").performTextInput("First light")
            waitForIdle()
            onNodeWithTag("draft-editor-body").performTextInput("Wrote this on the Compose host. The timeline should show it under today.")
            waitForIdle()
            shot("02-draft")

            onNodeWithText("Save").performClick()
            waitForIdle()
            shot("03-saved-entry")

            onNodeWithText("New entry").performClick()
            waitForIdle()
            shot("04-new-entry-beside-the-timeline")

            // Search (J4a): the first field is now the search box, then the
            // editor's title and body.
            onNodeWithTag("draft-editor-title").performTextInput("A second entry")
            waitForIdle()
            onNodeWithText("Save").performClick()
            waitForIdle()
            onNodeWithTag("search-input").performTextInput("compose")
            waitForIdle()
            shot("05-search-results")
            onNodeWithTag("search-input").performTextInput(" nothing-matches-this")
            waitForIdle()
            shot("06-no-matches")

            // Stars (J4b): star the open entry, then filter to starred ones.
            onNodeWithText("Clear").performClick()
            waitForIdle()
            onNodeWithText("Star").performClick()
            waitForIdle()
            onNodeWithText("Starred only").performClick()
            waitForIdle()
            shot("07-starred-only")

            // Tags (J4d): tag the open entry, then filter by the tag.
            onNodeWithText("Starred only").performClick()
            waitForIdle()
            onNodeWithTag("tags-input").performTextInput("travel, family")
            waitForIdle()
            onNodeWithText("Save").performClick()
            waitForIdle()
            onNodeWithText("#travel (1)").performClick()
            waitForIdle()
            shot("08-tag-filter")

            // An entry's day (J4e): an impossible date is refused in words.
            onNodeWithTag("date-input").performTextReplacement("2026-02-30")
            waitForIdle()
            onNodeWithText("Save").performClick()
            waitForIdle()
            shot("09-draft-error")

            // Journals (J4f): add a Work journal (selected, so the lists
            // start empty), write in it, then try a name already taken.
            onNodeWithTag("journal-name-input").performTextInput("Work")
            waitForIdle()
            onNodeWithText("Add").performClick()
            waitForIdle()
            onNodeWithText("New entry").performClick()
            waitForIdle()
            onNodeWithTag("draft-editor-title").performTextInput("Standup notes")
            waitForIdle()
            onNodeWithText("Save").performClick()
            waitForIdle()
            shot("10-work-journal")
            onNodeWithTag("journal-name-input").performTextInput("personal")
            waitForIdle()
            onNodeWithText("Add").performClick()
            waitForIdle()
            shot("11-journal-error")

            // An entry's journal (J4g): the open entry is in Work; the
            // editor's picker (the pane comes first, so its option is the
            // last "Personal") moves it, and it leaves the Work list.
            onAllNodes(hasText("Personal") and hasClickAction()).onLast().performClick()
            waitForIdle()
            onNodeWithText("Save").performClick()
            waitForIdle()
            shot("12-moved-to-personal")
        }
    }
}
