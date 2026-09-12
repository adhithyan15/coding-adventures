import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.toAwtImage
import androidx.compose.ui.test.ExperimentalTestApi
import androidx.compose.ui.test.captureToImage
import androidx.compose.ui.test.runSkikoComposeUiTest
import java.io.File
import javax.imageio.ImageIO
import org.junit.Test

// Renders VisiCalc to PNGs so a layout change can be LOOKED AT.
//
// The third product to get one, after Trestle (#14799) and Engram (#14828).
// VisiCalc is the one where looking has paid off most directly: #14829 was
// found this way, and the grid is the hardest thing in any of the five
// products to judge from a semantics tree alone, because "every cell exists
// and is named" is true of a grid whose headers sit nowhere near their
// columns.
//
// Same narrow claim as the other two: rendering SUCCEEDS and produces a
// surface of the size asked for. NOT a pixel-diff gate -- a strict baseline
// needs a pinned font stack and renderer, which is a separate decision
// (#14798).
//
// Run:
//   scripts/render-compose.sh /tmp/visicalc-shots
//
// Skipped unless MOSAIC_SHOT_DIR is set, so it never slows the normal gates.

// Wide enough that the sheet is the thing under observation rather than the
// window. At 1280 the grid lays out columns A-P and collapses Q-Z to zero
// width (#14842) -- worth knowing that this viewport is inside that regime.
private val SHOT_VIEWPORT = Size(1280f, 900f)

class VisiCalcScreenshots {
    @OptIn(ExperimentalTestApi::class)
    @Test
    fun renderTheSheet() {
        val target = System.getenv("MOSAIC_SHOT_DIR") ?: return
        val dir = File(target).apply { mkdirs() }

        runSkikoComposeUiTest(size = SHOT_VIEWPORT) {
            val host = checkNotNull(MosaicRuntimeHost.load()) {
                "standard Compose binding did not load the VisiCalc Rust runtime"
            }
            setContent { MosaicApp(host) }
            waitForIdle()

            val image = captureToImage().toAwtImage()
            val file = File(dir, "01-sheet.png")
            ImageIO.write(image, "png", file)
            // A window that rendered nothing still writes a valid PNG, so
            // check the surface is the size we asked for rather than
            // trusting that the write succeeded.
            check(image.width == SHOT_VIEWPORT.width.toInt()) {
                "rendered ${image.width}px wide, expected ${SHOT_VIEWPORT.width.toInt()}"
            }
            println("SHOT ${file.absolutePath} ${image.width}x${image.height}")
        }
    }
}
