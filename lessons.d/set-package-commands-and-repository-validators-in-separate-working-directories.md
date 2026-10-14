# Set package commands and repository validators in separate working directories

A validation chain mixed package-local `npm` scripts with a repository-root
lesson validator, then ran the whole chain from the worktree root. The root has
no `package.json`, so npm failed before any intended check ran. Run package
commands with the package as the explicit working directory and repository
validators in a separate root-scoped command. Do not make one working directory
serve unrelated command scopes.
