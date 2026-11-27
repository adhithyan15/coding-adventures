package unixhistory

import "strings"

type Result struct {
	Output string  `json:"output"`
	Events []Event `json:"events"`
}

func RunCommand(command string) Result {
	kernel := NewKernel(DemoImage(), NewDescriptorTable(4))
	result := Result{Events: []Event{}}
	var output strings.Builder
	add := func(event Event) {
		result.Events = append(result.Events, event)
		if code, ok := event["error"].(string); ok {
			output.WriteString("error: " + code + "\n")
		}
	}
	words := strings.FieldsFunc(command, func(r rune) bool { return strings.ContainsRune(" \t\n\r\f\v", r) })
	switch {
	case len(words) == 0:
	case words[0] == "cat" && len(words) == 2:
		opened := kernel.Open(words[1])
		add(opened)
		if fd, ok := opened["fd"].(int); ok {
			for {
				read, data := kernel.Read(fd, 8)
				add(read)
				if data == "" {
					break
				}
				output.WriteString(data)
			}
			add(kernel.Close(fd))
		}
	case words[0] == "ls" && len(words) <= 2:
		path := "/"
		if len(words) == 2 {
			path = words[1]
		}
		names, listed := kernel.List(path)
		add(listed)
		for _, name := range names {
			output.WriteString(name + "\n")
		}
	default:
		output.WriteString("usage: cat PATH or ls [PATH]\n")
	}
	result.Output = output.String()
	return result
}
