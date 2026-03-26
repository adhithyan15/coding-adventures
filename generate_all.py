import os
import subprocess
from pathlib import Path
import copy

root = Path("/Users/adhithya/Downloads/antigravity/coding-adventures")
grammars_dir = root / "code" / "grammars"

def to_camel_case(snake_str):
    components = snake_str.replace('-', '_').split('_')
    return "".join(x.title() for x in components)

def get_pkg_name(lang, grammar_name, kind):
    if lang == "go":
        if grammar_name == "xml_rust": return "xmllexer"
        return f"{grammar_name.replace('-', '')}{kind}"
    if lang == "typescript":
        return f"{grammar_name}-{kind}"
    if lang in ("python", "ruby", "lua", "elixir"):
        return f"{grammar_name}_{kind}"
    if lang == "rust":
        return f"{grammar_name}-{kind}"
    return f"{grammar_name}-{kind}"

def get_output_path(base_dir, lang, grammar_name, kind):
    fname_base = f"{grammar_name}_tokens" if kind == "lexer" else f"{grammar_name}_grammar"
    if lang == "typescript":
        fname_base = fname_base.replace("_", "-")

    if lang == "go":
        return base_dir / f"{fname_base}.go"
    if lang == "python":
        pkg = f"{grammar_name}_{kind}"
        return base_dir / "src" / pkg / f"{fname_base}.py"
    if lang == "ruby":
        pkg = f"{grammar_name}_{kind}"
        return base_dir / "lib" / "coding_adventures" / pkg / f"{fname_base}.rb"
    if lang == "rust":
        return base_dir / "src" / f"{fname_base}.rs"
    if lang == "typescript":
        return base_dir / "src" / f"{fname_base}.ts"
    if lang == "lua":
        pkg = f"{grammar_name}_{kind}"
        return base_dir / "src" / "coding_adventures" / pkg / f"{fname_base}.lua"
    if lang == "elixir":
        return base_dir / "lib" / f"{fname_base}.ex"

for file in grammars_dir.iterdir():
    if not file.is_file():
        continue
    name, ext = file.stem, file.suffix
    if ext == ".tokens":
        kind = "lexer"
        cmd_base = "compile-tokens"
        var_suffix = "Tokens"
    elif ext == ".grammar":
        kind = "parser"
        cmd_base = "compile-grammar"
        var_suffix = "Grammar"
    else:
        continue

    grammar_name = name

    # Search in all languages
    for lang in ["go", "python", "ruby", "rust", "typescript", "lua", "elixir"]:
        lang_dir = root / "code" / "packages" / lang
        if not lang_dir.exists():
            lang_dir = root / "code" / "src" / lang
            if not lang_dir.exists():
                continue

        gn = grammar_name
        if lang == "rust" and gn == "xml_rust":
            gn = "xml"
            
        possible_dirs = [
            lang_dir / f"{gn}-{kind}",
            lang_dir / f"{gn}_{kind}"
        ]
        
        target_dir = None
        for pd in possible_dirs:
            if pd.exists() and pd.is_dir():
                target_dir = pd
                break
        
        if not target_dir:
            continue
            
        out_path = get_output_path(target_dir, lang, gn, kind)
        
        tool_dir = None
        if lang == "elixir" and (root / "code" / "programs" / "elixir" / "grammar-tools").exists():
            tool_dir = root / "code" / "programs" / "elixir" / "grammar-tools"
        elif lang == "typescript" and (root / "code" / "programs" / "typescript" / "grammar-tools").exists():
            tool_dir = root / "code" / "programs" / "typescript" / "grammar-tools"
        elif (lang_dir / "grammar-tools").exists():
            tool_dir = lang_dir / "grammar-tools"
        elif (lang_dir / "grammar_tools").exists():
            tool_dir = lang_dir / "grammar_tools"
        else:
            continue
            
        grammar_file_rel = os.path.relpath(file, tool_dir)
        varName = to_camel_case(gn) + var_suffix
        pkgName = get_pkg_name(lang, gn, kind)

        cmd = []
        if lang == "go":
            cmd = ["go", "run", "cmd/grammar-tools/main.go", cmd_base, grammar_file_rel, pkgName, varName]
        elif lang == "python":
            cmd = ["python3", "-m", "grammar_tools", cmd_base, grammar_file_rel, varName]
        elif lang == "ruby":
            cmd = ["ruby", "bin/grammar-tools", cmd_base, grammar_file_rel, varName]
        elif lang == "rust":
            cmd = ["cargo", "run", "-q", "--bin", "grammar-tools", "--", cmd_base, grammar_file_rel, varName]
        elif lang == "typescript":
            cmd = ["npx", "tsx", "cli.ts", cmd_base, grammar_file_rel, varName]
        elif lang == "lua":
            cmd = ["./grammar-tools", cmd_base, grammar_file_rel, varName]
        elif lang == "elixir":
            cmd = ["mix", f"grammar_tools.{cmd_base.replace('-', '_')}", grammar_file_rel, varName]
            
        print(f"Generating for {lang} {gn} {kind} ...")
        env = copy.deepcopy(os.environ)
        if lang == "lua":
            env["LUA_PATH"] = "./src/?.lua;./src/?/init.lua;;"
        if lang == "python":
            env["PYTHONPATH"] = "./src"
            
        res = subprocess.run(cmd, cwd=tool_dir, env=env, capture_output=True, text=True)
        if res.returncode == 0:
            output = res.stdout
            
            extracted_output = []
            capturing = False
            for line in output.split("\n"):
                if capturing:
                    extracted_output.append(line)
                elif "AUTO-GENERATED FILE" in line:
                    capturing = True
                    extracted_output.append(line)
            
            if extracted_output:
                out_path.parent.mkdir(parents=True, exist_ok=True)
                with open(out_path, "w") as f:
                    f.write("\n".join(extracted_output))
                print(f"  -> Saved {out_path}")
            else:
                print(f"  -> FAILED to find AUTO-GENERATED header in stdout for {lang} {gn}")
                print(res.stdout)
        else:
            print(f"  -> Error {lang}:\n{res.stderr}\n{res.stdout}")
