// Drives the emitted Compose host against the conformance runtime.
//
// This crate's other tests assert on the TEXT of the emitted host. That cannot
// tell you it compiles -- three missing imports survived every one of them --
// let alone that it behaves. The defect this exists for is behavioural: before
// the host completed effects, an `await` was dropped and the app waited forever
// with nothing reporting it.
//
// One scenario per process, chosen by MOSAIC_PROBE_CASE. The JVM cannot change
// its own environment, and the host reads MOSAIC_APP_STATE_PATH once at load --
// so a single process sharing one state file would let one scenario restore
// another's count, and an assertion would read a number its host never produced.
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit

interface MosaicComposeHost : AutoCloseable {
    fun props(): Map<String, Any?>?
    fun handleEvent(event: Map<String, Any?>): Map<String, Any?>?
    fun setPropsChangedHandler(handler: (() -> Unit)?) {}
    override fun close() {}
}

private var failures = 0

private fun check(ok: Boolean, what: String) {
    println(what.padEnd(60) + if (ok) "ok" else "FAIL")
    if (!ok) failures += 1
}

@Suppress("UNCHECKED_CAST")
private fun propsOf(update: Map<String, Any?>?): Map<String, Any?> =
    (update?.get("props") as? Map<String, Any?>) ?: emptyMap()

// A missing key must not read as zero: comparing against 0 would pass when the
// props are empty, which is how the Qt version of this file first passed.
private fun awaited(props: Map<String, Any?>, context: String): Int {
    val value = props["awaitedEffects"]
    if (value !is Number) {
        println("VACUOUS: $context carries no awaitedEffects key")
        failures += 1
        return -1
    }
    return value.toInt()
}

// Likewise: a missing count must not compare equal to the value under test.
private fun count(props: Map<String, Any?>, context: String): Int {
    val value = props["count"]
    if (value !is Number) {
        println("VACUOUS: $context carries no count key")
        failures += 1
        return Int.MIN_VALUE
    }
    return value.toInt()
}

private fun status(props: Map<String, Any?>): String = props["status"] as? String ?: ""

/**
 * Whether a snapshot was actually produced, and why not when it was not.
 *
 * The host reports a refusal two different ways -- a non-OK transport status
 * throws `MosaicRuntimeException`, while an app-level refusal comes back as an
 * `error` key -- and which one a pending effect takes is the runtime's business,
 * not this driver's. Both count as refused; only a real snapshot counts as made.
 */
private fun snapshotMade(host: MosaicRuntimeHost): Pair<Boolean, String> =
    runCatching { host.snapshot() }.fold(
        onSuccess = { snap ->
            when {
                snap == null -> false to "the app produced no snapshot"
                snap["error"] != null -> false to snap["error"].toString()
                else -> true to ""
            }
        },
        onFailure = { false to (it.message ?: it::class.qualifiedName ?: "threw") },
    )

private fun request(host: MosaicRuntimeHost, batch: Boolean = false): Map<String, Any?> =
    propsOf(
        host.handleEvent(
            mapOf(
                "event" to if (batch) "requestEffectBatch" else "requestEffect",
                "notify" to false,
            ),
        ),
    )

private fun host(): MosaicRuntimeHost =
    checkNotNull(MosaicRuntimeHost.load()) {
        "the emitted Compose host did not load the conformance runtime"
    }

private fun isAwait(delivery: String) = delivery.lowercase() == "await"

/** A fresh app awaits nothing, and an await nobody answers is failed, not dropped. */
private fun caseUnhandled() {
    val host = host()
    host.use {
        check(awaited(propsOf(host.props()), "startup") == 0, "a fresh app awaits nothing")

        // No handler connected -- what every native host did before completion
        // existed. The effect must come back failed, not sit pending forever.
        val unhandled = request(host)
        check(awaited(unhandled, "unhandled") == 0, "an unanswered await is failed, not dropped")
        check(
            status(unhandled).contains("no host handler"),
            "the app is told why, rather than just waiting",
        )
        check(count(unhandled, "unhandled") == 0, "a failed completion does not advance the app")
    }
}

/** A handler that answers properly settles the effect and moves the app. */
private fun caseAnswered() {
    val host = host()
    host.use {
        host.effectHandler = { id, _, _, delivery ->
            if (isAwait(delivery)) host.completeEffect(id, mapOf("ok" to mapOf("amount" to 5)))
        }
        val answered = request(host)
        check(awaited(answered, "handled") == 0, "an answered await is settled")
        check(count(answered, "handled") == 5, "the handler's value reached the app")
    }
}

/**
 * A batch where the handler answers BOTH, each chaining.
 *
 * One slot holding "the last update" drops the first answer's minted effect: it
 * exists in no collection the host kept, so it is never emitted, never failed,
 * and permanently pending -- which kills snapshot and restore. Answering a
 * single effect cannot reach it.
 */
private fun caseBatchBothAnswered() {
    val host = host()
    host.use {
        var answers = 0
        host.effectHandler = { id, _, _, delivery ->
            if (isAwait(delivery)) {
                val chain = answers < 2 // chain only the first two, or this never ends
                answers += 1
                host.completeEffect(id, mapOf("ok" to mapOf("amount" to 1, "chain" to chain)))
            }
        }
        val batch = request(host, batch = true)
        check(answers >= 2, "the handler answered both effects of the batch")
        check(
            awaited(batch, "both-answered batch") == 0,
            "a fully-answered chaining batch leaves nothing outstanding",
        )
        val (made, why) = snapshotMade(host)
        check(made, "snapshot still works after a fully-answered chaining batch" + suffix(made, why))
    }
}

