#ifndef PHOTO_PICKER_QT_EFFECTS_H
#define PHOTO_PICKER_QT_EFFECTS_H

class MosaicHost;

// Answer UI59's `files.open` effect on Qt -- see
// code/specs/UI59-files-open-effect.md for the full contract.
//
// Everything else about the application -- props, events, snapshot, restore --
// goes through the standard Mosaic host. This is the one thing a generated
// host cannot do on the application's behalf, because only the host has a
// window to hang a dialog off (the same reason engram-app's Qt host needs its
// own `engram_effects.h`, which this mirrors).
//
// Installed by the generated `main.cpp` through `[host_effects]`, immediately
// after the host is constructed.
void installPhotoPickerEffects(MosaicHost &host);

#endif
