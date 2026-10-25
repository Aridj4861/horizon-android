# Preparing an Android fork for publication

The Android work derives from [ludovic111/horizon](https://github.com/ludovic111/horizon).
Keep the original MIT license/copyright, third-party notices, vendor licenses and
contribution attribution. The Android README identifies this fork and does not
claim upstream parity results certify Android. The remote fork is [Vasysik/horizon-android](https://github.com/Vasysik/horizon-android).
The local `android` branch preserves upstream history; `origin` is the fork
and `upstream` is the original repository.

## Reviewable source snapshot

```sh
python -m unittest discover -s tests -p test_android_source_export.py
python tools/export_android_source.py
```

The ZIP and adjacent SHA-256 report are written under `.local/android/public/`.
`SOURCE-MANIFEST.json` records every exported file and its hash. The exporter
selects source directories, excludes caches, saves, build products, game inputs,
APKs and signing material, and blocks recognized credential patterns. This is
a bounded credential scan, not a guarantee against every possible secret format.
It rejects symbolic links and unexpected large files instead of following them.
Generated tables and Mojang downloads remain private in `.local/`.

Upstream GitHub Actions are retained under `.github/upstream-workflows/` with
`.disabled` extensions in the export. The only active exported workflow is a
manual Android source check: read-only token, no self-hosted runner, no release,
deployment or telemetry upload. The fork also keeps these workflows inactive in its Git tree; upstream history
preserves their original versions.

## Restore a real upstream relationship

The original source directory lacked `.git`. Its files were compared by Git
blob hashes against 309 first-parent upstream revisions. The selected base,
`d65282fb0f0fd7ba57876aef87dc4c6c2b5fa04b`, matched 3106 of 3147 shared source
files; the 41 differing existing files correspond to port work and fixes.
The actual upstream commit graph was restored, not recreated with invented
original authors. Evidence is recorded in [android-upstream.json](android-upstream.json).

Develop on `android` and push to `origin`. Fetch upstream separately and review
merges; do not replace a working Android engine with newer upstream files by
copying a snapshot. The fork's original `main` is preserved. Before publishing,
run `python tools/check_public_tree.py --history`; `.githooks` supplies the
commit guard. Agent commits carry their own Co-Authored-By trailer.

The inactive upstream workflows and the sole manual Android check now also
apply to the Android Git branch, rather than only to ZIP exports. Original
Bedrock textures are supplied through `tools/android_bedrock_controls.py` to
private `.local/android/bedrock-controls.zip`, with pinned Git blob checks and
the original Mojang LICENSE.md. The PNGs and generated ZIP never enter Git.

## Publishing the Android application

The Android application can have public APK releases, just as upstream publishes
its desktop application. Upstream's `xtask/src/main.rs` explicitly excludes
Minecraft files from its packages and ships `horizon-prepare`; the launcher then
prepares the user's resources separately. See the
[upstream packager](https://github.com/ludovic111/horizon/blob/74768d2eeb2218c81f42680f51ac3a8ec92f9c57/xtask/src/main.rs)
and [resource preparation](https://github.com/ludovic111/horizon/blob/74768d2eeb2218c81f42680f51ac3a8ec92f9c57/launcher/src/backend.rs).
Publishing this fork does not require permission from upstream under its MIT
license; retain its copyright and license notices.

Developer APKs contain generated `engine-data.zip` and the original Bedrock
sample control textures. They remain private local build products. A public
source tree or a passing privacy scan does not authorize redistribution of those
Mojang files. Do not upload the existing developer APKs to GitHub Releases.
The [Minecraft EULA](https://www.minecraft.net/en-us/eula) distinguishes original
tools/mods from redistribution of Minecraft content.

`python tools/android_build.py --abi arm64-v8a --release-candidate` creates a
non-debuggable, optimized local candidate, keeps the existing installation's
private signing identity, remaps personal compiler paths and runs
`tools/check_android_release.py`. Its report checks archive paths, known token/key
formats, packaged private data and manifest debuggability; it prints no suspected
credential values. The report explicitly does not clear game-resource licensing.
Keys, account tokens, reports and generated APKs stay in `.local/`.

Before a public playable APK, separate user-owned resource preparation/import
from the application, remove automatic development control, establish the
publisher's production account configuration and review platform/service URLs.
Only then attach a signed APK, its SHA-256 and an audit summary to a prerelease.
The owner supplied four original Android screenshots on 26 October 2026, now
shown in the README. Their instantaneous counters are not benchmark results.
Emulator frame counters are not phone benchmarks.

The Android fork still shares upstream service URLs in parts of the desktop
code. They are not replaced with invented fork services. Public branding,
repository ownership, publisher identity and production endpoints must be real
values chosen by the publisher.

## Upstream integration on 25 October 2026

Merge `6a63e5d8` integrates 286 commits through `6a065fa4`, retaining the Android
entry point, native launcher, touch rays and compatibility rendering. The old
working state is preserved locally at `backup/android-0.5.1`; no history was
rewritten. The new mod-registry workflow is retained inactive like the other
upstream workflows. The source manifest now records the merged upstream revision.


Merge `dc6d3126` integrates 138 more commits through `7fd1f311`, including sprint
state, transfers/cookies, remote potion contents and container fixes. Android
0.6.6 also fixes player-profile team resolution and session effects, and provides
standard touch settings with circle size and optional analog movement.

Merge `4a2840e1` integrates 59 more commits through `74768d2e` on 26 October,
including wire item components, localized UI, furnace XP fixes and the desktop
OpenGL fallback. Android continues selecting its existing Vulkan modes. The
upstream mod UI remains experimental and requires a mod to opt in explicitly.
