"""Tests for IPCManager -- the central IPC coordinator.

These tests verify:
1. Pipe creation, retrieval, close, and destruction
2. Message queue creation, retrieval, and deletion
3. Shared memory creation, retrieval, and deletion
4. List operations (list_pipes, list_message_queues, list_shared_regions)
5. Idempotent creation (creating the same name returns the existing object)
6. Non-existent resource handling
"""

from ipc import IPCManager

# ========================================================================
# Pipe management
# ========================================================================


class TestIPCManagerPipes:
    """Test pipe lifecycle through the IPCManager."""

    def test_create_pipe(self) -> None:
        """create_pipe returns (pipe_id, read_fd, write_fd)."""
        mgr = IPCManager()
        pipe_id, read_fd, write_fd = mgr.create_pipe()
        assert isinstance(pipe_id, int)
        assert isinstance(read_fd, int)
        assert isinstance(write_fd, int)
        # FDs start at 3 (0/1/2 are stdin/stdout/stderr)
        assert read_fd >= 3
        assert write_fd >= 3
        assert read_fd != write_fd

    def test_get_pipe(self) -> None:
        mgr = IPCManager()
        pipe_id, _, _ = mgr.create_pipe()
        pipe = mgr.get_pipe(pipe_id)
        assert pipe is not None

    def test_get_pipe_not_found(self) -> None:
        mgr = IPCManager()
        assert mgr.get_pipe(999) is None

    def test_pipe_write_read_through_manager(self) -> None:
        """Create a pipe, write through it, read back."""
        mgr = IPCManager()
        pipe_id, _, _ = mgr.create_pipe()
        pipe = mgr.get_pipe(pipe_id)
        assert pipe is not None
        pipe.write(b"through the manager")
        assert pipe.read(19) == b"through the manager"

    def test_create_pipe_custom_capacity(self) -> None:
        mgr = IPCManager()
        pipe_id, _, _ = mgr.create_pipe(capacity=128)
        pipe = mgr.get_pipe(pipe_id)
        assert pipe is not None
        assert pipe.capacity == 128

    def test_close_pipe_read(self) -> None:
        """Closing the read end via manager makes writes raise BrokenPipeError."""
        mgr = IPCManager()
        pipe_id, _, _ = mgr.create_pipe()
        mgr.close_pipe_read(pipe_id)
        pipe = mgr.get_pipe(pipe_id)
        assert pipe is not None
        import pytest

        with pytest.raises(BrokenPipeError):
            pipe.write(b"broken")

    def test_close_pipe_write(self) -> None:
        """Closing the write end via manager puts pipe in EOF state."""
        mgr = IPCManager()
        pipe_id, _, _ = mgr.create_pipe()
        mgr.close_pipe_write(pipe_id)
        pipe = mgr.get_pipe(pipe_id)
        assert pipe is not None
        assert pipe.is_eof

    def test_close_pipe_nonexistent(self) -> None:
        """Closing a non-existent pipe does nothing (no crash)."""
        mgr = IPCManager()
        mgr.close_pipe_read(999)  # should not raise
        mgr.close_pipe_write(999)

    def test_destroy_pipe(self) -> None:
        mgr = IPCManager()
        pipe_id, _, _ = mgr.create_pipe()
        assert mgr.destroy_pipe(pipe_id) is True
        assert mgr.get_pipe(pipe_id) is None

    def test_destroy_pipe_nonexistent(self) -> None:
        mgr = IPCManager()
        assert mgr.destroy_pipe(999) is False

    def test_multiple_pipes(self) -> None:
        """Create multiple pipes, each gets a unique ID."""
        mgr = IPCManager()
        id1, _, _ = mgr.create_pipe()
        id2, _, _ = mgr.create_pipe()
        id3, _, _ = mgr.create_pipe()
        assert id1 != id2 != id3
        assert len(mgr.list_pipes()) == 3

    def test_unique_fds(self) -> None:
        """Each pipe creation allocates unique file descriptors."""
        mgr = IPCManager()
        _, r1, w1 = mgr.create_pipe()
        _, r2, w2 = mgr.create_pipe()
        fds = {r1, w1, r2, w2}
        assert len(fds) == 4  # all unique


# ========================================================================
# Message queue management
# ========================================================================


