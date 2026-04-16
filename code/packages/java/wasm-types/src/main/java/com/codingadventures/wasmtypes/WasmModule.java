package com.codingadventures.wasmtypes;

import java.util.ArrayList;
import java.util.List;

public final class WasmModule {
    public final List<WasmTypes.FuncType> types = new ArrayList<>();
    public final List<WasmTypes.Import> imports = new ArrayList<>();
    public final List<Integer> functions = new ArrayList<>();
    public final List<WasmTypes.TableType> tables = new ArrayList<>();
    public final List<WasmTypes.MemoryType> memories = new ArrayList<>();
    public final List<WasmTypes.Global> globals = new ArrayList<>();
    public final List<WasmTypes.Export> exports = new ArrayList<>();
    public Integer start = null;
    public final List<WasmTypes.Element> elements = new ArrayList<>();
    public final List<WasmTypes.FunctionBody> code = new ArrayList<>();
    public final List<WasmTypes.DataSegment> data = new ArrayList<>();
    public final List<WasmTypes.CustomSection> customs = new ArrayList<>();
}
