import sys
import os
import json
import textwrap

# Add package paths to sys.path for monorepo development
ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "../../../.."))
sys.path.insert(0, os.path.join(ROOT, "code/packages/python/cli-builder/src"))
sys.path.insert(0, os.path.join(ROOT, "code/packages/python/state-machine/src"))
sys.path.insert(0, os.path.join(ROOT, "code/packages/python/directed-graph/src"))

from cli_builder import Parser, ParseResult, HelpResult, VersionResult, ParseErrors

def get_bubble_borders(is_think, length):
    if is_think:
        return "(", ")", "(", ")", "(", ")", "(", ")", "o"
    if length == 1:
        return "<", ">", "", "", "", "", "", "", "\\"
    return "/", "\\", "\\", "/", "|", "|", "|", "|", "\\"

def format_bubble(lines, is_think, width):
    if not lines:
        return ""
    
    max_len = max(len(line) for line in lines)
    border_top = " " + "_" * (max_len + 2)
    border_bottom = " " + "-" * (max_len + 2)
    
    result = [border_top]
    
    if len(lines) == 1:
        start, end = ("(", ")") if is_think else ("<", ">")
        result.append(f"{start} {lines[0].ljust(max_len)} {end}")
    else:
        for i, line in enumerate(lines):
            if i == 0:
                start, end = ("(", ")") if is_think else ("/", "\\")
            elif i == len(lines) - 1:
                start, end = ("(", ")") if is_think else ("\\", "/")
            else:
                start, end = ("(", ")") if is_think else ("|", "|")
            result.append(f"{start} {line.ljust(max_len)} {end}")
            
    result.append(border_bottom)
    return "\n".join(result)

def load_cow(cow_name, ROOT):
    cow_path = os.path.join(ROOT, f"code/specs/cows/{cow_name}.cow")
    if not os.path.exists(cow_path):
        # Fallback to default if not found
        cow_path = os.path.join(ROOT, "code/specs/cows/default.cow")
    
    with open(cow_path, "r") as f:
        content = f.read()
    
    # Simple parser for $the_cow = <<EOC; ... EOC
    import re
    match = re.search(r"<<EOC;\n(.*?)EOC", content, re.DOTALL)
    if match:
        return match.group(1)
    return content

def main():
    spec_path = os.path.join(ROOT, "code/specs/cowsay.json")
    
    try:
        parser = Parser(spec_path, sys.argv)
        result = parser.parse()
    except ParseErrors as e:
        print(e)
        sys.exit(1)
    except Exception as e:
        print(f"Error: {e}")
        sys.exit(1)

    if isinstance(result, HelpResult):
        print(result.text)
        return
    if isinstance(result, VersionResult):
        print(result.version)
        return
    
    # ParseResult
    flags = result.flags
    args = result.arguments
    
    # Handle message
    message_parts = args.get("message", [])
    if isinstance(message_parts, str):
        message = message_parts
    elif not message_parts:
        # Check stdin
        if not sys.stdin.isatty():
            message = sys.stdin.read().strip()
        else:
            # No message provided
            return
    else:
        message = " ".join(message_parts)
    
    if not message:
        return

    # Handle modes
    eyes = flags.get("eyes", "oo")
    tongue = flags.get("tongue", "  ")
    
    if flags.get("borg"): eyes = "=="
    if flags.get("dead"): 
        eyes = "XX"
        tongue = "U "
    if flags.get("greedy"): eyes = "$$"
    if flags.get("paranoid"): eyes = "@@"
    if flags.get("stoned"):
        eyes = "xx"
        tongue = "U "
    if flags.get("tired"): eyes = "--"
    if flags.get("wired"): eyes = "OO"
    if flags.get("youthful"): eyes = ".."

    # Force 2 chars for eyes
    eyes = (eyes + "  ")[:2]
    tongue = (tongue + "  ")[:2]

    # Handle wrapping
    if flags.get("nowrap"):
        lines = message.splitlines()
    else:
        width = flags.get("width", 40)
        lines = []
        for line in message.splitlines():
            if not line:
                lines.append("")
            else:
                lines.extend(textwrap.wrap(line, width=width))

    # Handle speech vs thought
    is_think = flags.get("think", False)
    # Check if we were called as 'cowthink'
    if os.path.basename(sys.argv[0]) == "cowthink":
        is_think = True
        
    thoughts = "o" if is_think else "\\"
    
    # Generate bubble
    bubble = format_bubble(lines, is_think, flags.get("width", 40))
    
    # Load and render cow
    cow_template = load_cow(flags.get("cowfile", "default"), ROOT)
    
    # Replace placeholders
    cow = cow_template.replace("$eyes", eyes).replace("$tongue", tongue).replace("$thoughts", thoughts)
    
    # Final unescape for backslashes if they were escaped in the .cow file
    cow = cow.replace("\\\\", "\\")
    
    print(bubble)
    print(cow)

if __name__ == "__main__":
    main()
