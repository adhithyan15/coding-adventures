import base64
import json
import sys


for line in sys.stdin:
    message = json.loads(line)
    response = {
        "protocol": "chief-agent-stdio-v1",
        "kind": "response",
        "input_message_id": message["message_id"],
        "content_type": "text/plain; charset=utf-8",
        "payload_b64": base64.b64encode(b"python-world").decode("ascii"),
    }
    sys.stdout.write(json.dumps(response, separators=(",", ":")) + "\n")
    sys.stdout.flush()
