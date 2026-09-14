# The diagnostic I added to fix one bug immediately exposed the next

Having replaced the apt guard with one that prints what survived, the first
green run said:

```
pruned 2 vendor source list(s): microsoft-prod.list azure-cli.sources
apt sources remaining: google-chrome.sources ubuntu.sources
```

`google-chrome.sources` is a repository nothing here installs from, and it can
403 exactly the way `packages.microsoft.com` did. The fix I had just merged
closed the reported instance and left an identical hole — the acceptance
criterion said "a third-party repository that this repository never installs
from cannot fail a required job", and that was still false for Chrome's.

I had enumerated vendors: microsoft, azure-cli. That is a **denylist over a set
somebody else controls** — the contents of a runner image — so it can only ever
be as current as the last time a person looked at one, and every miss is silent
until an outage. Adding `google-chrome*` would fix today and not the next image
change.

The correction is the same one this file already records for archive member
names: what *we* depend on is a closed set. Every package installed across all
six workflows comes from the Ubuntu archive, and no workflow runs
`add-apt-repository`. So keep `ubuntu.sources` and drop everything else, with an
`APT_KEEP` escape hatch for the day a workflow legitimately adds a PPA —
otherwise the wrapper would delete a repository a previous step just added and
fail with "unable to locate package", pointing at the package rather than at
itself.

Two things worth carrying:

- **Print what you saw, not what you concluded — the evidence finds the next
  bug.** This diagnostic was added to save a round trip diagnosing a misfiring
  guard. It paid for itself immediately by naming a repository I did not know
  was there. A log line that reports a verdict could not have.
- **I made the same structural mistake twice in one day**, on filename hazards
  and then on vendor repositories, having written the lesson down in between.
  The tell is identical both times: *am I enumerating the bad cases, and is that
  set maintained by someone other than me?* If so, enumerate the good ones.
