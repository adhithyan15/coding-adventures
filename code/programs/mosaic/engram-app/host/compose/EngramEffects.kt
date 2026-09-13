// Answer Engram's Anki file-dialog effects on Compose Desktop.
//
// Everything else about the application -- props, events, snapshot, restore --
// goes through the standard Mosaic host. This is the one thing a generated host
// cannot do on the application's behalf, because only the host has a window to
// hang a dialog off.
//
// Installed by the generated `Main.kt` through `[host_effects]`, immediately
// after the host is loaded.
//
// DEFERRED, and marshalled to the EDT. Two separate reasons, both from the
// host's own contract:
//
//   1. The host's monitor is HELD across the handler call. A modal dialog run
//      inline would hold it for as long as the dialog is open, and the
//      documented escape -- blocking on another thread from inside the handler
//      -- is the deadlock the contract warns against. `deferEffect` is what it
//      offers instead.
//   2. "The props-changed handler runs on whatever thread answered. Compose
//      state must be written from the UI thread, so an app answering off-thread
//      should marshal there itself." The generated app writes Compose state
//      directly from that handler, so answering off the EDT would be a
//      cross-thread write into the composition.
//
// `SwingUtilities.invokeLater` satisfies both, and is where a Swing dialog has
// to run anyway.

import java.io.File
import java.util.Base64
import javax.swing.JFileChooser
import javax.swing.SwingUtilities
import javax.swing.filechooser.FileNameExtensionFilter

// The three tagged outcomes the protocol defines. Cancellation is not a
// failure: Escape in a file dialog is an ordinary thing for a person to do, and
// the application says "Import cancelled." rather than "Import failed."
private fun okOutcome(value: Map<String, Any?> = emptyMap()): Map<String, Any?> = mapOf("ok" to value)

private fun cancelledOutcome(): Map<String, Any?> = mapOf("cancelled" to emptyMap<String, Any?>())

private fun failedOutcome(message: String): Map<String, Any?> =
    mapOf("failed" to mapOf("message" to message))

// The largest package this host will read into memory.
//
// Aligned with what the engine will actually accept: the package layer refuses
// a collection past 256 MiB on native targets, so a larger file cannot import
// whatever this does. Checked BEFORE the read, because the point is to never
// make the allocation -- one import costs several times the file's size in
// flight (the bytes, their base64, that base64 as a String, the JSON envelope,
// and the runtime's own copy), and the application's own cap sits at the far
// end of all of it.
private const val MAX_IMPORT_BYTES = 256L * 1024 * 1024

// The extensions the application sent, so the picker shows what this build can
// actually read rather than a hardcoded guess that drifts from the engine.
//
// An extension is accepted only if it looks like one. The payload comes from
// this application rather than a user today, but a filter widened to everything
// is not something to leave to that staying true.
//
// `\A` and `\z`, NOT `^` and `$` -- the fourth engine to be asked the anchor
// question, and the fourth different answer.
//
// Measured here rather than carried over: Java's `$` concedes a trailing LF,
// CRLF, CR, NEL *and* U+2028 -- broader than PCRE2's LF-only default (the hole
// fixed in the Qt handler) and about the same as ICU's. Rust refuses all of
// them. No two of the four agree, which is why each one gets checked.
//
// `Regex.matches()` requires the whole input, so it rejects every one of those
// even with `^...$`, and the anchors are redundant *as written today*. They are
// here because that redundancy depends on the method: switch to
// `containsMatchIn` and `$` starts accepting `"apkg\n"` again, while `\A\z`
// does not. The pattern is correct on its own rather than correct because of
// its caller.
private val EXTENSION_SHAPE = Regex("""\A\.?[A-Za-z0-9_-]{1,16}\z""")

@Suppress("UNCHECKED_CAST")
private fun allowedExtensions(payload: Any?, key: String, fallback: List<String>): List<String> {
    val declared = (payload as? Map<String, Any?>)?.get(key) as? List<*>
    val source = declared?.mapNotNull { it as? String }?.takeIf { it.isNotEmpty() } ?: fallback
    val accepted = source.filter { EXTENSION_SHAPE.matches(it) }.map { it.removePrefix(".") }
    // Everything was refused, so fall back rather than hand the chooser an
    // empty filter, which shows the person a dialog listing nothing.
    return accepted.ifEmpty { fallback.map { it.removePrefix(".") } }
}

