// Answer UI59's `files.open` effect on Compose Desktop -- see
// code/specs/UI59-files-open-effect.md for the full contract.
//
// Everything else about the application -- props, events, snapshot, restore --
// goes through the standard Mosaic host. This is the one thing a generated
// host cannot do on the application's behalf, because only the host has a
// window to hang a dialog off (the same reason engram-app's Compose host
// needs its own `EngramEffects.kt`, which this mirrors).
//
// Installed by the generated `Main.kt` through `[host_effects]`, immediately
// after the host is loaded.
//
// DEFERRED, and marshalled to the EDT -- exactly Engram's own two reasons
// (see EngramEffects.kt's own header comment): the host's monitor is held
// across the handler call, so a modal dialog run inline would hold it for as
// long as the dialog is open; and Compose state must be written from the UI
// thread, which is where the generated app's own props-changed handler runs.
// `SwingUtilities.invokeLater` satisfies both, and is where a Swing dialog
// has to run anyway.

import java.io.File
import java.io.FileInputStream
import java.util.Base64
import javax.swing.JFileChooser
import javax.swing.SwingUtilities
import javax.swing.filechooser.FileNameExtensionFilter

// The three tagged outcomes UI59 §3 defines. Cancellation is not a failure:
// Escape in a file dialog is an ordinary thing for a person to do.
private fun okOutcome(value: Map<String, Any?>): Map<String, Any?> = mapOf("ok" to value)

private fun cancelledOutcome(): Map<String, Any?> = mapOf("cancelled" to emptyMap<String, Any?>())

private fun failedOutcome(message: String): Map<String, Any?> =
    mapOf("failed" to mapOf("message" to message))

// Common photo MIME types this app's own request asks for (see
// photo-picker-mosaic-app's ACCEPT_IMAGE_TYPES) mapped to the file extensions
// Swing's `FileNameExtensionFilter` actually wants -- it filters by extension,
// not MIME type, so the handler owns this table rather than the app needing
// to know Compose-specific filter syntax (UI59 §3), matching the table the
// XAML and Qt handlers each carry for the same reason.
private val MIME_TYPE_EXTENSIONS: Map<String, List<String>> = mapOf(
    "image/jpeg" to listOf("jpg", "jpeg"),
    "image/png" to listOf("png"),
    "image/webp" to listOf("webp"),
    "image/gif" to listOf("gif"),
    "image/bmp" to listOf("bmp"),
    "image/tiff" to listOf("tif", "tiff"),
)

// The reverse of the table above, for reporting the picked file's MIME type
// back to the app (UI59 §3's `mimeType` result field) -- a chosen `File`
// carries no MIME type of its own, so this is recovered from the extension.
private fun mimeTypeForExtension(extension: String): String {
    val lowered = extension.lowercase()
    for ((mimeType, extensions) in MIME_TYPE_EXTENSIONS) {
        if (lowered in extensions) return mimeType
    }
    return "application/octet-stream"
}

// `accept` MIME types this host doesn't recognise are dropped rather than
// failing the request (UI59 §3); if nothing was recognised, no filter at all
// (Swing shows "All Files" when `fileFilter` is left unset).
@Suppress("UNCHECKED_CAST")
private fun extensionsFromAccept(payload: Any?): List<String> {
    val accept = (payload as? Map<String, Any?>)?.get("accept") as? List<*> ?: emptyList<Any?>()
    return accept.mapNotNull { it as? String }.flatMap { MIME_TYPE_EXTENSIONS[it].orEmpty() }
}

// A picked file is read fully into memory and base64-encoded (UI59 §3's
// `bytes` field is the whole file); without a cap, a caller picking an
// arbitrarily large file costs an arbitrarily large amount of host memory.
// 50 MiB matches the XAML and Qt handlers' own cap, for parity across
// backends.
private const val MAX_PICKED_FILE_BYTES = 50L * 1024 * 1024

// Reads at most `MAX_PICKED_FILE_BYTES` and returns null if the file ran
// over, rather than trusting `File.length()` checked once beforehand --
// Engram's own Compose handler (and the XAML handler's first cut) does
// exactly that single up-front check, which `/security-review` found is
// TOCTOU on the XAML PR (#15218): the file can grow between the check and
// the read, so a pre-check alone doesn't actually bound anything. Reading in
// bounded chunks here, the way the Qt handler (#15252) was hardened to from
// the start, means this handler never has that gap to begin with.
private fun readBounded(file: File): ByteArray? {
    val out = java.io.ByteArrayOutputStream()
    val buffer = ByteArray(64 * 1024)
    FileInputStream(file).use { stream ->
        while (true) {
            val got = stream.read(buffer)
            if (got < 0) return out.toByteArray()
            if (out.size() + got > MAX_PICKED_FILE_BYTES) return null
            out.write(buffer, 0, got)
        }
    }
}

