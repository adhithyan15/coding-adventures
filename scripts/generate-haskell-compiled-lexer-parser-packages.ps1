param(
    [string]$Root = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
)

$ErrorActionPreference = 'Stop'

$grammarToolsDir = Join-Path $Root 'code\packages\haskell\grammar-tools'

function Add-GrammarToolsDependency {
    param([string]$PackageDir)

    $cabalPath = (Get-ChildItem $PackageDir -Filter *.cabal | Select-Object -First 1).FullName
    $content = Get-Content $cabalPath -Raw
    $updated = $false
    if ($content -notmatch '(?m)^\s*,\s*containers\s*$') {
        $content = [regex]::Replace(
            $content,
            '(build-depends:\s+base >=4\.14\r?\n)',
            "`$1                    , containers`r`n",
            1
        )
        $updated = $true
    }
    if ($content -notmatch '(?m)^\s*,\s*grammar-tools\s*$') {
        $content = [regex]::Replace(
            $content,
            '(build-depends:\s+base >=4\.14\r?\n)',
            "`$1                    , grammar-tools`r`n",
            1
        )
        $updated = $true
    }
    if ($updated) {
        Set-Content $cabalPath $content
    }

    $projectPath = Join-Path $PackageDir 'cabal.project'
    $project = Get-Content $projectPath -Raw
    if ($project -notmatch '(?m)^\s*\.\./grammar-tools\s*$') {
        $project = $project -replace '(packages:\r?\n\s*\.)', "`$1`r`n  ../grammar-tools"
        Set-Content $projectPath $project
    }
}

function Ensure-OtherModule {
    param(
        [string]$PackageDir,
        [string]$ModuleName
    )

    $cabalPath = (Get-ChildItem $PackageDir -Filter *.cabal | Select-Object -First 1).FullName
    $content = Get-Content $cabalPath -Raw
    if ($content -notmatch [regex]::Escape($ModuleName)) {
        $content = [regex]::Replace(
            $content,
            '(exposed-modules:\s+[^\r\n]+\r?\n)',
            "`$1    other-modules:    $ModuleName`r`n",
            1
        )
        Set-Content $cabalPath $content
    }
}

function Write-LexerPackage {
    param(
        [string]$PackageDir,
        [string]$ModuleName,
        [string]$BasePascal,
        [string]$BaseCamel,
        [string]$GrammarPath
    )

    Add-GrammarToolsDependency $PackageDir
    Ensure-OtherModule $PackageDir 'Generated.TokenGrammar'

    $srcDir = Join-Path $PackageDir 'src'
    New-Item -ItemType Directory -Force -Path (Join-Path $srcDir 'Generated') | Out-Null

    Push-Location $grammarToolsDir
    try {
        cabal exec grammar-tools-cli -- compile-tokens $GrammarPath 'Generated.TokenGrammar' (Join-Path $srcDir 'Generated\TokenGrammar.hs') | Out-Null
    }
    finally {
        Pop-Location
    }

    $packageName = Split-Path $PackageDir -Leaf
    $wrapper = @"
module $ModuleName
    ( description
    , ${BaseCamel}LexerKeywords
    , tokenize$BasePascal
    ) where

import Generated.TokenGrammar (tokenGrammarData)
import GrammarTools.TokenGrammar (tokenGrammarKeywords)
import Lexer (LexerError, Token, tokenizeWithGrammar)

description :: String
description = "Haskell $packageName backed by compiled token grammar data"

${BaseCamel}LexerKeywords :: [String]
${BaseCamel}LexerKeywords = tokenGrammarKeywords tokenGrammarData

tokenize$BasePascal :: String -> Either LexerError [Token]
tokenize$BasePascal = tokenizeWithGrammar tokenGrammarData
"@
    Set-Content (Join-Path $srcDir ($ModuleName + '.hs')) $wrapper
}

