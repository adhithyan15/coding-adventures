defmodule CodingAdventures.DocumentAstTest do
  use ExUnit.Case, async: true
  alias CodingAdventures.DocumentAst

  doctest CodingAdventures.DocumentAst

  describe "block node constructors" do
    test "document/1 creates a document node" do
      assert DocumentAst.document([]) == %{type: :document, children: []}
    end

    test "document/1 with children" do
      para = DocumentAst.paragraph([DocumentAst.text("hello")])
      doc = DocumentAst.document([para])
      assert doc == %{type: :document, children: [%{type: :paragraph, children: [%{type: :text, value: "hello"}]}]}
    end

    test "heading/2 creates heading nodes for all levels" do
      for level <- 1..6 do
        h = DocumentAst.heading(level, [])
        assert h.type == :heading
        assert h.level == level
        assert h.children == []
      end
    end

    test "paragraph/1 creates a paragraph node" do
      p = DocumentAst.paragraph([DocumentAst.text("content")])
      assert p.type == :paragraph
      assert length(p.children) == 1
    end

    test "code_block/2 creates a code block" do
      cb = DocumentAst.code_block("elixir", "IO.puts(1)\n")
      assert cb == %{type: :code_block, language: "elixir", value: "IO.puts(1)\n"}
    end

    test "code_block/2 allows nil language" do
      cb = DocumentAst.code_block(nil, "some code\n")
      assert cb.language == nil
    end

    test "blockquote/1 creates a blockquote" do
      bq = DocumentAst.blockquote([])
      assert bq == %{type: :blockquote, children: []}
    end

    test "list/4 creates an unordered list" do
      lst = DocumentAst.list(false, nil, true, [])
      assert lst == %{type: :list, ordered: false, start: nil, tight: true, children: []}
    end

    test "list/4 creates an ordered list with start" do
      lst = DocumentAst.list(true, 3, false, [])
      assert lst == %{type: :list, ordered: true, start: 3, tight: false, children: []}
    end

    test "list_item/1 creates a list item" do
      item = DocumentAst.list_item([])
      assert item == %{type: :list_item, children: []}
    end

    test "thematic_break/0 creates a thematic break" do
      tb = DocumentAst.thematic_break()
      assert tb == %{type: :thematic_break}
    end

    test "raw_block/2 creates a raw block" do
      rb = DocumentAst.raw_block("html", "<div>raw</div>\n")
      assert rb == %{type: :raw_block, format: "html", value: "<div>raw</div>\n"}
    end
  end

  describe "inline node constructors" do
    test "text/1 creates a text node" do
      t = DocumentAst.text("Hello & world")
      assert t == %{type: :text, value: "Hello & world"}
    end

    test "emphasis/1 creates an emphasis node" do
      em = DocumentAst.emphasis([DocumentAst.text("hello")])
      assert em.type == :emphasis
      assert length(em.children) == 1
    end

    test "strong/1 creates a strong node" do
      s = DocumentAst.strong([DocumentAst.text("bold")])
      assert s.type == :strong
    end

    test "code_span/1 creates a code span" do
      cs = DocumentAst.code_span("const x = 1")
      assert cs == %{type: :code_span, value: "const x = 1"}
    end

    test "link/3 creates a link node" do
      ln = DocumentAst.link("https://example.com", "Example", [DocumentAst.text("click")])
      assert ln.type == :link
      assert ln.destination == "https://example.com"
      assert ln.title == "Example"
    end

    test "link/3 allows nil title" do
      ln = DocumentAst.link("https://example.com", nil, [])
      assert ln.title == nil
    end

    test "image/3 creates an image node" do
      img = DocumentAst.image("cat.png", nil, "a cat")
      assert img == %{type: :image, destination: "cat.png", title: nil, alt: "a cat"}
    end

    test "autolink/2 creates an autolink node" do
      al = DocumentAst.autolink("user@example.com", true)
      assert al.type == :autolink
      assert al.is_email == true
    end

    test "raw_inline/2 creates a raw inline node" do
      ri = DocumentAst.raw_inline("html", "<em>raw</em>")
      assert ri == %{type: :raw_inline, format: "html", value: "<em>raw</em>"}
    end

    test "hard_break/0 creates a hard break" do
      hb = DocumentAst.hard_break()
      assert hb == %{type: :hard_break}
    end

    test "soft_break/0 creates a soft break" do
      sb = DocumentAst.soft_break()
      assert sb == %{type: :soft_break}
    end
  end

  describe "nested document structure" do
    test "can build a complete document tree" do
      doc =
        DocumentAst.document([
          DocumentAst.heading(1, [DocumentAst.text("Title")]),
          DocumentAst.paragraph([
            DocumentAst.text("Hello "),
            DocumentAst.emphasis([DocumentAst.text("world")])
          ]),
          DocumentAst.list(false, nil, true, [
            DocumentAst.list_item([
              DocumentAst.paragraph([DocumentAst.text("item 1")])
            ])
          ])
        ])

      assert doc.type == :document
      assert length(doc.children) == 3
      assert hd(doc.children).type == :heading
    end
  end
end
