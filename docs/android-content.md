# Android add-on files

Use **Mods**, **Packs** or **Shaders → Import file**, or open **Installed** and
press Import beside the desired group. Select a file in Android's document picker.
The in-game Open Mod/Resource Pack/Shader Pack Folder buttons start the same flow.

To download a `.hmod` from a link, paste its HTTPS URL in the Mods search field
and press **Import .hmod** (the Search button changes when a URL is entered).
File import and link import are both available.

Successful imports and catalog installs in the launcher open Installed with the
matching group expanded. Errors and cancellation keep the current page. Import
notifications and conflict dialogs use the launcher's bundled font.

Catalog searches run separately from installation jobs, so switching tabs does
not wait for a network response. At most one search runs at once; rapid changes
replace the pending request, and replies for an old tab cannot replace the current
list. The last eight query pages are cached for 30 seconds; Search explicitly
refreshes its query. Changing tabs does not unpack installed `.hmod` WASM files.

Supported files:

- Mods: native `.hmod` archives; Java `.jar` mods are not accepted. Executing
  `.hmod` mods requires a 64-bit APK, but the files can be managed on 32-bit builds.
- Resource packs: ZIPs with valid `pack.mcmeta` at the archive root.
- Shaders: native `.hshader` packs, native shader ZIPs with `horizon-shaders.toml`
  (stored as `.hshader`), or classic ZIPs with shader sources under `shaders/`.
  A classic ZIP's structure being valid does not guarantee support for all of its
  shader features; the existing game loader reports compatibility errors.

Open Installed to expand the three groups. The list includes regular imported and
downloaded archive files in the current profile. Selection applies at the next
game launch. Removing an add-on requires confirmation and removes only that file,
its browser record and its enabled selection; worlds are not touched.

Imports copy the granted content URI to a temporary Android cache file and validate
it before committing. Existing files are never silently replaced: **Keep both**
chooses a new filename, **Replace** explicitly replaces the selected collision,
and **Cancel** leaves the existing file intact. A native mod with the same manifest
ID must be replaced rather than loaded twice. Cancel is checked between copy and
validation blocks; a document provider may delay an in-flight read.

Limits: 256 MiB input, 512 MiB declared expansion, 32,768 entries and 1,024-byte
entry paths. Native mods are limited to 32 MiB input and 64 MiB expanded data.
Archive entry paths, duplicates, links, CRC/decompression errors and resource-pack
metadata are checked. Archives are kept as files, not extracted into arbitrary
filesystem paths. Invalid imports and interrupted temporary files do not replace
existing content. Original URI grants are not retained after copying.

This does not expose the application's private folders to other apps. It does not
require MANAGE_EXTERNAL_STORAGE. Worlds, external game-folder selection and a
DocumentsProvider are outside this implementation.

Developer checks: `python tools/test_android_content.py` covers the shared import
validator, streamed copy cancellation/limits and upload scheduling without starting
the game. Test the complete APK on Android for picker cancellation, file-provider
permissions, duplicate-name choices, deletion and keyboard/background recovery.

For a native shader smoke test, package Horizon's included Golden Hour example:

```sh
cargo run -p horizon-shader-api --bin horizon-shader -- pack crates/horizon-shader-api/packs/golden_hour --out .local/golden_hour-1.0.0.hshader
```

Transfer the resulting file to the phone, import it under Shaders, then enable it
in Installed before Play, or in the game's Shader Packs settings. It changes the
sky and adds two post-processing passes; it is a functional example, not an
Android performance benchmark.
