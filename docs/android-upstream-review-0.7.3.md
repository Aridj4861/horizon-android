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
