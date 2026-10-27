---
category: Repo policy / workflow reminders
---

# A pgrep for a cargo command line also matches another worktree's identical build, so wait on your own pid

While verifying LANG83 PR7d, a wait loop of the form
`until ! pgrep -f "cargo test -p lang-aot"; do sleep 5; done` was used to learn
when this worktree's `lang_matrix` run finished. Several builders share this
machine, and another worktree (`vm-storereg`) was running the same
`cargo test -p lang-aot` command at the same time, so the loop would have kept
waiting on that build after ours finished, and a disk monitor written the same
way would have watched the wrong build.

Fix: find your own process and wait on its pid. `readlink /proc/<pid>/cwd`
shows which worktree a cargo process belongs to. Or, better, run the build in
the background yourself and wait for that task to finish.

A related trap in the same run: a background command's `timeout` is a hard
limit on the whole run. A `lang_matrix` build plus run takes more than ten
minutes, so `timeout: 600000` killed it partway through. Give long cargo test
runs the maximum timeout and send their output to a log file you can read
afterwards.
