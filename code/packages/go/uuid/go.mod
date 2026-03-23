module github.com/adhithyan15/coding-adventures/code/packages/go/uuid

go 1.26

require (
	github.com/adhithyan15/coding-adventures/code/packages/go/md5 v0.0.0
	github.com/adhithyan15/coding-adventures/code/packages/go/sha1 v0.0.0
)

replace (
	github.com/adhithyan15/coding-adventures/code/packages/go/md5 => ../md5
	github.com/adhithyan15/coding-adventures/code/packages/go/sha1 => ../sha1
)
