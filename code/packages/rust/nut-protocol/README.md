# nut-protocol

Strict, bounded framing for the read-only Network UPS Tools `LIST VAR`
exchange standardized by RFC 9271.

The package encodes one exact UPS-name request and decodes a correlated list
of variable names and quoted values. It rejects malformed escaping, duplicate
variables, mismatched UPS names, error replies, trailing records, and responses
outside fixed line, value, variable-count, and total-size limits.

Authentication, writable variables, instant commands, forced shutdown,
enumeration, subscriptions, and TLS negotiation are intentionally absent.
