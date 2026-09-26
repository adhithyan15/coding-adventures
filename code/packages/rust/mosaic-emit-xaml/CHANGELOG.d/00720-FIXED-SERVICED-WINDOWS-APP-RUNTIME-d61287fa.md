### Fixed - Serviced Windows App Runtime

Generated WinUI projects now pin Windows App SDK `1.8.260710003`, its required
Windows SDK BuildTools `10.0.26100.4654`, and the matching 1.8 framework
libraries. The Windows App SDK is bundled self-contained, removing the
system-wide runtime installation prerequisite and insulating generated apps
from machine runtime registration failures.

