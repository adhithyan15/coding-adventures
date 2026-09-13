import androidx.compose.runtime.mutableStateOf
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.toAwtImage
import androidx.compose.ui.semantics.SemanticsActions
import androidx.compose.ui.test.*
import androidx.compose.ui.text.TextLayoutResult
import java.io.File
import javax.imageio.ImageIO
import org.junit.Test

class TypographyTest {
    @OptIn(ExperimentalTestApi::class)
    @Test fun scalesAndFallsBack() = runSkikoComposeUiTest(size = Size(800f, 700f)) {
        val size = mutableStateOf(13.0)
        setContent { ScaledText(textSize = size.value, dispatch = {}) }
        for ((input, expected) in listOf(13.0 to 13f, 19.5 to 19.5f, 26.0 to 26f,
            Double.NaN to 13f, Double.POSITIVE_INFINITY to 13f, 0.0 to 13f, -1.0 to 13f, 1e100 to 13f)) {
            runOnIdle { size.value = input }
            waitForIdle()
            for (label in listOf("Typography", "Action", "A number or formula", "Inherited", "Editor")) {
                val results = mutableListOf<TextLayoutResult>()
                onNodeWithText(label, useUnmergedTree = true).performSemanticsAction(SemanticsActions.GetTextLayoutResult) { it(results) }
                check(results.single().layoutInput.style.fontSize.value == expected) {
                    "$label at $input: ${results.single().layoutInput.style.fontSize} expected $expected"
                }
            }
            println("TYPOGRAPHY $input -> $expected on all five text surfaces")
            if (input == 26.0) ImageIO.write(captureToImage().toAwtImage(), "png", File("typography-200.png"))
        }
    }
}
