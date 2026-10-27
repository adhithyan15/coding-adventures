// Each of the two 400px blocks sits inside one of the two shapes under
// test. `compact` is false by default (the Qt shell does not set it), so
// the live branch is the two-child `Else`. The viewport is about 520px
// tall, so the content outgrows it only when BOTH the region and the
// branch body publish their implicit height: either one collapsing to 0
// leaves less content than viewport.
layout Shell {
  Column [ app-shell ] {
    Row [ app-header ] {
      Text ( content : slot: title )
    }
    HostScroll [ content-scroll ] {
      Column [ content ] {
        Box [ region ] {
          Column [ region-body ] {
            Text ( content : "A padded region with no size of its own" )
            Box [ region-block ] { }
          }
        }
        If ( when: slot: compact ) {
          Text ( content : "Compact" )
        } Else {
          Text ( content : "A branch body with two children" )
          Box [ details-block ] { }
        }
      }
    }
  }
}