class TestIPCManagerMessageQueues:
    """Test message queue lifecycle through the IPCManager."""

    def test_create_message_queue(self) -> None:
        mgr = IPCManager()
        mq = mgr.create_message_queue("work")
        assert mq is not None

    def test_get_message_queue(self) -> None:
        mgr = IPCManager()
        mgr.create_message_queue("work")
        mq = mgr.get_message_queue("work")
        assert mq is not None

    def test_get_message_queue_not_found(self) -> None:
        mgr = IPCManager()
        assert mgr.get_message_queue("nope") is None

    def test_idempotent_create(self) -> None:
        """Creating a queue with the same name returns the same object."""
        mgr = IPCManager()
        mq1 = mgr.create_message_queue("work")
        mq2 = mgr.create_message_queue("work")
        assert mq1 is mq2

    def test_send_receive_through_manager(self) -> None:
        mgr = IPCManager()
        mq = mgr.create_message_queue("tasks")
        mq.send(1, b"do this")
        result = mq.receive()
        assert result == (1, b"do this")

    def test_create_with_custom_limits(self) -> None:
        mgr = IPCManager()
        mq = mgr.create_message_queue("small", max_messages=5, max_message_size=32)
        assert mq.max_messages == 5
        assert mq.max_message_size == 32

    def test_delete_message_queue(self) -> None:
        mgr = IPCManager()
        mgr.create_message_queue("work")
        assert mgr.delete_message_queue("work") is True
        assert mgr.get_message_queue("work") is None

    def test_delete_nonexistent_queue(self) -> None:
        mgr = IPCManager()
        assert mgr.delete_message_queue("nope") is False


# ========================================================================
# Shared memory management
# ========================================================================


class TestIPCManagerSharedMemory:
    """Test shared memory lifecycle through the IPCManager."""

    def test_create_shared_memory(self) -> None:
        mgr = IPCManager()
        region = mgr.create_shared_memory("cache", size=4096, owner_pid=1)
        assert region is not None
        assert region.name == "cache"
        assert region.size == 4096

    def test_get_shared_memory(self) -> None:
        mgr = IPCManager()
        mgr.create_shared_memory("cache", size=4096)
        region = mgr.get_shared_memory("cache")
        assert region is not None

    def test_get_shared_memory_not_found(self) -> None:
        mgr = IPCManager()
        assert mgr.get_shared_memory("nope") is None

    def test_idempotent_create(self) -> None:
        """Creating shared memory with the same name returns the same object."""
        mgr = IPCManager()
        r1 = mgr.create_shared_memory("buf", size=1024)
        r2 = mgr.create_shared_memory("buf", size=2048)  # different size ignored
        assert r1 is r2
        assert r1.size == 1024  # original size preserved

    def test_write_read_through_manager(self) -> None:
        mgr = IPCManager()
        region = mgr.create_shared_memory("data", size=256, owner_pid=1)
        region.write(0, b"managed")
        assert region.read(0, 7) == b"managed"

    def test_delete_shared_memory(self) -> None:
        mgr = IPCManager()
        mgr.create_shared_memory("cache", size=4096)
        assert mgr.delete_shared_memory("cache") is True
        assert mgr.get_shared_memory("cache") is None

    def test_delete_nonexistent_region(self) -> None:
        mgr = IPCManager()
        assert mgr.delete_shared_memory("nope") is False


# ========================================================================
# List operations
# ========================================================================


class TestIPCManagerLists:
    """Test list_pipes, list_message_queues, list_shared_regions."""

    def test_list_pipes_empty(self) -> None:
        mgr = IPCManager()
        assert mgr.list_pipes() == []

    def test_list_pipes(self) -> None:
        mgr = IPCManager()
        id1, _, _ = mgr.create_pipe()
        id2, _, _ = mgr.create_pipe()
        pipes = mgr.list_pipes()
        assert id1 in pipes
        assert id2 in pipes

    def test_list_message_queues_empty(self) -> None:
        mgr = IPCManager()
        assert mgr.list_message_queues() == []

    def test_list_message_queues(self) -> None:
        mgr = IPCManager()
        mgr.create_message_queue("a")
        mgr.create_message_queue("b")
        queues = mgr.list_message_queues()
        assert "a" in queues
        assert "b" in queues

    def test_list_shared_regions_empty(self) -> None:
        mgr = IPCManager()
        assert mgr.list_shared_regions() == []

    def test_list_shared_regions(self) -> None:
        mgr = IPCManager()
        mgr.create_shared_memory("x", size=128)
        mgr.create_shared_memory("y", size=256)
        regions = mgr.list_shared_regions()
        assert "x" in regions
        assert "y" in regions

    def test_list_after_deletion(self) -> None:
        """After deleting an item, it no longer appears in the list."""
        mgr = IPCManager()
        mgr.create_message_queue("keep")
        mgr.create_message_queue("remove")
        mgr.delete_message_queue("remove")
        assert mgr.list_message_queues() == ["keep"]