/** A batch where the handler answers one and ignores the other. */
private fun caseBatchPartlyAnswered() {
    val host = host()
    host.use {
        var answeredOne = false
        host.effectHandler = { id, _, _, delivery ->
            if (isAwait(delivery) && !answeredOne) {
                answeredOne = true
                host.completeEffect(id, mapOf("ok" to mapOf("amount" to 3, "chain" to true)))
            }
        }
        val batch = request(host, batch = true)
        check(
            awaited(batch, "mixed batch") == 0,
            "a partly-answered batch leaves nothing outstanding",
        )
        val (made, why) = snapshotMade(host)
        check(made, "snapshot still works after a partly-answered batch" + suffix(made, why))
    }
}

/**
 * A handler that throws.
 *
 * The handler runs inside the settle loop, so an escaping exception leaves the
 * id in `awaiting` with nothing left to discharge it -- and the runtime refuses
 * to snapshot or restore while anything is pending, so one throwing handler
 * costs the process its persistence for good.
 */
private fun caseThrowingHandler() {
    val host = host()
    host.use {
        host.effectHandler = { _, _, _, delivery ->
            if (isAwait(delivery)) throw IllegalStateException("the dialog exploded")
        }
        val thrown = request(host)
        check(
            awaited(thrown, "throwing handler") == 0,
            "a handler that throws does not leave the effect pending",
        )
        check(
            status(thrown).contains("the dialog exploded"),
            "the app is told the handler failed, and why",
        )
        val (made, why) = snapshotMade(host)
        check(made, "snapshot survives a handler that threw" + suffix(made, why))
    }
}

/**
 * The shape a real file-dialog handler writes first: hand the chosen path back
 * as the `File` the dialog returned.
 *
 * `toJsonElement` has no case for it and throws, from inside the handler, from
 * inside the settle -- the same wedge as above by a route an app reaches on its
 * first run rather than only when something is wrong.
 */
private fun caseUnconvertibleResult() {
    val host = host()
    host.use {
        host.effectHandler = { id, _, _, delivery ->
            if (isAwait(delivery)) {
                host.completeEffect(
                    id,
                    mapOf("ok" to mapOf("path" to java.io.File("/tmp/deck.apkg"))),
                )
            }
        }
        val bad = request(host)
        check(
            awaited(bad, "unconvertible result") == 0,
            "an unconvertible effect result does not wedge persistence",
        )
        check(
            status(bad).contains("Unsupported Mosaic"),
            "the app is told the result value could not be converted",
        )
        val (made, why) = snapshotMade(host)
        check(made, "snapshot survives an unconvertible effect result" + suffix(made, why))
    }
}

/**
 * The shape a real file dialog needs, and the one that could not be written
 * before: take ownership, answer LATER, from another thread.
 *
 * No thread-affinity assertion here, unlike the SwiftUI driver beside this one.
 * SwiftUI requires state mutation on the main thread, so that host hops; Compose
 * writes `mutableStateOf` through the snapshot system, which accepts writes from
 * any thread, so requiring a hop here would be inventing a rule Compose does not
 * have. What must hold is that the late answer reaches the UI at all.
 */
private fun caseDeferred() {
    val host = host()
    host.use {
        var deferredId: Long? = null
        host.effectHandler = { id, _, _, delivery ->
            if (isAwait(delivery)) {
                deferredId = id
                host.deferEffect(id) // "the dialog is open"
            }
        }
        var pushes = 0
        host.setPropsChangedHandler { pushes += 1 }

        val afterRequest = request(host)
        val pushesBeforeAnswer = pushes
        // Deferring something the runtime is not waiting on must be refused, or
        // the fail sweep is switched off for an effect nothing will ever answer.
        check(!host.deferEffect(99_999L), "deferring an effect nothing awaits is refused")
        check(deferredId != null, "the handler was offered the effect")
        check(
            awaited(afterRequest, "deferred") == 1,
            "a deferred effect stays outstanding rather than being failed",
        )
        val (madeWhilePending, _) = snapshotMade(host)
        check(!madeWhilePending, "snapshot is refused while a deferred effect is outstanding")

        // ...the dialog closes, on another thread, whenever.
        val done = CountDownLatch(1)
        Thread {
            host.completeEffect(deferredId!!, mapOf("ok" to mapOf("amount" to 9)))
            done.countDown()
        }.start()
        check(done.await(5, TimeUnit.SECONDS), "answering from another thread does not deadlock")

        // `>= 1` would have been satisfied by the push from the request above --
        // the assertion has to be that THIS answer pushed.
        check(pushes > pushesBeforeAnswer, "the late answer reached the UI as a props change")
        val final = propsOf(host.props())
        check(awaited(final, "answered-late") == 0, "answering a deferred effect settles it")
        check(count(final, "answered-late") == 9, "the deferred answer's value reached the app")
        val (made, why) = snapshotMade(host)
        check(made, "snapshot works again once the deferred effect is answered" + suffix(made, why))
    }
}

private fun suffix(ok: Boolean, why: String) = if (ok) "" else " [$why]"

fun main() {
    when (val case = System.getenv("MOSAIC_PROBE_CASE") ?: "") {
        "unhandled" -> caseUnhandled()
        "answered" -> caseAnswered()
        "batch-both" -> caseBatchBothAnswered()
        "batch-mixed" -> caseBatchPartlyAnswered()
        "throwing" -> caseThrowingHandler()
        "unconvertible" -> caseUnconvertibleResult()
        "deferred" -> caseDeferred()
        else -> {
            println("unknown MOSAIC_PROBE_CASE `$case`")
            kotlin.system.exitProcess(2)
        }
    }
    println(if (failures == 0) "case passed" else "$failures check(s) failed")
    kotlin.system.exitProcess(if (failures == 0) 0 else 1)
}