// Returns the outcome rather than answering, so every path funnels to
// exactly one `completeEffect` at the call site -- same discipline as
// Engram's `runImport`/`runExport` (the effect has been deferred by then, so
// an early return that forgot to answer would wedge the app permanently).
private fun runPickPhoto(payload: Any?): Map<String, Any?> {
    val chooser = JFileChooser(System.getProperty("user.home"))
    chooser.dialogTitle = "Pick a Photo"
    chooser.fileSelectionMode = JFileChooser.FILES_ONLY
    chooser.isMultiSelectionEnabled = false
    val extensions = extensionsFromAccept(payload)
    if (extensions.isNotEmpty()) {
        chooser.fileFilter = FileNameExtensionFilter("Images", *extensions.toTypedArray())
    }

    if (chooser.showOpenDialog(null) != JFileChooser.APPROVE_OPTION) {
        return cancelledOutcome()
    }
    val chosen = chooser.selectedFile ?: return cancelledOutcome()

    if (!chosen.isFile) {
        return failedOutcome("that is not a regular file")
    }

    // Not `error.message`/a raw I/O exception's own text -- that can embed
    // the full local filesystem path, and `failed.message` is app-visible
    // data (the same reasoning that moved the XAML and Qt handlers off raw
    // exception messages, UI59 §4.2/§7.2). This file is meant to be copied
    // near-verbatim, like Engram's own `*Effects.*` family, so the safer
    // default is the one worth setting even though Engram's own Compose
    // handler does surface `error.message` for its own, already-merged,
    // existing effect.
    val bytes = try {
        readBounded(chosen)
    } catch (error: Exception) {
        return failedOutcome("couldn't read the selected file")
    } ?: return failedOutcome(
        "the selected file is too large or couldn't be read (limit is $MAX_PICKED_FILE_BYTES bytes)"
    )

    return okOutcome(
        mapOf(
            "name" to chosen.name,
            "mimeType" to mimeTypeForExtension(chosen.extension),
            "bytes" to Base64.getEncoder().encodeToString(bytes),
        )
    )
}

fun installPhotoPickerEffects(host: MosaicRuntimeHost) {
    host.effectHandler = { id, kind, payload, delivery ->
        // Only the awaited kind is answered. An awaited kind this host does
        // not know is left alone on purpose -- the host's own sweep fails it
        // with a reason the application shows (same convention as Engram's
        // Compose handler).
        if (delivery.lowercase() == "await" && kind == "files.open") {
            // Ownership first, inside the settle. `deferEffect` returns false
            // for an id the runtime is not waiting on, and then the right
            // move is to do nothing at all rather than open a dialog whose
            // answer would be refused.
            if (host.deferEffect(id)) {
                SwingUtilities.invokeLater {
                    // Nothing may escape this block -- the effect left the
                    // runtime's fail sweep the moment `deferEffect` returned
                    // true, so anything that unwinds to the EDT's uncaught
                    // handler leaves the id awaited for the life of the
                    // process (the runtime gates snapshot AND restore on
                    // nothing being pending).
                    //
                    // `Throwable`, not `Exception` -- a deliberate departure
                    // from Engram's Compose handler and the Qt handler's own
                    // `catch (...)`, both of which reason "an `Error` means
                    // the JVM is already going down." That reasoning doesn't
                    // hold for THIS handler: reading up to 50 MiB through
                    // `readBounded`'s `ByteArrayOutputStream` (whose doubling
                    // growth can transiently hold ~2x the accumulated size),
                    // then `toByteArray()` (a full copy), then
                    // `Base64.getEncoder().encodeToString` (another ~1.33x)
                    // can transiently need well over 100 MiB of live heap for
                    // a single in-cap pick -- a real, user-triggerable
                    // `OutOfMemoryError` on a JVM with a modest heap, not a
                    // sign the process is dying. Caught by `/security-review`
                    // before this ever shipped.
                    val outcome = try {
                        runPickPhoto(payload)
                    } catch (error: Throwable) {
                        failedOutcome("couldn't pick a photo")
                    }
                    host.completeEffect(id, outcome)
                }
            }
        }
    }
}
