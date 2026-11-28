# DB12c2b2 — Admit an intact fixed WAL tail for recovery

The first recovery profile accepts a captured reservation classified as
full or vacant. It uses the admitted record prefix, including any complete
commit that survived a lost acknowledgement. Carry the capture's hard
errors through recovery before opening the data write path.
[WAL scan](DB12b2-go-wal-scan.md) · [Shared capture](DB12c2b1-go-wal-reservation.md)

If capture classifies a damaged tail, report the first affected log page
and the decoder cause using the append package's typed damaged-tail error.
Make no data writes and issue no data barriers. Preserve the raw log.
The same rule applies when the damaged slot is the first page.

A profile that discards a damaged terminal slot needs an independently
verified write history or durable boundary. It must establish that the
accepted prefix preserves all prior acknowledged commits and that the
terminal write can have only the observed interrupted outcome. An ordinary
scanner result supplies the slot's bytes, classification and original
rank. A future profile needs evidence for its additional assumptions
about external overwrites, retention and device behavior.

## Identical bytes after distinct histories

An independent caller used the exact Go HDD, WAL append, pager and scanner
packages to produce two four-slot captures. Both histories acknowledged
the same begin and page-image records. The first cut persisted eleven
bytes of the commit slot before a lost barrier response. The second
acknowledged the complete commit, then an external writer replaced the
same slot with its first eleven bytes followed by zeros.

Both restarts returned the same 224 log bytes. The scanner returned
records 1 and 2 and a damaged page at index 2. The blank fourth slot
was identical. The test passed race and vet. The same terminal bytes
can therefore accompany different prior acknowledgement histories.
