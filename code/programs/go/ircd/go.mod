module github.com/adhithyan15/coding-adventures/code/programs/go/ircd

go 1.23

require (
	github.com/adhithyan15/coding-adventures/code/packages/go/irc-framing v0.0.0
	github.com/adhithyan15/coding-adventures/code/packages/go/irc-net-stdlib v0.0.0
	github.com/adhithyan15/coding-adventures/code/packages/go/irc-proto v0.0.0
	github.com/adhithyan15/coding-adventures/code/packages/go/irc-server v0.0.0
)

replace (
	github.com/adhithyan15/coding-adventures/code/packages/go/irc-framing => ../../../packages/go/irc-framing
	github.com/adhithyan15/coding-adventures/code/packages/go/irc-net-stdlib => ../../../packages/go/irc-net-stdlib
	github.com/adhithyan15/coding-adventures/code/packages/go/irc-proto => ../../../packages/go/irc-proto
	github.com/adhithyan15/coding-adventures/code/packages/go/irc-server => ../../../packages/go/irc-server
)
