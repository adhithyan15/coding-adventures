module github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-checkpoint-capture

go 1.26

require (
	github.com/adhithyan15/coding-adventures/code/packages/go/block-device-contract v0.0.0
	github.com/adhithyan15/coding-adventures/code/packages/go/hdd-block-device v0.0.0
	github.com/adhithyan15/coding-adventures/code/packages/go/memory-block-device v0.0.0
	github.com/adhithyan15/coding-adventures/code/packages/go/storage-page-io v0.0.0
	github.com/adhithyan15/coding-adventures/code/packages/go/storage-page-layout v0.0.0
	github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-checkpoint-layout v0.0.0
	github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-checkpoint-record v0.0.0
	github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-checkpoint-selection v0.0.0
	github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-checkpoint-slot v0.0.0
	github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-page-slot v0.0.0
)

require (
	github.com/adhithyan15/coding-adventures/code/packages/go/hdd-write-crash-cuts v0.0.0 // indirect
	github.com/adhithyan15/coding-adventures/code/packages/go/hdd-writeback-cache v0.0.0 // indirect
	github.com/adhithyan15/coding-adventures/code/packages/go/storage-crc32c v0.0.0 // indirect
	github.com/adhithyan15/coding-adventures/code/packages/go/storage-record-frame v0.0.0 // indirect
	github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-data-layout v0.0.0 // indirect
	github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-record v0.0.0 // indirect
	github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-reservation v0.0.0 // indirect
	github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-scan v0.0.0 // indirect
)

replace github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-checkpoint-record => ../storage-wal-checkpoint-record

replace github.com/adhithyan15/coding-adventures/code/packages/go/block-device-contract => ../block-device-contract

replace github.com/adhithyan15/coding-adventures/code/packages/go/storage-record-frame => ../storage-record-frame

replace github.com/adhithyan15/coding-adventures/code/packages/go/storage-crc32c => ../storage-crc32c

replace github.com/adhithyan15/coding-adventures/code/packages/go/storage-page-io => ../storage-page-io

replace github.com/adhithyan15/coding-adventures/code/packages/go/storage-page-layout => ../storage-page-layout

replace github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-data-layout => ../storage-wal-data-layout

replace github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-page-slot => ../storage-wal-page-slot

replace github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-reservation => ../storage-wal-reservation

replace github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-record => ../storage-wal-record

replace github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-scan => ../storage-wal-scan

replace github.com/adhithyan15/coding-adventures/code/packages/go/hdd-write-crash-cuts => ../hdd-write-crash-cuts

replace github.com/adhithyan15/coding-adventures/code/packages/go/hdd-block-device => ../hdd-block-device

replace github.com/adhithyan15/coding-adventures/code/packages/go/hdd-writeback-cache => ../hdd-writeback-cache

replace github.com/adhithyan15/coding-adventures/code/packages/go/memory-block-device => ../memory-block-device

replace github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-checkpoint-slot => ../storage-wal-checkpoint-slot

replace github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-checkpoint-layout => ../storage-wal-checkpoint-layout

replace github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-checkpoint-selection => ../storage-wal-checkpoint-selection
