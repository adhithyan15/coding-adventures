# frozen_string_literal: true

module CodingAdventures
  module TetradRuntime
    Program = Data.define(:forms)
    FunctionDef = Data.define(:name, :params, :body)
    LetStmt = Data.define(:name, :expr)
    ReturnStmt = Data.define(:expr)
    ExprStmt = Data.define(:expr)
    NumberLit = Data.define(:value)
    VarRef = Data.define(:name)
    Binary = Data.define(:left, :op, :right)
    Call = Data.define(:name, :args)
  end
end
