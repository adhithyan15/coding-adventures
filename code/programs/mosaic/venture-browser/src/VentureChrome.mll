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
      HostButton [ bookmarks-button ] (
        label : slot: bookmarks-label ,
        disabled : slot: bookmarks-disabled ,
        state-when-disabled : slot: bookmarks-disabled ,
        onClick : emit: onBookmarksOpen
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
      HostButton [ share-page-button ] (
        label : "Share" ,
        disabled : slot: share-page-disabled ,
        state-when-disabled : slot: share-page-disabled ,
        onClick : emit: onSharePage
      )
      HostButton [ page-info-button ] (
        label : "Info" ,
        disabled : slot: page-info-disabled ,
        state-when-disabled : slot: page-info-disabled ,
        onClick : emit: onPageInfo
      )
      HostButton [ zoom-out-button ] (
        label : "Zoom Out" ,
        disabled : slot: zoom-out-disabled ,
        state-when-disabled : slot: zoom-out-disabled ,
        onClick : emit: onZoomOut
      )
      HostButton [ zoom-reset-button ] (
        label : slot: zoom-label ,
        disabled : slot: zoom-reset-disabled ,
        state-when-disabled : slot: zoom-reset-disabled ,
        onClick : emit: onZoomReset
      )
      HostButton [ zoom-in-button ] (
        label : "Zoom In" ,
        disabled : slot: zoom-in-disabled ,
        state-when-disabled : slot: zoom-in-disabled ,
        onClick : emit: onZoomIn
      )
      HostButton [ view-source-button ] (
        label : "Source" ,
        disabled : slot: view-source-disabled ,
        state-when-disabled : slot: view-source-disabled ,
        onClick : emit: onViewSource
      )
    }

    If ( when: slot: bookmarks-open ) {
      Column [ bookmarks-panel ] {
        Row [ bookmarks-header ] {
          Text [ bookmarks-heading ] ( content : "Bookmarks" , a11y-role : heading )
          Text [ bookmarks-position ] ( content : slot: bookmarks-position )
          HostButton [ bookmarks-close-button ] (
            label : "Close" ,
            onClick : emit: onBookmarksClose
          )
        }
        Text [ bookmarks-title ] ( content : slot: bookmarks-title )
        Text [ bookmarks-address ] ( content : slot: bookmarks-address )
        Row [ bookmarks-actions ] {
          HostButton [ bookmarks-previous-button ] (
            label : "Previous" ,
            disabled : slot: bookmarks-previous-disabled ,
            state-when-disabled : slot: bookmarks-previous-disabled ,
            onClick : emit: onBookmarksPrevious
          )
          HostButton [ bookmarks-next-button ] (
            label : "Next" ,
            disabled : slot: bookmarks-next-disabled ,
            state-when-disabled : slot: bookmarks-next-disabled ,
            onClick : emit: onBookmarksNext
          )
          HostButton [ bookmarks-open-button ] (
            label : "Open" ,
            disabled : slot: bookmarks-navigate-disabled ,
            state-when-disabled : slot: bookmarks-navigate-disabled ,
            onClick : emit: onBookmarksNavigate
          )
        }
      }
    }

    If ( when: slot: page-info-open ) {
      Column [ page-info-panel ] {
        Row [ page-info-header ] {
          Text [ page-info-heading ] ( content : "Page Information" , a11y-role : heading )
          HostButton [ page-info-close-button ] (
            label : "Close" ,
            onClick : emit: onPageInfoClose
          )
        }
        Row [ page-info-title-row ] {
          Text [ page-info-title-label ] ( content : "Title" )
          Text [ page-info-title-value ] ( content : slot: page-info-title )
        }
        Row [ page-info-address-row ] {
          Text [ page-info-address-label ] ( content : "Address" )
          Text [ page-info-address-value ] ( content : slot: page-info-address )
        }
        Row [ page-info-requested-row ] {
          Text [ page-info-requested-label ] ( content : "Requested" )
          Text [ page-info-requested-value ] ( content : slot: page-info-requested-address )
        }
        Row [ page-info-status-row ] {
          Text [ page-info-status-label ] ( content : "Status" )
          Text [ page-info-status-value ] ( content : slot: page-info-status )
        }
        Row [ page-info-resources-row ] {
          Text [ page-info-resources-label ] ( content : "Resources" )
          Text [ page-info-resources-value ] ( content : slot: page-info-resources )
        }
      }
    }

    If ( when: slot: view-source-open ) {
      Column [ view-source-panel ] {
        Row [ view-source-header ] {
          Text [ view-source-heading ] ( content : "Page Source" , a11y-role : heading )
          HostButton [ view-source-copy-button ] (
            label : "Copy Source" ,
            onClick : emit: onViewSourceCopy
          )
          HostButton [ view-source-close-button ] (
            label : "Close Source" ,
            onClick : emit: onViewSourceClose
          )
        }
        Text [ view-source-title ] ( content : slot: view-source-title )
        Text [ view-source-address ] ( content : slot: view-source-address )
        Text [ view-source-content ] ( content : slot: view-source-content )
      }
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
