import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.toAwtImage
import androidx.compose.ui.test.ExperimentalTestApi
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performTextInput
import androidx.compose.ui.test.runSkikoComposeUiTest
import java.io.File
import javax.imageio.ImageIO
import org.junit.Test

// Renders TaskApp to PNGs so a layout change can be LOOKED AT.
//
// Every other gate in this directory asserts SEMANTICS: that a node exists,
// carries the right accessible name, and is marked displayed. All of them can
// pass while the app is unreadable -- and they did. The first render of this
// app (#14798) showed run-together text, a chip collapsed to a one-character
// vertical column, a clipped progress label, duplicated segment buttons, and
// two thirds of the window left blank white. Green CI meant "the semantics
// tree is correct", which is not the same claim as "this looks like a product".
//
// This is not a pixel-diff gate. It asserts only that rendering SUCCEEDS and
// produces a non-trivial image; the images themselves are for a person (or a
// reviewer on a PR) to look at. A strict pixel baseline would need a pinned
// font stack and renderer to avoid failing on every platform difference, and
// that is a separate decision -- see #14798.
//
// Run:
//   MOSAIC_SHOT_DIR=/some/dir gradle -p <generated>/compose test --tests TaskAppScreenshots
//
// Skipped unless MOSAIC_SHOT_DIR is set, so it never slows the normal gates.
// The window these are rendered at.
//
// #14771 (in flight) declares the same 1280 x 900 as the acceptance test's
// viewport. This does not reference that constant, because this harness is
// useful now and #14771 sits behind a blocked PR. The two should be unified
// into one constant when it lands -- until then they are two numbers that have
// to be kept equal by hand, which is worth exactly one line of follow-up and
// is tracked in #14798.
private val SHOT_VIEWPORT = Size(1280f, 900f)

class TaskAppScreenshots {
    @OptIn(ExperimentalTestApi::class)
    @Test
    fun renderTheProductStates() {
        val target = System.getenv("MOSAIC_SHOT_DIR") ?: return
        val dir = File(target).apply { mkdirs() }

        runSkikoComposeUiTest(size = SHOT_VIEWPORT) {
            val host = checkNotNull(MosaicRuntimeHost.load()) {
                "standard Compose binding did not load the TaskApp Rust runtime"
            }
            setContent { MosaicApp(host) }
            waitForIdle()

            fun shot(name: String) {
                val image = captureToImage().toAwtImage()
                val file = File(dir, "$name.png")
                ImageIO.write(image, "png", file)
                // A window that rendered nothing still writes a valid PNG, so
                // check the surface is the size we asked for rather than
                // trusting that the write succeeded.
                check(image.width == SHOT_VIEWPORT.width.toInt()) {
                    "rendered ${image.width}px wide, expected ${SHOT_VIEWPORT.width.toInt()}"
                }
                println("SHOT ${file.absolutePath} ${image.width}x${image.height}")
            }

            shot("01-empty-inbox")

            onNodeWithTag("name-input").performTextInput("Draft release notes")
            waitForIdle()
            onNodeWithTag("due-input").performTextInput("2026-01-09")
            waitForIdle()
            onNodeWithTag("add-btn").performClick()
            waitForIdle()
            shot("02-one-task")

            onNodeWithTag("toggle").performClick()
            waitForIdle()
            shot("03-task-complete")
        }
    }
}
