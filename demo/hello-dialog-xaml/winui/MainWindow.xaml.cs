using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Mosaic.Generated;

namespace Mosaic.HelloDialog.Demo;

public sealed partial class MainWindow : Window
{
    public MainWindow()
    {
        this.InitializeComponent();
    }

    private async void OnOpenButtonClick(object sender, RoutedEventArgs e)
    {
        var xamlRoot = (sender as FrameworkElement)?.XamlRoot;
        if (xamlRoot is null)
        {
            this.StatusText.Text = "Error: button has no XamlRoot";
            return;
        }
        try
        {
            // The generated HelloDialog *IS* a ContentDialog (post-patch).
            // Set its DPs, attach its XamlRoot, subscribe to Dispatch,
            // and show.
            var dlg = new Mosaic.Generated.HelloDialog
            {
                DialogTitle = "Hello from Mosaic",
                Message = "This dialog was authored in Mosaic, compiled to XAML by mosaic-emit-xaml, and rendered by WinUI 3.",
                XamlRoot = xamlRoot,
            };
            dlg.Dispatch += (s, ev) =>
            {
                var label = ev switch
                {
                    HelloDialogEvent.Close => "Dispatch: HelloDialogEvent.Close received — dialog closed.",
                    _ => "Dispatch: <unknown event>"
                };
                this.StatusText.Text = label;
            };
            await dlg.ShowAsync();
        }
        catch (System.Exception ex)
        {
            this.StatusText.Text = $"Exception: {ex.GetType().Name}: {ex.Message}";
        }
    }
}
