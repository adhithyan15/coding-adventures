"""
hyperloglog — HyperLogLog approximate cardinality estimation.

HyperLogLog (Flajolet, Fusy, Gandouet, Meunier — 2007) estimates the number
of distinct elements in a data stream using a tiny, fixed amount of memory.

The Redis PFADD / PFCOUNT / PFMERGE commands implement this algorithm.
At precision=14 (the Redis default), memory is ~12 KB with ±0.81% error.

Quick start:

    >>> from hyperloglog import HyperLogLog
    >>> hll = HyperLogLog(precision=14)
    >>> for item in ["alice", "bob", "alice", "carol"]:
    ...     hll.add(item)
    >>> hll.count()  # approximately 3
    3

Merging two sketches (union, not intersection):

    >>> hll1 = HyperLogLog()
    >>> hll2 = HyperLogLog()
    >>> for i in range(1000): hll1.add(f"a_{i}")
    >>> for i in range(1000): hll2.add(f"b_{i}")
    >>> hll1.merge(hll2).count()  # approximately 2000
"""

from hyperloglog.hyperloglog import HyperLogLog

__all__ = ["HyperLogLog"]
__version__ = "0.1.0"
