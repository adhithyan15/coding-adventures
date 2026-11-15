package com.codingadventures.hamurabi;

/**
 * Functional result type representing either success ({@link Ok}) or failure ({@link Err}).
 *
 * @param <T> Success value type
 * @param <E> Error value type
 */
public sealed interface Result<T, E> permits Result.Ok, Result.Err {

    record Ok<T, E>(T value) implements Result<T, E> {
        @Override
        public boolean isOk() {
            return true;
        }

        @Override
        public boolean isErr() {
            return false;
        }

        @Override
        public T unwrap() {
            return value;
        }

        @Override
        public E unwrapErr() {
            throw new IllegalStateException("Called unwrapErr on Ok: " + value);
        }
    }

    record Err<T, E>(E error) implements Result<T, E> {
        @Override
        public boolean isOk() {
            return false;
        }

        @Override
        public boolean isErr() {
            return true;
        }

        @Override
        public T unwrap() {
            throw new IllegalStateException("Called unwrap on Err: " + error);
        }

        @Override
        public E unwrapErr() {
            return error;
        }
    }

    static <T, E> Result<T, E> ok(T value) {
        return new Ok<>(value);
    }

    static <T, E> Result<T, E> err(E error) {
        return new Err<>(error);
    }

    boolean isOk();
    boolean isErr();
    T unwrap();
    E unwrapErr();
}
