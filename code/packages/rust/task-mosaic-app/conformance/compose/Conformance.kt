interface MosaicComposeHost : AutoCloseable {
    fun props(): Map<String, Any?>?
    fun handleEvent(event: Map<String, Any?>): Map<String, Any?>?
    fun setPropsChangedHandler(handler: (() -> Unit)?) {}
    override fun close() {}
}

private const val TASK_NAME = "Native acceptance task"
private const val PERSISTED_TASK_NAME = "Persisted native task"
private const val DUE = "2026-01-09"
private const val SCHEDULE = "2026-01-05 → 2026-01-05"

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

private fun props(update: Any?, assertion: String): Map<String, Any?> =
    objectMap(objectMap(update, assertion)["props"], "$assertion props")

private fun rows(props: Map<String, Any?>, assertion: String): List<List<Any?>> {
    val value = props["task-rows"]
    requireConformance(value is List<*>, "$assertion task-rows was not a list")
    return (value as List<*>).map { row ->
        requireConformance(row is List<*>, "$assertion task row was not a list")
        (row as List<*>).toList()
    }
}

private fun dispatch(
    host: MosaicComposeHost,
    name: String,
    payload: Map<String, Any?> = emptyMap(),
): Map<String, Any?> = props(
    host.handleEvent(mapOf("name" to name, "payload" to payload)),
    name,
)

private fun requireTask(rows: List<List<Any?>>, name: String) {
    requireConformance(rows.size == 1, "one task row")
    val row = rows.single()
    requireConformance(row.size >= 4, "task row projection width")
    requireConformance(row[1] == name, "task name projection")
    requireConformance(row[2] == "due $DUE", "task due projection")
    requireConformance(row[3] == SCHEDULE, "Rust schedule start/finish projection")
}

fun main() {
    val restoredOnLaunch = System.getenv("MOSAIC_EXPECT_RESTORED") == "1"
    val host = checkNotNull(MosaicRuntimeHost.load()) {
        "standard Compose binding did not load the TaskApp Rust runtime"
    }
    try {
        var current = props(host.props(), "startup update")
        if (restoredOnLaunch) {
            requireTask(rows(current, "restored startup"), PERSISTED_TASK_NAME)
            current = dispatch(host, "onDeleteTask", mapOf("index" to 0))
            requireConformance(rows(current, "restored delete").isEmpty(), "delete restored task")
            println("TaskApp Compose persisted-restart conformance passed")
            return
        }

        requireConformance(rows(current, "fresh startup").isEmpty(), "fresh task list")
        val before = host.snapshot()
        val rejected = runCatching {
            host.handleEvent(
                mapOf(
                    "name" to "onNewTaskNameChange",
                    "payload" to mapOf("value" to 7),
                ),
            )
        }.isFailure
        requireConformance(rejected, "invalid input rejected")
        requireConformance(host.snapshot() == before, "invalid input preserved state")

        dispatch(host, "onNewTaskNameChange", mapOf("value" to TASK_NAME))
        dispatch(host, "onNewTaskDueChange", mapOf("value" to DUE))
        current = dispatch(host, "onAddTask")
        requireConformance(rows(current, "created task").single()[3] == "", "Board mode hides schedule")
        current = dispatch(host, "onToggleProjectComplexity")
        requireTask(rows(current, "created task"), TASK_NAME)

        current = dispatch(host, "onToggleTask", mapOf("index" to 0))
        requireConformance(rows(current, "completed task").single()[0] == "✓", "complete task")
        requireConformance(current["ring-percent"] == "100%", "completion projection")
        current = dispatch(host, "onToggleTask", mapOf("index" to 0))
        requireConformance(rows(current, "reopened task").single()[0] == "○", "reopen task")
        current = dispatch(host, "onDeleteTask", mapOf("index" to 0))
        requireConformance(rows(current, "deleted task").isEmpty(), "delete task")

        dispatch(host, "onNewTaskNameChange", mapOf("value" to PERSISTED_TASK_NAME))
        dispatch(host, "onNewTaskDueChange", mapOf("value" to DUE))
        current = dispatch(host, "onAddTask")
        requireTask(rows(current, "persisted task"), PERSISTED_TASK_NAME)
    } finally {
        host.close()
    }

    println("TaskApp Compose native lifecycle conformance passed")
}
