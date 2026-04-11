"""
bloom_filter — Space-efficient probabilistic set membership filter.

A Bloom filter answers "Have I seen this element before?" with guaranteed
correctness in one direction and tunable accuracy in the other:

  "Definitely NOT in set" — zero false negatives. If the filter says NO,
                             trust it completely.

  "Probably in set"       — small, tunable false positive rate. If the filter
                             says YES, it is usually correct but occasionally
                             wrong (this is fine for pre-flight checks).

Quick start:

    >>> from bloom_filter import BloomFilter
    >>> bf = BloomFilter(expected_items=1000, false_positive_rate=0.01)
    >>> bf.add("hello")
    >>> "hello" in bf
    True
    >>> "world" in bf
    False

See bloom_filter.py for full algorithm documentation, diagrams, and the math.
"""

from bloom_filter.bloom_filter import BloomFilter

__all__ = ["BloomFilter"]
