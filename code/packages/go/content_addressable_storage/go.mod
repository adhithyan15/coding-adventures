module github.com/adhithyan15/coding-adventures/code/packages/go/content_addressable_storage

go 1.26

require (
	github.com/adhithyan15/coding-adventures/code/packages/go/sha1 v0.0.0
)

replace (
	github.com/adhithyan15/coding-adventures/code/packages/go/sha1 => ../sha1
)
