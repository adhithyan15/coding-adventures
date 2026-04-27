package com.codingadventures.inmemorydatastoreprotocol;

import com.codingadventures.respprotocol.RespValue;

import java.util.List;

public sealed interface EngineResponse permits EngineResponse.SimpleString, EngineResponse.ErrorString, EngineResponse.IntegerValue, EngineResponse.BulkString, EngineResponse.ArrayValue {
    RespValue toRespValue();

    record SimpleString(String value) implements EngineResponse {
        @Override
        public RespValue toRespValue() {
            return new RespValue.SimpleString(value);
        }
    }

    record ErrorString(String value) implements EngineResponse {
        @Override
        public RespValue toRespValue() {
            return new RespValue.ErrorString(value);
        }
    }

    record IntegerValue(long value) implements EngineResponse {
        @Override
        public RespValue toRespValue() {
            return new RespValue.IntegerValue(value);
        }
    }

    record BulkString(byte[] value) implements EngineResponse {
        @Override
        public RespValue toRespValue() {
            return new RespValue.BulkString(value);
        }
    }

    record ArrayValue(List<EngineResponse> value) implements EngineResponse {
        @Override
        public RespValue toRespValue() {
            if (value == null) {
                return new RespValue.ArrayValue(null);
            }
            return new RespValue.ArrayValue(value.stream().map(EngineResponse::toRespValue).toList());
        }
    }

    static SimpleString simpleString(String value) {
        return new SimpleString(value);
    }

    static ErrorString error(String value) {
        return new ErrorString(value);
    }

    static IntegerValue integer(long value) {
        return new IntegerValue(value);
    }

    static BulkString bulkString(byte[] value) {
        return new BulkString(value);
    }

    static BulkString nullBulkString() {
        return new BulkString(null);
    }

    static ArrayValue array(List<EngineResponse> value) {
        return new ArrayValue(value);
    }

    static SimpleString ok() {
        return new SimpleString("OK");
    }
}
