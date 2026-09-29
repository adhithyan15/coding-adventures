### Added — live numeric typography (#15556)

Project positive numeric font-size literals and page-scoped numeric slots on Text,
Input/HostInput and HostButton. A component-scoped attached property restores the
original local value or native inheritance for invalid runtime values. Reject
unsupported expressions and template slot scope; keep HostTable unsupported.
Two generated components are compiled together and exercised with WinUI in CI.


