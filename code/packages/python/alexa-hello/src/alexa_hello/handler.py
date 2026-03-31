# ============================================================================
# handler.py — The Brain of Our Alexa Skill
# ============================================================================
#
# How Alexa Skills Work (The Big Picture)
# ----------------------------------------
#
# When you say "Alexa, open hello adhithya", here's what happens:
#
#   ┌─────────┐    ┌───────────────┐    ┌──────────────┐    ┌────────────┐
#   │  You    │───▶│  Echo Device  │───▶│  Alexa Cloud  │───▶│  Our Code  │
#   │ (voice) │    │ (microphone)  │    │  (NLP engine) │    │  (Lambda)  │
#   └─────────┘    └───────────────┘    └──────────────┘    └────────────┘
#                                              │
#                                              ▼
#                                    Converts speech to a
#                                    structured JSON request
#                                    (LaunchRequest, IntentRequest, etc.)
#
# Our code receives a JSON request and returns a JSON response containing
# the text Alexa should speak back. That's it!
#
# Request Types
# -------------
#
# Alexa sends us different request types depending on what the user said:
#
#   ┌─────────────────────┬──────────────────────────────────────────────┐
#   │ Request Type        │ When It's Sent                              │
#   ├─────────────────────┼──────────────────────────────────────────────┤
#   │ LaunchRequest       │ User opens the skill without a specific ask │
#   │ IntentRequest       │ User says something that maps to an intent  │
#   │ SessionEndedRequest │ User exits or an error occurs               │
#   └─────────────────────┴──────────────────────────────────────────────┘
#
# For our simple skill, we only care about:
#   - LaunchRequest → Say "Hello Adhithya!"
#   - HelpIntent    → Tell the user what this skill does
#   - Cancel/Stop   → Say goodbye
#   - Fallback      → Handle anything unexpected
#
# The ASK SDK (Alexa Skills Kit)
# ------------------------------
#
# Amazon provides a Python SDK that handles all the JSON parsing for us.
# We just write "handler" classes — each one declares which request types
# it can handle, and what to do when it gets one. The SDK's dispatcher
# tries each handler in order until one says "I can handle this!"
#
# ============================================================================

from __future__ import annotations

from ask_sdk_core.dispatch_components import (
    AbstractExceptionHandler,
    AbstractRequestHandler,
)
from ask_sdk_core.handler_input import HandlerInput
from ask_sdk_core.skill_builder import SkillBuilder
from ask_sdk_model import Response
from ask_sdk_model.ui import SimpleCard

# ── The Greeting ─────────────────────────────────────────────────────────
# This is the whole point of the skill! Change this string to personalize
# the greeting for anyone.

GREETING = "Hello Adhithya!"

# ── Handler 1: LaunchRequest ─────────────────────────────────────────────
#
# This fires when the user says "Alexa, open hello adhithya" without
# any further instructions. It's the "front door" of the skill.
#
# We respond with our greeting and a card (the visual element shown
# in the Alexa app on the user's phone).


class LaunchRequestHandler(AbstractRequestHandler):
    """Handles the initial launch of the skill.

    Example: "Alexa, open hello adhithya"
    """

    def can_handle(self, handler_input: HandlerInput) -> bool:
        """Return True if this is a LaunchRequest.

        The SDK calls can_handle() on each registered handler, in order,
        until one returns True. Think of it like a chain of responsibility.
        """
        from ask_sdk_core.utils import is_request_type

        return is_request_type("LaunchRequest")(handler_input)

    def handle(self, handler_input: HandlerInput) -> Response:
        """Speak the greeting and show a card in the Alexa app."""
        return (
            handler_input.response_builder.speak(GREETING)
            .set_card(SimpleCard("Hello!", GREETING))
            .set_should_end_session(True)
            .response
        )


# ── Handler 2: HelpIntent ───────────────────────────────────────────────
#
# Built-in intent that fires when the user says "Alexa, help" while
# inside the skill. We tell them what the skill does.


class HelpIntentHandler(AbstractRequestHandler):
    """Handles AMAZON.HelpIntent — tells the user what this skill does."""

    def can_handle(self, handler_input: HandlerInput) -> bool:
        """Return True if this is the built-in Help intent."""
        from ask_sdk_core.utils import is_intent_name

        return is_intent_name("AMAZON.HelpIntent")(handler_input)

    def handle(self, handler_input: HandlerInput) -> Response:
        """Explain what the skill does and keep the session open."""
        speech = "This skill greets you by name. Just open it to hear your greeting!"
        return (
            handler_input.response_builder.speak(speech)
            .set_card(SimpleCard("Help", speech))
            .set_should_end_session(False)
            .response
        )


# ── Handler 3: Cancel & Stop ────────────────────────────────────────────
#
# Built-in intents for "Alexa, cancel" and "Alexa, stop". We handle
# both with the same handler since the behavior is identical: say goodbye.


