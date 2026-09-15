# A grep for a shape finds the shapes it can see

Fixing the media amplification, I grepped `pub data: Vec<u8>` and found two
types. Review found a third: the exported `.apkg`, which is a **local** handed
straight to `ok_with` and matches no field pattern. It measured at 4.00x wire
and 5.5–8.25x peak — the largest of the three, since a legacy `.apkg` stores
media uncompressed.

It is also the one I could not fix in the same change, and finding out why was
the useful part: **six host adapters read that payload** through
`jsonByteArray` helpers expecting an array of numbers. The two struct fields
cross the facade and no host reads them; this one every host reads. Same bug,
same fix, completely different blast radius — and nothing about the two sites
distinguished them until I went looking at consumers.

Grep for the shape, then grep for the *consumers* of each hit. The second
search is what tells you which fixes are one commit and which are six.
