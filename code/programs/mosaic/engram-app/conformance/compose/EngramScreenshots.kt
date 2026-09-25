import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.toAwtImage
import androidx.compose.ui.test.ExperimentalTestApi
import androidx.compose.ui.test.hasTestTag
import androidx.compose.ui.test.hasText
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.captureToImage
import androidx.compose.ui.test.runSkikoComposeUiTest
import java.io.File
import javax.imageio.ImageIO
import org.junit.Test

// Renders Engram to PNGs so a layout change can be LOOKED AT.
//
// Engram is gated on swiftui and qt in CI and has no Compose lane at all, so
// until now the only way to know what it looked like on Compose was to
// generate the project by hand. That is how UI60's defect was found (#14828)
// and it is not a process that runs again on its own -- the deck-stat chips
// have been drawing the count on top of the label for as long as they have
// existed, and every semantics gate stayed green throughout, because a node
// that overlaps another node is still present, still named and still
// "displayed".
//
// This is the same harness Trestle got in #14799, and it makes the same
// narrow claim: rendering SUCCEEDS and produces a surface of the size asked
// for. It is NOT a pixel-diff gate. A strict baseline needs a pinned font
// stack and renderer, which is a separate decision (#14798).
//
// Run:
//   MOSAIC_SHOT_DIR=/some/dir gradle -p <generated>/compose test --tests EngramScreenshots
//
// Skipped unless MOSAIC_SHOT_DIR is set, so it never slows the normal gates.

// Engram's screens are authored with `max-width` caps between 760px and
// 1100px, so the window has to be wider than the widest of them for the caps
// to be the thing under observation rather than the window. 1280 also matches
// the viewport Trestle renders at, which keeps the two products comparable
// when they are looked at side by side.
//
// Compose drops every one of those caps today (#14833), which is exactly the
// kind of fact this harness exists to make visible.
private val SHOT_VIEWPORT = Size(1280f, 900f)

class EngramScreenshots {
    @OptIn(ExperimentalTestApi::class)
    @Test
    fun renderTheProductScreens() {
        val target = System.getenv("MOSAIC_SHOT_DIR") ?: return
        val dir = File(target).apply { mkdirs() }

        runSkikoComposeUiTest(size = SHOT_VIEWPORT) {
            val host = checkNotNull(MosaicRuntimeHost.load()) {
                "standard Compose binding did not load the Engram Rust runtime"
            }
            // `MosaicApp` takes the start response since the standard binding
            // began rendering from it; this harness still passed only the host
            // and no longer compiled (it is not run in CI).
            val initialResponse = checkNotNull(host.props()) {
                "standard Compose binding returned no Engram startup props"
            }
            setContent { MosaicApp(host, initialResponse) }
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

            shot("01-decks")

            // Every screen, through the nav's own control. "Decks" is also the
            // deck list's label, so each option is matched by the toolkit
            // SegmentedControl's `segmented-option` tag as well as its label
            // (the nav is the first SegmentedControl mount, so its parts keep
            // the unsuffixed names).
            listOf("Study", "Browse", "Add", "Stats", "Options").forEachIndexed { i, label ->
                onNode(hasTestTag("segmented-option") and hasText(label)).performClick()
                waitForIdle()
                shot("0${i + 2}-${label.lowercase()}")
            }
        }
    }
}
