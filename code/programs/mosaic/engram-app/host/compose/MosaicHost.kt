import com.sun.jna.Library
import com.sun.jna.Native
import com.sun.jna.Pointer
import java.io.File
import org.json.JSONArray
import org.json.JSONObject

class MosaicHost {
    private val capi: EngramCapi? = loadCapi()
    private val session: Pointer? = capi?.eg_session_new_demo()

    init {
        Runtime.getRuntime().addShutdownHook(Thread {
            val api = capi
            val handle = session
            if (api != null && handle != null) {
                api.eg_session_free(handle)
            }
        })
    }

    fun props(): Map<String, Any?> {
        val api = capi ?: return emptyMap()
        val handle = session ?: return emptyMap()
        return hostResponseFromJson(takeCString(api.eg_engram_app_props(handle, deckId(), nowMs()), api))
    }

    fun handleEvent(event: Map<String, Any?>): Map<String, Any?> {
        val api = capi ?: return emptyMap()
        val handle = session ?: return emptyMap()
        val eventJson = JSONObject(event).toString()
        return hostResponseFromJson(
            takeCString(api.eg_handle_engram_app_event(handle, eventJson, deckId(), nowMs()), api)
        )
    }

    private fun hostResponseFromJson(json: String): Map<String, Any?> {
        if (json.isBlank()) return emptyMap()
        val root = runCatching { JSONObject(json) }.getOrNull() ?: return emptyMap()
        if (!root.optBoolean("ok", true)) {
            return mapOf("error" to jsonValue(root.opt("error")))
        }

        val response = mutableMapOf<String, Any?>(
            "props" to jsonObjectToMap(root.optJSONObject("props") ?: JSONObject())
        )
        val hostIntent = root.optJSONObject("hostIntent")
        if (hostIntent != null) {
            response["hostIntent"] = jsonObjectToMap(hostIntent)
        }
        return response
    }

    private fun takeCString(pointer: Pointer?, api: EngramCapi): String {
        if (pointer == null) {
            return """{"ok":false,"error":"Engram native host returned null"}"""
        }
        return try {
            pointer.getString(0, "UTF-8")
        } finally {
            api.eg_string_free(pointer)
        }
    }

    private fun deckId(): String = System.getenv("ENGRAM_DECK_ID") ?: ""

    private fun nowMs(): Long = System.currentTimeMillis()

    private fun loadCapi(): EngramCapi? {
        val names = listOf(nativeLibraryFileName(), "engram_capi")
        val roots = listOfNotNull(
            File(System.getProperty("user.dir")),
            runCatching {
                File(MosaicHost::class.java.protectionDomain.codeSource.location.toURI()).parentFile
            }.getOrNull()
        )
        val candidates = roots.flatMap { root -> names.map { File(root, it).path } } + names
        for (candidate in candidates) {
            val loaded = runCatching { Native.load(candidate, EngramCapi::class.java) }.getOrNull()
            if (loaded != null) {
                return loaded
            }
        }
        println("Engram Compose MosaicHost could not load engram-capi")
        return null
    }

    private fun nativeLibraryFileName(): String {
        val os = System.getProperty("os.name").lowercase()
        return when {
            os.contains("win") -> "engram_capi.dll"
            os.contains("mac") || os.contains("darwin") -> "libengram_capi.dylib"
            else -> "libengram_capi.so"
        }
    }
}

interface EngramCapi : Library {
    fun eg_session_new_demo(): Pointer?
    fun eg_session_free(session: Pointer?)
    fun eg_string_free(value: Pointer?)
    fun eg_engram_app_props(session: Pointer?, deckId: String, nowMs: Long): Pointer?
    fun eg_handle_engram_app_event(
        session: Pointer?,
        eventJson: String,
        deckId: String,
        nowMs: Long
    ): Pointer?
}

private fun jsonObjectToMap(value: JSONObject): Map<String, Any?> =
    value.keys().asSequence().associateWith { key -> jsonValue(value.opt(key)) }

private fun jsonArrayToList(value: JSONArray): List<Any?> =
    (0 until value.length()).map { index -> jsonValue(value.opt(index)) }

private fun jsonValue(value: Any?): Any? =
    when (value) {
        null, JSONObject.NULL -> null
        is JSONObject -> jsonObjectToMap(value)
        is JSONArray -> jsonArrayToList(value)
        else -> value
    }
