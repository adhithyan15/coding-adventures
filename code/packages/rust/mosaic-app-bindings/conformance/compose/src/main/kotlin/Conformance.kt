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
        val refused = host.reportEnvironment(compact + ("sizeClass" to "enormous"))
        requireConformance(refused?.containsKey("error") == true,
            "an invalid environment is refused")
        requireConformance(props(objectMap(host.props(), "props after refusal"), "props after refusal")
            == restoredProps, "a refused environment leaves the props")
        val expanded = compact + ("sizeClass" to "expanded") + ("orientation" to "landscape")
        requireConformance(host.reportEnvironment(expanded) != null,
            "a changed size class is sent")
        requireConformance(host.reportEnvironment(compact) != null,
            "a refused report was not remembered, and going back is a change")
        requireConformance(MosaicRuntimeHost.initialEnvironment()["pointer"] == "fine" &&
            MosaicRuntimeHost.initialEnvironment()["hover"] == "hover",
            "a desktop start context carries a fine, hovering pointer")
    } finally {
        host.close()
    }

    println("Mosaic Compose Rust runtime conformance passed")
}
