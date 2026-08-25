### Added - `<track>/chapters.d/`, one file per chapter (HL21 §5.1)

- Twenty of the twenty-three chapter ledgers are now stored as
  `<track>/chapters.d/<NNNN>.json`, one file per chapter, named for the chapter
  number and zero-padded to four digits. The shards are the source of truth;
  `<track>/chapters.json` remains as a **generated artifact** gated by
  `npm run check:shards`. Two authors adding two chapters now write two
  different files and do not collide there.
- **Round trip verified byte-exact against the real committed ledgers**, before
  and after. Folding each shard set back together reproduced the committed
  `chapters.json` with an unchanged SHA-256 for all twenty tracks:

  | track | sha256 of the rebuilt monolith |
  | --- | --- |
  | arabic | `c12ea1004e424c95565b6a07ecad8f921ab581d69e6896c9e6f984aca60a6a6c` |
  | bengali | `70e320ae1746b69555a5f2a7448735de18970be376139ecb77eb69e2766936ee` |
  | chinese | `d0f14241684e3c796b8c5cbf10c12aaf01ac3a98a335276f6e0e6a6695624c1f` |
  | german | `e1e0ddfcf601ecfbc32f04d821c92f2f99c8b432b2a64aa9ba235e25bb3fd2e3` |
  | gujarati | `87f45ca989153de9c580002f75433dea6179228f7e7533380100267c4d7ff0cd` |
  | hindi | `20833fac76c126feae3fac3bc246980bafae699a57785fed6586841f2278c12c` |
  | italian | `4e148da035b63e7b320f06d51316f7b5bcc9d4b89e8d1972e59ea6e6ac38761e` |
  | kannada | `b7c79176b53dfb4d0985a7420ac913d30b35f5d370208c68a1fe3ba7f4df3188` |
  | latin | `74582e67fcfc375b99a0a3a285429e230f9bcf3498a3194b5cf4dce37efcf092` |
  | malayalam | `ccb6402a8eee5ee62bcf3b46c46d7739593a44f2201b530f10dc8fd93459c152` |
  | marathi | `b58a6124229d9a4f8a5771308d0878cb187c8f5b3e9a5589ea5e7025bfab5549` |
  | persian | `7ba2e8889a48f482410edbab278c36ceb3c1d82c68ca29beb3774579a26aacbf` |
  | portuguese | `b6f54850d8c01a235556029ee9080ae13e633957f01a85dcc7faef9a9264e7e9` |
  | punjabi | `6967f61f0757cd7b39c562d4499767705dd3fa18aedb7d0c097af3380d1914b5` |
  | russian | `0d7b3e7556516b9ccdae89b21d7e40ac814e42c40a4bdd0146aba419e6b2d0fc` |
  | sanskrit | `23f829dfe1298e098c01b0871b2efd3d87c09c31e6edac5454ec9c556d025f73` |
  | spanish | `86eac84690073429d65c633bb647ac1bcd74424c6336210dcce7bdf8608bbc0a` |
  | tamil | `2d71531d58e61c7df74a37b703f53e773404ad9d666b37ded572fea940ff9a43` |
  | telugu | `2b93ddb5b10991d082b11152604e81efe8197b70ec0164ee9233977522d95171` |
  | urdu | `c6490ebb397b761bccf2033ac1607e5cd69858d6850ebdc132ea20cc1272df6f` |

- **The order trap is live in every track.** Shard filenames carry the chapter
  number zero-padded, and unpadded names would re-sort all twenty — including
  the smallest, where eleven chapters are enough for `10.json` and `11.json` to
  sort before `2.json`. Tests assert both halves: that sorted shard order
  reproduces authored order, and that unpadded names would not have.

