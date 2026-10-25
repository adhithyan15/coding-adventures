---
category: BUILD files & dependency management
---

# A dotnet artifacts-path restore does not populate the default obj tree

When a package BUILD front runs `dotnet test --artifacts-path .artifacts`, its
restore writes assets below that artifacts root rather than the project's
default `obj/` directory.  A later `dotnet format --no-restore` or `dotnet pack
--no-restore` against the project therefore fails with `NETSDK1004` even though
tests just restored successfully.  Run an ordinary project restore first, or
give every command the same artifacts path.
