import java.io.File

interface MosaicComposeHost : AutoCloseable {
    fun props(): Map<String, Any?>?
    fun handleEvent(event: Map<String, Any?>): Map<String, Any?>?
    fun setPropsChangedHandler(handler: (() -> Unit)?) {}
    override fun close() {}
}

private fun requireConformance(condition: Boolean, assertion: String) {
    check(condition) { "Failed assertion: $assertion" }
}

private fun objectMap(value: Any?, assertion: String): Map<String, Any?> {
    requireConformance(value is Map<*, *>, "$assertion was not an object")
    @Suppress("UNCHECKED_CAST")
    val result = value as Map<String, Any?>
    requireConformance("error" !in result, "$assertion returned an error")
    return result
}

private fun props(update: Map<String, Any?>, assertion: String): Map<String, Any?> =
    objectMap(update["props"], "$assertion props")

private fun integer(value: Any?, assertion: String): Long {
    requireConformance(value is Number, "$assertion was not numeric")
    return (value as Number).toLong()
}

private fun expectedPlatform(): String {
    val os = System.getProperty("os.name", "").lowercase()
    return when {
        "mac" in os || "darwin" in os -> "apple"
        "win" in os -> "windows"
        else -> "linux"
    }
}

fun main() {
    val restoredOnLaunch = System.getenv("MOSAIC_EXPECT_RESTORED") == "1"
    val expectWarning = System.getenv("MOSAIC_EXPECT_PERSISTENCE_WARNING") == "1"
    val initialCount = if (restoredOnLaunch) 4L else 0L
    val host = checkNotNull(MosaicRuntimeHost.load()) {
        "standard Compose binding did not load the Rust app"
    }
    try {
        val started = objectMap(host.props(), "startup update")
        val startedProps = props(started, "startup update")
        requireConformance(
            ("persistenceWarning" in started) == expectWarning,
            "startup persistence warning",
        )
        requireConformance(integer(started["revision"], "startup revision") == 1L,
            "startup revision")
        requireConformance(integer(startedProps["count"], "initial count") == initialCount,
            "initial count")
        requireConformance(startedProps["platform"] == expectedPlatform(), "startup platform")
        requireConformance(
            startedProps["status"] == if (restoredOnLaunch) "restored" else "started",
            "startup status",
        )

        var notificationCount = 0
        host.setPropsChangedHandler { notificationCount += 1 }

        val dispatched = objectMap(
            host.handleEvent(
                mapOf(
                    "event" to "increment",
                    "amount" to 4,
                ),
            ),
            "dispatch update",
        )
        val dispatchedProps = props(dispatched, "dispatch update")
        requireConformance(integer(dispatched["revision"], "dispatch revision") == 2L,
            "dispatch revision")
        requireConformance(integer(dispatchedProps["count"], "dispatched count") == initialCount + 4L,
            "dispatched count")
        requireConformance(dispatchedProps["status"] == "dispatched", "dispatch status")
        requireConformance(notificationCount == 1, "dispatch props-change notification")

        val snapshot = objectMap(host.snapshot(), "snapshot")
        requireConformance(snapshot["schema"] == "mosaic-app-conformance/counter",
            "snapshot schema")
        requireConformance(integer(snapshot["version"], "snapshot version") == 1L,
            "snapshot version")
        requireConformance((snapshot["bytes"] as? List<*>)?.size == 8, "snapshot bytes")

        val restored = objectMap(host.restore(snapshot), "restore update")
        val restoredProps = props(restored, "restore update")
        requireConformance(integer(restored["revision"], "restore revision") == 3L,
            "restore revision")
        requireConformance(integer(restoredProps["count"], "restored count") == initialCount + 4L,
            "restored count")
        requireConformance(restoredProps["status"] == "restored", "restore status")
        requireConformance(notificationCount == 2, "restore props-change notification")

        // UI48 ENV4 (§7.4). The conformance app does not react to its
        // environment, so the runtime answers with the current revision and no
        // props -- and the host keeps showing what it showed.
        val compact = MosaicRuntimeHost.initialEnvironment() + mapOf(
            "colorScheme" to "light",
            "sizeClass" to "compact",
            "orientation" to "portrait",
        )
        val reported = objectMap(host.reportEnvironment(compact), "environment report")
        requireConformance(integer(reported["revision"], "environment revision") == 3L,
            "an ignored environment keeps the revision")
        requireConformance(props(reported, "environment report") == restoredProps,
            "an ignored environment keeps the props")
        requireConformance(props(objectMap(host.props(), "props after report"), "props after report")
            == restoredProps, "the host still shows the same props")
        requireConformance(host.reportEnvironment(compact) == null,
            "an unchanged environment is not sent again")
        val invalid = compact + ("sizeClass" to "enormous")
        val refused = host.reportEnvironment(invalid)
        requireConformance(refused?.containsKey("error") == true,
            "an invalid environment is refused")
        requireConformance(host.reportEnvironment(invalid) == null,
            "the same refused report is not resent")
        requireConformance(props(objectMap(host.props(), "props after refusal"), "props after refusal")
            == restoredProps, "a refused environment leaves the props")
        val expanded = compact + ("sizeClass" to "expanded") + ("orientation" to "landscape")
        requireConformance(host.reportEnvironment(expanded) != null,
            "a changed size class is sent")
        requireConformance(host.reportEnvironment(compact) != null,
            "a refused report was not remembered, and going back is a change")

        // Only an invalid report is held back. While failEnvironment is on,
        // every report that reaches the app is an app error -- a failure that
        // is not the report's fault -- so an error answer proves a report was
        // sent and null that it was held back.
        fun failEnvironment(fail: Boolean, assertion: String): Map<String, Any?> = objectMap(
            host.handleEvent(mapOf("name" to "failEnvironment", "payload" to mapOf("fail" to fail))),
            assertion,
        )
        failEnvironment(true, "the app is told to fail environment changes")
        requireConformance(host.reportEnvironment(compact) == null,
            "the report last taken is still held back")
        val tablet = compact + ("sizeClass" to "regular")
        requireConformance(
            (host.reportEnvironment(tablet)?.get("error") as? String)
                ?.startsWith("Mosaic application error") == true,
            "an app's failure on a report is an error answer",
        )
        requireConformance(host.reportEnvironment(tablet)?.containsKey("error") == true,
            "a report that failed transiently is sent again")
        requireConformance(
            host.reportEnvironment(invalid)?.containsKey("error") == true &&
                host.reportEnvironment(invalid) == null,
            "an invalid report is still held back after one refusal",
        )
        requireConformance(host.reportEnvironment(tablet)?.containsKey("error") == true,
            "holding back an invalid report holds back nothing else")
        failEnvironment(false, "the app is told to take environment changes again")

        // An ignored report writes no state: nothing the app saves changed.
        val statePath = System.getenv("MOSAIC_APP_STATE_PATH")
        if (statePath.isNullOrBlank()) {
            println("MOSAIC_APP_STATE_PATH unset: skipped the checks that an ignored report writes no state")
        } else {
            val state = File(statePath)
            requireConformance(state.isFile, "the state persists")
            requireConformance(state.delete(), "the state file removed")
            val revisionBefore = integer(objectMap(host.props(), "props before retry")["revision"], "revision")
            val retried = objectMap(host.reportEnvironment(tablet), "retried report")
            requireConformance(integer(retried["revision"], "retried revision") == revisionBefore,
                "once the failure passes, the same report is taken")
            requireConformance(!state.exists(), "an ignored report does not rewrite the state file")
            // With the state path a (non-empty) directory, any write fails and
            // says so. With no failed save pending, an ignored report
            // attempts none, so it raises no warning; an event still does, on
            // its own answer. Once a save has failed, the next ignored report
            // retries it -- so a kill before the next event does not lose that
            // revision -- and its answer carries the warning's clearing.
            requireConformance(File(state, "occupied").let { state.mkdirs() && it.createNewFile() },
                "the state path made unwritable")
            requireConformance(
                "persistenceWarning" !in objectMap(host.reportEnvironment(compact), "report while unwritable"),
                "an ignored report raises no persistence warning",
            )
            requireConformance("persistenceWarning" in failEnvironment(false, "an event while saving fails"),
                "an event that cannot persist surfaces the warning at once")
            requireConformance(
                "persistenceWarning" in objectMap(host.reportEnvironment(tablet), "report while still failing"),
                "an ignored report while saving still fails keeps the warning",
            )
            requireConformance(state.deleteRecursively(), "the state path made writable again")
            val cleared = objectMap(host.reportEnvironment(compact), "report after the path is writable")
            requireConformance(state.isFile && "persistenceWarning" !in cleared,
                "an ignored report retries a failed save and clears the warning")
            failEnvironment(true, "the app is told to fail environment changes once more")
            requireConformance(host.reportEnvironment(compact) == null,
                "the report that retried the save is held back")
            failEnvironment(false, "the app is left taking environment changes")
        }
        requireConformance(MosaicRuntimeHost.initialEnvironment()["pointer"] == "fine" &&
            MosaicRuntimeHost.initialEnvironment()["hover"] == "hover",
            "a desktop start context carries a fine, hovering pointer")
    } finally {
        host.close()
    }

    println("Mosaic Compose Rust runtime conformance passed")
}
