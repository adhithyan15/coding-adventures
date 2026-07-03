import com.sun.jna.Library
import com.sun.jna.Native
import com.sun.jna.Pointer
import java.awt.GraphicsEnvironment
import java.io.File
import javax.swing.JFileChooser
import javax.swing.filechooser.FileNameExtensionFilter
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
        val handled = if (response.containsKey("error")) response else handleHostIntent(response)
        if (!handled.containsKey("error")) {
            persistSnapshot()
        }
        return handled
    }

    private fun handleHostIntent(response: Map<String, Any?>): Map<String, Any?> {
        val hostIntent = response["hostIntent"] as? Map<*, *> ?: return response
        return when (hostIntent["type"] as? String) {
            "importAnki" -> importAnkiPackage(response, hostIntent)
            "exportAnki" -> exportAnkiPackage(response, hostIntent)
            else -> response
        }
    }

    private fun importAnkiPackage(
        response: Map<String, Any?>,
        hostIntent: Map<*, *>
    ): Map<String, Any?> {
        val api = capi ?: return hostResultResponse(response, hostIntent, "unavailable")
        val handle = session ?: return hostResultResponse(response, hostIntent, "unavailable")
        if (GraphicsEnvironment.isHeadless()) {
            return hostResultResponse(response, hostIntent, "unsupported")
        }

        val file = chooseAnkiImportFile(hostIntent)
            ?: return hostResultResponse(response, hostIntent, "cancelled")
        val bytes = runCatching { file.readBytes() }.getOrElse {
            println("Engram Compose MosaicHost could not read Anki package: ${it.message}")
            return hostResultResponse(response, hostIntent, "read-error", file.path, it.message)
        }

        val imported = hostResponseFromJson(
            takeCString(api.eg_merge_anki_apkg(handle, bytes, bytes.size.toLong()), api)
        )
        if (imported.containsKey("error")) {
            println("Engram Compose MosaicHost could not import Anki package: ${imported["error"]}")
            return hostResultResponse(response, hostIntent, "import-error", file.path, imported["error"])
        }

        persistSnapshot()
        val refreshed = props().toMutableMap()
        refreshed["hostIntent"] = jsonMap(hostIntent)
        val hostResult = mapOf(
            "status" to "imported",
            "path" to file.path
        )
        refreshed["hostResult"] = hostResult
        return withHostStatusProps(refreshed, hostResult)
    }

    private fun exportAnkiPackage(
        response: Map<String, Any?>,
        hostIntent: Map<*, *>
    ): Map<String, Any?> {
        val api = capi ?: return hostResultResponse(response, hostIntent, "unavailable")
        val handle = session ?: return hostResultResponse(response, hostIntent, "unavailable")
        if (GraphicsEnvironment.isHeadless()) {
            return hostResultResponse(response, hostIntent, "unsupported")
        }

        val file = chooseAnkiExportFile(hostIntent)
            ?: return hostResultResponse(response, hostIntent, "cancelled")
        val outputFile = if (file.extension.isBlank()) File("${file.path}.apkg") else file
        val exported = runCatching { JSONObject(takeCString(api.eg_export_anki_apkg(handle), api)) }
            .getOrElse {
                println("Engram Compose MosaicHost could not parse exported Anki package: ${it.message}")
                return hostResultResponse(response, hostIntent, "export-error", outputFile.path, it.message)
            }
        if (!exported.optBoolean("ok", true)) {
            val error = jsonValue(exported.opt("error"))
            println("Engram Compose MosaicHost could not export Anki package: $error")
            return hostResultResponse(response, hostIntent, "export-error", outputFile.path, error)
        }

        val bytes = jsonByteArray(exported, "apkg")
        if (bytes.isEmpty()) {
            return hostResultResponse(
                response,
                hostIntent,
                "export-error",
                outputFile.path,
                "Engram native host returned an empty APKG"
            )
        }

        val wrote = runCatching {
            outputFile.parentFile?.mkdirs()
            outputFile.writeBytes(bytes)
        }
        if (wrote.isFailure) {
            println("Engram Compose MosaicHost could not save Anki package: ${wrote.exceptionOrNull()?.message}")
            return hostResultResponse(
                response,
                hostIntent,
                "write-error",
                outputFile.path,
                wrote.exceptionOrNull()?.message
            )
        }

        return hostResultResponse(response, hostIntent, "exported", outputFile.path)
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

    private fun chooseAnkiImportFile(hostIntent: Map<*, *>): File? {
        val chooser = JFileChooser()
        chooser.dialogTitle = "Import Anki package"
        chooser.fileSelectionMode = JFileChooser.FILES_ONLY
        chooser.fileFilter = ankiFileFilter(
            hostIntentExtensions(hostIntent, "accept", listOf(".apkg", ".colpkg"))
        )
        return if (chooser.showOpenDialog(null) == JFileChooser.APPROVE_OPTION) {
            chooser.selectedFile
        } else {
            null
        }
    }

    private fun chooseAnkiExportFile(hostIntent: Map<*, *>): File? {
        val chooser = JFileChooser(System.getProperty("user.home"))
        chooser.dialogTitle = "Export Anki package"
        chooser.fileSelectionMode = JFileChooser.FILES_ONLY
        chooser.selectedFile = File(System.getProperty("user.home"), suggestedAnkiFileName(hostIntent))
        chooser.fileFilter = ankiFileFilter(
            hostIntentExtensions(hostIntent, "extensions", listOf(".apkg"))
        )
        return if (chooser.showSaveDialog(null) == JFileChooser.APPROVE_OPTION) {
            chooser.selectedFile
        } else {
            null
        }
    }

    private fun hostResultResponse(
        response: Map<String, Any?>,
        hostIntent: Map<*, *>,
        status: String,
        path: String? = null,
        error: Any? = null
    ): Map<String, Any?> {
        val out = response.toMutableMap()
        out["hostIntent"] = jsonMap(hostIntent)
        val hostResult = mutableMapOf<String, Any?>("status" to status)
        if (!path.isNullOrBlank()) {
            hostResult["path"] = path
        }
        if (error != null && error.toString().isNotBlank()) {
            hostResult["error"] = error.toString()
        }
        out["hostResult"] = hostResult
        return withHostStatusProps(out, hostResult)
    }

    private fun withHostStatusProps(
        response: Map<String, Any?>,
        hostResult: Map<String, Any?>
    ): Map<String, Any?> {
        val statusProps = hostStatusProps(hostResult)
        if (statusProps.isEmpty()) return response
        val out = response.toMutableMap()
        val props = (out["props"] as? Map<*, *>)?.let(::jsonMap)?.toMutableMap()
            ?: mutableMapOf()
        props.putAll(statusProps)
        out["props"] = props
        return out
    }

    private fun hostStatusProps(hostResult: Map<String, Any?>): Map<String, Any?> {
        val status = hostResult["status"] as? String ?: return emptyMap()
        if (status.isBlank()) return emptyMap()
        return mapOf(
            "host-status-visible" to true,
            "host-status-kind" to status,
            "host-status-label" to hostStatusLabel(status),
            "host-status-message" to hostStatusMessage(hostResult, status)
        )
    }

    private fun hostStatusLabel(status: String): String =
        when (status) {
            "imported" -> "Import complete"
            "exported" -> "Export complete"
            "cancelled" -> "Import cancelled"
            "read-error", "import-error" -> "Import failed"
            "export-error", "write-error" -> "Export failed"
            "unavailable", "unsupported" -> "Host unavailable"
            else -> "Host status"
        }

    private fun hostStatusMessage(hostResult: Map<String, Any?>, status: String): String {
        val file = hostResultFile(hostResult)
        val error = hostResult["error"]?.toString().orEmpty()
        return when (status) {
            "imported" -> if (file.isBlank()) "Anki package imported." else "Imported $file."
            "exported" -> if (file.isBlank()) "Anki package exported." else "Saved $file."
            "cancelled" -> "No Anki package was selected."
            "read-error" -> if (file.isBlank()) {
                if (error.isBlank()) "Could not read the selected file." else "Could not read the selected file: $error"
            } else {
                if (error.isBlank()) "Could not read $file." else "Could not read $file: $error"
            }
            "import-error" -> if (file.isBlank()) {
                if (error.isBlank()) "Could not import the selected package." else "Could not import the selected package: $error"
            } else {
                if (error.isBlank()) "Could not import $file." else "Could not import $file: $error"
            }
            "export-error" -> if (error.isBlank()) {
                "Could not export Anki package."
            } else {
                "Could not export Anki package: $error"
            }
            "write-error" -> if (file.isBlank()) {
                if (error.isBlank()) "Could not save the Anki package." else "Could not save the Anki package: $error"
            } else {
                if (error.isBlank()) "Could not save $file." else "Could not save $file: $error"
            }
            "unavailable" -> "Engram native host is unavailable."
            "unsupported" -> "This host does not support native Anki file dialogs yet."
            else -> if (error.isBlank()) {
                if (file.isBlank()) status else file
            } else {
                error
            }
        }
    }

    private fun hostResultFile(hostResult: Map<String, Any?>): String {
        val path = hostResult["path"] as? String ?: return ""
        return File(path).name
    }

    private fun hostIntentExtensions(
        hostIntent: Map<*, *>,
        property: String,
        fallback: List<String>
    ): List<String> {
        val raw = hostIntent[property] as? List<*> ?: return fallback
        val extensions = raw.mapNotNull { value ->
            val extension = value?.toString()?.trim().orEmpty()
            when {
                extension.isBlank() -> null
                extension.startsWith(".") -> extension
                else -> ".$extension"
            }
        }
        return extensions.ifEmpty { fallback }
    }

    private fun ankiFileFilter(extensions: List<String>): FileNameExtensionFilter =
        FileNameExtensionFilter(
            "Anki packages",
            *extensions.map { it.removePrefix(".") }.toTypedArray()
        )

    private fun suggestedAnkiFileName(hostIntent: Map<*, *>): String {
        val raw = hostIntent["deckId"]?.toString()?.trim()?.takeIf { it.isNotEmpty() }
            ?: "engram-collection"
        val safe = raw.replace(Regex("""[\/\\:*?"<>|]"""), "-")
        return if (safe.lowercase().endsWith(".apkg")) safe else "$safe.apkg"
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
    fun eg_export_anki_apkg(session: Pointer?): Pointer?
    fun eg_merge_anki_apkg(session: Pointer?, data: ByteArray, dataLen: Long): Pointer?
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

private fun jsonMap(value: Map<*, *>): Map<String, Any?> =
    value.entries.mapNotNull { entry ->
        val key = entry.key as? String ?: return@mapNotNull null
        key to entry.value
    }.toMap()

private fun jsonByteArray(root: JSONObject, property: String): ByteArray {
    val values = root.optJSONArray(property) ?: return ByteArray(0)
    return ByteArray(values.length()) { index -> values.optInt(index).toByte() }
}

private fun jsonOk(json: String): Boolean =
    runCatching { JSONObject(json).optBoolean("ok", true) }.getOrDefault(false)
