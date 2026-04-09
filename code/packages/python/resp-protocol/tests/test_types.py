"""
Tests for RespError and related type behaviour.
"""

import pytest

from resp_protocol import RespError


class TestRespError:
    def test_message_attribute(self) -> None:
        err = RespError("ERR something went wrong")
        assert err.message == "ERR something went wrong"

    def test_error_type_first_word(self) -> None:
        err = RespError("ERR something went wrong")
        assert err.error_type == "ERR"

    def test_detail_rest(self) -> None:
        err = RespError("ERR something went wrong")
        assert err.detail == "something went wrong"

    def test_wrongtype(self) -> None:
        err = RespError("WRONGTYPE value is not a list")
        assert err.error_type == "WRONGTYPE"
        assert err.detail == "value is not a list"

    def test_single_word_message(self) -> None:
        # If there's no space, detail is empty
        err = RespError("ERR")
        assert err.error_type == "ERR"
        assert err.detail == ""

    def test_repr(self) -> None:
        err = RespError("ERR msg")
        assert "ERR msg" in repr(err)

    def test_equality(self) -> None:
        a = RespError("ERR msg")
        b = RespError("ERR msg")
        assert a == b

    def test_inequality(self) -> None:
        a = RespError("ERR msg1")
        b = RespError("ERR msg2")
        assert a != b

    def test_not_equal_to_str(self) -> None:
        err = RespError("ERR msg")
        assert err != "ERR msg"

    def test_hashable(self) -> None:
        err = RespError("ERR msg")
        # Should not raise
        s: set = {err}
        assert err in s
