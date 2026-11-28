module github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-record

go 1.26

require github.com/adhithyan15/coding-adventures/code/packages/go/storage-record-frame v0.0.0

require github.com/adhithyan15/coding-adventures/code/packages/go/storage-crc32c v0.0.0 // indirect

replace github.com/adhithyan15/coding-adventures/code/packages/go/storage-record-frame => ../storage-record-frame

replace github.com/adhithyan15/coding-adventures/code/packages/go/storage-crc32c => ../storage-crc32c
