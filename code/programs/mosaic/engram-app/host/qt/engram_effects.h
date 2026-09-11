#ifndef ENGRAM_QT_EFFECTS_H
#define ENGRAM_QT_EFFECTS_H

class MosaicHost;

// Answer Engram's Anki file-dialog effects on Qt.
//
// Everything else about the application — props, events, snapshot, restore —
// goes through the standard Mosaic host. This is the one thing a generated host
// cannot do on the application's behalf, because only the host has a window to
// hang a dialog off.
//
// Installed by the generated `main.cpp` through `[host_effects]`, immediately
// after the host is constructed.
void installEngramEffects(MosaicHost &host);

#endif
