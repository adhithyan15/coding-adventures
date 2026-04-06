# frozen_string_literal: true

# Regex-based HTML Sanitizer
#
# Operates on an opaque HTML string with no DOM dependency. The sanitizer
# makes multiple passes over the string, each targeting a specific threat
# category:
#
#   Pass 1 — Strip HTML comments (can hide payloads; <!--...--> )
#   Pass 2 — Remove dangerous elements + their content (script, iframe, …)
#   Pass 3 — Process every surviving tag:
#               a. Strip drop_attributes (on*, srcdoc, formaction, custom)
#               b. Sanitize href/src attribute values (URL scheme check)
#               c. Sanitize/strip style attributes (CSS expression check)
#
# == Why regex and not a real HTML parser?
#
# The spec mandates regex/string operations for portability across
# Go, Python, Rust, Elixir, Lua, and edge JS runtimes — none of which
# share a common DOM API. A regex-based sanitizer can be ported to any
# language in ~200 lines. DOM parsing requires environment-specific code.
#
# == Security posture
#
# The regex approach handles well-formed HTML correctly. Pathological
# inputs (malformed tags, polyglots) are a known limitation. For
# maximum security, run BOTH the AST sanitizer (stage 1) THEN this
# HTML sanitizer (stage 2) — belt AND suspenders.
#
# == Regex architecture
#
# The core challenge is processing HTML tags without a real parser.
# We use a single tokenising regex that matches:
#   - Opening tags:  <tagname attr1="v1" attr2='v2' attr3=v3 ...>
#   - Closing tags:  </tagname>
#   - Self-closing:  <tagname ... />
#   - Comments:      <!-- ... -->
#   - Everything else: text between tags
#
# For each matched token we apply the policy, then reassemble the string.

require_relative "url_utils"

