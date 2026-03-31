# ============================================================================
# test_handler.py — Unit Tests for the Alexa Hello Skill
# ============================================================================
#
# How to Test an Alexa Skill Locally
# -----------------------------------
#
# Alexa skills receive JSON requests and return JSON responses. We don't
# need a real Echo device or AWS Lambda to test — we can simulate the
# exact JSON that Alexa would send and verify our responses.
#
# The ASK SDK provides test utilities, but for maximum clarity we'll
# build our own request objects. This way you can see exactly what
# Alexa sends and what we return.
#
# Test Structure
# ──────────────
#
# Each test follows this pattern:
#   1. Build a fake Alexa request (JSON → Python dict)
#   2. Feed it to our skill's handler
#   3. Assert the response contains the right speech text
#
# ============================================================================

from __future__ import annotations

import json
from typing import Any
from unittest.mock import MagicMock

from ask_sdk_core.handler_input import HandlerInput
from ask_sdk_core.serialize import DefaultSerializer
from ask_sdk_model import RequestEnvelope

from alexa_hello.handler import (
    GREETING,
    CancelAndStopIntentHandler,
    CatchAllExceptionHandler,
    FallbackIntentHandler,
    HelpIntentHandler,
    LaunchRequestHandler,
    SessionEndedRequestHandler,
    sb,
)

# ============================================================================
# Helper: Build Alexa Request Envelopes
# ============================================================================
#
# Alexa sends a "RequestEnvelope" — a JSON structure containing:
#   - version: always "1.0"
#   - session: info about the current conversation
#   - request: the actual request (LaunchRequest, IntentRequest, etc.)
#
# We build these as Python dicts, then deserialize them into the SDK's
# model objects using RequestEnvelope.deserialize().
# ============================================================================


def _build_request_envelope(request: dict[str, Any]) -> dict[str, Any]:
    """Build a minimal Alexa RequestEnvelope dict.

    This is the outer wrapper that Alexa sends to Lambda. The `request`
    parameter is the inner request object (LaunchRequest, IntentRequest, etc.).

    A real request has more fields (context, session attributes, etc.),
    but the SDK only requires these for our handlers to work.
    """
    return {
        "version": "1.0",
        "session": {
            "new": True,
            "sessionId": "amzn1.echo-api.session.test-session-id",
            "application": {
                "applicationId": "amzn1.ask.skill.test-skill-id"
            },
            "user": {
                "userId": "amzn1.ask.account.test-user-id"
            },
        },
        "request": request,
        "context": {
            "System": {
                "application": {
                    "applicationId": "amzn1.ask.skill.test-skill-id"
                },
                "user": {
                    "userId": "amzn1.ask.account.test-user-id"
                },
            }
        },
    }


def _build_launch_request() -> dict[str, Any]:
    """Build a LaunchRequest — what Alexa sends when you say 'open hello adhithya'."""
    return _build_request_envelope(
        {
            "type": "LaunchRequest",
            "requestId": "amzn1.echo-api.request.test-request-id",
            "timestamp": "2026-03-30T12:00:00Z",
            "locale": "en-US",
        }
    )


def _build_intent_request(intent_name: str) -> dict[str, Any]:
    """Build an IntentRequest for a given intent name.

    IntentRequests are sent when Alexa recognizes a specific intent
    (like AMAZON.HelpIntent, AMAZON.StopIntent, etc.).
    """
    return _build_request_envelope(
        {
            "type": "IntentRequest",
            "requestId": "amzn1.echo-api.request.test-request-id",
            "timestamp": "2026-03-30T12:00:00Z",
            "locale": "en-US",
            "intent": {
                "name": intent_name,
                "confirmationStatus": "NONE",
            },
        }
    )


def _build_session_ended_request() -> dict[str, Any]:
    """Build a SessionEndedRequest — sent when the user exits the skill."""
    return _build_request_envelope(
        {
            "type": "SessionEndedRequest",
            "requestId": "amzn1.echo-api.request.test-request-id",
            "timestamp": "2026-03-30T12:00:00Z",
            "locale": "en-US",
            "reason": "USER_INITIATED",
        }
    )


def _make_handler_input(request_dict: dict[str, Any]) -> HandlerInput:
    """Convert a request dict into a HandlerInput the SDK handlers expect.

    The SDK's HandlerInput wraps a deserialized RequestEnvelope along with
    a response builder and other utilities. We use the SDK's own
    deserialization to ensure our test requests are structurally valid.

    The DefaultSerializer.deserialize() expects a JSON *string*, not a dict,
    so we serialize to JSON first, then let the SDK parse it back into
    its typed model objects.
    """
    serializer = DefaultSerializer()
    envelope = serializer.deserialize(json.dumps(request_dict), RequestEnvelope)
    handler_input = HandlerInput(request_envelope=envelope)
    return handler_input


