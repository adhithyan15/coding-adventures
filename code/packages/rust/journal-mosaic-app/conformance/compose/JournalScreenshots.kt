import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.toAwtImage
import androidx.compose.ui.test.ExperimentalTestApi
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performTextInput
import androidx.compose.ui.test.hasSetTextAction
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

            // The editor's two text fields: the title, then the body.
            val fields = onAllNodes(hasSetTextAction())
            fields[0].performTextInput("First light")
            waitForIdle()
            fields[1].performTextInput("Wrote this on the Compose host. The timeline should show it under today.")
            waitForIdle()
            shot("02-draft")

            onNodeWithText("Save").performClick()
            waitForIdle()
            shot("03-saved-entry")

            onNodeWithText("New entry").performClick()
            waitForIdle()
            shot("04-new-entry-beside-the-timeline")
        }
    }
}
