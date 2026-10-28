# Android 0.7.3 upstream integration

Reviewed 51 commits on stable `upstream/main` after `b6c936f8`, through
`e1b2b0b2`. This includes merge history, not 51 independent fixes.

- World entry: keep server chunk workers supplied between ticks while players'
  chunks load; respect paused state and respawn searches.
- Gameplay: spawn eggs configure trial spawners, with their state reset.
- Refactoring: move furnace recipe logic into its own module and share biome
  spawn-entry/cost parsing. Chat messages are logged once.
- Desktop launcher: source-only `.hnmod` bundles and a pinned Rust build
  toolchain, offline-name cleanup, skins and Windows server-path fixes.
- Build tooling: nightly source archives, scheduling and harness path fixes.

Android's shared browser filters source-only native mods and rejects their
installation. It retains `.hmod` support and does not build source mods on the
phone. Shared account and skin changes are retained. The Android upload scheduler,
SAF bridge, launcher and touch controls remain in place. Upstream deployment
workflows remain under `.github/upstream-workflows/` with `.disabled` suffixes;
their tests read those reference files without activating them.

Validation uses ARM64/ARMv7 release-profile Clippy, Android source/content tests
and upstream nightly-tool tests. Full gameplay parity and on-device world-entry
timings remain separate. Upstream desktop timings are not Android FPS results.

## Follow-up through d0b6e489

Four further stable commits add two changes: cache feature decoration by biome
source in `GeneratorData` and preload the Overworld's decoration on world screens;
bound the standalone server supervisor's HTTP request/header lines to 8 KiB.
The decoration cache has four entries, uses `OnceLock` to share concurrent builds,
and is tied to its resource bundle. It does not reuse seed-dependent generation.
The HTTP change affects the standalone supervisor, not the Android launcher.

Follow-up validation: ARM64/ARMv7 release Clippy, Android source tests, and
`horizon-world`'s `decoration_is_shared_by_biome_source` Rust test with prepared
worldgen resources. No complete game APK was rebuilt for this check.

## Follow-up through f66f66c3

19 commits including merge history, after `d0b6e489`:

- Bound ZIP raw allocation by archive length and use saturating, capped
  decompression-budget arithmetic for untrusted declared sizes. The host content
  test runner now includes the actual archive reader and its malformed ZIP64 test.
- Store the cursor at the window centre when pointer grab changes, preserving
  Android's existing physical-mouse capture and touch input paths.
- Open Create New World after deleting the final world or refreshing an empty
  world list. Cancelling returns to the title screen.
- Add regression coverage for shulker rotation, entity view-scale calculation and
  remote mining sound cadence; these net changes are chiefly tests.

Validation: 12 archive/import/upload Rust tests, 10 Android source tests, the two
capture tests and one remote-mining fixture test, plus ARM64/ARMv7 release Clippy.
No full APK or game session is required for these checks. Deployment workflows
remain inactive and the explicitly registered Android document callback remains.
