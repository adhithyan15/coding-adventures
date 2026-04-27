module github.com/adhithyan15/coding-adventures/code/packages/go/hkdf

go 1.26

require (
	github.com/adhithyan15/coding-adventures/code/packages/go/hmac v0.0.0
	github.com/adhithyan15/coding-adventures/code/packages/go/sha256 v0.0.0
	github.com/adhithyan15/coding-adventures/code/packages/go/sha512 v0.0.0
)

require (
	github.com/adhithyan15/coding-adventures/code/packages/go/md5 v0.0.0 // indirect
	github.com/adhithyan15/coding-adventures/code/packages/go/sha1 v0.0.0 // indirect
)

replace (
	github.com/adhithyan15/coding-adventures/code/packages/go/hmac => ../hmac
	github.com/adhithyan15/coding-adventures/code/packages/go/md5 => ../md5
	github.com/adhithyan15/coding-adventures/code/packages/go/sha1 => ../sha1
	github.com/adhithyan15/coding-adventures/code/packages/go/sha256 => ../sha256
	github.com/adhithyan15/coding-adventures/code/packages/go/sha512 => ../sha512
)
