---
category: BUILD files & dependency management
---

# Hatch rejects local Python development references without allow-direct-references

Adding a sibling package as a PEP 508 file reference in a Python optional
dependency is not enough for Hatch metadata. Editable installation fails while
preparing package metadata unless `tool.hatch.metadata.allow-direct-references`
is set to `true`. Add the setting to the consuming package and keep the local
package in its development extra; install it from the BUILD file in leaf-to-root
order. The development reference is not a runtime dependency and does not by
itself create a Go build-tool selection edge.
