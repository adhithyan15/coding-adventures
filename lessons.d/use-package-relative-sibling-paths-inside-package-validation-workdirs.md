# Use package-relative sibling paths inside package validation workdirs

A validation command ran from `code/packages/typescript/der-asn1` but tried to
open a repository-root-relative `code/packages/...` path, duplicating the path
under the package directory. When a command changes its working directory to a
package, use package-relative sibling paths such as `../der-tlv`, or run
repository-wide inspections from the repository root. Confirm the workdir and
resolve one target before chaining related reads.
