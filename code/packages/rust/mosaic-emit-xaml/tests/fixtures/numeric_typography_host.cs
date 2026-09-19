using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Media;
using System;
using System.Collections.Generic;
using System.IO;
using System.Threading.Tasks;

namespace Mosaic.Generated;

public static class MosaicHost
{
    public static string ApplyProps(Typography component)
    {
        component.Headers = new[] { "Header" };
        component.Rows = new[] { new[] { "Cell" } };
        return "Typography table fixture";
    }
    public static void RunInteractionAcceptance(Window window, Typography component)
    {
        component.Loaded += async (_, _) =>
        {
            var output = Environment.GetEnvironmentVariable("MOSAIC_TYPOGRAPHY_RESULT");
            if (string.IsNullOrEmpty(output)) return;
            try
            {
                await Task.Delay(100);
                var text = Find<TextBlock>(component, x => x.Text == "Title");
                var input = Find<TextBox>(component, x => Microsoft.UI.Xaml.Automation.AutomationProperties.GetAutomationId(x) == "input");
                var button = Find<Button>(component, x => (string)x.Content == "Action");
                var header = Find<TextBlock>(component, x => x.Text == "Header");
                var cell = Find<TextBlock>(component, x => x.Text == "Cell");
                var editor = Find<TextBox>(component, x => x.Text == "Cell");
                var fixedLabel = Find<TextBlock>(component, x => x.Text == "Fixed");
                var outside = Find<TextBlock>(component, x => x.Text == "Outside");
                var headerDefault = header.FontSize;
                var cellDefault = cell.FontSize;
                var editorDefault = editor.FontSize;
                var outsideDefault = outside.FontSize;
                if (!editor.Focus(FocusState.Programmatic)) throw new Exception("Editor could not receive focus");
                foreach (var size in new[] { 18.0, 27.0, 36.0, 0.0, double.NaN, double.PositiveInfinity, 24.0, -1.0 })
                {
                    component.TextSize = size;
                    await Task.Delay(50);
                    var valid = double.IsFinite(size) && size > 0;
                    Check(text.FontSize, valid ? size : 18);
                    Check(input.FontSize, valid ? size : 16);
                    Check(button.FontSize, valid ? size : 14);
                    Check(header.FontSize, valid ? size : headerDefault);
                    Check(cell.FontSize, valid ? size : cellDefault);
                    Check(editor.FontSize, valid ? size : editorDefault);
                    Check(fixedLabel.FontSize, 22);
                    Check(outside.FontSize, outsideDefault);
                    if (cell.FontFamily.Source != "Consolas") throw new Exception("Monospaced family lost");
                    if (editor.Text != "Cell" || editor.FocusState == FocusState.Unfocused)
                        throw new Exception("Scaling changed editor content or focus");
                }
                component.TextSize = 30;
                component.Rows = new[] { new[] { "New cell" } };
                await Task.Delay(100);
                Check(Find<TextBlock>(component, x => x.Text == "New cell").FontSize, 30);
                Check(Find<TextBox>(component, x => x.Text == "New cell").FontSize, 30);
                // A control without a local FontSize must recover native inheritance.
                var inherited = new TextBlock();
                var original = inherited.FontSize;
                TypographyMosaicFontSize.SetValue(inherited, 30);
                Check(inherited.FontSize, 30);
                TypographyMosaicFontSize.SetValue(inherited, double.NaN);
                Check(inherited.FontSize, original);
                if (inherited.ReadLocalValue(TextBlock.FontSizeProperty) != DependencyProperty.UnsetValue)
                    throw new Exception("Invalid value left a local font-size override");
                File.WriteAllText(output, "PASS: live table/header/editor typography, focus/content retention, new rows, child precedence, invalid-value restoration");
            }
            catch (Exception error) { File.WriteAllText(output, "FAIL: " + error); }
            finally { window.Close(); }
        };
    }
    private static void Check(double actual, double expected)
    {
        if (actual != expected) throw new Exception($"FontSize expected {expected}, got {actual}");
    }
    private static T Find<T>(DependencyObject root, Func<T, bool> predicate) where T : DependencyObject
    {
        var stack = new Stack<DependencyObject>(); stack.Push(root);
        while (stack.Count > 0)
        {
            var node = stack.Pop();
            if (node is T match && predicate(match)) return match;
            for (var i = 0; i < VisualTreeHelper.GetChildrenCount(node); i++) stack.Push(VisualTreeHelper.GetChild(node, i));
        }
        throw new Exception($"Missing {typeof(T).Name}");
    }
}
