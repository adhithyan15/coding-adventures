# frozen_string_literal: true

require_relative "coding_adventures/draw_instructions/version"

module CodingAdventures
  # Backend-neutral 2D drawing primitives.
  #
  # == Architectural Boundary
  #
  # The key idea is separation of concerns:
  # - *Producer* packages decide WHAT should be drawn (e.g. Code 39 barcode bars)
  # - *Renderer* packages decide HOW to serialize or paint it (e.g. SVG, PNG)
  #
  # This package lives in the middle. It provides generic rectangles, text,
  # lines, groups, clip regions, and scenes. It does not know about barcodes,
  # charts, or any other domain.
  #
  # == Immutability
  #
  # All instruction structs are frozen after creation. This prevents accidental
  # mutation and makes scenes safe to share across threads or renderers.
  #
  # == Metadata
  #
  # Every instruction accepts an optional +metadata+ hash. Producers use it to
  # attach domain meaning (e.g. source character, pipeline stage) without
  # polluting the shared scene model with domain-specific fields.
  #
  # == Duck-Typed Renderers
  #
  # A renderer is any object with a +render(scene)+ method. This keeps the
  # coupling loose: no base class, no interface gem, just Ruby duck typing.
  module DrawInstructions
    module_function

    # ------------------------------------------------------------------
    # Struct definitions
    # ------------------------------------------------------------------
    # We use Ruby Struct for lightweight, immutable value objects. Each
    # struct is frozen immediately after creation so that scenes are
    # safe to share and reason about.
    # ------------------------------------------------------------------

    # A filled and/or stroked rectangle in scene coordinates.
    #
    # Rectangles are the workhorse primitive. A 1D barcode bar is just a
    # tall, thin rectangle. A 2D barcode module is a small square. A table
    # cell background is a wide, short rectangle with a fill color.
    #
    # +stroke+ and +stroke_width+ are optional. When +stroke+ is nil, no
    # border is drawn. When present, +stroke_width+ defaults to 1.
    DrawRectInstruction = Struct.new(
      :kind, :x, :y, :width, :height, :fill, :stroke, :stroke_width, :metadata,
      keyword_init: true
    )

    # A text label positioned in scene coordinates.
    #
    # +align+ controls horizontal anchor: "start", "middle", or "end".
    # +font_weight+ is either "normal" or "bold" (nil means "normal").
    DrawTextInstruction = Struct.new(
      :kind, :x, :y, :value, :fill, :font_family, :font_size, :align, :font_weight, :metadata,
      keyword_init: true
    )

    # A group provides hierarchical structure without transforms.
    #
    # Groups let producers preserve semantic structure. For example, one
    # group per encoded barcode symbol, or one group per table row.
    DrawGroupInstruction = Struct.new(
      :kind, :children, :metadata,
      keyword_init: true
    )

    # A straight line segment between two points.
    #
    # Lines are always stroked (never filled). They are the backbone of
    # grid rendering: a table's horizontal and vertical grid lines are
    # each one DrawLineInstruction.
    DrawLineInstruction = Struct.new(
      :kind, :x1, :y1, :x2, :y2, :stroke, :stroke_width, :metadata,
      keyword_init: true
    )

    # A clipping region that constrains its children.
    #
    # Any drawing by children that falls outside the clip rectangle is
    # invisible. This is how the table prevents cell text from bleeding
    # into adjacent columns.
    #
    # Clip instructions nest: a child clip intersects with its parent.
    DrawClipInstruction = Struct.new(
      :kind, :x, :y, :width, :height, :children, :metadata,
      keyword_init: true
    )

    # A complete scene: the unit that renderers consume.
    #
    # Width and height are explicit so renderers don't have to infer
    # bounds from instructions. Background is the canvas fill color.
    DrawScene = Struct.new(
      :width, :height, :background, :instructions, :metadata,
      keyword_init: true
    )

    # ------------------------------------------------------------------
    # Convenience constructors
    # ------------------------------------------------------------------
    # These factory methods provide sensible defaults so callers can stay
    # terse for the common case while still having full control.
    # ------------------------------------------------------------------

    # Create a rectangle instruction.
    #
    #   draw_rect(x: 0, y: 0, width: 10, height: 20)
    #   draw_rect(x: 0, y: 0, width: 10, height: 20, fill: "#ff0000",
    #             stroke: "#000000", stroke_width: 2)
    #
    def draw_rect(x:, y:, width:, height:, fill: "#000000", stroke: nil, stroke_width: nil, metadata: {})
      DrawRectInstruction.new(
        kind: "rect",
        x: x,
        y: y,
        width: width,
        height: height,
        fill: fill,
        stroke: stroke,
        stroke_width: stroke_width,
        metadata: metadata,
      ).freeze
    end

    # Create a text instruction.
    #
    #   draw_text(x: 50, y: 100, value: "Hello")
    #   draw_text(x: 50, y: 100, value: "Bold!", font_weight: "bold")
    #
    def draw_text(x:, y:, value:, fill: "#000000", font_family: "monospace", font_size: 16, align: "middle", font_weight: nil, metadata: {})
      DrawTextInstruction.new(
        kind: "text",
        x: x,
        y: y,
        value: value,
        fill: fill,
        font_family: font_family,
        font_size: font_size,
        align: align,
        font_weight: font_weight,
        metadata: metadata,
      ).freeze
    end

    # Create a group instruction.
    #
    #   draw_group(children: [rect1, rect2], metadata: { layer: "bars" })
    #
    def draw_group(children:, metadata: {})
      DrawGroupInstruction.new(
        kind: "group",
        children: children,
        metadata: metadata,
      ).freeze
    end

    # Create a line instruction.
    #
    #   draw_line(x1: 0, y1: 0, x2: 100, y2: 0)
    #   draw_line(x1: 0, y1: 0, x2: 100, y2: 0, stroke: "#ccc", stroke_width: 2)
    #
    def draw_line(x1:, y1:, x2:, y2:, stroke: "#000000", stroke_width: 1, metadata: {})
      DrawLineInstruction.new(
        kind: "line",
        x1: x1,
        y1: y1,
        x2: x2,
        y2: y2,
        stroke: stroke,
        stroke_width: stroke_width,
        metadata: metadata,
      ).freeze
    end

    # Create a clip instruction.
    #
    #   draw_clip(x: 10, y: 10, width: 80, height: 30, children: [text])
    #
    def draw_clip(x:, y:, width:, height:, children:, metadata: {})
      DrawClipInstruction.new(
        kind: "clip",
        x: x,
        y: y,
        width: width,
        height: height,
        children: children,
        metadata: metadata,
      ).freeze
    end

    # Create a complete scene.
    #
    #   create_scene(width: 200, height: 100, instructions: [rect, text])
    #   create_scene(width: 200, height: 100, instructions: [],
    #                background: "#f0f0f0", metadata: { label: "My Scene" })
    #
    def create_scene(width:, height:, instructions:, background: "#ffffff", metadata: {})
      DrawScene.new(
        width: width,
        height: height,
        background: background,
        instructions: instructions,
        metadata: metadata,
      ).freeze
    end

    # Delegate rendering to a backend implementation.
    #
    # A renderer is any object responding to +render(scene)+. This keeps
    # coupling loose via duck typing rather than inheritance.
    #
    #   svg_output = render_with(scene, SvgRenderer.new)
    #
    def render_with(scene, renderer)
      renderer.render(scene)
    end
  end
end
