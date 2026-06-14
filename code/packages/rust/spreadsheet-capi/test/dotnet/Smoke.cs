// Smoke.cs — prove the spreadsheet C ABI is callable from .NET via P/Invoke
// (the path XAML / WinUI uses), computing the same results as the other
// engines. Run with CAPI_LIB pointing at the built shared library:
//   CAPI_LIB=.../libspreadsheet_capi.dylib dotnet run --project test/dotnet
using System;
using System.Runtime.InteropServices;

static class Sc
{
    // Resolve "spreadsheet_capi" to the path in CAPI_LIB at load time.
    static Sc()
    {
        NativeLibrary.SetDllImportResolver(typeof(Sc).Assembly, (name, asm, path) =>
            name == "spreadsheet_capi"
                ? NativeLibrary.Load(Environment.GetEnvironmentVariable("CAPI_LIB")!)
                : IntPtr.Zero);
    }

    [DllImport("spreadsheet_capi")] public static extern IntPtr sc_session_new();
    [DllImport("spreadsheet_capi")] public static extern void sc_session_free(IntPtr s);
    [DllImport("spreadsheet_capi")]
    public static extern IntPtr sc_set_cell(IntPtr s,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string a1,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string raw);
    [DllImport("spreadsheet_capi")]
    public static extern IntPtr sc_get_value(IntPtr s,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string a1);
    [DllImport("spreadsheet_capi")] public static extern void sc_string_free(IntPtr p);

    public static string Take(IntPtr p)
    {
        if (p == IntPtr.Zero) return "(null)";
        var s = Marshal.PtrToStringUTF8(p) ?? "";
        sc_string_free(p);
        return s;
    }
}

class Program
{
    static int failures = 0;
    static void Check(string label, string got, string needle)
    {
        bool ok = got.Contains(needle);
        if (!ok) failures++;
        Console.WriteLine($"{(ok ? "ok  " : "FAIL")}  {label}: {got}");
    }

    static int Main()
    {
        var s = Sc.sc_session_new();
        foreach (var (a, v) in new[] { ("B1","15"), ("B2","8"), ("B3","12"), ("B4","4"), ("B5","7") })
            Sc.Take(Sc.sc_set_cell(s, a, v));
        Sc.Take(Sc.sc_set_cell(s, "B6", "=SUM(B1:B5)"));
        Sc.Take(Sc.sc_set_cell(s, "B7", "=AVERAGE(B1:B5)"));
        Sc.Take(Sc.sc_set_cell(s, "C1", "=1/0"));

        Check("B6 SUM",        Sc.Take(Sc.sc_get_value(s, "B6")), "\"value\":46");
        Check("B7 AVERAGE",    Sc.Take(Sc.sc_get_value(s, "B7")), "\"value\":9.2");
        Check("C1 div-by-0",   Sc.Take(Sc.sc_get_value(s, "C1")), "#DIV/0!");
        Sc.Take(Sc.sc_set_cell(s, "B1", "115"));
        Check("B6 after edit", Sc.Take(Sc.sc_get_value(s, "B6")), "\"value\":146");

        Sc.sc_session_free(s);
        Console.WriteLine(failures == 0 ? "\nALL PASS" : $"\n{failures} FAILURE(S)");
        return failures == 0 ? 0 : 1;
    }
}
