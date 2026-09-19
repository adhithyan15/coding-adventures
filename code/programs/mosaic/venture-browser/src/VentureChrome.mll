// VentureChrome — shared browser-chrome layout.
//
// `content-surface` mounts the host-owned native page viewport through Mosaic's
// typed node-slot boundary; browser chrome remains authored once here.

layout VentureChrome {
  Column [ app-shell ] {
    Row [ title-bar ] {
      Text [ brand ] ( content : "Venture" )
      Text [ page-title ] ( content : slot: page-title , a11y-role : heading )
    }

    Row [ toolbar ] {
      HostButton [ back-button ] (
        label : "Back" ,
        disabled : slot: back-disabled ,
        state-when-disabled : slot: back-disabled ,
        onClick : emit: onBack
      )
      HostButton [ forward-button ] (
        label : "Forward" ,
        disabled : slot: forward-disabled ,
        state-when-disabled : slot: forward-disabled ,
        onClick : emit: onForward
      )
      HostButton [ home-button ] ( label : "Home" , onClick : emit: onHome )
      HostButton [ reload-button ] (
        label : "Reload" ,
        disabled : slot: navigation-disabled ,
        state-when-disabled : slot: navigation-disabled ,
        onClick : emit: onReload
      )
      HostButton [ bookmark-button ] (
        label : slot: bookmark-label ,
        disabled : slot: bookmark-disabled ,
        state-when-disabled : slot: bookmark-disabled ,
        onClick : emit: onToggleBookmark
      )
      HostInput [ address-input ] (
        value : slot: address ,
        placeholder : "Enter a URL" ,
        read-only : slot: navigation-disabled ,
        onChange : emit: onAddressChange ,
        onCommit : emit: onNavigate
      )
      HostButton [ go-button ] (
        label : "Go" ,
        disabled : slot: navigation-disabled ,
        state-when-disabled : slot: navigation-disabled ,
        onClick : emit: onNavigate
      )
      HostButton [ find-button ] (
        label : "Find" ,
        disabled : slot: find-disabled ,
        state-when-disabled : slot: find-disabled ,
        onClick : emit: onFindOpen
      )
    }

    Row [ page-actions ] {
      HostButton [ copy-address-button ] (
        label : "Copy" ,
        disabled : slot: copy-address-disabled ,
        state-when-disabled : slot: copy-address-disabled ,
        onClick : emit: onCopyAddress
      )
      HostButton [ open-page-button ] (
        label : "New Window" ,
        disabled : slot: open-page-disabled ,
        state-when-disabled : slot: open-page-disabled ,
        onClick : emit: onOpenPageInNewWindow
      )
      HostButton [ save-page-button ] (
        label : "Save" ,
        disabled : slot: save-page-disabled ,
        state-when-disabled : slot: save-page-disabled ,
        onClick : emit: onSavePage
      )
      HostButton [ print-page-button ] (
        label : "Print" ,
        disabled : slot: print-page-disabled ,
        state-when-disabled : slot: print-page-disabled ,
        onClick : emit: onPrintPage
      )
      HostButton [ view-source-button ] (
        label : "Source" ,
        disabled : slot: view-source-disabled ,
        state-when-disabled : slot: view-source-disabled ,
        onClick : emit: onViewSource
      )
    }

    If ( when: slot: find-open ) {
      Row [ find-bar ] {
        HostInput [ find-input ] (
          value : slot: find-query ,
          placeholder : "Find in page" ,
          read-only : slot: find-disabled ,
          onChange : emit: onFindChange ,
          onCommit : emit: onFindNext
        )
        Text [ find-result ] ( content : slot: find-result-label )
        HostButton [ find-previous-button ] (
          label : "Previous" ,
          disabled : slot: find-disabled ,
          state-when-disabled : slot: find-disabled ,
          onClick : emit: onFindPrevious
        )
        HostButton [ find-next-button ] (
          label : "Next" ,
          disabled : slot: find-disabled ,
          state-when-disabled : slot: find-disabled ,
          onClick : emit: onFindNext
        )
        HostButton [ find-close-button ] (
          label : "Close Find" ,
          onClick : emit: onFindClose
        )
      }
    }

    HostSurface [ content-surface ] (
      content : slot: content-surface
    )

    Row [ status-bar ] {
      Text [ status-text ] ( content : slot: status-text )
    }
  }
}
