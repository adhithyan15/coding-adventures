package com.codingadventures.respprotocol;

import java.util.List;

public sealed interface RespValue permits RespValue.SimpleString, RespValue.ErrorString, RespValue.IntegerValue, RespValue.BulkString, RespValue.ArrayValue {
    record SimpleString(String value) implements RespValue {}
    record ErrorString(String value) implements RespValue {}
    record IntegerValue(long value) implements RespValue {}
    record BulkString(byte[] value) implements RespValue {
        public boolean isNull() {
            return value == null;
        }
    }
    record ArrayValue(List<RespValue> value) implements RespValue {
        public boolean isNull() {
            return value == null;
        }
    }
}
