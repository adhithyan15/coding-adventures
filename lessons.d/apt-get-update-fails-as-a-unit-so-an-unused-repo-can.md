# `apt-get update` fails as a unit, so an unused repo can fail a required job

A docs-only pull request — four Markdown files, no source, no CI change —
failed its required `build (ubuntu-latest)` job because
`packages.microsoft.com` returned 403. `apt-get update` exits 100 if *any*
configured repository fails, and the GitHub runner images preinstall vendor
repositories this project never installs from. The step only wanted
`libcairo2-dev` from the Ubuntu archive, which fetched fine.

The fix is to drop the unused vendor lists before updating, so update's exit
status again means something about the archive we depend on. Deliberately not
`apt-get update || true`: that hides a genuinely unreachable archive and turns
a loud failure into a confusing one later, when the install fails for a reason
that no longer names the cause.

Three things this turned up that are worth keeping:

- **On Ubuntu 24.04 the main archive lives in
  `/etc/apt/sources.list.d/ubuntu.sources`**, in deb822 format —
  `/etc/apt/sources.list` is a stub. So the obvious fix, "clear
  `sources.list.d`", deletes the archive every package comes from, and the job
  then fails to find `libcairo2-dev` rather than failing to reach Microsoft.
  Pruning has to be by explicit vendor pattern, and the script asserts an
  Ubuntu archive survives before installing anything — a guard against its own
  pattern list being widened too far later.

- **The reported instance was not the population.** The issue said 10 sites in
  5 workflows; the actual count was 14 in 6, including a workflow the issue did
  not list. Fixing only what was reported leaves the next outage to red-flag a
  different required job. A test now asserts no workflow calls `apt-get update`
  directly, so the invariant is checked rather than remembered.

- **A survey finds things outside its own scope, and those should be logged,
  not smuggled in.** Six more sites run `apt-get install` with no update at all,
  and two of those pass `-q` (quiet) where they meant `-y` (assume yes) — with
  no TTY, apt prompts and aborts. They pass today only because the packages are
  already in the runner image, which makes the step a silent no-op. Different
  construct, different fix, real behaviour change to route them through the
  wrapper. Filed separately rather than expanding the PR.
