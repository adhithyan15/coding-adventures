package codingadventures.unixhistory

/** Immutable inode contents with insertion-ordered directory entries. */
sealed interface Node
class DirectoryNode(entries: Map<String, Int>) : Node {
    val entries: Map<String, Int> = java.util.Collections.unmodifiableMap(LinkedHashMap(entries))
}
class FileNode(bytes: ByteArray) : Node {
    val bytes: ByteArray = bytes.copyOf()
}
class FsError(val code: String) : RuntimeException(code)

class FileSystem(nodes: Map<Int, Node>) {
    private val nodes = nodes.toMap()

    companion object {
        fun demo() = FileSystem(linkedMapOf(
            1 to DirectoryNode(linkedMapOf("hello" to 2, "empty" to 3, "etc" to 4)),
            2 to FileNode("Hello, Unix!\n".toByteArray(Charsets.UTF_8)),
            3 to FileNode(byteArrayOf()),
            4 to DirectoryNode(linkedMapOf("notes" to 5)),
            5 to FileNode("A directory maps a name to an inode.\nAn open descriptor tracks a byte offset.\n".toByteArray(Charsets.UTF_8)),
        ))
    }

    fun resolve(path: String): Pair<Int, Node> {
        if (path.isEmpty() || '\u0000' in path) throw FsError("EINVAL")
        var inode = 1
        val parents = mutableListOf<Int>()
        for (part in path.split('/').filter { it.isNotEmpty() }) {
            val directory = nodes[inode] ?: throw FsError("ENOENT")
            if (directory !is DirectoryNode) throw FsError("ENOTDIR")
            when (part) {
                "." -> Unit
                ".." -> inode = if (parents.isEmpty()) 1 else parents.removeAt(parents.lastIndex)
                else -> {
                    val child = directory.entries[part] ?: throw FsError("ENOENT")
                    parents.add(inode)
                    inode = child
                }
            }
        }
        val node = nodes[inode] ?: throw FsError("ENOENT")
        if (path.endsWith('/') && node is FileNode) throw FsError("ENOTDIR")
        return inode to node
    }

    fun read(inode: Int, offset: Int, count: Int): ByteArray {
        val node = nodes[inode] ?: throw FsError("ENOENT")
        if (node !is FileNode) throw FsError("EISDIR")
        if (offset < 0 || count < 0) throw FsError("EINVAL")
        val start = offset.coerceAtMost(node.bytes.size)
        return node.bytes.copyOfRange(start, start + count.coerceAtMost(node.bytes.size - start))
    }

    fun names(inode: Int): List<String> {
        val node = nodes[inode] ?: throw FsError("ENOENT")
        if (node !is DirectoryNode) throw FsError("ENOTDIR")
        return node.entries.keys.toList()
    }
}

data class Descriptor(val inode: Int, val offset: Long)
class DescriptorTable(capacity: Int) {
    private val slots = MutableList<Descriptor?>(capacity.also { require(it >= 0) }) { null }
    fun get(fd: Int): Descriptor? = slots.getOrNull(fd)
    fun open(inode: Int): Int? {
        val fd = slots.indexOfFirst { it == null }
        if (fd < 0) return null
        slots[fd] = Descriptor(inode, 0)
        return fd
    }
    fun advance(fd: Int, count: Int): Boolean {
        val slot = get(fd) ?: return false
        if (count < 0 || Long.MAX_VALUE - slot.offset < count) return false
        slots[fd] = slot.copy(offset = slot.offset + count)
        return true
    }
    fun close(fd: Int): Boolean {
        if (get(fd) == null) return false
        slots[fd] = null
        return true
    }
}

class Kernel(private val files: FileSystem, val descriptors: DescriptorTable) {
    fun open(path: String): Map<String, Any> = try {
        val (inode, node) = files.resolve(path)
        if (node is DirectoryNode) throw FsError("EISDIR")
        val fd = descriptors.open(inode) ?: throw FsError("EMFILE")
        linkedMapOf("call" to "open", "path" to path, "fd" to fd, "inode" to inode, "offset" to 0)
    } catch (error: FsError) {
        linkedMapOf("call" to "open", "path" to path, "error" to error.code)
    }

    fun read(fd: Int, count: Int): Pair<Map<String, Any>, ByteArray?> {
        val slot = descriptors.get(fd) ?: return linkedMapOf("call" to "read", "fd" to fd, "error" to "EBADF") to null
        return try {
            val data = files.read(slot.inode, slot.offset.coerceAtMost(Int.MAX_VALUE.toLong()).toInt(), count)
            descriptors.advance(fd, data.size)
            linkedMapOf("call" to "read", "fd" to fd, "inode" to slot.inode, "offset" to slot.offset,
                "bytes" to data.size, "nextOffset" to slot.offset + data.size) to data
        } catch (error: FsError) {
            linkedMapOf("call" to "read", "fd" to fd, "error" to error.code) to null
        }
    }

    fun close(fd: Int): Map<String, Any> = linkedMapOf<String, Any>("call" to "close", "fd" to fd).also {
        if (!descriptors.close(fd)) it["error"] = "EBADF"
    }

    fun list(path: String): Map<String, Any> = try {
        val (inode, _) = files.resolve(path)
        linkedMapOf("call" to "list", "path" to path, "inode" to inode, "names" to files.names(inode))
    } catch (error: FsError) {
        linkedMapOf("call" to "list", "path" to path, "error" to error.code)
    }
}

fun runCommand(command: String): Map<String, Any> {
    val kernel = Kernel(FileSystem.demo(), DescriptorTable(4))
    val events = mutableListOf<Map<String, Any>>()
    val output = StringBuilder()
    val words = command.trim().split(Regex("[\\t\\n\\u000B\\f\\r ]+")).filter { it.isNotEmpty() }
    fun record(event: Map<String, Any>) {
        events.add(event)
        event["error"]?.let { output.append("error: $it\n") }
    }
    if (words.firstOrNull() == "cat" && words.size == 2) {
        val opened = kernel.open(words[1]); record(opened)
        (opened["fd"] as? Int)?.let { fd ->
            while (true) {
                val (event, data) = kernel.read(fd, 8); record(event)
                if (data == null || data.isEmpty()) break
                output.append(data.toString(Charsets.UTF_8))
            }
            record(kernel.close(fd))
        }
    } else if (words.firstOrNull() == "ls" && words.size <= 2 && words.isNotEmpty()) {
        val listed = kernel.list(words.getOrElse(1) { "/" }); record(listed)
        @Suppress("UNCHECKED_CAST")
        (listed["names"] as? List<String>)?.forEach { output.append(it).append('\n') }
    } else if (words.isNotEmpty()) output.append("usage: cat PATH or ls [PATH]\n")
    return linkedMapOf("output" to output.toString(), "events" to events)
}
