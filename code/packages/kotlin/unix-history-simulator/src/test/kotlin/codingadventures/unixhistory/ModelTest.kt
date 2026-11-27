package codingadventures.unixhistory

import org.junit.jupiter.api.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNull
import kotlin.test.assertTrue

class ModelTest {
    @Test
    fun `resolves paths and rejects traversal through a regular file`() {
        val files = FileSystem.demo()
        assertEquals(1, files.resolve("//etc/../").first)
        assertEquals(listOf("notes"), files.names(4))
        val error = runCatching { files.resolve("/hello/child") }.exceptionOrNull()
        assertTrue(error is FsError)
        assertEquals("ENOTDIR", (error as FsError).code)
    }

    @Test
    fun `allocates the lowest descriptor with independent offsets`() {
        val table = DescriptorTable(2)
        val first = table.open(2)
        val second = table.open(2)
        assertEquals(0, first)
        assertEquals(1, second)
        assertNull(table.open(3))
        assertTrue(table.advance(first!!, 3))
        assertEquals(3, table.get(first)?.offset)
        assertEquals(0, table.get(second!!)?.offset)
        assertTrue(table.close(first))
        assertEquals(first, table.open(3))
        assertTrue(table.close(first))
        assertFalse(table.close(first))
    }

    @Test
    fun `kernel methods preserve failure state and expose syscall-shaped events`() {
        val kernel = Kernel(FileSystem.demo(), DescriptorTable(1))
        val opened = kernel.open("/hello")
        assertEquals(0, opened["fd"])
        assertEquals("EMFILE", kernel.open("/hello")["error"])
        val (read, bytes) = kernel.read(0, 5)
        assertEquals(5, read["bytes"])
        assertEquals("Hello", bytes!!.toString(Charsets.UTF_8))
        assertEquals(5, kernel.descriptors.get(0)?.offset)
        assertEquals("EBADF", kernel.read(9, 1).first["error"])
        assertEquals("EBADF", kernel.close(9)["error"])
        assertNull(kernel.close(0)["error"])
        assertEquals("EBADF", kernel.close(0)["error"])
        assertEquals("EISDIR", kernel.open("/")["error"])
        assertEquals("ENOTDIR", kernel.list("/hello")["error"])
    }
}