module CodingAdventures
  module DocumentHtmlSanitizer
    # ─── Public Entry Point ────────────────────────────────────────────────────

    # Sanitize an HTML string by stripping dangerous elements and attributes.
    #
    # @param html [String] The input HTML.
    # @param policy [HtmlSanitizationPolicy]
    # @return [String] The sanitized HTML.
    def self.sanitize_html(html, policy)
      result = html

      # Pass 1: Strip comments before anything else so they can't hide payloads.
      result = strip_comments(result) if policy.drop_comments

      # Pass 2: Drop dangerous elements (including all their content).
      unless policy.drop_elements.empty?
        result = drop_elements(result, policy.drop_elements)
      end

      # Pass 3: Process surviving tags — strip bad attributes, sanitize URLs,
      # sanitize style.
      process_tags(result, policy)
    end

    # ─── Pass 1: Comment stripping ─────────────────────────────────────────────
    #
    # HTML comments: <!-- arbitrary content -->
    #
    # We use /m (MULTILINE in Ruby = DOTALL — "." matches newlines) so a
    # comment that spans multiple lines is captured in one match.
    #
    # Attack vector being mitigated:
    #   <!--<script>alert(1)</script>-->  — hidden behind comment
    #   <!--[if IE]><script>…</script><![endif]-->  — conditional comment
    #
    # @param html [String]
    # @return [String]
    def self.strip_comments(html)
      result = +""
      pos = 0

      while pos < html.length
        start = html.index("<!--", pos)
        if start.nil?
          result << html[pos..]
          break
        end

        result << html[pos...start]
        finish = find_comment_end(html, start + 4)
        if finish.nil?
          result << html[start..]
          break
        end
        pos = finish
      end

      result
    end
    private_class_method :strip_comments

    def self.find_comment_end(html, start_index)
      index = start_index
      while index < html.length - 2
        return index + 3 if html[index, 3] == "-->"
        return index + 4 if html[index, 4] == "--!>"
        index += 1
      end
      nil
    end
    private_class_method :find_comment_end

    # ─── Pass 2: Drop dangerous elements ──────────────────────────────────────
    #
    # Remove the start tag, all inner content, and the end tag for every
    # element in the drop list. This is important for elements like <script>
    # and <style> where the content itself is dangerous.
    #
    # Implementation detail: we process one element name at a time, using a
    # case-insensitive regex. The inner content match uses /m so nested
    # content spanning newlines is captured.
    #
    # Limitation: naively nested same-element tags (e.g. <div><div></div></div>)
    # are NOT handled by this pass — but none of our default drop_elements are
    # typically nested with themselves in practice. For <script> and <style>,
    # nesting is not valid HTML so this limitation does not matter.
    #
    # @param html [String]
    # @param elements [Array<String>] lowercase element names
    # @return [String]
    def self.drop_elements(html, elements)
      elements.each do |tag|
        # Matches: opening tag (incl. attributes), inner content, closing tag.
        # (?:…) non-capturing group for the entire element.
        html = html.gsub(
          /<#{Regexp.escape(tag)}(?:\s[^>]*)?>.*?<\/#{Regexp.escape(tag)}>/im,
          ""
        )
        # Also remove void / self-closing instances and orphan open tags
        # (e.g. <meta ...> which has no closing tag).
        html = html.gsub(/<#{Regexp.escape(tag)}(?:\s[^>]*)?(?:\/)?>(\n)?/im, "")
      end
      html
    end
    private_class_method :drop_elements

    # ─── Pass 3: Process surviving tags ───────────────────────────────────────
    #
    # Walk the HTML token by token. For each opening/self-closing tag, apply:
    #   - Attribute dropping (on*, srcdoc, formaction, custom list)
    #   - URL sanitization (href, src)
    #   - Style sanitization (expression(), url() with unsafe argument)
    #
    # Closing tags and raw text pass through unchanged.
    #
    # @param html [String]
    # @param policy [HtmlSanitizationPolicy]
    # @return [String]
    def self.process_tags(html, policy)
      # Tokenise HTML. Named captures are used so Regexp.last_match works
      # reliably inside the gsub block (avoids $1/$2 ordering confusion).
      #
      # Token types:
      #   :selfclose — <tagname attrs />
      #   :open      — <tagname attrs>
      #   :close     — </tagname>
      #   :text      — anything that is not a tag
      result = +""
      pos = 0
      len = html.length

      while pos < len
        # Find the next "<"
        lt = html.index("<", pos)
        if lt.nil?
          # No more tags — rest is plain text.
          result << html[pos..]
          break
        end

        # Emit plain text before the "<".
        result << html[pos, lt - pos] if lt > pos

        # Try to match a tag starting at lt.
        tail = html[lt..]
        if (m = tail.match(/\A<([a-zA-Z][a-zA-Z0-9-]*)([^>]*?)(\s*\/)>/m))
          # Self-closing tag: <tagname attrs />
          tag_name = m[1]
          attrs_raw = m[2]
          result << sanitize_open_tag(tag_name, attrs_raw, self_close: true, policy: policy)
          pos = lt + m[0].length
        elsif (m = tail.match(/\A<([a-zA-Z][a-zA-Z0-9-]*)([^>]*)>/m))
          # Opening tag: <tagname attrs>
          tag_name = m[1]
          attrs_raw = m[2]
          result << sanitize_open_tag(tag_name, attrs_raw, self_close: false, policy: policy)
          pos = lt + m[0].length
        elsif (m = tail.match(/\A<\/[a-zA-Z][a-zA-Z0-9-]*>/m))
          # Closing tag — pass through verbatim.
          result << m[0]
          pos = lt + m[0].length
        else
          # Lone "<" that is not a valid tag (e.g. in text: "1 < 2").
          result << "<"
          pos = lt + 1
        end
      end

      result
    end
    private_class_method :process_tags

    # Sanitize a single opening/self-closing tag.
    #
    # @param tag_name [String] The element name (already extracted).
    # @param attrs_raw [String] The raw attribute string portion.
    # @param self_close [Boolean] Whether the original was self-closing.
    # @param policy [HtmlSanitizationPolicy]
    # @return [String] The sanitized tag including < and >.
    def self.sanitize_open_tag(tag_name, attrs_raw, self_close:, policy:)
      attrs = parse_attributes(attrs_raw)
      sanitized_attrs = filter_attributes(attrs, policy)
      attr_str = render_attributes(sanitized_attrs)
      if self_close
        "<#{tag_name}#{attr_str} />"
      else
        "<#{tag_name}#{attr_str}>"
      end
    end
    private_class_method :sanitize_open_tag

    # ─── Attribute parsing ─────────────────────────────────────────────────────
    #
    # HTML attributes come in three syntactic forms:
    #
    #   name="value"   — double-quoted
    #   name='value'   — single-quoted
    #   name=value     — unquoted (value ends at whitespace or >)
    #   name           — boolean (no value)
    #
    # We use a single regex with alternation to capture all four.
    # The result is an Array of [name, value | nil] pairs to preserve
    # attribute order.
    ATTR_RE = /
      (\w[\w-]*)               # attribute name
      (?:
        \s*=\s*
        (?:
          "([^"]*)"            # double-quoted value
          |
          '([^']*)'            # single-quoted value
          |
          ([^\s>\/]+)          # unquoted value
        )
      )?                       # value is optional (boolean attrs)
    /x

    # Parse an attribute string into an ordered array of [name, value] pairs.
    # Value is nil for boolean attributes.
    #
    # @param attrs_raw [String]
    # @return [Array<[String, String|nil]>]
    def self.parse_attributes(attrs_raw)
      attrs = []
      attrs_raw.scan(ATTR_RE) do
        name = $1
        value = $2 || $3 || $4  # double, single, or unquoted
        attrs << [name, value]
      end
      attrs
    end
    private_class_method :parse_attributes

    # Filter attributes according to the policy.
    #
    # Behavior depends on whether sanitization is active:
    #
    # "Active sanitization" is defined as: allowed_url_schemes is NOT nil,
    # OR sanitize_style_attributes is true, OR drop_attributes is non-empty.
    # When sanitization is inactive (all of the above are nil/false/[]),
    # attributes pass through completely unchanged — this is the PASSTHROUGH
    # behavior.
    #
    # When sanitization is active:
    #   - Drop all event handler attributes (on*)
    #   - Drop srcdoc and formaction (always dangerous)
    #   - Drop attributes in policy.drop_attributes
    #   - Sanitize href and src URL values
    #   - Sanitize/strip dangerous style attributes
    #
    # @param attrs [Array<[String, String|nil]>]
    # @param policy [HtmlSanitizationPolicy]
    # @return [Array<[String, String|nil]>]
    def self.filter_attributes(attrs, policy)
      # Determine whether any sanitization is active.
      # If ALL of these are their "no-op" values, pass everything through.
      sanitizing = !policy.allowed_url_schemes.nil? ||
        policy.sanitize_style_attributes ||
        !policy.drop_attributes.empty? ||
        !policy.drop_elements.empty?

      return attrs unless sanitizing

      always_drop = Set.new(%w[srcdoc formaction])

      attrs.filter_map do |(name, value)|
        lname = name.downcase

        # Drop all on* event handlers.
        next nil if lname.start_with?("on")

        # Drop srcdoc and formaction (dangerous regardless of value).
        next nil if always_drop.include?(lname)

        # Drop attributes in the policy's custom drop list.
        next nil if policy.drop_attributes.any? { |d| d.downcase == lname }

        # Sanitize href and src URL values.
        if %w[href src].include?(lname) && value
          safe_value = UrlUtils.sanitize_url(value, policy.allowed_url_schemes)
          next [name, safe_value]
        end

        # Sanitize or strip style attributes.
        if lname == "style" && policy.sanitize_style_attributes && value
          next nil if dangerous_style?(value)
        end

        [name, value]
      end
    end
    private_class_method :filter_attributes

    # ─── Style attribute safety check ─────────────────────────────────────────
    #
    # CSS injection attacks typically use:
    #
    #   expression(...)   — IE4+ CSS expression, executes JavaScript
    #   url(javascript:…) — CSS url() pointing to a JS URI
    #   url(data:…)       — CSS url() pointing to a data URI
    #
    # We strip the entire style attribute rather than attempting CSS parsing,
    # matching the spec's "strip the full style attribute" rule.
    #
    # @param style_value [String] The raw value of a style attribute.
    # @return [Boolean] true if the style value should be dropped.
    def self.dangerous_style?(style_value)
      # Case-insensitive checks.
      lower = style_value.downcase

      # IE CSS expression() — executes arbitrary JS.
      return true if lower.include?("expression(")

      # url() with a potentially dangerous argument.
      # We allow url() pointing only at http:// and https:// URIs.
      # Anything else (javascript:, data:, blob:, relative references with
      # suspicious content) is rejected for safety.
      lower.scan(/url\s*\(\s*["']?([^"')]+)["']?\s*\)/) do |groups|
        uri = groups[0].strip
        # Allow only http/https — everything else is suspicious in a style.
        return true unless uri.start_with?("http://", "https://")
      end

      false
    end
    private_class_method :dangerous_style?

    # ─── Attribute rendering ───────────────────────────────────────────────────

    # Render a list of [name, value] pairs back to an attribute string.
    # Boolean attributes (nil value) are rendered without a value.
    #
    # @param attrs [Array<[String, String|nil]>]
    # @return [String] Leading space included when non-empty.
    def self.render_attributes(attrs)
      return "" if attrs.empty?

      parts = attrs.map do |(name, value)|
        if value.nil?
          name
        else
          # Use double-quote delimiters. Escape any " inside the value.
          safe_value = value.gsub('"', "&quot;")
          "#{name}=\"#{safe_value}\""
        end
      end

      " #{parts.join(" ")}"
    end
    private_class_method :render_attributes
  end
end
