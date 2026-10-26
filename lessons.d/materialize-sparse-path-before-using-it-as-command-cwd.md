# Materialize sparse path before using it as command cwd

A command cannot start in a directory omitted by sparse checkout, even when
its first shell operation would add that directory. The process working
directory is resolved before the shell runs, so the sparse-checkout repair is
never reached. Run `git sparse-checkout add <path>` from an already materialized
repository directory first, then launch commands with the new path as their
working directory.
