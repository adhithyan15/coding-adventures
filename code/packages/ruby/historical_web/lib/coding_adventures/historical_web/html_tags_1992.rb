# frozen_string_literal: true

module CodingAdventures
  module HistoricalWeb
    # Text and elements are siblings in a flat arena. Their child edges are
    # integer IDs, so deeply nested markup never creates recursive ownership.
    HtmlTextNode = Struct.new(:kind, :text)
    HtmlElementNode = Struct.new(:kind, :tag, :attributes, :children)
    HtmlDocument = Struct.new(:nodes, :roots)

    DraftTextNode = Struct.new(:kind, :fragments)
    DraftElementNode = Struct.new(:kind, :tag, :attributes, :children)

    HTML_TAGS_1992 = %w[
      title address a dl li xmp listing plaintext ul menu dir dt dd p isindex
      h1 h2 h3 h4 h5 h6
    ].freeze

    class HtmlTreeStateError < StandardError; end
    private_constant :DraftTextNode, :DraftElementNode, :HTML_TAGS_1992, :HtmlTreeStateError

    # Parse the archived profile. Ruby Strings are admitted only as valid UTF-8
    # or valid US-ASCII; scanner positions after admission are byte offsets.
    def self.parse_html_tags_1992(source)
      case source
      when String
        # String.new below makes a concrete snapshot, excluding subclass methods.
      else
        return Result.failure(:invalid_source)
      end

      text = String.new(source)
      unless [Encoding::UTF_8, Encoding::US_ASCII].include?(text.encoding) && text.valid_encoding?
        return Result.failure(:invalid_source)
      end

      normalized = text.gsub("\r\n", "\n")
      nodes = []
      roots = []
      stack = []
      positions = {}
      i = 0
      text_start = 0

      while i < normalized.bytesize
        start = find_byte(normalized, 0x3c, i)
        unless start
          fragment = normalized.byteslice(text_start, normalized.bytesize - text_start)
          append_text(nodes, roots, stack, fragment, decode: true) if fragment && !fragment.empty?
          break
        end

        if start > text_start
          append_text(nodes, roots, stack, normalized.byteslice(text_start, start - text_start), decode: true)
        end

        if bytes_at?(normalized, start, "<!--")
          comment_end = find_sequence(normalized, "-->".b, start + 4)
          return finalize_document(nodes, roots) unless comment_end

          i = comment_end + 3
          text_start = i
          next
        end

        token = tag_at(normalized, start)
        unless token
          append_text(nodes, roots, stack, "<".encode(normalized.encoding), decode: false)
          i = start + 1
          text_start = i
          next
        end

        name = token[:name]
        unless HTML_TAGS_1992.include?(name)
          i = token[:finish]
          text_start = i
          next
        end

        if !token[:closing] && name == "plaintext"
          id = append_element(nodes, roots, stack, name, {})
          append_text(nodes, roots, [id], normalized.byteslice(token[:finish], normalized.bytesize - token[:finish]), decode: false)
          return finalize_document(nodes, roots)
        end

        if !token[:closing] && %w[xmp listing].include?(name)
          id = append_element(nodes, roots, stack, name, {})
          ending = "</#{name}>".b
          raw_end = find_ascii_folded(normalized, ending, token[:finish])
          unless raw_end
            append_text(nodes, roots, [id], normalized.byteslice(token[:finish], normalized.bytesize - token[:finish]), decode: false)
            return finalize_document(nodes, roots)
          end

          append_text(nodes, roots, [id], normalized.byteslice(token[:finish], raw_end - token[:finish]), decode: false)
          i = raw_end + ending.bytesize
          text_start = i
          next
        end

        if token[:closing]
          unless %w[p isindex].include?(name)
            matching = top_position(positions, name)
            truncate_stack(nodes, stack, positions, matching) if matching
          end
          i = token[:finish]
          text_start = i
          next
        end

        if %w[p isindex].include?(name)
          append_element(nodes, roots, stack, name, {})
          i = token[:finish]
          text_start = i
          next
        end

        close_names, boundaries = implied_close_sets(name)
        close_at = latest_position(positions, close_names)
        boundary_at = latest_position(positions, boundaries)
        if close_at && (!boundary_at || close_at > boundary_at)
          truncate_stack(nodes, stack, positions, close_at)
        end

        attributes = if name == "a"
          anchor_attributes(token[:attributes])
        else
          {}
        end
        id = append_element(nodes, roots, stack, name, attributes)
        (positions[name] ||= []) << stack.length
        stack << id
        i = token[:finish]
        text_start = i
      end

      finalize_document(nodes, roots)
    rescue HtmlTreeStateError
      Result.failure(:invalid_tree_state)
    end

    def self.find_byte(source, byte, from)
      index = from
      while index < source.bytesize
        return index if source.getbyte(index) == byte

        index += 1
      end
      nil
    end
    private_class_method :find_byte

    def self.bytes_at?(source, start, needle)
      return false if start + needle.bytesize > source.bytesize

      index = 0
      while index < needle.bytesize
        return false unless source.getbyte(start + index) == needle.getbyte(index)

        index += 1
      end
      true
    end
    private_class_method :bytes_at?

    def self.find_sequence(source, needle, from)
      last = source.bytesize - needle.bytesize
      index = from
      while index <= last
        return index if bytes_at?(source, index, needle)

        index += 1
      end
      nil
    end
    private_class_method :find_sequence

    # Recognize names and whitespace with byte tests only; Unicode case
    # folding and Ruby's locale-sensitive parsing are deliberately absent.
    def self.tag_at(source, start)
      return nil unless source.getbyte(start) == 0x3c

      index = start + 1
      closing = source.getbyte(index) == 0x2f
      index += 1 if closing
      return nil unless ascii_letter?(source.getbyte(index))

      name_start = index
      index += 1 while index < source.bytesize && name_character?(source.getbyte(index))
      name_end = index
      after_name = source.getbyte(index)
      return nil if index < source.bytesize && after_name != 0x3e && !html_space?(after_name)

      quote = nil
      while index < source.bytesize
        byte = source.getbyte(index)
        return nil if byte == 0x3c

        if quote
          quote = nil if byte == quote
        elsif byte == 0x22 || byte == 0x27
          quote = byte
        elsif byte == 0x3e
          raw_name = source.byteslice(name_start, name_end - name_start)
          return {
            name: ascii_lower(raw_name),
            attributes: source.byteslice(name_end, index - name_end),
            closing: closing,
            finish: index + 1
          }
        end
        index += 1
      end
      nil
    end
    private_class_method :tag_at

    def self.html_space?(byte)
      byte == 0x20 || byte == 0x09 || byte == 0x0a || byte == 0x0d || byte == 0x0c
    end
    private_class_method :html_space?

    def self.ascii_letter?(byte)
      byte&.between?(0x41, 0x5a) || byte&.between?(0x61, 0x7a)
    end
    private_class_method :ascii_letter?

    def self.name_character?(byte)
      ascii_letter?(byte) || byte&.between?(0x30, 0x39) || byte == 0x2d
    end
    private_class_method :name_character?

    def self.ascii_lower(value)
      value.tr("A-Z", "a-z")
    end
    private_class_method :ascii_lower

    def self.decode_entities(source)
      parts = []
      text_start = 0
      index = 0
      while index < source.bytesize
        if source.getbyte(index) == 0x26
          ending = if bytes_at?(source, index, "&lt;".b)
            [4, "<"]
          elsif bytes_at?(source, index, "&gt;".b)
            [4, ">"]
          elsif bytes_at?(source, index, "&amp;".b)
            [5, "&"]
          end
          if ending
            parts << source.byteslice(text_start, index - text_start)
            parts << ending[1].encode(source.encoding)
            index += ending[0]
            text_start = index
            next
          end
        end
        index += 1
      end
      return source if parts.empty?

      parts << source.byteslice(text_start, source.bytesize - text_start)
      result = String.new(encoding: source.encoding)
      parts.each { |part| result << part }
      result
    end
    private_class_method :decode_entities

    def self.anchor_attributes(source)
      attributes = {}
      index = 0
      while index < source.bytesize
        space_start = index
        index += 1 while index < source.bytesize && html_space?(source.getbyte(index))
        break if index == space_start || index >= source.bytesize || !ascii_letter?(source.getbyte(index))

        name_start = index
        index += 1 while index < source.bytesize && name_character?(source.getbyte(index))
        name_end = index
        name = ascii_lower(source.byteslice(name_start, name_end - name_start))
        index += 1 while index < source.bytesize && html_space?(source.getbyte(index))
        if source.getbyte(index) == 0x3d
          index += 1
          index += 1 while index < source.bytesize && html_space?(source.getbyte(index))
          value, index = attribute_value(source, index)
        else
          # The separator belongs to the next attribute. Its leading-space
          # check decides whether another token can be read.
          index = name_end
          value = nil
        end

        case name
        when "href"
          attributes["href"] = value unless attributes.key?("href")
        when "name"
          attributes["name"] = value unless attributes.key?("name")
        when "type"
          attributes["type"] = value unless attributes.key?("type")
        end
      end
      attributes
    end
    private_class_method :anchor_attributes

    def self.attribute_value(source, index)
      return [nil, index] if index >= source.bytesize

      quote = source.getbyte(index)
      if quote == 0x22 || quote == 0x27
        index += 1
        start = index
        index += 1 while index < source.bytesize && source.getbyte(index) != quote
        return [nil, index] if index >= source.bytesize

        [decode_entities(source.byteslice(start, index - start)), index + 1]
      else
        start = index
        while index < source.bytesize
          byte = source.getbyte(index)
          break if html_space?(byte) || byte == 0x22 || byte == 0x27 || byte == 0x3d || byte == 0x3c || byte == 0x3e

          index += 1
        end
        return [nil, index] if index == start

        [decode_entities(source.byteslice(start, index - start)), index]
      end
    end
    private_class_method :attribute_value

    def self.implied_close_sets(name)
      if name == "li"
        [%w[li], %w[ul menu dir]]
      elsif name == "dt" || name == "dd"
        [%w[dt dd], %w[dl]]
      elsif name.bytesize == 2 && name.getbyte(0) == 0x68 && name.getbyte(1).between?(0x31, 0x36)
        [%w[h1 h2 h3 h4 h5 h6], []]
      else
        [[], []]
      end
    end
    private_class_method :implied_close_sets

    def self.top_position(positions, name)
      entries = positions[name]
      entries&.last
    end
    private_class_method :top_position

    def self.latest_position(positions, names)
      latest = nil
      names.each do |name|
        position = top_position(positions, name)
        latest = position if position && (!latest || position > latest)
      end
      latest
    end
    private_class_method :latest_position

    def self.append_element(nodes, roots, stack, tag, attributes)
      id = nodes.length
      node = DraftElementNode.new(kind: :element, tag: tag, attributes: attributes, children: [])
      nodes << node
      parent = stack.empty? ? nil : nodes[stack.last]
      if parent.nil?
        roots << id
      elsif parent.kind == :element
        parent.children << id
      else
        raise HtmlTreeStateError
      end
      id
    end
    private_class_method :append_element

    def self.append_text(nodes, roots, stack, text, decode:)
      return if text.nil? || text.empty?

      fragment = decode ? decode_entities(text) : text
      parent = stack.empty? ? nil : nodes[stack.last]
      if parent && parent.kind != :element
        raise HtmlTreeStateError
      end
      siblings = parent ? parent.children : roots
      previous = siblings.empty? ? nil : nodes[siblings.last]
      if previous&.kind == :text
        previous.fragments << fragment
      else
        id = nodes.length
        nodes << DraftTextNode.new(kind: :text, fragments: [fragment])
        siblings << id
      end
    end
    private_class_method :append_text

    def self.truncate_stack(nodes, stack, positions, at)
      while stack.length > at
        id = stack.pop
        raise HtmlTreeStateError unless id

        node = nodes[id]
        raise HtmlTreeStateError unless node&.kind == :element

        positions[node.tag]&.pop
      end
    end
    private_class_method :truncate_stack

    def self.find_ascii_folded(source, needle, from)
      index = from
      last = source.bytesize - needle.bytesize
      while index <= last
        if source.getbyte(index) == 0x3c && ascii_equal_folded_at?(source, index, needle)
          return index
        end
        index += 1
      end
      nil
    end
    private_class_method :find_ascii_folded

    def self.ascii_equal_folded_at?(source, start, needle)
      index = 0
      while index < needle.bytesize
        left = source.getbyte(start + index)
        right = needle.getbyte(index)
        left += 0x20 if left&.between?(0x41, 0x5a)
        right += 0x20 if right&.between?(0x41, 0x5a)
        return false unless left == right

        index += 1
      end
      true
    end
    private_class_method :ascii_equal_folded_at?

    def self.finalize_document(nodes, roots)
      final_nodes = nodes.map do |node|
        if node.kind == :text
          text = String.new(encoding: node.fragments.first&.encoding || Encoding::UTF_8)
          node.fragments.each { |fragment| text << fragment }
          HtmlTextNode.new(kind: :text, text: text.freeze).freeze
        else
          return Result.failure(:invalid_tree_state) unless node.children.all? { |child| child.is_a?(Integer) && child >= 0 && child < nodes.length }

          attributes = {}
          node.attributes.each do |name, value|
            attributes[name.freeze] = value&.freeze
          end

          HtmlElementNode.new(
            kind: :element,
            tag: node.tag.freeze,
            attributes: attributes.freeze,
            children: node.children.freeze
          ).freeze
        end
      end.freeze
      return Result.failure(:invalid_tree_state) unless roots.all? { |root| root.is_a?(Integer) && root >= 0 && root < nodes.length }

      Result.success(HtmlDocument.new(nodes: final_nodes, roots: roots.freeze).freeze)
    end
    private_class_method :finalize_document
  end
end
