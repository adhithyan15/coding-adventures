package rpc

// codec.go — RpcCodec: the message serialization interface
//
// # What is a codec?
//
// A codec (coder-decoder) translates between two representations of the same
// information:
//   - The structured, typed representation: RpcMessage[V]
//   - The byte-level, wire representation: []byte
//
// The codec is the "translation layer" between the RPC layer (which thinks in
// terms of requests, responses, and methods) and the framer/transport layer
// (which thinks in terms of raw byte slices).
//
// Think of it like a human translator at a phone call between diplomats: the
// diplomat says something (RpcMessage), the translator converts it to the other
// language ([]byte), the words travel over the phone line (framer + transport),
// and the other translator converts back (Decode). The phone line doesn't know
// anything about the languages; the diplomats don't know anything about the phone.
//
// # Codec is stateless
//
// A single RpcCodec instance should be usable for multiple Encode/Decode calls
// in any order. Codecs must not accumulate state between calls. This makes them
// safe to share across goroutines if needed.
//
// # Concrete implementations (live in codec-specific packages)
//
//   - JsonCodec    — encodes as JSON, V = json.RawMessage or map[string]any
//   - MsgpackCodec — encodes as MessagePack, V = rmpv.Value
//   - ProtobufCodec— encodes as Protobuf, V = proto.Message
//
// # The V type parameter
//
// V is the codec's native dynamic value type. For JSON it is typically
// map[string]interface{} or json.RawMessage. For MessagePack it is the library's
// Value union type. The rpc layer never inspects V — it treats it as an opaque
// payload that flows through handlers unchanged.

// RpcCodec translates between RpcMessage[V] and raw bytes.
//
// Implementors are responsible for two things:
//
//  1. Encoding: converting a typed RpcMessage[V] into the byte representation
//     that the chosen serialization format specifies. The result is handed to
//     RpcFramer.WriteFrame — no framing envelopes should be included.
//
//  2. Decoding: converting raw bytes (as produced by RpcFramer.ReadFrame) into
//     a typed RpcMessage[V]. On failure the codec returns an RpcErrorResponse
//     with ParseError or InvalidRequest code so the server can send a proper
//     error reply.
//
// Example implementation sketch for JSON:
//
//	type JsonCodec struct{}
//
//	func (c *JsonCodec) Encode(msg rpc.RpcMessage[json.RawMessage]) ([]byte, error) {
//	    switch m := msg.(type) {
//	    case *rpc.RpcRequest[json.RawMessage]:
//	        return json.Marshal(map[string]any{"jsonrpc":"2.0","id":m.Id,"method":m.Method,...})
//	    // ...
//	    }
//	}
//
//	func (c *JsonCodec) Decode(data []byte) (rpc.RpcMessage[json.RawMessage], error) {
//	    var obj map[string]any
//	    if err := json.Unmarshal(data, &obj); err != nil {
//	        return nil, &rpc.RpcErrorResponse[json.RawMessage]{Code: rpc.ParseError, ...}
//	    }
//	    // discriminate on key presence ...
//	}
type RpcCodec[V any] interface {
	// Encode serializes an RpcMessage to bytes ready for the framer.
	//
	// The returned bytes must be exactly the payload — no framing envelope
	// (no Content-Length header, no length prefix, no newline). The framer
	// adds the envelope.
	//
	// Encode must succeed for any valid RpcMessage[V]. Returning an error here
	// indicates a programming error (e.g., an un-marshalable V value), not a
	// normal operational failure.
	Encode(msg RpcMessage[V]) ([]byte, error)

	// Decode deserializes a raw byte slice (as returned by RpcFramer.ReadFrame)
	// into a typed RpcMessage[V].
	//
	// On failure, Decode must return nil and a non-nil error. The error should
	// be an *RpcErrorResponse[V] so the server can forward it as a proper error
	// reply (with null id if the id could not be recovered).
	//
	// Failures fall into two categories:
	//   - ParseError (-32700): bytes are not valid for the encoding format.
	//   - InvalidRequest (-32600): bytes decoded fine but do not represent a
	//     valid RPC message shape.
	Decode(data []byte) (RpcMessage[V], error)
}
