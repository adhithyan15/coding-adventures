// Answer Engram's Anki import and export on Android (UI89 §3.12).
//
// The desktop handler (`host/compose/EngramEffects.kt`) opens Swing file
// choosers, which Android does not have. Here the platform library lends its
// document picker instead (UI89 §3.11): `MosaicActivity` installs this
// handler first and the library's router around it, and the router passes
// `importAnki` and `exportAnki` on to this handler because the manifest's
// `compose-android` entry claims them. For each, this handler hands the
// request back to that router -- `openForApp` or `saveForApp` -- which:
//
//   - defers the effect before anything is shown, or shows nothing for an id
//     the runtime is not awaiting;
//   - keeps the one-file-operation-at-a-time rule;
//   - reads or writes on its background thread, with the size limit enforced
//     while reading; and
//   - answers exactly once, on the main thread.
//
// So this handler neither defers nor answers once it has handed a request
// over. The one place it answers itself is a request it refuses before any
// picker is shown, and that answer is synchronous: the effect was never
// deferred, so the runtime is still awaiting it.
//
// The Android counterpart of the iOS branch of `host/swiftui/EngramEffects.swift`,
// rule for rule.

import java.io.File
import java.util.Base64

private fun failedOutcome(message: String): Map<String, Any?> =
    mapOf("failed" to mapOf("message" to message))

// The largest package this host will read: what the engine accepts, as on
// every other host. The router enforces it WHILE reading, so a larger file is
// never read into memory at all.
private const val MAX_IMPORT_BYTES = 256L * 1024 * 1024

// An extension is accepted only if it looks like one -- `\A` and `\z`, which
// unlike `$` concede no trailing line break (see the desktop handler for the
// measurement).
private val EXTENSION_SHAPE = Regex("""\A\.?[A-Za-z0-9_-]{1,16}\z""")

// The extensions the application sent, lowercased (the library compares
// without case, but a picker's MIME guess is made from the lowercase form),
// or the fallback when none survive.
@Suppress("UNCHECKED_CAST")
private fun allowedExtensions(payload: Any?, key: String, fallback: List<String>): List<String> {
    val declared = (payload as? Map<String, Any?>)?.get(key) as? List<*>
    val source = declared?.mapNotNull { it as? String }?.takeIf { it.isNotEmpty() } ?: fallback
    val accepted = source.filter { EXTENSION_SHAPE.matches(it) }.map { it.removePrefix(".").lowercase() }
    return accepted.ifEmpty { fallback }
}

// The name an export is offered under: the payload's `suggestedName` reduced
// to its last path component, ending in `.apkg` -- the picker cannot add an
// extension, and the library refuses a name whose extension is not an
// accepted one. A suggestion the library would refuse (`.hidden`, `CON`, a
// colon) falls back to `engram.apkg` rather than failing the export: it is
// only a suggestion, which the person can still change in the picker.
@Suppress("UNCHECKED_CAST")
private fun exportName(payload: Any?): String {
    val suggested = ((payload as? Map<String, Any?>)?.get("suggestedName") as? String)
        ?.let { File(it).name }
        ?.takeUnless { it.isEmpty() || it == "." || it == ".." }
        ?: "engram.apkg"
    val named = if (suggested.substringAfterLast('.', "").lowercase() == "apkg") suggested else "$suggested.apkg"
    return if (!mosaicIsPlainFileName(named) || mosaicIsReservedDeviceName(named)) "engram.apkg" else named
}

// The package an export carries, decoded and checked, or the message to fail
// with. Strict decoding: `Base64.getDecoder()` refuses a character outside the
// alphabet rather than skipping it, which would write a corrupt `.apkg` that
// only fails later, inside Anki. Then the zip local file header an `.apkg`
// begins with -- not an is-it-empty check, because padding-only input decodes
// successfully to a byte or two.
@Suppress("UNCHECKED_CAST")
private fun decodedExportPackage(payload: Any?): Pair<ByteArray?, String?> {
    val encoded = (payload as? Map<String, Any?>)?.get("apkg") as? String
    if (encoded.isNullOrEmpty()) {
        return null to "the export carried no package"
    }
    val decoded = try {
        Base64.getDecoder().decode(encoded)
    } catch (error: IllegalArgumentException) {
        return null to "the export package was not valid base64"
    }
    if (decoded.size < 4 ||
        decoded[0] != 0x50.toByte() || decoded[1] != 0x4B.toByte() ||
        decoded[2] != 0x03.toByte() || decoded[3] != 0x04.toByte()
    ) {
        return null to "the export package was not a valid Anki package"
    }
    return decoded to null
}

/// Answer Engram's awaited Anki effects through the platform library's
/// document picker.
fun installEngramAndroidEffects(host: MosaicRuntimeHost) {
    // Answer a refusal made before any picker, without letting the host's own
    // refusal (an id it is not awaiting, or a closed host) throw into the
    // router -- on Android, the main thread.
    fun refuse(id: Long, message: String) {
        try {
            host.completeEffect(id, failedOutcome(message))
        } catch (ignored: Exception) {
        }
    }

    host.effectHandler = { id, kind, payload, delivery ->
        // Only the awaited kinds are answered. `openCard` arrives as a
        // `Notify`, with nothing waiting on it.
        if (delivery.lowercase() == "await") {
            when (kind) {
                "importAnki", "exportAnki" -> {
                    // Looked up per request rather than captured: the router
                    // is installed AFTER this handler, around it, so it does
                    // not exist yet when this function runs.
                    val router = mosaicPlatformRouter(host)
                    if (router == null) {
                        // Kinds this host knows and cannot serve: saying so
                        // beats the host's generic "unanswered".
                        refuse(id, "file dialogs are not available on this platform")
                    } else if (kind == "importAnki") {
                        val accept = MosaicAccept(
                            emptyList(),
                            allowedExtensions(payload, "accept", listOf("apkg", "colpkg")),
                        )
                        // The application decodes and merges; the host's
                        // whole job is the bytes, bounded by what the engine
                        // will accept. Built on the router's background
                        // thread, so it only builds the answer.
                        router.openForApp(id, accept, MAX_IMPORT_BYTES) { _, bytes ->
                            mapOf("apkg" to Base64.getEncoder().encodeToString(bytes))
                        }
                    } else {
                        val (decoded, refusal) = decodedExportPackage(payload)
                        if (decoded == null) {
                            refuse(id, refusal ?: "the export package was not a valid Anki package")
                        } else {
                            // Always `.apkg`, not the payload's list: an
                            // export IS an Anki package, and a fixed list
                            // leaves nothing for a payload to widen.
                            router.saveForApp(
                                id,
                                exportName(payload),
                                decoded,
                                MosaicAccept(emptyList(), listOf("apkg")),
                            ) { emptyMap() }
                        }
                    }
                }
                // An awaited kind this host does not know is left alone on
                // purpose: the host's own sweep fails it with a reason the
                // application shows.
                else -> Unit
            }
        }
    }
}
