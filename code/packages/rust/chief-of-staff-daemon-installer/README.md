# chief-of-staff-daemon-installer

Secure local mutation boundary for `chief-of-staff install-daemon`. It combines
the pure service-file renderers with retry-safe publication and shell-free
native registration.

Callers supply explicit absolute paths for the daemon, configuration, user
service root, and native supervisor executable. Planning is pure and portable.
Application is allowed only when the plan matches the current operating system.

The installer creates missing service subdirectories, rejects links and
non-regular inputs, rejects group- or world-writable Unix service directories,
and validates the native supervisor executable before mutation. It writes the
complete definition to a unique private sibling, synchronizes it, and
atomically claims the final absent name with a hard link. Existing definitions
are never overwritten: byte-identical content is accepted for registration
retries, while any difference fails closed.

Native commands are direct program-plus-argument vectors. No shell, command
string, environment expansion, or caller-controlled command flag is used.

## Validation

```sh
sh chief-of-staff-daemon-installer/BUILD
```
