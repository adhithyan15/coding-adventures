namespace CodingAdventures.Brainfuck.Tests

open System
open Xunit
open CodingAdventures.Brainfuck.FSharp

module BrainfuckTests =
    [<Fact>]
    let ``translate maps commands and ignores comments`` () =
        let program = Brainfuck.translate "hello + world -><.,"
        let opcodes = program |> Array.map (fun instruction -> instruction.Opcode)

        Assert.Equal<BrainfuckOpcode array>(
            [|
                Increment
                Decrement
                Right
                Left
                Output
                Input
                Halt
            |],
            opcodes)

    [<Fact>]
    let ``translate patches loop targets`` () =
        let program = Brainfuck.translate "[>+<-]"

        Assert.Equal(LoopStart, program[0].Opcode)
        Assert.Equal(Some 6, program[0].Operand)
        Assert.Equal(LoopEnd, program[5].Opcode)
        Assert.Equal(Some 0, program[5].Operand)

    [<Fact>]
    let ``translate patches nested loops`` () =
        let program = Brainfuck.translate "[[]]"

        Assert.Equal(Some 4, program[0].Operand)
        Assert.Equal(Some 3, program[1].Operand)
        Assert.Equal(Some 1, program[2].Operand)
        Assert.Equal(Some 0, program[3].Operand)

    [<Fact>]
    let ``translate rejects mismatched brackets`` () =
        Assert.Throws<BrainfuckTranslationException>(fun () -> Brainfuck.translate "[" |> ignore) |> ignore
        Assert.Throws<BrainfuckTranslationException>(fun () -> Brainfuck.translate "]" |> ignore) |> ignore
        Assert.Throws<BrainfuckTranslationException>(fun () -> Brainfuck.translate "[[]" |> ignore) |> ignore

    [<Fact>]
    let ``execute handles empty program and comments`` () =
        let empty = Brainfuck.execute ""
        let comments = Brainfuck.execute "this is all comments"

        Assert.Equal("", empty.Output)
        Assert.Equal(0, empty.Pointer)
        Assert.Equal(1, empty.Steps)
        Assert.Equal("", comments.Output)
        Assert.Equal(1, comments.Steps)

    [<Fact>]
    let ``execute supports cell arithmetic and wrapping`` () =
        Assert.Equal(1uy, (Brainfuck.execute "+").Tape[0])
        Assert.Equal(255uy, (Brainfuck.execute "-").Tape[0])
        Assert.Equal(0uy, (Brainfuck.execute (String('+', 256))).Tape[0])

    [<Fact>]
    let ``execute supports pointer movement and bounds errors`` () =
        Assert.Equal(3, (Brainfuck.execute ">>>").Pointer)
        Assert.Throws<BrainfuckExecutionException>(fun () -> Brainfuck.execute "<" |> ignore) |> ignore
        Assert.Throws<BrainfuckExecutionException>(fun () -> Brainfuck.execute (String('>', Brainfuck.TapeSize)) |> ignore) |> ignore

    [<Fact>]
    let ``execute supports loops`` () =
        let addition = Brainfuck.execute "++>+++++[<+>-]"
        let move = Brainfuck.execute "+++++[>+<-]"
        let skipped = Brainfuck.execute "[]+++"

        Assert.Equal(7uy, addition.Tape[0])
        Assert.Equal(0uy, addition.Tape[1])
        Assert.Equal(0uy, move.Tape[0])
        Assert.Equal(5uy, move.Tape[1])
        Assert.Equal(3uy, skipped.Tape[0])

    [<Fact>]
    let ``execute supports nested loops`` () =
        let result = Brainfuck.execute "++>+++<[>[>+>+<<-]>>[<<+>>-]<<<-]"

        Assert.Equal(6uy, result.Tape[2])
        Assert.Equal(0uy, result.Tape[0])

    [<Fact>]
    let ``execute supports output and input`` () =
        let h = Brainfuck.execute "+++++++++[>++++++++<-]>."
        let echo = Brainfuck.executeWithInput ",.,.,." "ABC"
        let eof = Brainfuck.executeWithInput ",," "A"

        Assert.Equal("H", h.Output)
        Assert.Equal("ABC", echo.Output)
        Assert.Equal(0uy, eof.Tape[0])

    [<Fact>]
    let ``execute runs hello world`` () =
        let helloWorld =
            "++++++++[>++++[>++>+++>+++>+<<<<-]>+>+>->>+[<]<-]" +
            ">>.>---.+++++++..+++.>>.<-.<.+++.------.--------.>>+.>++."

        Assert.Equal("Hello World!\n", (Brainfuck.execute helloWorld).Output)

    [<Fact>]
    let ``execute accepts translated program`` () =
        let program = Brainfuck.translate "+++."
        let result = Brainfuck.executeProgram program [||]

        Assert.Equal(char 3, result.Output[0])
        Assert.Equal(5, result.Steps)

    [<Fact>]
    let ``null inputs are rejected`` () =
        Assert.Throws<ArgumentNullException>(fun () -> Brainfuck.translate null |> ignore) |> ignore
        Assert.Throws<ArgumentNullException>(fun () -> Brainfuck.execute null |> ignore) |> ignore
        Assert.Throws<ArgumentNullException>(fun () -> Brainfuck.executeWithInput "+" null |> ignore) |> ignore
        Assert.Throws<ArgumentNullException>(fun () -> Brainfuck.executeProgram null [||] |> ignore) |> ignore
