# Changelog

All notable changes to the `alexa-hello` package will be documented in this file.

## [0.1.0] - 2026-03-30

### Added
- Initial implementation of the Alexa "Hello Adhithya" skill
- `LaunchRequestHandler` — responds with "Hello Adhithya!" when the skill is opened
- `HelpIntentHandler` — explains what the skill does
- `CancelAndStopIntentHandler` — says goodbye when the user exits
- `FallbackIntentHandler` — handles unrecognized utterances gracefully
- `SessionEndedRequestHandler` — clean session termination
- `CatchAllExceptionHandler` — friendly error recovery
- Interaction model for en-US locale with built-in intents
- Skill manifest (`skill.json`) for reference
- Comprehensive unit tests covering all handlers and integration
