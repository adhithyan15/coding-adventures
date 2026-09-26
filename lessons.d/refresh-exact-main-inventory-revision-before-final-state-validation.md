# Refresh exact-main inventory revision before final state validation

The remote-tracking `origin/main` advanced during the final validation window,
so the persisted inventory revision no longer matched the exact current main
commit even though it was current when implementation began. Recheck and fetch
`origin/main` immediately before committing, inspect the intervening paths for
package-topology changes, regenerate the inventory when topology changed, and
rebase plus update the recorded revision before accepting the state check.
