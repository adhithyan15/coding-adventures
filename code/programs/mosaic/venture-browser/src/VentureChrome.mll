// VentureChrome — shared browser-chrome layout.
//
// `content-surface` mounts the host-owned Metal or Direct2D viewport through
// Mosaic's typed node-slot boundary; browser chrome remains authored once here.

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
    }

    HostSurface [ content-surface ] (
      content : slot: content-surface
    )

    Row [ status-bar ] {
      Text [ status-text ] ( content : slot: status-text )
    }
  }
}
