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
    public static void RunInteractionAcceptance(Window window, Typography component)
    {
        component.Loaded += async (_, _) =>
        {
            var output = Environment.GetEnvironmentVariable("MOSAIC_TYPOGRAPHY_RESULT");
            if (string.IsNullOrEmpty(output)) return;
            try
            {
                var text = Find<TextBlock>(component, x => x.Text == "Title");
                var input = Find<TextBox>(component, _ => true);
                var button = Find<Button>(component, x => (string)x.Content == "Action");
                foreach (var size in new[] { 18.0, 27.0, 36.0, 0.0, double.NaN, double.PositiveInfinity, 24.0, -1.0 })
                {
                    component.TextSize = size;
                    await Task.Delay(50);
                    var valid = double.IsFinite(size) && size > 0;
                    Check(text.FontSize, valid ? size : 18);
                    Check(input.FontSize, valid ? size : 16);
                    Check(button.FontSize, valid ? size : 14);
                }
                // A control without a local FontSize must recover native inheritance.
                var inherited = new TextBlock();
                var original = inherited.FontSize;
                TypographyMosaicFontSize.SetValue(inherited, 30);
                Check(inherited.FontSize, 30);
                TypographyMosaicFontSize.SetValue(inherited, double.NaN);
                Check(inherited.FontSize, original);
                if (inherited.ReadLocalValue(TextBlock.FontSizeProperty) != DependencyProperty.UnsetValue)
                    throw new Exception("Invalid value left a local font-size override");
                File.WriteAllText(output, "PASS: live 1x/1.5x/2x typography, invalid-value restoration, native default inheritance");
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
