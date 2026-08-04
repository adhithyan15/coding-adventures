import androidx.compose.foundation.Image
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.ExperimentalComposeUiApi
import androidx.compose.ui.graphics.ImageBitmap
import androidx.compose.ui.graphics.toComposeImageBitmap
import androidx.compose.ui.input.pointer.PointerEventType
import androidx.compose.ui.input.pointer.onPointerEvent
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import com.sun.jna.Library
import com.sun.jna.Memory
import com.sun.jna.Native
import com.sun.jna.Pointer
import com.sun.jna.ptr.DoubleByReference
import com.sun.jna.ptr.IntByReference
import java.util.concurrent.atomic.AtomicInteger
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonArray
import kotlinx.serialization.json.JsonElement
import kotlinx.serialization.json.JsonNull
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.booleanOrNull
import kotlinx.serialization.json.doubleOrNull
import kotlinx.serialization.json.longOrNull
import org.jetbrains.skia.ColorAlphaType
import org.jetbrains.skia.ColorType
import org.jetbrains.skia.Image as SkiaImage
import org.jetbrains.skia.ImageInfo

private interface VentureNative : Library {
    fun venture_browser_compose_new(startUrl: String, width: Double, height: Double): Pointer?
    fun venture_browser_compose_free(host: Pointer)
    fun venture_browser_compose_apply_props(host: Pointer): Pointer?
    fun venture_browser_compose_handle_event(host: Pointer, name: String, value: String): Pointer?
    fun venture_browser_compose_scroll(host: Pointer, deltaY: Double): Byte
    fun venture_browser_compose_activate_link(host: Pointer, x: Double, y: Double): Byte
    fun venture_browser_compose_update_hover(host: Pointer, x: Double, y: Double): Byte
    fun venture_browser_compose_scroll_metrics(
        host: Pointer,
        offsetY: DoubleByReference,
        viewportHeight: DoubleByReference,
        contentHeight: DoubleByReference,
        maxOffsetY: DoubleByReference,
    ): Byte
    fun venture_browser_compose_resize(host: Pointer, width: Double, height: Double): Byte
    fun venture_browser_compose_render_rgba(
        host: Pointer,
        output: Pointer?,
        capacity: Long,
        width: IntByReference,
        height: IntByReference,
    ): Long
    fun venture_browser_compose_string_free(value: Pointer)
}

private data class OpenedVentureHost(val native: VentureNative, val handle: Pointer)

private data class RgbaFrame(val width: Int, val height: Int, val pixels: ByteArray) {
    fun toSkiaImage(): SkiaImage = SkiaImage.makeRaster(
        ImageInfo(width, height, ColorType.RGBA_8888, ColorAlphaType.UNPREMUL),
        pixels,
        width * 4,
    )
}

