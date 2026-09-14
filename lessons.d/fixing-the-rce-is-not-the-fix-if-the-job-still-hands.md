# Fixing the RCE is not the fix if the job still hands the attacker a write token

The security review of the latexmk hardening above found the real ranking, and it was not
the one the PR started with. Closing the `latexmkrc` `eval` mattered — but the same job
was `pull_request`-triggered with `permissions: contents: write`, ran `npm ci` (without
`--ignore-scripts`), a TypeScript build and seven `node dist/*.js` checks, all
pull-request-controlled, and `actions/checkout@v7` defaults to
`persist-credentials: true`, which leaves the write-scoped token in `.git/config` **inside
the workspace latexmk `cd`s into**. Anyone who could have used the latexmk hole already
had a dozen easier ones.

**Ask what the job is holding, not only what it is running.** The durable fix is
structural: the job that executes repository content gets `contents: read` and
`persist-credentials: false`; a separate job holds `contents: write`, runs no repository
code, has no `actions/checkout` at all, and is gated at JOB level (not step level) on
`github.event_name == 'push' && github.ref == 'refs/heads/main'`. Step-level `if:` on the
publish steps was already there and was not enough — the token is scoped to the job, so it
is live for every earlier step in it regardless.

When splitting, remember the aggregating gate job: `needs: [detect, build-and-publish]`
and `needs['build-and-publish'].result` both have to move, and the new publish job must
**not** become a dependency of the gate, or the gate can never pass on a pull request.
