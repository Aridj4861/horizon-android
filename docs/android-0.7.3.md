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
  system picker. The corresponding in-game Open Folder buttons launch import.
- Split Installed into three collapsible groups. Imported and downloaded add-on
  files can be selected or removed with confirmation.
- Stage imports in Android's cache, validate archives on a background thread,
  support cancellation, and ask before replacing an existing file. Keep both
  renames pack files; native mods require unique manifest IDs.
- Explicitly register the Activity result callback with JNI. This fixes a crash
  on file selection and Back caused by Java not finding a library loaded by
  NativeActivity. The isolated Rust document probe checks selection and cancellation.
- Use the bundled UI font for import notifications and dialogs, and open the
  matching Installed group after successful launcher imports/downloads.
- Add detailed buffer creation/error-scope/growth timings and source/movement
  metadata to performance reports. These diagnostics do not establish an FPS gain.

The only project Java runtime class is the explicitly approved
`HorizonActivity.onActivityResult` bridge. It forwards the selected URI or cancel
result to Rust. Intent creation, ContentResolver access, validation, progress,
selection, UI and file operations remain Rust. The NativeActivity host and single
engine library are retained.

No world-folder migration, DocumentsProvider or general storage permission is
included. World import/export remains a separate task. See
[Android add-on files](android-content.md).

The package uses versionName `0.7.3` and versionCode `24`. Source checks do not
establish a phone FPS improvement or replace an on-device picker/import test.
