"""ca-capability-analyzer — Static analysis for OS capability detection.

This package walks the Python AST to detect what OS-level capabilities
(filesystem, network, process, environment, FFI) a source file uses.
It compares detected capabilities against a package's
required_capabilities.json manifest and reports violations.

The analyzer serves two roles in the capability security system:

1. **CI gate (Layer 4):** Runs independently in the publish pipeline to
   detect capability usage that the linter might have missed.

2. **Developer tool:** Runs locally to show what capabilities a package
   uses before committing.

## How It Works

The analyzer uses Python's built-in `ast` module to parse source files
into abstract syntax trees. It then walks each tree looking for patterns
that indicate OS capability usage:

- `import os` → filesystem or process capability
- `open("file.txt")` → filesystem read/write capability
- `import socket` → network capability
- `subprocess.run(...)` → process execution capability
- `os.environ["KEY"]` → environment variable access
- `eval(...)` → banned dynamic execution construct

The key insight is that Python's `import` statement and built-in function
calls have well-defined AST node types. An `import socket` statement
always produces an `ast.Import` node with `names[0].name == "socket"`.
This makes static detection reliable for direct usage.

Dynamic usage (`__import__("socket")`, `eval("import socket")`) is caught
by the banned constructs detector, which flags these patterns outright.

## Limitations

The analyzer cannot detect capabilities accessed through patterns it
doesn't recognize. For example, a carefully crafted sequence of string
operations that produces a module name and passes it to a banned function
would evade detection. This is why the sandbox fuzz layer (Layer 6) exists
as defense-in-depth — it provides kernel-level runtime verification.
"""

from ca_capability_analyzer.analyzer import (
    CapabilityAnalyzer,
    DetectedCapability,
    analyze_directory,
    analyze_file,
)
from ca_capability_analyzer.banned import (
    BannedConstructDetector,
    BannedConstructViolation,
    detect_banned_constructs,
)
from ca_capability_analyzer.manifest import (
    ComparisonResult,
    Manifest,
    compare_capabilities,
    load_manifest,
)

__all__ = [
    "CapabilityAnalyzer",
    "DetectedCapability",
    "analyze_file",
    "analyze_directory",
    "Manifest",
    "load_manifest",
    "compare_capabilities",
    "ComparisonResult",
    "BannedConstructDetector",
    "BannedConstructViolation",
    "detect_banned_constructs",
]
