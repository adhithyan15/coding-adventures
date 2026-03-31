# alexa-hello

A minimal Alexa skill that says "Hello Adhithya!" when launched. This is the simplest possible starting point for Alexa skill development — a foundation to build on incrementally.

## How It Works

```
  You                Echo Device          Alexa Cloud           Our Lambda
   │                     │                     │                     │
   │  "Alexa, open       │                     │                     │
   │   hello adhithya"   │                     │                     │
   │────────────────────▶│                     │                     │
   │                     │   Audio stream      │                     │
   │                     │────────────────────▶│                     │
   │                     │                     │  LaunchRequest JSON  │
   │                     │                     │────────────────────▶│
   │                     │                     │                     │
   │                     │                     │  "Hello Adhithya!"  │
   │                     │                     │◀────────────────────│
   │                     │   Speech audio      │                     │
   │                     │◀────────────────────│                     │
   │  "Hello Adhithya!"  │                     │                     │
   │◀────────────────────│                     │                     │
```

The skill has four handlers:

| Handler | Trigger | Response |
|---------|---------|----------|
| `LaunchRequestHandler` | "Alexa, open hello adhithya" | "Hello Adhithya!" |
| `HelpIntentHandler` | "Alexa, help" | Explains the skill |
| `CancelAndStopIntentHandler` | "Alexa, stop" / "Alexa, cancel" | "Goodbye!" |
| `FallbackIntentHandler` | Anything unrecognized | Friendly error |

## Project Structure

```
alexa-hello/
├── BUILD                         # Build system integration
├── README.md                     # This file
├── CHANGELOG.md                  # Version history
├── pyproject.toml                # Package config (hatchling)
├── skill.json                    # Alexa skill manifest (reference)
├── interactionModels/
│   └── en-US.json                # What Alexa understands (intents)
├── src/
│   └── alexa_hello/
│       ├── __init__.py           # Package init
│       └── handler.py            # Lambda handler (the brain)
└── tests/
    └── test_handler.py           # Unit + integration tests
```

## Running Tests

```bash
cd code/packages/python/alexa-hello
uv venv --quiet --clear
uv pip install -e ".[dev]" --quiet
.venv/bin/python -m pytest tests/ -v
```

## Deploying to AWS Lambda

1. **Create an AWS account** at aws.amazon.com (free tier)
2. **Create a Lambda function** (Python 3.12 runtime)
3. **Upload the code** as a ZIP containing `alexa_hello/` and its dependencies
4. **Create the Alexa skill** in the Alexa Developer Console
5. **Paste the interaction model** from `interactionModels/en-US.json`
6. **Set the endpoint** to your Lambda function's ARN
7. **Test** in the Alexa Simulator: "Alexa, open hello adhithya"

## Dependencies

- [`ask-sdk-core`](https://github.com/alexa/alexa-skills-kit-sdk-for-python) — The official Alexa Skills Kit SDK for Python

## How It Fits in the Stack

This is a standalone skill — it doesn't depend on any other packages in the monorepo. It serves as a learning exercise for voice-driven programming and cloud deployment (AWS Lambda).
