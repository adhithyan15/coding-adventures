package resp

import (
	"bytes"
	"errors"
	"fmt"
	"io"
	"strconv"
)

type Kind int

const (
	SimpleStringKind Kind = iota
	ErrorKind
	IntegerKind
	BulkStringKind
	ArrayKind
)

type Value struct {
	Kind    Kind
	Text    string
	Integer int64
	Bulk    []byte
	Array   []Value
	Nil     bool
}

func SimpleString(text string) Value {
	return Value{Kind: SimpleStringKind, Text: text}
}

func ErrorValue(message string) Value {
	return Value{Kind: ErrorKind, Text: message}
}

func Integer(value int64) Value {
	return Value{Kind: IntegerKind, Integer: value}
}

func BulkString(value []byte) Value {
	if value == nil {
		return Value{Kind: BulkStringKind, Nil: true}
	}
	return Value{Kind: BulkStringKind, Bulk: append([]byte(nil), value...)}
}

func NullBulkString() Value {
	return Value{Kind: BulkStringKind, Nil: true}
}

func Array(values []Value) Value {
	if values == nil {
		return Value{Kind: ArrayKind, Nil: true}
	}
	return Value{Kind: ArrayKind, Array: cloneValues(values)}
}

func NullArray() Value {
	return Value{Kind: ArrayKind, Nil: true}
}

func (v Value) Clone() Value {
	switch v.Kind {
	case BulkStringKind:
		if v.Nil {
			return NullBulkString()
		}
		return BulkString(v.Bulk)
	case ArrayKind:
		if v.Nil {
			return NullArray()
		}
		return Array(v.Array)
	default:
		return v
	}
}

func Encode(value Value) ([]byte, error) {
	var buf bytes.Buffer
	if err := encodeValue(&buf, value); err != nil {
		return nil, err
	}
	return buf.Bytes(), nil
}

func EncodeMany(values []Value) ([]byte, error) {
	var buf bytes.Buffer
	for _, value := range values {
		if err := encodeValue(&buf, value); err != nil {
			return nil, err
		}
	}
	return buf.Bytes(), nil
}

func encodeValue(buf *bytes.Buffer, value Value) error {
	switch value.Kind {
	case SimpleStringKind:
		buf.WriteByte('+')
		buf.WriteString(value.Text)
		buf.WriteString("\r\n")
	case ErrorKind:
		buf.WriteByte('-')
		buf.WriteString(value.Text)
		buf.WriteString("\r\n")
	case IntegerKind:
		buf.WriteByte(':')
		buf.WriteString(strconv.FormatInt(value.Integer, 10))
		buf.WriteString("\r\n")
	case BulkStringKind:
		if value.Nil {
			buf.WriteString("$-1\r\n")
			return nil
		}
		buf.WriteByte('$')
		buf.WriteString(strconv.Itoa(len(value.Bulk)))
		buf.WriteString("\r\n")
		buf.Write(value.Bulk)
		buf.WriteString("\r\n")
	case ArrayKind:
		if value.Nil {
			buf.WriteString("*-1\r\n")
			return nil
		}
		buf.WriteByte('*')
		buf.WriteString(strconv.Itoa(len(value.Array)))
		buf.WriteString("\r\n")
		for _, item := range value.Array {
			if err := encodeValue(buf, item); err != nil {
				return err
			}
		}
	default:
		return fmt.Errorf("resp: unknown kind %d", value.Kind)
	}
	return nil
}

type DecodeError struct {
	Message string
}

func (e *DecodeError) Error() string {
	return e.Message
}

func DecodeAll(data []byte) ([]Value, error) {
	var values []Value
	offset := 0
	for offset < len(data) {
		value, next, err := decodeValue(data, offset)
		if err != nil {
			return nil, err
		}
		values = append(values, value)
		offset = next
	}
	return values, nil
}

func DecodeOne(data []byte) (Value, int, error) {
	return decodeValue(data, 0)
}

