# An issue saying "the shared half is done" is a claim, not a fact

#14024 read: *"Depends on `build-native.sh`, which already builds the engine,
emits the project, and places the library — **the shared half is done**; what
remains is this backend's toolchain."* I took that as given, built the
publishing layer on top, wired the job into the release lane, and CI failed
before reaching a single line of my code:

```
error CS0101: The namespace 'Mosaic.Generated' already contains a definition
  for 'EngramAppEvent'
```

`engram-app` ships two layout variants, and the XAML emitter writes an event
type for each into the same namespace. `--backend xaml --build` has never
compiled this package. The build half was not done; nobody had run it, because
it only runs on Windows and everyone works on macOS.

The near-miss is the part worth keeping. I had added `build-xaml` to the
publish job's `needs` — so merging would have put a job that **cannot pass**
into the dependency list of every future release. A release-blocking regression,
introduced by trusting a sentence.

- **Verify the precondition, not just your own work.** The five minutes to run
  `--backend xaml` and look at the emitted files would have found this before
  any of the publishing layer was written. "Depends on X, which is done" is
  exactly the claim to spot-check, and it is cheap when X has a command.
- **Adding a job to `publish.needs` is a release-blocking act.** Every other
  backend's job was added the same way and was fine because each had been run
  locally first. This one could not be, which is precisely when the wiring
  deserves more caution rather than less.
- **Ship the verified part, hold back the part that would break things, and say
  which is which.** The PE export parser, the archiver, `-p:Platform=x64`, and
  the trigger paths are all correct and tested. Only the release wiring is held,
  behind a filed blocker, with a comment in the code saying what unblocks it.
