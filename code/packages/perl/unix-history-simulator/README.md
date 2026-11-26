# Unix History Simulator (Perl)

The [HOS00](../../../specs/HOS00-historical-os-simulator.md) library exposes a
filesystem, descriptor table and kernel. Use `perl bin/unix-history cat /hello`
to inspect the shared JSON trace. Run `prove -l t/` to replay the fixtures.