function Write-ParserPackage {
    param(
        [string]$PackageDir,
        [string]$ModuleName,
        [string]$BasePascal,
        [string]$BaseCamel,
        [string]$GrammarPath
    )

    Add-GrammarToolsDependency $PackageDir
    Ensure-OtherModule $PackageDir 'Generated.ParserGrammar'

    $srcDir = Join-Path $PackageDir 'src'
    New-Item -ItemType Directory -Force -Path (Join-Path $srcDir 'Generated') | Out-Null

    Push-Location $grammarToolsDir
    try {
        cabal exec grammar-tools-cli -- compile-grammar $GrammarPath 'Generated.ParserGrammar' (Join-Path $srcDir 'Generated\ParserGrammar.hs') | Out-Null
    }
    finally {
        Pop-Location
    }

    $packageName = Split-Path $PackageDir -Leaf
    $wrapper = @"
module $ModuleName
    ( description
    , ${BasePascal}ParserError(..)
    , parse${BasePascal}Tokens
    , tokenizeAndParse$BasePascal
    ) where

import Generated.ParserGrammar (parserGrammarData)
import Lexer (LexerError, Token)
import Parser (ASTNode, ParseError, parseWithGrammar)
import qualified ${BasePascal}Lexer

description :: String
description = "Haskell $packageName backed by compiled parser grammar data"

data ${BasePascal}ParserError
    = ${BasePascal}ParserLexerError LexerError
    | ${BasePascal}ParserParseError ParseError
    deriving (Eq, Show)

parse${BasePascal}Tokens :: [Token] -> Either ParseError ASTNode
parse${BasePascal}Tokens = parseWithGrammar parserGrammarData

tokenizeAndParse$BasePascal :: String -> Either ${BasePascal}ParserError ASTNode
tokenizeAndParse$BasePascal source =
    case ${BasePascal}Lexer.tokenize$BasePascal source of
        Left err -> Left (${BasePascal}ParserLexerError err)
        Right tokens ->
            case parse${BasePascal}Tokens tokens of
                Left err -> Left (${BasePascal}ParserParseError err)
                Right ast -> Right ast
"@
    Set-Content (Join-Path $srcDir ($ModuleName + '.hs')) $wrapper
}

