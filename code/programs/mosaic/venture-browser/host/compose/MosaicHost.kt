import androidx.compose.foundation.Image
import androidx.compose.foundation.focusable
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.ExperimentalComposeUiApi
import androidx.compose.ui.graphics.ImageBitmap
import androidx.compose.ui.graphics.toComposeImageBitmap
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.input.key.Key
import androidx.compose.ui.input.key.KeyEventType
import androidx.compose.ui.input.key.isAltPressed
import androidx.compose.ui.input.key.isCtrlPressed
import androidx.compose.ui.input.key.isMetaPressed
import androidx.compose.ui.input.key.isShiftPressed
import androidx.compose.ui.input.key.key
import androidx.compose.ui.input.key.onPreviewKeyEvent
import androidx.compose.ui.input.key.type
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
import java.util.UUID
import javax.swing.JFileChooser
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
    fun venture_browser_compose_control_key(host: Pointer, key: String, shift: Byte): Byte
    fun venture_browser_compose_control_text(host: Pointer, text: String): Byte
    fun venture_browser_compose_control_copy(host: Pointer): Pointer?
    fun venture_browser_compose_control_cut(host: Pointer): Pointer?
    fun venture_browser_compose_control_paste(host: Pointer, text: String): Byte
    fun venture_browser_compose_caret_tick(host: Pointer, elapsedMilliseconds: Long): Byte
    fun venture_browser_compose_ime_candidate_rect(host: Pointer): Pointer?
    fun venture_browser_compose_file_picker_request(host: Pointer): Pointer?
    fun venture_browser_compose_control_file(
        host: Pointer,
        key: String,
        opaqueId: String,
        displayName: String,
        mediaType: String?,
        bytes: ByteArray,
        length: Long,
        append: Byte,
    ): Byte
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
    var lastAuxiliaryDocument: Map<String, Any?>? = null
        private set
    val renderedFrameCount = AtomicInteger(0)

    private val contentSurface: @Composable () -> Unit = { VentureContentSurface(this) }

    override fun props(): Map<String, Any?> = decorate(decodeResponse(native.venture_browser_compose_apply_props(handle)))

    override fun handleEvent(event: Map<String, Any?>): Map<String, Any?> {
        val decoded = decodeResponse(
                native.venture_browser_compose_handle_event(
                    handle,
                    event["event"]?.toString().orEmpty(),
                    event["value"]?.toString().orEmpty(),
                ),
            )
        consumeEffect(decoded)
        val response = decorate(decoded)
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

    fun controlKey(key: String, shift: Boolean = false): Boolean {
        val changed = native.venture_browser_compose_control_key(
            handle,
            key,
            (if (shift) 1 else 0).toByte(),
        ).toInt() != 0
        if (changed) surfaceChanged()
        return changed
    }

    fun controlText(text: String): Boolean {
        val changed = native.venture_browser_compose_control_text(handle, text).toInt() != 0
        if (changed) surfaceChanged()
        return changed
    }

    fun updateHover(x: Double, y: Double) {
        if (native.venture_browser_compose_update_hover(handle, x, y).toInt() != 0) surfaceChanged()
    }

    fun activateLink(x: Double, y: Double) {
        if (native.venture_browser_compose_activate_link(handle, x, y).toInt() != 0) {
            surfaceChanged()
            presentFilePickerIfRequested()
        }
    }

    private fun presentFilePickerIfRequested() {
        val request = decodeOptional(native.venture_browser_compose_file_picker_request(handle))
            ?: return
        val key = request["key"] as? String ?: return
        val chooser = JFileChooser().apply {
            isMultiSelectionEnabled = request["multiple"] == true
            fileSelectionMode = JFileChooser.FILES_ONLY
        }
        if (chooser.showOpenDialog(null) != JFileChooser.APPROVE_OPTION) return
        val files = if (chooser.isMultiSelectionEnabled) chooser.selectedFiles.toList()
            else listOfNotNull(chooser.selectedFile)
        files.forEachIndexed { index, file ->
            val bytes = file.inputStream().use { it.readNBytes(16 * 1024 * 1024 + 1) }
            native.venture_browser_compose_control_file(
                handle,
                key,
                "compose:${UUID.randomUUID()}",
                file.name,
                null,
                bytes,
                bytes.size.toLong(),
                (if (index == 0) 0 else 1).toByte(),
            )
        }
        surfaceChanged()
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

    @Suppress("UNCHECKED_CAST")
    private fun decodeOptional(value: Pointer?): Map<String, Any?>? {
        value ?: return null
        return try {
            Json.parseToJsonElement(value.getString(0, "UTF-8")).toHostValue()
                as? Map<String, Any?>
        } finally {
            native.venture_browser_compose_string_free(value)
        }
    }

    private fun decorate(response: Map<String, Any?>): Map<String, Any?> {
        val props = propsMap(response).toMutableMap()
        props["content-surface"] = contentSurface
        return response + ("props" to props)
    }

    @Suppress("UNCHECKED_CAST")
    private fun consumeEffect(response: Map<String, Any?>) {
        val effect = response["effect"] as? Map<String, Any?> ?: return
        if (effect["type"] == "open-auxiliary-document") {
            lastAuxiliaryDocument = effect["document"] as? Map<String, Any?>
        }
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
        val focusRequester = remember { FocusRequester() }
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
                .focusRequester(focusRequester)
                .focusable()
                .onPreviewKeyEvent { event ->
                    if (event.type != KeyEventType.KeyDown) return@onPreviewKeyEvent false
                    val command = event.isCtrlPressed || event.isMetaPressed
                    val key = when {
                        command && event.key == Key.A -> "select-all"
                        command && event.key == Key.Z && event.isShiftPressed -> "redo"
                        command && event.key == Key.Z -> "undo"
                        command && event.key == Key.Y -> "redo"
                        event.isAltPressed && event.key == Key.DirectionLeft -> "word-left"
                        event.isAltPressed && event.key == Key.DirectionRight -> "word-right"
                        event.key == Key.Backspace -> "backspace"
                        event.key == Key.Delete -> "delete"
                        event.key == Key.DirectionLeft -> "arrow-left"
                        event.key == Key.DirectionRight -> "arrow-right"
                        event.key == Key.DirectionUp -> "arrow-up"
                        event.key == Key.DirectionDown -> "arrow-down"
                        event.key == Key.MoveHome -> "home"
                        event.key == Key.MoveEnd -> "end"
                        event.key == Key.Enter -> "enter"
                        event.key == Key.Spacebar -> "space"
                        else -> null
                    }
                    key != null && host.controlKey(key, event.isShiftPressed)
                }
                .onPointerEvent(PointerEventType.Scroll) { event ->
                    event.changes.firstOrNull()?.scrollDelta?.y?.let { host.scrollBy(it.toDouble()) }
                }
                .onPointerEvent(PointerEventType.Move) { event ->
                    event.changes.firstOrNull()?.position?.let { host.updateHover(it.x.toDouble(), it.y.toDouble()) }
                }
                .onPointerEvent(PointerEventType.Press) { event ->
                    focusRequester.requestFocus()
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
