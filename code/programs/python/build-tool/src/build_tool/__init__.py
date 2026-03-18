"""
Build Tool -- Incremental, Parallel Monorepo Build System
=========================================================

A CLI tool that discovers packages in a monorepo via DIRS/BUILD files, resolves
their dependencies, hashes source files, and only rebuilds packages whose source
(or dependency source) has changed. Independent packages are built in parallel.
"""

__version__ = "0.1.0"