// The bytes travel, not the path.
//
// The application built this package and sent it out in the payload, so the
// host only has to write it. That is the arrangement sandboxing forces
// elsewhere in this family of hosts, and keeping all five the same is worth
// more than shaving a copy on the one platform that would allow it.
//
// Returns the outcome rather than answering, so every path through the dialog
// funnels to exactly one `completeEffect` at the call site. The effect has been
// deferred by then, which takes it out of the fail sweep -- an early `return`
// that forgot to answer would wedge the app permanently, because the runtime
// gates snapshot and restore on nothing being pending.
@Suppress("UNCHECKED_CAST")
private fun runExport(payload: Any?): Map<String, Any?> {
    val fields = payload as? Map<String, Any?>

    // Forced to a bare filename. A suggestion of `../.ssh/authorized_keys`
    // would otherwise open the dialog in a different directory with only the
    // basename visible. Nothing sends this key today; that is not a reason to
    // trust it.
    val suggested = (fields?.get("suggestedName") as? String)
        ?.let { File(it).name }
        ?.takeUnless { it.isEmpty() || it == "." || it == ".." }
        ?: "engram.apkg"

    val chooser = JFileChooser(System.getProperty("user.home"))
    chooser.dialogTitle = "Export Anki package"
    chooser.fileSelectionMode = JFileChooser.FILES_ONLY
    chooser.selectedFile = File(suggested)
    val extensions = allowedExtensions(payload, "extensions", listOf("apkg"))
    chooser.fileFilter = FileNameExtensionFilter("Anki packages", *extensions.toTypedArray())

    if (chooser.showSaveDialog(null) != JFileChooser.APPROVE_OPTION) {
        return cancelledOutcome()
    }
    var target = chooser.selectedFile ?: return cancelledOutcome()
    if (target.extension.isEmpty()) {
        target = File(target.parentFile, target.name + ".apkg")
    }

    val encoded = fields?.get("apkg") as? String
    if (encoded.isNullOrEmpty()) {
        return failedOutcome("the export carried no package")
    }
    // Strict decoding. `Base64.getDecoder()` rejects a character outside the
    // alphabet rather than skipping it, which matters because silently
    // discarding one would write a corrupt `.apkg` that only fails later,
    // inside Anki, where nothing points back here.
    val decoded = try {
        Base64.getDecoder().decode(encoded)
    } catch (error: IllegalArgumentException) {
        return failedOutcome("the export package was not valid base64")
    }
    // A zip, which is what an `.apkg` is. Not an is-it-empty check: padding-only
    // input decodes SUCCESSFULLY to a byte or two, so an emptiness test would
    // pass it and write a file Anki cannot open.
    if (decoded.size < 4 ||
        decoded[0] != 0x50.toByte() || decoded[1] != 0x4B.toByte() ||
        decoded[2] != 0x03.toByte() || decoded[3] != 0x04.toByte()
    ) {
        return failedOutcome("the export package was not a valid Anki package")
    }

    return try {
        // Written to a sibling temp file and moved into place, so a failure
        // partway leaves no truncated `.apkg` looking like a real one.
        val temp = File.createTempFile("engram-export", ".part", target.parentFile)
        temp.writeBytes(decoded)
        if (!temp.renameTo(target)) {
            temp.delete()
            failedOutcome("could not write ${target.name}")
        } else {
            okOutcome()
        }
    } catch (error: Exception) {
        failedOutcome(error.message ?: "the export could not be written")
    }
}

