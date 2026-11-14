---
category: BUILD files & dependency management
---

# A Python dev-extra that names a sibling package is an allowed BUILD reference, not a graph edge, so edits to that sibling do not rebuild the consumer

The Python ct-compare port needs the test-only Warrant runner (`python/warrant-corpus`) at BUILD
time. The validator wants that reference declared in pyproject.toml, and the convention
(arm1-gatelevel) is to list it in `[project.optional-dependencies] dev`. But
`parsePythonDeps` reads only `[project] dependencies`; the `dev` list feeds
`parsePythonOptionalDeps`, which merely marks a BUILD reference as *allowed*
(no `undeclared local package refs`). It adds no edge to the graph.

So with only the dev-extra, an edit to the runner selects the runner's own tests and nothing else:
the consumer whose committed manifest embeds the runner's source digest is never rebuilt, and the
drift it exists to catch lands on `main` unseen. Putting the runner in `[project] dependencies`
would fix selection by making a security library depend on test infrastructure at runtime, which is
the wrong trade.

Fix: register the consumer as a repository input (`repository_inputs.go`) with a slash-terminated
prefix for the runner package plus the exact spec/corpus paths, and pin it with a test. Do that
whenever a package's BUILD reads a sibling through a dev-extra or a path outside its own directory.
