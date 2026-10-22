---
category: BUILD files & dependency management
---

# Run package BUILD commands from their package directory

The package `BUILD` file contains Cargo commands whose workspace discovery
depends on the current directory. Running it as `bash path/to/BUILD` from the
repository root made both commands fail to find `Cargo.toml`. Change into the
package directory before executing its BUILD contract, even when the file can
be addressed from elsewhere.
