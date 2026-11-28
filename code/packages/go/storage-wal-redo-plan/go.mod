module github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-redo-plan

go 1.26

require (
	github.com/adhithyan15/coding-adventures/code/packages/go/block-device-contract v0.0.0
	github.com/adhithyan15/coding-adventures/code/packages/go/storage-page-layout v0.0.0
	github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-record v0.0.0
)

require (
	github.com/adhithyan15/coding-adventures/code/packages/go/storage-crc32c v0.0.0 // indirect
	github.com/adhithyan15/coding-adventures/code/packages/go/storage-record-frame v0.0.0 // indirect
)

replace github.com/adhithyan15/coding-adventures/code/packages/go/storage-page-layout => ../storage-page-layout

replace github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-record => ../storage-wal-record

replace github.com/adhithyan15/coding-adventures/code/packages/go/block-device-contract => ../block-device-contract

replace github.com/adhithyan15/coding-adventures/code/packages/go/storage-record-frame => ../storage-record-frame

replace github.com/adhithyan15/coding-adventures/code/packages/go/storage-crc32c => ../storage-crc32c