# ============================================================================
# Tests: can_handle() — Does each handler recognize the right request?
# ============================================================================


class TestCanHandle:
    """Verify each handler's can_handle() returns True/False correctly.

    This is important because the SDK dispatches requests by iterating
    through handlers and calling can_handle() on each one. If a handler
    incorrectly claims it can handle a request, we get wrong behavior.
    """

    def test_launch_handler_recognizes_launch_request(self) -> None:
        """LaunchRequestHandler should handle LaunchRequest."""
        hi = _make_handler_input(_build_launch_request())
        assert LaunchRequestHandler().can_handle(hi) is True

    def test_launch_handler_rejects_intent_request(self) -> None:
        """LaunchRequestHandler should NOT handle IntentRequests."""
        hi = _make_handler_input(_build_intent_request("AMAZON.HelpIntent"))
        assert LaunchRequestHandler().can_handle(hi) is False

    def test_help_handler_recognizes_help_intent(self) -> None:
        """HelpIntentHandler should handle AMAZON.HelpIntent."""
        hi = _make_handler_input(_build_intent_request("AMAZON.HelpIntent"))
        assert HelpIntentHandler().can_handle(hi) is True

    def test_help_handler_rejects_other_intents(self) -> None:
        """HelpIntentHandler should NOT handle AMAZON.StopIntent."""
        hi = _make_handler_input(_build_intent_request("AMAZON.StopIntent"))
        assert HelpIntentHandler().can_handle(hi) is False

    def test_cancel_stop_handler_recognizes_cancel(self) -> None:
        """CancelAndStopIntentHandler should handle AMAZON.CancelIntent."""
        hi = _make_handler_input(_build_intent_request("AMAZON.CancelIntent"))
        assert CancelAndStopIntentHandler().can_handle(hi) is True

    def test_cancel_stop_handler_recognizes_stop(self) -> None:
        """CancelAndStopIntentHandler should handle AMAZON.StopIntent."""
        hi = _make_handler_input(_build_intent_request("AMAZON.StopIntent"))
        assert CancelAndStopIntentHandler().can_handle(hi) is True

    def test_cancel_stop_handler_rejects_help(self) -> None:
        """CancelAndStopIntentHandler should NOT handle AMAZON.HelpIntent."""
        hi = _make_handler_input(_build_intent_request("AMAZON.HelpIntent"))
        assert CancelAndStopIntentHandler().can_handle(hi) is False

    def test_fallback_handler_recognizes_fallback(self) -> None:
        """FallbackIntentHandler should handle AMAZON.FallbackIntent."""
        hi = _make_handler_input(_build_intent_request("AMAZON.FallbackIntent"))
        assert FallbackIntentHandler().can_handle(hi) is True

    def test_fallback_handler_rejects_help(self) -> None:
        """FallbackIntentHandler should NOT handle AMAZON.HelpIntent."""
        hi = _make_handler_input(_build_intent_request("AMAZON.HelpIntent"))
        assert FallbackIntentHandler().can_handle(hi) is False

    def test_session_ended_handler_recognizes_session_ended(self) -> None:
        """SessionEndedRequestHandler should handle SessionEndedRequest."""
        hi = _make_handler_input(_build_session_ended_request())
        assert SessionEndedRequestHandler().can_handle(hi) is True

    def test_session_ended_handler_rejects_launch(self) -> None:
        """SessionEndedRequestHandler should NOT handle LaunchRequest."""
        hi = _make_handler_input(_build_launch_request())
        assert SessionEndedRequestHandler().can_handle(hi) is False


# ============================================================================
# Tests: handle() — Does each handler produce the right speech output?
# ============================================================================


