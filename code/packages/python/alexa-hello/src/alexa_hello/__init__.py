# ============================================================================
# alexa-hello — A Minimal Alexa Skill
# ============================================================================
#
# This is the simplest possible Alexa skill: when you say "Alexa, open
# hello adhithya", it responds with "Hello Adhithya!"
#
# Think of an Alexa skill like a phone call:
#
#   1. You dial a number    → "Alexa, open hello adhithya"  (invocation)
#   2. Someone picks up     → Alexa routes to our Lambda    (LaunchRequest)
#   3. They say hello       → We return speech: "Hello!"    (response)
#   4. You hang up          → "Alexa, stop"                 (SessionEndedRequest)
#
# The entire skill is in handler.py — just four request handlers that cover
# every possible thing a user can say to this skill.
# ============================================================================

"""A minimal Alexa skill that greets you by name."""

__version__ = "0.1.0"
