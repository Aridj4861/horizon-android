# Android 0.7.3 development

Changes after the published 0.7.2:

- Integrate 74 upstream commits, including feeding server chunk workers between
  ticks during world entry and caching biome-source feature decoration. See
  [integration review](android-upstream-review-0.7.3.md).
- Spread completed terrain mesh uploads over frames on Android instead of
  draining an unbounded batch. Use an 8 MiB transfer target and 2–8 ms of work,
  adapting to the previous frame's GPU/display wait. Existing stricter user
  budgets remain effective; one oversized section can advance to avoid starvation.
  This changes scheduling, not render distance, simulation or texture quality.
- Add timings for vertex/index allocation, vertex staging and upload submission.
  Reports also include pending mesh bytes, menu blur radius and shader-pack state.
- Add the Shaders catalog tab. It uses Horizon's registry; unpublished shader
  packs are not fabricated. Classic shader ZIPs can be imported, with experimental
  compatibility checked by the game's existing loader.
- Import `.hmod`, resource-pack ZIP and shader ZIP/`.hshader` files from Android's
  system picker. Open Folder separately browses the corresponding directory in
  system Files through Horizon Android's DocumentsProvider.
- Split Installed into three collapsible groups. Imported and downloaded add-on
  files can be selected or removed with confirmation.
- Stage imports in Android's cache, validate archives on a background thread,
  support cancellation, and ask before replacing an existing file. Keep both
  renames pack files; native mods require unique manifest IDs.
- Explicitly register the Activity result callback with JNI. This fixes a crash
  on file selection and Back caused by Java not finding a library loaded by
  NativeActivity. The isolated Rust document probe checks selection and cancellation.
- Remove duplicate import toasts, retain the status line and bundled-font dialogs, and open the
  matching Installed group after successful launcher imports/downloads.
- Keep catalog tabs responsive during searches, cache recent query pages and
  avoid unpacking installed mod code when browsing tabs or receiving search results.
- Add detailed buffer creation/error-scope/growth timings and source/movement
  metadata to performance reports. These diagnostics do not establish an FPS gain.

- Expose Mods, Resource Packs, Shader Packs and read-only Reports in system Files.
  Keep game files in app storage and screenshots in Pictures/HorizonAndroid.
- Restore Android keyboard input when editing signs; hide gameplay touch controls
  over the sign editor, retain the IME between lines and dismiss it on Done.
- Use Bionic `flock` for resource-pack cache synchronization on Android instead
  of the unsupported standard-library file lock.
- Include the generated structure rotation/mirror table in public APKs and restore
  it locally for existing installs. Its omission disabled structure generation.
  Existing chunks are not regenerated; new structures require unexplored terrain.

Two explicitly approved Java platform adapters forward Activity results and
DocumentsProvider callbacks to Rust. File operations, validation, game and UI
remain Rust. The NativeActivity host and single engine library are retained.

No world-folder migration or general storage permission is included.
World import/export remains a separate task. See
[Android add-on files](android-content.md).

Issue #7 also reports a potion-pickup disconnect, without a packet/error log or
reproduction fixture; that problem is not claimed fixed. Far terrain is currently
disabled by the engine for external multiplayer servers, independently of Android.

The package uses versionName `0.7.3` and versionCode `24`. Source checks do not
establish a phone FPS improvement or replace an on-device picker/import test.
