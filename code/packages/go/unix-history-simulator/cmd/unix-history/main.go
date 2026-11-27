package main

import (
	"encoding/json"
	"fmt"
	"os"
	"strings"

	unixhistory "github.com/adhithyan15/coding-adventures/code/packages/go/unix-history-simulator"
)

func main() {
	command := "cat /hello"
	if len(os.Args) > 1 {
		command = strings.Join(os.Args[1:], " ")
	}
	encoder := json.NewEncoder(os.Stdout)
	encoder.SetIndent("", "  ")
	if err := encoder.Encode(unixhistory.RunCommand(command)); err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}
}
