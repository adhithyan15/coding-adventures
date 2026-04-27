namespace CodingAdventures.CompilerSourceMap.Tests

open Xunit
open CodingAdventures.CompilerSourceMap.FSharp

module CompilerSourceMapTests =
    let private buildTestChain () =
        let chain = SourceMapChain.New()
        chain.SourceToAst.Add({ File = "hello.bf"; Line = 1; Column = 1; Length = 1 }, 0)
        chain.SourceToAst.Add({ File = "hello.bf"; Line = 1; Column = 2; Length = 1 }, 1)
        chain.AstToIr.Add(0, [ 2L; 3L; 4L; 5L ])
        chain.AstToIr.Add(1, [ 6L; 7L ])

        let identity = IrToIr("identity")

        for i in 0L .. 7L do
            identity.AddMapping(i, [ i ])

        chain.AddOptimizerPass identity

        let machineCode = IrToMachineCode()
        machineCode.Add(0L, 0x00L, 8L)
        machineCode.Add(1L, 0x08L, 4L)
        machineCode.Add(2L, 0x0CL, 8L)
        machineCode.Add(3L, 0x14L, 4L)
        machineCode.Add(4L, 0x18L, 4L)
        machineCode.Add(5L, 0x1CL, 8L)
        machineCode.Add(6L, 0x24L, 8L)
        machineCode.Add(7L, 0x2CL, 8L)
        chain.IrToMachineCode <- Some machineCode
        chain

    [<Fact>]
    let ``source position formats and compares`` () =
        let position = { File = "hello.bf"; Line = 1; Column = 3; Length = 1 }

        Assert.Equal("hello.bf:1:3 (len=1)", position.ToString())
        Assert.Equal(position, { File = "hello.bf"; Line = 1; Column = 3; Length = 1 })
        Assert.NotEqual(position, { File = "hello.bf"; Line = 1; Column = 4; Length = 1 })

    [<Fact>]
    let ``source to ast adds and looks up node ids`` () =
        let segment = SourceToAst()
        let position = { File = "a.bf"; Line = 1; Column = 1; Length = 1 }

        segment.Add(position, 42)

        Assert.Equal(Some position, segment.LookupByNodeId 42)
        Assert.Equal(None, segment.LookupByNodeId 999)

    [<Fact>]
    let ``ast to ir supports forward and reverse lookup`` () =
        let segment = AstToIr()

        segment.Add(5, [ 20L; 21L ])
        segment.Add(6, [ 22L ])

        Assert.Equal(Some [ 20L; 21L ], segment.LookupByAstNodeId 5)
        Assert.Equal(5, segment.LookupByIrId 21L)
        Assert.Equal(6, segment.LookupByIrId 22L)
        Assert.Equal(-1, segment.LookupByIrId 999L)

    [<Fact>]
    let ``ir to ir handles mappings deletion and reverse lookup`` () =
        let segment = IrToIr("contraction")

        segment.AddMapping(7L, [ 100L ])
        segment.AddMapping(8L, [ 100L ])
        segment.AddDeletion 9L

        Assert.Equal(Some [ 100L ], segment.LookupByOriginalId 7L)
        Assert.Equal(7L, segment.LookupByNewId 100L)
        Assert.Equal(None, segment.LookupByOriginalId 9L)
        Assert.Contains(9L, segment.Deleted)
        Assert.Equal("contraction", segment.PassName)

    [<Fact>]
    let ``ir to machine code finds instruction ranges`` () =
        let segment = IrToMachineCode()

        segment.Add(5L, 0x20L, 4L)
        segment.Add(6L, 0x24L, 8L)

        Assert.Equal((0x20L, 4L), segment.LookupByIrId 5L)
        Assert.Equal((-1L, 0L), segment.LookupByIrId 999L)
        Assert.Equal(5L, segment.LookupByMachineCodeOffset 0x23L)
        Assert.Equal(6L, segment.LookupByMachineCodeOffset 0x24L)
        Assert.Equal(-1L, segment.LookupByMachineCodeOffset 0xFFL)

    [<Fact>]
    let ``new chain starts empty`` () =
        let chain = SourceMapChain.New()

        Assert.Empty(chain.SourceToAst.Entries)
        Assert.Empty(chain.AstToIr.Entries)
        Assert.Empty(chain.IrToIr)
        Assert.Equal(None, chain.IrToMachineCode)

    [<Fact>]
    let ``forward lookup composes all segments`` () =
        let chain = buildTestChain ()

        let results = chain.SourceToMc({ File = "hello.bf"; Line = 1; Column = 1; Length = 1 })

        match results with
        | None -> failwith "expected machine-code entries"
        | Some entries ->
            Assert.Equal<int64 list>([ 2L; 3L; 4L; 5L ], entries |> List.map _.IrId)
            Assert.Equal(0x0CL, entries.Head.MachineCodeOffset)

    [<Fact>]
    let ``reverse lookup composes segments back to source`` () =
        let chain = buildTestChain ()

        let plus = chain.McToSource 0x14L
        let dot = chain.McToSource 0x2CL

        Assert.Equal(Some { File = "hello.bf"; Line = 1; Column = 1; Length = 1 }, plus)
        Assert.Equal(Some { File = "hello.bf"; Line = 1; Column = 2; Length = 1 }, dot)

    [<Fact>]
    let ``incomplete or missing mappings return none`` () =
        let chain = SourceMapChain.New()
        chain.SourceToAst.Add({ File = "a.bf"; Line = 1; Column = 1; Length = 1 }, 0)
        chain.AstToIr.Add(0, [ 1L; 2L ])

        Assert.Equal(None, chain.SourceToMc({ File = "a.bf"; Line = 1; Column = 1; Length = 1 }))
        Assert.Equal(None, chain.McToSource 0L)

        chain.IrToMachineCode <- Some(IrToMachineCode())
        Assert.Equal(None, chain.SourceToMc({ File = "a.bf"; Line = 9; Column = 1; Length = 1 }))

    [<Fact>]
    let ``optimizer contraction feeds forward lookup`` () =
        let chain = SourceMapChain.New()
        let position = { File = "t.bf"; Line = 1; Column = 1; Length = 1 }
        chain.SourceToAst.Add(position, 0)
        chain.AstToIr.Add(0, [ 1L; 2L; 3L ])

        let pass = IrToIr("contraction")
        pass.AddMapping(1L, [ 100L ])
        pass.AddMapping(2L, [ 100L ])
        pass.AddMapping(3L, [ 100L ])
        chain.AddOptimizerPass pass

        let machineCode = IrToMachineCode()
        machineCode.Add(100L, 0L, 4L)
        chain.IrToMachineCode <- Some machineCode

        match chain.SourceToMc position with
        | None -> failwith "expected machine-code entries"
        | Some results ->
            Assert.Equal(3, results.Length)
            Assert.All(results, fun entry -> Assert.Equal(100L, entry.IrId))

    [<Fact>]
    let ``optimizer deletion drops forward results`` () =
        let chain = SourceMapChain.New()
        let position = { File = "t.bf"; Line = 1; Column = 1; Length = 1 }
        chain.SourceToAst.Add(position, 0)
        chain.AstToIr.Add(0, [ 1L; 2L ])

        let pass = IrToIr("dead-store")
        pass.AddMapping(1L, [ 1L ])
        pass.AddDeletion 2L
        chain.AddOptimizerPass pass

        let machineCode = IrToMachineCode()
        machineCode.Add(1L, 0L, 4L)
        machineCode.Add(2L, 4L, 4L)
        chain.IrToMachineCode <- Some machineCode

        match chain.SourceToMc position with
        | None -> failwith "expected one machine-code entry"
        | Some results ->
            Assert.Single(results) |> ignore
            Assert.Equal(1L, results.Head.IrId)

    [<Fact>]
    let ``reverse lookup fails when optimizer trace is missing`` () =
        let chain = SourceMapChain.New()
        chain.SourceToAst.Add({ File = "t.bf"; Line = 1; Column = 1; Length = 1 }, 0)
        chain.AstToIr.Add(0, [ 1L ])

        let pass = IrToIr("renumber")
        pass.AddMapping(1L, [ 100L ])
        chain.AddOptimizerPass pass

        let machineCode = IrToMachineCode()
        machineCode.Add(200L, 0L, 4L)
        chain.IrToMachineCode <- Some machineCode

        Assert.Equal(None, chain.McToSource 0L)
