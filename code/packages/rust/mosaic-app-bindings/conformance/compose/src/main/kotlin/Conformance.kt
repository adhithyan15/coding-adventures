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
    val host = checkNotNull(MosaicRuntimeHost.load()) {
        "standard Compose binding did not load the Rust app"
    }
    try {
        val started = objectMap(host.props(), "startup update")
        val startedProps = props(started, "startup update")
        requireConformance(integer(started["revision"], "startup revision") == 1L,
            "startup revision")
        requireConformance(integer(startedProps["count"], "initial count") == 0L,
            "initial count")
        requireConformance(startedProps["platform"] == expectedPlatform(), "startup platform")
        requireConformance(startedProps["status"] == "started", "startup status")

        var notificationCount = 0
        host.setPropsChangedHandler { notificationCount += 1 }

        val dispatched = objectMap(
            host.handleEvent(
                mapOf(
                    "name" to "increment",
                    "payload" to mapOf("amount" to 4),
                ),
            ),
            "dispatch update",
        )
        val dispatchedProps = props(dispatched, "dispatch update")
        requireConformance(integer(dispatched["revision"], "dispatch revision") == 2L,
            "dispatch revision")
        requireConformance(integer(dispatchedProps["count"], "dispatched count") == 4L,
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
        requireConformance(integer(restoredProps["count"], "restored count") == 4L,
            "restored count")
        requireConformance(restoredProps["status"] == "restored", "restore status")
        requireConformance(notificationCount == 2, "restore props-change notification")
    } finally {
        host.close()
    }

    println("Mosaic Compose Rust runtime conformance passed")
}
