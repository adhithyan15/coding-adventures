module github.com/adhithyan15/coding-adventures/code/packages/go/hdd-barrier-plan

go 1.26

require (
	github.com/adhithyan15/coding-adventures/code/packages/go/block-device-contract v0.0.0
	github.com/adhithyan15/coding-adventures/code/packages/go/hdd-geometry v0.0.0
	github.com/adhithyan15/coding-adventures/code/packages/go/hdd-seek-timing v0.0.0
	github.com/adhithyan15/coding-adventures/code/packages/go/hdd-write-crash-cuts v0.0.0
	github.com/adhithyan15/coding-adventures/code/packages/go/hdd-writeback-cache v0.0.0
)

replace github.com/adhithyan15/coding-adventures/code/packages/go/block-device-contract => ../block-device-contract

replace github.com/adhithyan15/coding-adventures/code/packages/go/hdd-geometry => ../hdd-geometry

replace github.com/adhithyan15/coding-adventures/code/packages/go/hdd-seek-timing => ../hdd-seek-timing

replace github.com/adhithyan15/coding-adventures/code/packages/go/hdd-write-crash-cuts => ../hdd-write-crash-cuts

replace github.com/adhithyan15/coding-adventures/code/packages/go/hdd-writeback-cache => ../hdd-writeback-cache