class CancelAndStopIntentHandler(AbstractRequestHandler):
    """Handles AMAZON.CancelIntent and AMAZON.StopIntent."""

    def can_handle(self, handler_input: HandlerInput) -> bool:
        """Return True if this is Cancel or Stop."""
        from ask_sdk_core.utils import is_intent_name

        return is_intent_name("AMAZON.CancelIntent")(
            handler_input
        ) or is_intent_name("AMAZON.StopIntent")(handler_input)

    def handle(self, handler_input: HandlerInput) -> Response:
        """Say goodbye and end the session."""
        return (
            handler_input.response_builder.speak("Goodbye!")
            .set_card(SimpleCard("Goodbye", "See you next time!"))
            .set_should_end_session(True)
            .response
        )


# ── Handler 4: Fallback ─────────────────────────────────────────────────
#
# Catches anything the skill doesn't understand. This is a safety net
# so the user always gets a response rather than silence.


class FallbackIntentHandler(AbstractRequestHandler):
    """Handles AMAZON.FallbackIntent — catches unrecognized utterances."""

    def can_handle(self, handler_input: HandlerInput) -> bool:
        """Return True if this is the Fallback intent."""
        from ask_sdk_core.utils import is_intent_name

        return is_intent_name("AMAZON.FallbackIntent")(handler_input)

    def handle(self, handler_input: HandlerInput) -> Response:
        """Let the user know we didn't understand and suggest trying again."""
        speech = (
            "Sorry, I didn't understand that. "
            "You can just open the skill to hear your greeting."
        )
        return (
            handler_input.response_builder.speak(speech)
            .set_card(SimpleCard("Hmm...", speech))
            .set_should_end_session(False)
            .response
        )


# ── Handler 5: Session Ended ────────────────────────────────────────────
#
# Fires when the session ends for any reason (user said stop, timeout,
# or error). We can't send a response here — it's just for cleanup.


class SessionEndedRequestHandler(AbstractRequestHandler):
    """Handles SessionEndedRequest — cleanup when the session ends."""

    def can_handle(self, handler_input: HandlerInput) -> bool:
        """Return True if this is a SessionEndedRequest."""
        from ask_sdk_core.utils import is_request_type

        return is_request_type("SessionEndedRequest")(handler_input)

    def handle(self, handler_input: HandlerInput) -> Response:
        """Nothing to clean up for this simple skill."""
        return handler_input.response_builder.response


# ── Error Handler ────────────────────────────────────────────────────────
#
# Catches any unhandled exceptions so the skill doesn't crash silently.
# In production, you'd log the error; here we just apologize.


class CatchAllExceptionHandler(AbstractExceptionHandler):
    """Catches all exceptions and returns a friendly error message."""

    def can_handle(
        self, handler_input: HandlerInput, exception: Exception
    ) -> bool:
        """Handle all exceptions — this is the last line of defense."""
        return True

    def handle(self, handler_input: HandlerInput, exception: Exception) -> Response:
        """Apologize and end the session."""
        speech = "Sorry, something went wrong. Please try again."
        return (
            handler_input.response_builder.speak(speech)
            .set_card(SimpleCard("Error", speech))
            .set_should_end_session(True)
            .response
        )


# ============================================================================
# Skill Builder — Wiring It All Together
# ============================================================================
#
# The SkillBuilder is the assembly line: we register our handlers in
# priority order (first match wins), then call .lambda_handler() to get
# the function that AWS Lambda will invoke.
#
#   ┌──────────────────────┐
#   │   Incoming Request   │
#   └──────────┬───────────┘
#              │
#              ▼
#   ┌──────────────────────┐   can_handle() == True?
#   │  LaunchRequestHandler │──────────────────────▶ handle() → Response
#   └──────────┬───────────┘         No
#              │                      │
#              ▼                      ▼
#   ┌──────────────────────┐   can_handle() == True?
#   │   HelpIntentHandler  │──────────────────────▶ handle() → Response
#   └──────────┬───────────┘         No
#              │                      │
#              ▼                      ▼
#          ... and so on until a handler matches ...
#
# ============================================================================

sb = SkillBuilder()

# Request handlers — order matters! First match wins.
sb.add_request_handler(LaunchRequestHandler())
sb.add_request_handler(HelpIntentHandler())
sb.add_request_handler(CancelAndStopIntentHandler())
sb.add_request_handler(FallbackIntentHandler())
sb.add_request_handler(SessionEndedRequestHandler())

# Exception handler — catches anything that slips through.
sb.add_exception_handler(CatchAllExceptionHandler())

# This is the entry point AWS Lambda calls.
# When Lambda receives a request from Alexa, it calls:
#   handler(event, context)
# The SDK parses the event JSON and routes it to our handlers.
handler = sb.lambda_handler()
