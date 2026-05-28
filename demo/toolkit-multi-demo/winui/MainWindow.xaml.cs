// Showcase host for the mosaic-pkg-toolkit components.
//
// Each component is hosted by x:Name in MainWindow.xaml and wired
// here. Dispatch events bubble up to a single shared status text
// so you can see the .mil emits in action.
//
using Microsoft.UI.Xaml;

namespace Mosaic.Generated;

public sealed partial class MainWindow : Window
{
    public MainWindow()
    {
        this.InitializeComponent();
        this.MyButton.Dispatch += OnButtonDispatch;
        this.MyAlert.Dispatch += OnAlertDispatch;
    }

    private void OnButtonDispatch(object? sender, ButtonEvent ev)
    {
        switch (ev)
        {
            case ButtonEvent.Click:
                this.StatusText.Text = "Dispatch: Button.Click";
                break;
        }
    }

    private void OnAlertDispatch(object? sender, AlertEvent ev)
    {
        switch (ev)
        {
            case AlertEvent.Close:
                this.StatusText.Text = "Dispatch: Alert.Close";
                this.MyAlert.Visibility = Visibility.Collapsed;
                break;
        }
    }
}
