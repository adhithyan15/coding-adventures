import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.toAwtImage
import androidx.compose.ui.test.ExperimentalTestApi
import androidx.compose.ui.test.captureToImage
import androidx.compose.ui.semantics.SemanticsNode
import androidx.compose.ui.semantics.SemanticsProperties
import androidx.compose.ui.semantics.getOrNull
import androidx.compose.ui.test.onRoot
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

    // #14842 — the sheet must be able to REACH the columns past the
    // viewport. Not skipped when MOSAIC_SHOT_DIR is unset: this one is a
    // gate, not a screenshot.
    //
    // The original issue reported columns Q-Z as "zero width", measured
    // with `boundsInRoot`. That was a measurement artifact:
    // `boundsInRoot` is Rect.Zero for a node clipped outside the
    // viewport, so a correctly laid-out off-screen column and a
    // collapsed one are indistinguishable through it. The columns were
    // always laid out at the right size and the right 80px pitch --
    // `size` and `positionInRoot` say so. What was missing was any way
    // to scroll to them, because UI61's axis did not exist yet and
    // `HostScroll` defaulted to vertical.
    //
    // So this asserts the two things that were actually wrong, and would
    // both have passed on the old "does the column exist" reading:
    //   1. the viewport offers a HORIZONTAL scroll range at all
    //   2. the tail columns have real layout size beyond the viewport
    @OptIn(ExperimentalTestApi::class)
    @Test
    fun theSheetCanScrollToTheTailColumns() {
        runSkikoComposeUiTest(size = SHOT_VIEWPORT) {
            val host = checkNotNull(MosaicRuntimeHost.load()) {
                "standard Compose binding did not load the VisiCalc Rust runtime"
            }
            setContent { MosaicApp(host) }
            waitForIdle()

            var horizontalMax = 0f
            var horizontalRanges = 0
            val offscreen = mutableListOf<String>()
            fun walk(n: SemanticsNode) {
                n.config.getOrNull(SemanticsProperties.HorizontalScrollAxisRange)?.let {
                    horizontalRanges++
                    horizontalMax = maxOf(horizontalMax, it.maxValue())
                }
                val label = n.config.getOrNull(SemanticsProperties.Text)
                    ?.joinToString("") { it.text }
                if (label != null && label.length == 1 && label[0] in 'A'..'Z') {
                    // Laid out past the right edge of the viewport.
                    if (n.positionInRoot.x >= SHOT_VIEWPORT.width) {
                        check(n.size.width > 0 && n.size.height > 0) {
                            "column $label is off-screen AND has no layout size " +
                                "(${n.size.width}x${n.size.height}) — that would be the " +
                                "collapse #14842 originally claimed"
                        }
                        offscreen += label
                    }
                }
                n.children.forEach { walk(it) }
            }
            walk(onRoot().fetchSemanticsNode())

            check(horizontalRanges > 0 && horizontalMax > 0f) {
                "the sheet offers no horizontal scroll range (max=$horizontalMax), so the " +
                    "columns past the viewport cannot be reached — this is what #14842 was"
            }
            // If nothing is off-screen the assertion above proves nothing,
            // so require that this viewport is genuinely in the overflow
            // regime the issue describes.
            check(offscreen.isNotEmpty()) {
                "expected some columns beyond ${SHOT_VIEWPORT.width}px at this viewport; " +
                    "if the sheet got narrower this gate is no longer measuring anything"
            }
            println("SCROLLGATE horizontalMax=$horizontalMax offscreen=${offscreen.joinToString("")}")
        }
    }
}