$packageSpecs = @(
    @{ Base = 'algol'; Pascal = 'Algol'; Camel = 'algol'; LexerGrammar = 'code\grammars\algol.tokens'; ParserGrammar = 'code\grammars\algol.grammar' },
    @{ Base = 'csharp'; Pascal = 'Csharp'; Camel = 'csharp'; LexerGrammar = 'code\grammars\csharp\csharp12.0.tokens'; ParserGrammar = 'code\grammars\csharp\csharp12.0.grammar' },
    @{ Base = 'css'; Pascal = 'Css'; Camel = 'css'; LexerGrammar = 'code\grammars\css.tokens'; ParserGrammar = 'code\grammars\css.grammar' },
    @{ Base = 'dartmouth-basic'; Pascal = 'DartmouthBasic'; Camel = 'dartmouthBasic'; LexerGrammar = 'code\grammars\dartmouth_basic.tokens'; ParserGrammar = 'code\grammars\dartmouth_basic.grammar' },
    @{ Base = 'ecmascript-es1'; Pascal = 'EcmascriptEs1'; Camel = 'ecmascriptEs1'; LexerGrammar = 'code\grammars\ecmascript\es1.tokens'; ParserGrammar = $null },
    @{ Base = 'ecmascript-es3'; Pascal = 'EcmascriptEs3'; Camel = 'ecmascriptEs3'; LexerGrammar = 'code\grammars\ecmascript\es3.tokens'; ParserGrammar = $null },
    @{ Base = 'ecmascript-es5'; Pascal = 'EcmascriptEs5'; Camel = 'ecmascriptEs5'; LexerGrammar = 'code\grammars\ecmascript\es5.tokens'; ParserGrammar = $null },
    @{ Base = 'excel'; Pascal = 'Excel'; Camel = 'excel'; LexerGrammar = 'code\grammars\excel.tokens'; ParserGrammar = 'code\grammars\excel.grammar' },
    @{ Base = 'fsharp'; Pascal = 'Fsharp'; Camel = 'fsharp'; LexerGrammar = 'code\grammars\fsharp\fsharp10.tokens'; ParserGrammar = 'code\grammars\fsharp\fsharp10.grammar' },
    @{ Base = 'java'; Pascal = 'Java'; Camel = 'java'; LexerGrammar = 'code\grammars\java\java21.tokens'; ParserGrammar = 'code\grammars\java\java21.grammar' },
    @{ Base = 'javascript'; Pascal = 'Javascript'; Camel = 'javascript'; LexerGrammar = 'code\grammars\javascript.tokens'; ParserGrammar = 'code\grammars\javascript.grammar' },
    @{ Base = 'json'; Pascal = 'Json'; Camel = 'json'; LexerGrammar = 'code\grammars\json.tokens'; ParserGrammar = 'code\grammars\json.grammar' },
    @{ Base = 'lattice'; Pascal = 'Lattice'; Camel = 'lattice'; LexerGrammar = 'code\grammars\lattice.tokens'; ParserGrammar = 'code\grammars\lattice.grammar' },
    @{ Base = 'lisp'; Pascal = 'Lisp'; Camel = 'lisp'; LexerGrammar = 'code\grammars\lisp.tokens'; ParserGrammar = 'code\grammars\lisp.grammar' },
    @{ Base = 'mosaic'; Pascal = 'Mosaic'; Camel = 'mosaic'; LexerGrammar = 'code\grammars\mosaic.tokens'; ParserGrammar = 'code\grammars\mosaic.grammar' },
    @{ Base = 'nib'; Pascal = 'Nib'; Camel = 'nib'; LexerGrammar = 'code\grammars\nib.tokens'; ParserGrammar = 'code\grammars\nib.grammar' },
    @{ Base = 'python'; Pascal = 'Python'; Camel = 'python'; LexerGrammar = 'code\grammars\python.tokens'; ParserGrammar = 'code\grammars\python.grammar' },
    @{ Base = 'ruby'; Pascal = 'Ruby'; Camel = 'ruby'; LexerGrammar = 'code\grammars\ruby.tokens'; ParserGrammar = 'code\grammars\ruby.grammar' },
    @{ Base = 'sql'; Pascal = 'Sql'; Camel = 'sql'; LexerGrammar = 'code\grammars\sql.tokens'; ParserGrammar = 'code\grammars\sql.grammar' },
    @{ Base = 'starlark'; Pascal = 'Starlark'; Camel = 'starlark'; LexerGrammar = 'code\grammars\starlark.tokens'; ParserGrammar = 'code\grammars\starlark.grammar' },
    @{ Base = 'toml'; Pascal = 'Toml'; Camel = 'toml'; LexerGrammar = 'code\grammars\toml.tokens'; ParserGrammar = 'code\grammars\toml.grammar' },
    @{ Base = 'typescript'; Pascal = 'Typescript'; Camel = 'typescript'; LexerGrammar = 'code\grammars\typescript.tokens'; ParserGrammar = 'code\grammars\typescript.grammar' },
    @{ Base = 'verilog'; Pascal = 'Verilog'; Camel = 'verilog'; LexerGrammar = 'code\grammars\verilog.tokens'; ParserGrammar = 'code\grammars\verilog.grammar' },
    @{ Base = 'vhdl'; Pascal = 'Vhdl'; Camel = 'vhdl'; LexerGrammar = 'code\grammars\vhdl.tokens'; ParserGrammar = 'code\grammars\vhdl.grammar' }
)

foreach ($spec in $packageSpecs) {
    $lexerDir = Join-Path $Root ('code\packages\haskell\' + $spec.Base + '-lexer')
    if (Test-Path $lexerDir) {
        Write-LexerPackage $lexerDir ($spec.Pascal + 'Lexer') $spec.Pascal $spec.Camel (Join-Path $Root $spec.LexerGrammar)
    }

    if ($spec.ParserGrammar) {
        $parserDir = Join-Path $Root ('code\packages\haskell\' + $spec.Base + '-parser')
        if (Test-Path $parserDir) {
            Write-ParserPackage $parserDir ($spec.Pascal + 'Parser') $spec.Pascal $spec.Camel (Join-Path $Root $spec.ParserGrammar)
        }
    }
}
