package datastoreprotocol

import (
	"fmt"
	"strconv"
	"strings"

	resp "github.com/adhithyan15/coding-adventures/code/packages/go/resp-protocol"
)

type CommandFrame struct {
	Command string
	Args    [][]byte
}

func NewCommandFrame(command string, args [][]byte) CommandFrame {
	return CommandFrame{Command: strings.ToUpper(command), Args: cloneArgs(args)}
}

func FromParts(parts [][]byte) (CommandFrame, bool) {
	if len(parts) == 0 {
		return CommandFrame{}, false
	}
	return CommandFrame{
		Command: asciiUpper(parts[0]),
		Args:    cloneArgs(parts[1:]),
	}, true
}

func FrameFromResp(value resp.Value) (CommandFrame, bool) {
	if value.Kind != resp.ArrayKind || value.Nil || len(value.Array) == 0 {
		return CommandFrame{}, false
	}

	parts := make([][]byte, 0, len(value.Array))
	for _, item := range value.Array {
		switch item.Kind {
		case resp.SimpleStringKind:
			parts = append(parts, []byte(item.Text))
		case resp.BulkStringKind:
			if item.Nil {
				return CommandFrame{}, false
			}
			parts = append(parts, cloneBytes(item.Bulk))
		case resp.IntegerKind:
			parts = append(parts, []byte(strconv.FormatInt(item.Integer, 10)))
		default:
			return CommandFrame{}, false
		}
	}
	return FromParts(parts)
}

func ProtocolError(message string) resp.Value {
	return resp.ErrorValue(message)
}

func MapDecodeError(err error) error {
	return fmt.Errorf("invalid RESP payload: %w", err)
}

func asciiUpper(data []byte) string {
	return strings.ToUpper(string(data))
}

func cloneArgs(args [][]byte) [][]byte {
	if args == nil {
		return nil
	}
	result := make([][]byte, len(args))
	for i, arg := range args {
		result[i] = cloneBytes(arg)
	}
	return result
}

func cloneBytes(data []byte) []byte {
	if data == nil {
		return nil
	}
	return append([]byte(nil), data...)
}

func ParseFrameText(parts []string) CommandFrame {
	args := make([][]byte, 0, len(parts)-1)
	for _, part := range parts[1:] {
		args = append(args, []byte(part))
	}
	return NewCommandFrame(parts[0], args)
}

func FormatFrame(frame CommandFrame) string {
	rendered := make([]string, 0, len(frame.Args)+1)
	rendered = append(rendered, frame.Command)
	for _, arg := range frame.Args {
		rendered = append(rendered, string(arg))
	}
	return strings.Join(rendered, " ")
}

func EncodeFrame(frame CommandFrame) (resp.Value, error) {
	items := make([]resp.Value, 0, len(frame.Args)+1)
	items = append(items, resp.BulkString([]byte(frame.Command)))
	for _, arg := range frame.Args {
		items = append(items, resp.BulkString(arg))
	}
	return resp.Array(items), nil
}
