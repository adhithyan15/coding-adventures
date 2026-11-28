# Test refactors must remove obsolete imports before running the full module suite

category: Go

When replacing repeated test setup with a shared helper, remove imports that
were only used by the deleted setup. The first full build-tool suite after this
refactor failed at compile time because `dev_dependency_triggers_test.go` still
imported `internal/discovery`. The imported package remained needed elsewhere
in the test file's previous version, but no longer had a use after both
repository-backed tests switched to the snapshot helper. `go test ./...` caught
the stale import before running the root package's tests.

After test-only call-site refactors, run `gofmt` and compile the complete Go
module before spending time on long integration tests. Then inspect the actual
package results; other packages may pass even while the main test package fails
to build.
