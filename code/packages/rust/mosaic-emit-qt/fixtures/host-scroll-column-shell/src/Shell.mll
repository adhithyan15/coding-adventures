// The header sits outside the scroll, so it never scrolls away; the scroll
// grows (`flex-grow : 1`) into whatever height the shell has left, which is
// the only thing that makes it shorter than the 1600px block inside it.
layout Shell {
  Column [ app-shell ] {
    Row [ app-header ] {
      Text [ app-title ] ( content : slot: title )
    }
    HostScroll [ content-scroll ] {
      Column [ content ] {
        Text [ first-line ] ( content : "Top of the scrolled content" )
        Box [ tall-block ] { }
        Text [ last-line ] ( content : "Bottom of the scrolled content" )
      }
    }
  }
}
