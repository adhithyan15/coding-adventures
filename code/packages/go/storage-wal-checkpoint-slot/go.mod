module github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-checkpoint-slot

go 1.26

require (
	github.com/adhithyan15/coding-adventures/code/packages/go/hdd-block-device v0.0.0
	github.com/adhithyan15/coding-adventures/code/packages/go/hdd-write-crash-cuts v0.0.0
	github.com/adhithyan15/coding-adventures/code/packages/go/storage-record-frame v0.0.0
	github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-checkpoint-record v0.0.0
)

require (
	github.com/adhithyan15/coding-adventures/code/packages/go/block-device-contract v0.0.0 // indirect
	github.com/adhithyan15/coding-adventures/code/packages/go/hdd-writeback-cache v0.0.0 // indirect
	github.com/adhithyan15/coding-adventures/code/packages/go/storage-crc32c v0.0.0 // indirect
)

replace github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-checkpoint-record => ../storage-wal-checkpoint-record

replace github.com/adhithyan15/coding-adventures/code/packages/go/hdd-write-crash-cuts => ../hdd-write-crash-cuts

replace github.com/adhithyan15/coding-adventures/code/packages/go/hdd-block-device => ../hdd-block-device

replace github.com/adhithyan15/coding-adventures/code/packages/go/block-device-contract => ../block-device-contract

replace github.com/adhithyan15/coding-adventures/code/packages/go/hdd-writeback-cache => ../hdd-writeback-cache

replace github.com/adhithyan15/coding-adventures/code/packages/go/memory-block-device => ../memory-block-device

replace github.com/adhithyan15/coding-adventures/code/packages/go/storage-record-frame => ../storage-record-frame

replace github.com/adhithyan15/coding-adventures/code/packages/go/storage-crc32c => ../storage-crc32c
