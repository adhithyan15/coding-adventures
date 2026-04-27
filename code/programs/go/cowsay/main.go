package main

import (
	"bufio"
	"fmt"
	"io"
	"os"
	"path/filepath"
	"regexp"
	"strings"

	clibuilder "github.com/adhithyan15/coding-adventures/code/packages/go/cli-builder"
)

func wrapText(text string, width int) []string {
	if len(text) <= width {
		return []string{text}
	}

	var lines []string
	words := strings.Fields(text)
	if len(words) == 0 {
		return []string{""}
	}

	currentLine := ""
	for _, word := range words {
		if len(currentLine)+len(word)+1 <= width {
			if currentLine == "" {
				currentLine = word
			} else {
				currentLine += " " + word
			}
		} else {
			lines = append(lines, currentLine)
			currentLine = word
		}
	}
	if currentLine != "" {
		lines = append(lines, currentLine)
	}
	return lines
}

func formatBubble(lines []string, isThink bool) string {
	if len(lines) == 0 {
		return ""
	}

	maxLen := 0
	for _, line := range lines {
		if len(line) > maxLen {
			maxLen = len(line)
		}
	}

	borderTop := " " + strings.Repeat("_", maxLen+2)
	borderBottom := " " + strings.Repeat("-", maxLen+2)

	var result []string
	result = append(result, borderTop)

	if len(lines) == 1 {
		start, end := "<", ">"
		if isThink {
			start, end = "(", ")"
		}
		result = append(result, fmt.Sprintf("%s %-*s %s", start, maxLen, lines[0], end))
	} else {
		for i, line := range lines {
			var start, end string
			if isThink {
				start, end = "(", ")"
			} else {
				if i == 0 {
					start, end = "/", "\\"
				} else if i == len(lines)-1 {
					start, end = "\\", "/"
				} else {
					start, end = "|", "|"
				}
			}
			result = append(result, fmt.Sprintf("%s %-*s %s", start, maxLen, line, end))
		}
	}

	result = append(result, borderBottom)
	return strings.Join(result, "\n")
}

func loadCow(cowName string, root string) string {
	cowPath := filepath.Join(root, "code", "specs", "cows", cowName+".cow")
	if _, err := os.Stat(cowPath); os.IsNotExist(err) {
		cowPath = filepath.Join(root, "code", "specs", "cows", "default.cow")
	}

	content, err := os.ReadFile(cowPath)
	if err != nil {
		return "Error loading cow"
	}

	// Simple parser for $the_cow = <<EOC; ... EOC
	re := regexp.MustCompile(`(?s)<<EOC;\n(.*?)EOC`)
	match := re.FindStringSubmatch(string(content))
	if len(match) > 1 {
		return match[1]
	}
	return string(content)
}

func main() {
	// Find repo root
	executablePath, _ := os.Executable()
	_ = executablePath // Unused if we use relative paths from CWD
	
	// Better way to find root in this environment
	root, _ := os.Getwd()
	// If we are in code/programs/go/cowsay, we need to go up 4 levels
	// But let's assume we run from the repo root for simplicity during testing
	// Actually, let's try to find it.
	for i := 0; i < 10; i++ {
		if _, err := os.Stat(filepath.Join(root, "code/specs/cowsay.json")); err == nil {
			break
		}
		root = filepath.Dir(root)
	}

	specPath := filepath.Join(root, "code/specs/cowsay.json")
	parser, err := clibuilder.NewParser(specPath, os.Args)
	if err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}

	result, err := parser.Parse()
	if err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}

	switch r := result.(type) {
	case *clibuilder.HelpResult:
		fmt.Println(r.Text)
		os.Exit(0)
	case *clibuilder.VersionResult:
		fmt.Println(r.Version)
		os.Exit(0)
	case *clibuilder.ParseResult:
		handleParseResult(r, root)
	}
}

func handleParseResult(r *clibuilder.ParseResult, root string) {
	flags := r.Flags
	args := r.Arguments

	// Handle message
	var message string
	messageParts, ok := args["message"].([]any)
	if !ok || len(messageParts) == 0 {
		// Try to read from stdin
		stat, _ := os.Stdin.Stat()
		if (stat.Mode() & os.ModeCharDevice) == 0 {
			reader := bufio.NewReader(os.Stdin)
			content, _ := io.ReadAll(reader)
			message = strings.TrimSpace(string(content))
		} else {
			return
		}
	} else {
		parts := make([]string, len(messageParts))
		for i, p := range messageParts {
			parts[i] = fmt.Sprint(p)
		}
		message = strings.Join(parts, " ")
	}

	if message == "" {
		return
	}

	// Handle modes
	eyes := "oo"
	if e, ok := flags["eyes"].(string); ok {
		eyes = e
	}
	tongue := "  "
	if t, ok := flags["tongue"].(string); ok {
		tongue = t
	}

	if b, ok := flags["borg"].(bool); ok && b {
		eyes = "=="
	}
	if d, ok := flags["dead"].(bool); ok && d {
		eyes = "XX"
		tongue = "U "
	}
	if g, ok := flags["greedy"].(bool); ok && g {
		eyes = "$$"
	}
	if p, ok := flags["paranoid"].(bool); ok && p {
		eyes = "@@"
	}
	if s, ok := flags["stoned"].(bool); ok && s {
		eyes = "xx"
		tongue = "U "
	}
	if t, ok := flags["tired"].(bool); ok && t {
		eyes = "--"
	}
	if w, ok := flags["wired"].(bool); ok && w {
		eyes = "OO"
	}
	if y, ok := flags["youthful"].(bool); ok && y {
		eyes = ".."
	}

	// Force 2 chars
	if len(eyes) < 2 {
		eyes = (eyes + "  ")[:2]
	} else if len(eyes) > 2 {
		eyes = eyes[:2]
	}
	if len(tongue) < 2 {
		tongue = (tongue + "  ")[:2]
	} else if len(tongue) > 2 {
		tongue = tongue[:2]
	}

	// Handle wrapping
	var lines []string
	nowrap, _ := flags["nowrap"].(bool)
	if nowrap {
		lines = strings.Split(message, "\n")
	} else {
		width := 40
		if w, ok := flags["width"].(int); ok {
			width = w
		} else if wf, ok := flags["width"].(float64); ok {
			width = int(wf)
		}
		
		for _, line := range strings.Split(message, "\n") {
			if line == "" {
				lines = append(lines, "")
			} else {
				lines = append(lines, wrapText(line, width)...)
			}
		}
	}

	// Handle speech vs thought
	isThink, _ := flags["think"].(bool)
	if filepath.Base(os.Args[0]) == "cowthink" {
		isThink = true
	}
	thoughts := "\\"
	if isThink {
		thoughts = "o"
	}

	// Generate bubble
	bubble := formatBubble(lines, isThink)

	// Load and render cow
	cowfile := "default"
	if cf, ok := flags["cowfile"].(string); ok {
		cowfile = cf
	}
	cowTemplate := loadCow(cowfile, root)

	// Replace placeholders
	cow := strings.ReplaceAll(cowTemplate, "$eyes", eyes)
	cow = strings.ReplaceAll(cow, "$tongue", tongue)
	cow = strings.ReplaceAll(cow, "$thoughts", thoughts)
	
	// Final unescape
	cow = strings.ReplaceAll(cow, "\\\\", "\\")

	fmt.Println(bubble)
	fmt.Println(cow)
}
