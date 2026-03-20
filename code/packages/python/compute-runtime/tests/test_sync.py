"""Tests for synchronization primitives — Fence, Semaphore, Event."""

import pytest

from compute_runtime import Fence, Semaphore, Event


class TestFence:
    def test_default_unsignaled(self) -> None:
        fence = Fence()
        assert not fence.signaled

    def test_create_signaled(self) -> None:
        fence = Fence(signaled=True)
        assert fence.signaled

    def test_signal(self) -> None:
        fence = Fence()
        fence.signal()
        assert fence.signaled

    def test_wait_signaled(self) -> None:
        fence = Fence(signaled=True)
        result = fence.wait()
        assert result is True

    def test_wait_unsignaled(self) -> None:
        fence = Fence()
        result = fence.wait()
        assert result is False

    def test_reset(self) -> None:
        fence = Fence(signaled=True)
        fence.reset()
        assert not fence.signaled

    def test_reuse(self) -> None:
        fence = Fence()
        fence.signal()
        assert fence.signaled
        fence.reset()
        assert not fence.signaled
        fence.signal()
        assert fence.signaled

    def test_unique_ids(self) -> None:
        f1 = Fence()
        f2 = Fence()
        assert f1.fence_id != f2.fence_id

    def test_wait_cycles(self) -> None:
        fence = Fence()
        assert fence.wait_cycles == 0

    def test_reset_clears_wait_cycles(self) -> None:
        fence = Fence()
        fence.reset()
        assert fence.wait_cycles == 0


class TestSemaphore:
    def test_default_unsignaled(self) -> None:
        sem = Semaphore()
        assert not sem.signaled

    def test_signal(self) -> None:
        sem = Semaphore()
        sem.signal()
        assert sem.signaled

    def test_reset(self) -> None:
        sem = Semaphore()
        sem.signal()
        sem.reset()
        assert not sem.signaled

    def test_unique_ids(self) -> None:
        s1 = Semaphore()
        s2 = Semaphore()
        assert s1.semaphore_id != s2.semaphore_id


class TestEvent:
    def test_default_unsignaled(self) -> None:
        event = Event()
        assert not event.signaled

    def test_set(self) -> None:
        event = Event()
        event.set()
        assert event.signaled

    def test_reset(self) -> None:
        event = Event()
        event.set()
        event.reset()
        assert not event.signaled

    def test_status(self) -> None:
        event = Event()
        assert event.status() is False
        event.set()
        assert event.status() is True

    def test_unique_ids(self) -> None:
        e1 = Event()
        e2 = Event()
        assert e1.event_id != e2.event_id