func decodeValue(data []byte, offset int) (Value, int, error) {
	if offset >= len(data) {
		return Value{}, offset, io.EOF
	}

	switch data[offset] {
	case '+':
		line, next, err := readLine(data, offset+1)
		if err != nil {
			return Value{}, offset, err
		}
		return SimpleString(string(line)), next, nil
	case '-':
		line, next, err := readLine(data, offset+1)
		if err != nil {
			return Value{}, offset, err
		}
		return ErrorValue(string(line)), next, nil
	case ':':
		line, next, err := readLine(data, offset+1)
		if err != nil {
			return Value{}, offset, err
		}
		n, parseErr := strconv.ParseInt(string(line), 10, 64)
		if parseErr != nil {
			return Value{}, offset, &DecodeError{Message: fmt.Sprintf("invalid integer: %v", parseErr)}
		}
		return Integer(n), next, nil
	case '$':
		line, next, err := readLine(data, offset+1)
		if err != nil {
			return Value{}, offset, err
		}
		length, parseErr := strconv.Atoi(string(line))
		if parseErr != nil {
			return Value{}, offset, &DecodeError{Message: fmt.Sprintf("invalid bulk length: %v", parseErr)}
		}
		if length < 0 {
			return NullBulkString(), next, nil
		}
		if length > len(data)-next-2 {
			return Value{}, offset, io.ErrUnexpectedEOF
		}
		end := next + length
		bulk := append([]byte(nil), data[next:end]...)
		if !bytes.Equal(data[end:end+2], []byte("\r\n")) {
			return Value{}, offset, &DecodeError{Message: "bulk string missing CRLF"}
		}
		return BulkString(bulk), end + 2, nil
	case '*':
		line, next, err := readLine(data, offset+1)
		if err != nil {
			return Value{}, offset, err
		}
		count, parseErr := strconv.Atoi(string(line))
		if parseErr != nil {
			return Value{}, offset, &DecodeError{Message: fmt.Sprintf("invalid array length: %v", parseErr)}
		}
		if count < 0 {
			return NullArray(), next, nil
		}
		items := make([]Value, 0, count)
		cursor := next
		for i := 0; i < count; i++ {
			item, n, err := decodeValue(data, cursor)
			if err != nil {
				return Value{}, offset, err
			}
			items = append(items, item)
			cursor = n
		}
		return Array(items), cursor, nil
	default:
		return Value{}, offset, &DecodeError{Message: fmt.Sprintf("unknown RESP prefix %q", data[offset])}
	}
}

func readLine(data []byte, offset int) ([]byte, int, error) {
	end := bytes.Index(data[offset:], []byte("\r\n"))
	if end < 0 {
		return nil, offset, io.ErrUnexpectedEOF
	}
	end += offset
	line := append([]byte(nil), data[offset:end]...)
	return line, end + 2, nil
}

func cloneValues(values []Value) []Value {
	result := make([]Value, len(values))
	for i, value := range values {
		result[i] = value.Clone()
	}
	return result
}

func MustEncode(value Value) []byte {
	data, err := Encode(value)
	if err != nil {
		panic(err)
	}
	return data
}

func MustDecodeAll(data []byte) []Value {
	values, err := DecodeAll(data)
	if err != nil {
		panic(err)
	}
	return values
}

func (k Kind) String() string {
	switch k {
	case SimpleStringKind:
		return "simple_string"
	case ErrorKind:
		return "error"
	case IntegerKind:
		return "integer"
	case BulkStringKind:
		return "bulk_string"
	case ArrayKind:
		return "array"
	default:
		return fmt.Sprintf("kind(%d)", int(k))
	}
}

func (v Value) String() string {
	switch v.Kind {
	case SimpleStringKind:
		return fmt.Sprintf("SimpleString(%q)", v.Text)
	case ErrorKind:
		return fmt.Sprintf("Error(%q)", v.Text)
	case IntegerKind:
		return fmt.Sprintf("Integer(%d)", v.Integer)
	case BulkStringKind:
		if v.Nil {
			return "BulkString(nil)"
		}
		return fmt.Sprintf("BulkString(%q)", string(v.Bulk))
	case ArrayKind:
		if v.Nil {
			return "Array(nil)"
		}
		return fmt.Sprintf("Array(%v)", v.Array)
	default:
		return fmt.Sprintf("Value(kind=%d)", v.Kind)
	}
}

func Equal(left, right Value) bool {
	if left.Kind != right.Kind || left.Nil != right.Nil {
		return false
	}
	switch left.Kind {
	case SimpleStringKind, ErrorKind:
		return left.Text == right.Text
	case IntegerKind:
		return left.Integer == right.Integer
	case BulkStringKind:
		return bytes.Equal(left.Bulk, right.Bulk)
	case ArrayKind:
		if len(left.Array) != len(right.Array) {
			return false
		}
		for i := range left.Array {
			if !Equal(left.Array[i], right.Array[i]) {
				return false
			}
		}
		return true
	default:
		return false
	}
}

func IsError(value Value) bool {
	return value.Kind == ErrorKind
}

func Errorf(format string, args ...any) Value {
	return ErrorValue(fmt.Sprintf(format, args...))
}

var ErrProtocol = errors.New("protocol error")