/** Package-owned Compose bridge over Venture's shared Rust browser session. */
class MosaicHost private constructor(
    private val native: VentureNative,
    private val handle: Pointer,
) : MosaicComposeHost, AutoCloseable {
    private constructor(opened: OpenedVentureHost) : this(opened.native, opened.handle)

    /** Reflection entry point used by the emitted Compose project shell. */
    constructor() : this(openDefault())

    private val surfaceRevision = mutableIntStateOf(0)
    private var propsChangedHandler: (() -> Unit)? = null
    private var closed = false
    val renderedFrameCount = AtomicInteger(0)

    private val contentSurface: @Composable () -> Unit = { VentureContentSurface(this) }

    override fun props(): Map<String, Any?> = decorate(decodeResponse(native.venture_browser_compose_apply_props(handle)))

    override fun handleEvent(event: Map<String, Any?>): Map<String, Any?> {
        val response = decorate(
            decodeResponse(
                native.venture_browser_compose_handle_event(
                    handle,
                    event["event"]?.toString().orEmpty(),
                    event["value"]?.toString().orEmpty(),
                ),
            ),
        )
        surfaceChanged()
        return response
    }

    override fun setPropsChangedHandler(handler: (() -> Unit)?) {
        propsChangedHandler = handler
    }

    val scrollMetrics: Map<String, Double>?
        get() {
            val offset = DoubleByReference()
            val viewport = DoubleByReference()
            val content = DoubleByReference()
            val maximum = DoubleByReference()
            if (
                native.venture_browser_compose_scroll_metrics(
                    handle,
                    offset,
                    viewport,
                    content,
                    maximum,
                ).toInt() == 0
            ) {
                return null
            }
            return mapOf(
                "offset" to offset.value,
                "viewport" to viewport.value,
                "content" to content.value,
                "max" to maximum.value,
            )
        }

    val statusText: String
        get() = propsMap(decodeResponse(native.venture_browser_compose_apply_props(handle)))["status-text"]
            ?.toString()
            .orEmpty()

    fun scrollBy(deltaY: Double) {
        if (native.venture_browser_compose_scroll(handle, deltaY).toInt() != 0) surfaceChanged()
    }

    fun updateHover(x: Double, y: Double) {
        if (native.venture_browser_compose_update_hover(handle, x, y).toInt() != 0) surfaceChanged()
    }

    fun activateLink(x: Double, y: Double) {
        if (native.venture_browser_compose_activate_link(handle, x, y).toInt() != 0) surfaceChanged()
    }

    fun resize(width: Double, height: Double) {
        if (native.venture_browser_compose_resize(handle, width, height).toInt() != 0) surfaceChanged()
    }

    private fun renderFrame(): RgbaFrame {
        val width = IntByReference()
        val height = IntByReference()
        val length = native.venture_browser_compose_render_rgba(handle, null, 0, width, height)
        check(length > 0 && length <= Int.MAX_VALUE) {
            "shared Venture Cairo renderer returned an invalid frame length: $length"
        }
        check(width.value > 0 && height.value > 0) {
            "shared Venture Cairo renderer returned an empty frame"
        }
        val output = Memory(length)
        val written = native.venture_browser_compose_render_rgba(handle, output, length, width, height)
        check(written == length) { "shared Venture Cairo render changed size during copy" }
        return RgbaFrame(width.value, height.value, output.getByteArray(0, length.toInt()))
    }

    @Suppress("UNCHECKED_CAST")
    private fun decodeResponse(value: Pointer?): Map<String, Any?> {
        check(value != null) { "shared Venture host returned a null response" }
        return try {
            val element = Json.parseToJsonElement(value.getString(0, "UTF-8"))
            element.toHostValue() as? Map<String, Any?>
                ?: error("shared Venture host returned a non-object response")
        } finally {
            native.venture_browser_compose_string_free(value)
        }
    }

    private fun decorate(response: Map<String, Any?>): Map<String, Any?> {
        val props = propsMap(response).toMutableMap()
        props["content-surface"] = contentSurface
        return response + ("props" to props)
    }

    private fun surfaceChanged() {
        surfaceRevision.intValue += 1
        propsChangedHandler?.invoke()
    }

    override fun close() {
        if (closed) return
        closed = true
        propsChangedHandler = null
        native.venture_browser_compose_free(handle)
    }

    companion object {
        const val VIEWPORT_WIDTH = 1024
        const val VIEWPORT_HEIGHT = 640

        fun open(libraryPath: String, startUrl: String): MosaicHost = MosaicHost(openNative(libraryPath, startUrl))

        private fun openDefault(): OpenedVentureHost = openNative(
            System.getenv("VENTURE_BROWSER_COMPOSE_LIBRARY") ?: System.mapLibraryName("venture_browser_compose"),
            System.getenv("VENTURE_BROWSER_START_URL") ?: "http://info.cern.ch/",
        )

        private fun openNative(libraryPath: String, startUrl: String): OpenedVentureHost {
            val native = Native.load(
                libraryPath,
                VentureNative::class.java,
                mapOf(Library.OPTION_STRING_ENCODING to "UTF-8"),
            )
            val handle = native.venture_browser_compose_new(
                startUrl,
                VIEWPORT_WIDTH.toDouble(),
                VIEWPORT_HEIGHT.toDouble(),
            ) ?: error("shared Venture browser session failed to load $startUrl")
            return OpenedVentureHost(native, handle)
        }
    }

    @OptIn(ExperimentalComposeUiApi::class)
    @Composable
    private fun VentureContentSurface(host: MosaicHost) {
        val revision = host.surfaceRevision.intValue
        val frame = remember(revision) {
            val skiaImage = host.renderFrame().toSkiaImage()
            RenderedFrame(skiaImage, skiaImage.toComposeImageBitmap())
        }
        DisposableEffect(frame) {
            host.renderedFrameCount.incrementAndGet()
            onDispose { frame.image.close() }
        }
        Image(
            bitmap = frame.bitmap,
            contentDescription = "Venture live page",
            modifier = Modifier
                .size(VIEWPORT_WIDTH.dp, VIEWPORT_HEIGHT.dp)
                .testTag("venture-content-surface")
                .onPointerEvent(PointerEventType.Scroll) { event ->
                    event.changes.firstOrNull()?.scrollDelta?.y?.let { host.scrollBy(it.toDouble()) }
                }
                .onPointerEvent(PointerEventType.Move) { event ->
                    event.changes.firstOrNull()?.position?.let { host.updateHover(it.x.toDouble(), it.y.toDouble()) }
                }
                .onPointerEvent(PointerEventType.Press) { event ->
                    event.changes.firstOrNull()?.position?.let { host.activateLink(it.x.toDouble(), it.y.toDouble()) }
                },
        )
    }
}

private data class RenderedFrame(val image: SkiaImage, val bitmap: ImageBitmap)

private fun propsMap(response: Map<String, Any?>): Map<String, Any?> =
    (response["props"] as? Map<*, *>)
        ?.entries
        ?.mapNotNull { (key, value) -> (key as? String)?.let { it to value } }
        ?.toMap()
        .orEmpty()

private fun JsonElement.toHostValue(): Any? = when (this) {
    JsonNull -> null
    is JsonObject -> entries.associate { (key, value) -> key to value.toHostValue() }
    is JsonArray -> map { it.toHostValue() }
    is JsonPrimitive -> booleanOrNull ?: longOrNull ?: doubleOrNull ?: content
}
