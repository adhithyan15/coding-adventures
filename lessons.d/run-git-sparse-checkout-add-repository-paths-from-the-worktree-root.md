---
category: Repo policy / workflow reminders
---

# Run git sparse-checkout add repository paths from the worktree root

Running `git sparse-checkout add code/packages/go/cli-builder` from inside
`code/programs/go/scaffold-generator` returned success but did not add the
repository-relative cone. The next `go run . --help` still failed because the
local replacement directory was absent. Run sparse-checkout updates from the
worktree root, verify the new entries with `git sparse-checkout list`, and
confirm the required directory exists before invoking a tool that depends on
the expanded view.
