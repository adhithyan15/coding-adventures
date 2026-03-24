require "json"
require "pathname"

# Add package paths to LOAD_PATH for monorepo development
ROOT = File.expand_path("../../../..", __dir__)
$LOAD_PATH.unshift(File.join(ROOT, "code/packages/ruby/cli_builder/lib"))
$LOAD_PATH.unshift(File.join(ROOT, "code/packages/ruby/state_machine/lib"))
$LOAD_PATH.unshift(File.join(ROOT, "code/packages/ruby/directed_graph/lib"))

require "coding_adventures_cli_builder"

def wrap_text(text, width)
  return [text] if text.length <= width
  text.scan(/.{1,#{width}}(?:\s|$)/).map(&:strip)
end

def format_bubble(lines, is_think)
  return "" if lines.empty?
  
  max_len = lines.map(&:length).max
  border_top = " " + "_" * (max_len + 2)
  border_bottom = " " + "-" * (max_len + 2)
  
  result = [border_top]
  
  if lines.length == 1
    start, finish = is_think ? ["(", ")"] : ["<", ">"]
    result << "#{start} #{lines[0].ljust(max_len)} #{finish}"
  else
    lines.each_with_index do |line, i|
      if i == 0
        start, finish = is_think ? ["(", ")"] : ["/", "\\"]
      elsif i == lines.length - 1
        start, finish = is_think ? ["(", ")"] : ["\\", "/"]
      else
        start, finish = is_think ? ["(", ")"] : ["|", "|"]
      end
      result << "#{start} #{line.ljust(max_len)} #{finish}"
    end
  end
  
  result << border_bottom
  result.join("\n")
end

def load_cow(cow_name, root)
  cow_path = File.join(root, "code/specs/cows/#{cow_name}.cow")
  cow_path = File.join(root, "code/specs/cows/default.cow") unless File.exist?(cow_path)
  
  content = File.read(cow_path)
  
  # Simple parser for $the_cow = <<EOC; ... EOC
  if content =~ /<<EOC;\n(.*?)EOC/m
    $1
  else
    content
  end
end

def main
  spec_path = File.join(ROOT, "code/specs/cowsay.json")
  
  begin
    # Ruby's ARGV starts after the script name, but Parser expects argv[0] to be the program
    parser = CodingAdventures::CliBuilder::Parser.new(spec_path, [$0, *ARGV])
    result = parser.parse
  rescue CodingAdventures::CliBuilder::ParseErrors => e
    e.errors.each { |err| warn err.message }
    exit 1
  rescue StandardError => e
    warn "Error: #{e.message}"
    exit 1
  end

  case result
  when CodingAdventures::CliBuilder::HelpResult
    puts result.text
    return
  when CodingAdventures::CliBuilder::VersionResult
    puts result.version
    return
  end
  
  # ParseResult
  flags = result.flags
  args = result.arguments
  
  # Handle message
  message_parts = args["message"] || []
  if message_parts.is_a?(String)
    message = message_parts
  elsif message_parts.empty?
    # Check stdin
    if !$stdin.tty?
      message = $stdin.read.strip
    else
      return
    end
  else
    message = message_parts.join(" ")
  end
  
  return if message.nil? || message.empty?

  # Handle modes
  eyes = flags["eyes"] || "oo"
  tongue = flags["tongue"] || "  "
  
  eyes = "==" if flags["borg"]
  if flags["dead"]
    eyes = "XX"
    tongue = "U "
  end
  eyes = "$$" if flags["greedy"]
  eyes = "@@" if flags["paranoid"]
  if flags["stoned"]
    eyes = "xx"
    tongue = "U "
  end
  eyes = "--" if flags["tired"]
  eyes = "OO" if flags["wired"]
  eyes = ".." if flags["youthful"]

  # Force 2 chars
  eyes = (eyes + "  ")[0...2]
  tongue = (tongue + "  ")[0...2]

  # Handle wrapping
  if flags["nowrap"]
    lines = message.split("\n")
  else
    width = flags["width"] || 40
    lines = []
    message.split("\n").each do |line|
      if line.empty?
        lines << ""
      else
        lines.concat(wrap_text(line, width))
      end
    end
  end

  # Handle speech vs thought
  is_think = flags["think"] || File.basename($0) == "cowthink"
  thoughts = is_think ? "o" : "\\"
  
  # Generate bubble
  bubble = format_bubble(lines, is_think)
  
  # Load and render cow
  cow_template = load_cow(flags["cowfile"] || "default", ROOT)
  
  # Replace placeholders
  cow = cow_template.gsub("$eyes", eyes).gsub("$tongue", tongue).gsub("$thoughts", thoughts)
  
  # Final unescape
  cow = cow.gsub("\\\\", "\\")
  
  puts bubble
  puts cow
end

main if __FILE__ == $0
