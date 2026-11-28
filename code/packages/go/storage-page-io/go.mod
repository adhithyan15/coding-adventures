module github.com/adhithyan15/coding-adventures/code/packages/go/storage-page-io

go 1.26

require (
	github.com/adhithyan15/coding-adventures/code/packages/go/block-device-contract v0.0.0
	github.com/adhithyan15/coding-adventures/code/packages/go/hdd-block-device v0.0.0
	github.com/adhithyan15/coding-adventures/code/packages/go/hdd-write-crash-cuts v0.0.0
	github.com/adhithyan15/coding-adventures/code/packages/go/hdd-writeback-cache v0.0.0
	github.com/adhithyan15/coding-adventures/code/packages/go/memory-block-device v0.0.0
	github.com/adhithyan15/coding-adventures/code/packages/go/storage-page-layout v0.0.0
)

replace github.com/adhithyan15/coding-adventures/code/packages/go/block-device-contract => ../block-device-contract

replace github.com/adhithyan15/coding-adventures/code/packages/go/hdd-block-device => ../hdd-block-device

replace github.com/adhithyan15/coding-adventures/code/packages/go/hdd-write-crash-cuts => ../hdd-write-crash-cuts

replace github.com/adhithyan15/coding-adventures/code/packages/go/hdd-writeback-cache => ../hdd-writeback-cache

replace github.com/adhithyan15/coding-adventures/code/packages/go/memory-block-device => ../memory-block-device

replace github.com/adhithyan15/coding-adventures/code/packages/go/storage-page-layout => ../storage-page-layout