class TestHandle:
    """Verify each handler returns the expected speech and card content."""

    def test_launch_says_hello(self) -> None:
        """LaunchRequest should produce the greeting."""
        hi = _make_handler_input(_build_launch_request())
        response = LaunchRequestHandler().handle(hi)

        # The response's output_speech contains what Alexa will say
        assert response.output_speech.ssml == f"<speak>{GREETING}</speak>"
        assert response.should_end_session is True

    def test_launch_has_card(self) -> None:
        """LaunchRequest should include a SimpleCard for the Alexa app."""
        hi = _make_handler_input(_build_launch_request())
        response = LaunchRequestHandler().handle(hi)

        assert response.card is not None
        assert response.card.title == "Hello!"
        assert response.card.content == GREETING

    def test_help_explains_skill(self) -> None:
        """HelpIntent should explain what the skill does."""
        hi = _make_handler_input(_build_intent_request("AMAZON.HelpIntent"))
        response = HelpIntentHandler().handle(hi)

        assert "greets you" in response.output_speech.ssml.lower()
        assert response.should_end_session is False

    def test_cancel_says_goodbye(self) -> None:
        """CancelIntent should say goodbye and end the session."""
        hi = _make_handler_input(_build_intent_request("AMAZON.CancelIntent"))
        response = CancelAndStopIntentHandler().handle(hi)

        assert "goodbye" in response.output_speech.ssml.lower()
        assert response.should_end_session is True

    def test_stop_says_goodbye(self) -> None:
        """StopIntent should say goodbye and end the session."""
        hi = _make_handler_input(_build_intent_request("AMAZON.StopIntent"))
        response = CancelAndStopIntentHandler().handle(hi)

        assert "goodbye" in response.output_speech.ssml.lower()
        assert response.should_end_session is True

    def test_fallback_apologizes(self) -> None:
        """FallbackIntent should apologize and keep the session open."""
        hi = _make_handler_input(_build_intent_request("AMAZON.FallbackIntent"))
        response = FallbackIntentHandler().handle(hi)

        assert "sorry" in response.output_speech.ssml.lower()
        assert response.should_end_session is False

    def test_session_ended_returns_empty_response(self) -> None:
        """SessionEndedRequest should return an empty response (no speech)."""
        hi = _make_handler_input(_build_session_ended_request())
        response = SessionEndedRequestHandler().handle(hi)

        assert response.output_speech is None

    def test_exception_handler_apologizes(self) -> None:
        """Exception handler should apologize and end the session."""
        hi = _make_handler_input(_build_launch_request())
        exc = RuntimeError("Something broke")
        response = CatchAllExceptionHandler().handle(hi, exc)

        assert "sorry" in response.output_speech.ssml.lower()
        assert response.should_end_session is True

    def test_exception_handler_catches_all(self) -> None:
        """Exception handler should handle any exception type."""
        hi = _make_handler_input(_build_launch_request())
        assert CatchAllExceptionHandler().can_handle(hi, ValueError("test")) is True
        assert CatchAllExceptionHandler().can_handle(hi, RuntimeError("test")) is True


# ============================================================================
# Tests: Full Skill Integration — End-to-End via the SkillBuilder
# ============================================================================
#
# These tests invoke the skill the same way AWS Lambda would: by calling
# the lambda_handler with a raw JSON event. This verifies that our
# handlers are properly registered and dispatched.
# ============================================================================


class TestSkillIntegration:
    """End-to-end tests using the actual skill builder's lambda_handler."""

    def test_lambda_handler_launch(self) -> None:
        """The full skill should respond to a LaunchRequest."""
        event = _build_launch_request()
        context = MagicMock()  # Lambda context (we don't use it)

        response = sb.lambda_handler()(event, context)

        # The lambda_handler returns a raw dict (JSON-serializable)
        assert "response" in response
        assert GREETING in response["response"]["outputSpeech"]["ssml"]

    def test_lambda_handler_help(self) -> None:
        """The full skill should respond to AMAZON.HelpIntent."""
        event = _build_intent_request("AMAZON.HelpIntent")
        context = MagicMock()

        response = sb.lambda_handler()(event, context)

        assert "greets you" in response["response"]["outputSpeech"]["ssml"].lower()

    def test_lambda_handler_stop(self) -> None:
        """The full skill should respond to AMAZON.StopIntent."""
        event = _build_intent_request("AMAZON.StopIntent")
        context = MagicMock()

        response = sb.lambda_handler()(event, context)

        assert "goodbye" in response["response"]["outputSpeech"]["ssml"].lower()

    def test_lambda_handler_fallback(self) -> None:
        """The full skill should respond to AMAZON.FallbackIntent."""
        event = _build_intent_request("AMAZON.FallbackIntent")
        context = MagicMock()

        response = sb.lambda_handler()(event, context)

        assert "sorry" in response["response"]["outputSpeech"]["ssml"].lower()

    def test_lambda_handler_session_ended(self) -> None:
        """The full skill should handle SessionEndedRequest without crashing."""
        event = _build_session_ended_request()
        context = MagicMock()

        response = sb.lambda_handler()(event, context)

        # SessionEndedRequest gets an empty response
        assert "response" in response


# ============================================================================
# Tests: Greeting Constant
# ============================================================================


class TestGreeting:
    """Verify the greeting constant is what we expect."""

    def test_greeting_contains_name(self) -> None:
        """The greeting should contain 'Adhithya'."""
        assert "Adhithya" in GREETING

    def test_greeting_is_friendly(self) -> None:
        """The greeting should start with 'Hello'."""
        assert GREETING.startswith("Hello")
