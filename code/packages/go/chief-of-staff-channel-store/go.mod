module github.com/adhithyan15/coding-adventures/code/packages/go/chief-of-staff-channel-store

go 1.26

require (
	github.com/adhithyan15/coding-adventures/code/packages/go/chief-of-staff-channel-crypto v0.0.0
	github.com/adhithyan15/coding-adventures/code/packages/go/sha256 v0.0.0
	github.com/example/coding-adventures/code/packages/go/ed25519 v0.0.0
)

require (
	github.com/adhithyan15/coding-adventures/code/packages/go/chacha20-poly1305 v0.0.0 // indirect
	github.com/example/coding-adventures/code/packages/go/sha512 v0.0.0 // indirect
)

replace github.com/adhithyan15/coding-adventures/code/packages/go/chief-of-staff-channel-crypto => ../chief-of-staff-channel-crypto

replace github.com/adhithyan15/coding-adventures/code/packages/go/chacha20-poly1305 => ../chacha20-poly1305

replace github.com/adhithyan15/coding-adventures/code/packages/go/sha256 => ../sha256

replace github.com/example/coding-adventures/code/packages/go/ed25519 => ../ed25519

replace github.com/example/coding-adventures/code/packages/go/sha512 => ../sha512
