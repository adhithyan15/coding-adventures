"""Cache — configurable CPU cache hierarchy simulator.

This package simulates a multi-level cache hierarchy like those found in
modern CPUs. The same Cache class serves as L1, L2, or L3 by configuring
size, associativity, and latency differently.

Modules:
    cache_line  - CacheLine: the smallest unit of cached data
    cache_set   - CacheSet + CacheConfig: set-associative lookup with LRU
    cache       - Cache: a single configurable cache level
    hierarchy   - CacheHierarchy: L1I/L1D/L2/L3 composition
    stats       - CacheStats: hit rate, miss rate, eviction tracking

Quick start:
    >>> from cache import Cache, CacheConfig, CacheHierarchy
    >>> l1d = Cache(CacheConfig("L1D", 1024, 64, 4, 1))
    >>> l2 = Cache(CacheConfig("L2", 4096, 64, 8, 10))
    >>> hierarchy = CacheHierarchy(l1d=l1d, l2=l2)
    >>> result = hierarchy.read(0x1000)
    >>> result.served_by
    'memory'
"""

from cache.cache import Cache, CacheAccess
from cache.cache_line import CacheLine
from cache.cache_set import CacheConfig, CacheSet
from cache.hierarchy import CacheHierarchy, HierarchyAccess
from cache.stats import CacheStats

__all__ = [
    "Cache",
    "CacheAccess",
    "CacheConfig",
    "CacheHierarchy",
    "CacheLine",
    "CacheSet",
    "HierarchyAccess",
    "CacheStats",
]
