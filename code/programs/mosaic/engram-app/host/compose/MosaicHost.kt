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
        hydrateSession()
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
        val response = hostResponseFromJson(
            takeCString(api.eg_handle_engram_app_event(handle, eventJson, deckId(), nowMs()), api)
        )
        if (!response.containsKey("error")) {
            persistSnapshot()
        }
        return response
    }

    private fun hydrateSession() {
        val api = capi ?: return
        val handle = session ?: return
        val file = snapshotFile()
        if (file.exists()) {
            val loaded = runCatching {
                val json = takeCString(api.eg_load_snapshot(handle, file.readText()), api)
                jsonOk(json)
            }.getOrDefault(false)
            if (loaded) {
                return
            }
            println("Engram Compose MosaicHost persisted snapshot was invalid; using demo state")
        }
        persistSnapshot()
    }

    private fun persistSnapshot() {
        val api = capi ?: return
        val handle = session ?: return
        val json = takeCString(api.eg_snapshot(handle), api)
        val root = runCatching { JSONObject(json) }.getOrNull() ?: return
        if (!root.optBoolean("ok", true) || root.isNull("state")) {
            return
        }
        runCatching {
            val file = snapshotFile()
            file.parentFile?.mkdirs()
            file.writeText(root.get("state").toString())
        }
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

    private fun snapshotFile(): File =
        System.getenv("ENGRAM_SNAPSHOT_PATH")?.takeIf { it.isNotBlank() }?.let(::File)
            ?: File(File(System.getProperty("user.home"), ".engram"), "mosaic-snapshot.v1.json")

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
    fun eg_snapshot(session: Pointer?): Pointer?
    fun eg_load_snapshot(session: Pointer?, snapshotJson: String): Pointer?
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

private fun jsonOk(json: String): Boolean =
    runCatching { JSONObject(json).optBoolean("ok", true) }.getOrDefault(false)