private fun runImport(payload: Any?): Map<String, Any?> {
    val chooser = JFileChooser(System.getProperty("user.home"))
    chooser.dialogTitle = "Import Anki package"
    chooser.fileSelectionMode = JFileChooser.FILES_ONLY
    chooser.isMultiSelectionEnabled = false
    val extensions = allowedExtensions(payload, "accept", listOf("apkg", "colpkg"))
    chooser.fileFilter = FileNameExtensionFilter("Anki packages", *extensions.toTypedArray())

    if (chooser.showOpenDialog(null) != JFileChooser.APPROVE_OPTION) {
        return cancelledOutcome()
    }
    val chosen = chooser.selectedFile ?: return cancelledOutcome()

    // Sized up before it is opened. A read on a fifo or a character device
    // never reaches EOF -- and reports no length -- so the regular-file test is
    // doing real work here, not restating the size test.
    if (!chosen.isFile) {
        return failedOutcome("that is not a regular file")
    }
    val length = chosen.length()
    if (length <= 0L) {
        return failedOutcome("that file is empty")
    }
    if (length > MAX_IMPORT_BYTES) {
        return failedOutcome("that package is too large to open")
    }

    val bytes = try {
        chosen.readBytes()
    } catch (error: Exception) {
        return failedOutcome(error.message ?: "that file could not be read")
    }
    if (bytes.isEmpty()) {
        return failedOutcome("that file is empty")
    }

    // The application decodes and merges. Reading the file is the host's whole
    // job here, for the same reason the export writes it.
    return okOutcome(mapOf("apkg" to Base64.getEncoder().encodeToString(bytes)))
}

/// Answer Engram's awaited file-dialog effects.
fun installEngramEffects(host: MosaicRuntimeHost) {
    host.effectHandler = { id, kind, payload, delivery ->
        // Only the awaited kinds are answered. `openCard` arrives as a `Notify`
        // and is deliberately not handled: nothing is waiting on it, and
        // answering an effect the runtime is not awaiting is refused anyway.
        if (delivery.lowercase() == "await") {
            when (kind) {
                // Exactly the two kinds `effect_for_intent` mints as `Await`.
                // `openCard` is the third intent the facade can emit and it is
                // a `Notify`, so it never reaches the `await` guard above.
                "importAnki", "exportAnki" -> {
                    // Ownership first, inside the settle. `deferEffect` returns
                    // false for an id the runtime is not waiting on, and then
                    // the right move is to do nothing at all rather than open a
                    // dialog whose answer would be refused.
                    if (host.deferEffect(id)) {
                        SwingUtilities.invokeLater {
                            // Nothing may escape this block. The effect left the
                            // runtime's fail sweep the moment `deferEffect`
                            // returned true, so an exception that unwinds to the
                            // EDT's uncaught handler does not degrade the
                            // outcome -- it leaves the id awaited for the life of
                            // the process, and the runtime gates snapshot AND
                            // restore on nothing being pending.
                            //
                            // The `run*` functions guard their own file I/O, but
                            // the dialogs are outside that: `JFileChooser`'s
                            // constructor and `showSaveDialog`/`showOpenDialog`/
                            // `showOptionDialog` all throw `HeadlessException` on
                            // a display-less session, which is the one
                            // environment where every one of these fails at once.
                            //
                            // `Exception`, not `Throwable`: an `Error` means the
                            // JVM is already going down, and answering an effect
                            // on the way is neither possible to rely on nor worth
                            // swallowing an OOM for. Qt guards the same span with
                            // `catch (...)` for the same reason.
                            val outcome = try {
                                when (kind) {
                                    "importAnki" -> runImport(payload)
                                    "exportAnki" -> runExport(payload)
                                    // Unreachable: the outer `when` admits only
                                    // the two kinds above. Spelled out rather
                                    // than folded into `else -> runExport(...)`,
                                    // so widening the outer guard without
                                    // widening this one fails visibly instead of
                                    // silently running the wrong dialog.
                                    else -> failedOutcome("this host does not answer $kind")
                                }
                            } catch (error: Exception) {
                                failedOutcome(error.message ?: "the dialog could not be opened")
                            }
                            host.completeEffect(id, outcome)
                        }
                    }
                }
                // An awaited kind this host does not know is left alone on
                // purpose. The host's own sweep then fails it with a reason the
                // application shows, which is a better outcome than this file
                // inventing one -- and it is what keeps a new effect kind from
                // silently doing nothing while appearing handled.
                else -> Unit
            }
        }
    }
}
