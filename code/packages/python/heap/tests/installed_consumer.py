"""Exercise the wheel from an environment outside the source package."""

from __future__ import annotations

import operator
import sysconfig
from pathlib import Path

import heap
from heap import BoundedHeap, HeapFullError

installed_at = Path(sysconfig.get_paths()["purelib"]).resolve()
module_at = Path(heap.__file__).resolve()
assert module_at.is_relative_to(installed_at), (module_at, installed_at)
assert (module_at.parent / "py.typed").is_file()

queue = BoundedHeap.from_iterable([7, 2, 9], 3, operator.lt)
assert len(queue) == 3
assert queue.capacity == 3
assert queue.peek() == 2
assert queue.pop() == 2
queue.push(1)
assert [queue.pop() for _ in range(len(queue))] == [1, 7, 9]

full = BoundedHeap(1, operator.lt)
full.push(1)
try:
    full.push(0)
except HeapFullError:
    pass
else:
    raise AssertionError("full queue admitted an extra value")
