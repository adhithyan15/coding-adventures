module github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-scan

go 1.26

require (
	github.com/adhithyan15/coding-adventures/code/packages/go/storage-record-frame v0.0.0
	github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-page-slot v0.0.0
	github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-record v0.0.0
)

require github.com/adhithyan15/coding-adventures/code/packages/go/storage-crc32c v0.0.0 // indirect

replace github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-page-slot => ../storage-wal-page-slot

replace github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-record => ../storage-wal-record

replace github.com/adhithyan15/coding-adventures/code/packages/go/storage-record-frame => ../storage-record-frame

replace github.com/adhithyan15/coding-adventures/code/packages/go/storage-crc32c => ../storage-crc32c
