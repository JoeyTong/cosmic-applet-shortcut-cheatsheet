# Project guidance

## Overview

COSMIC Shortcut Cheatsheet is a native Rust/libcosmic panel applet for displaying
the user's effective COSMIC keyboard shortcuts. It targets COSMIC on Wayland,
uses Rust edition 2024 (minimum Rust 1.93), and is licensed under MPL-2.0.

Read `README.md` for setup and usage. Discuss ambiguous requirements and significant
design or behavior decisions with the user through Q&A before implementing them.

## Project map

- `src/main.rs`: entry point, panel launch, `--preview`, and `--print-shortcuts`.
- `src/app.rs`: panel icon, Wayland popup lifecycle, widgets, and theme styles.
- `src/popup.rs`: click-toggle state machine and stale-surface event tests.
- `src/shortcuts.rs`: configuration loading, merging, formatting, filesystem
  subscriptions, and shortcut/configuration tests.
- `src/lib.rs`: shared modules and application ID.
- `data/shortcut-cheatsheet-symbolic.svg`: reference-card icon, embedded in the
  executable and installed for desktop discovery.
- `data/io.github.joeytong.cosmic-applet-shortcut-cheatsheet.desktop`: panel discovery entry.
- `scripts/install.py`: release build and user-local installation/uninstallation.
- `Cargo.toml` / `Cargo.lock`: dependency revisions and reproducible build settings.

## Product behavior and design

- Clicking the panel icon opens the cheatsheet immediately; clicking again closes it.
- Hover opening, pointer-exit closing, and the **×** button were explicitly removed
  at the user's request. Do not restore them or the old hover/pin timer machinery.
- The popup stays open while the user works in other windows.
- Escape dismisses the focused popup or preview window.
- The popup has no main title. Use native COSMIC colors and typography, subtle
  section cards, accent-colored group icons/headings, individual keycaps, and a
  muted footer. Resolve colors from the active theme rather than hardcoding them.
- The panel uses a monochrome **reference-card icon**. This was deliberately chosen
  to distinguish it from fcitx's keyboard/input-method icon.
- The popup uses a **1.25×** content scale for readability: scale text, icons,
  keycaps, spacing, and surface dimensions together. Its width is 575 logical px.
- Its height limit is **900 logical px**, sized to show all selected default
  shortcuts and the footer without scrolling.

The requested groups are:

| Group | Actions |
| --- | --- |
| Open apps | Terminal, browser, file manager, launcher, screenshot |
| Window management | Maximize, close, float/tile, swap positions, window switcher |
| Workspace management | Workspaces 1–9, next workspace, previous workspace, workspace overview |

The separate **Move left/right/up/down** entries were explicitly removed at the
user's request. Directional keys may still appear in workspace navigation bindings.
The Close window row omits **Alt+F4**, leaving **Super+Q** with the default bindings.
Apply this display filter after merging live settings so disabled/reassigned keys
are still respected; do not change the user's desktop bindings.

## Implementation constraints

### Popup lifecycle

- Keep click opening free of filesystem I/O and artificial delays; use the cached
  cheatsheet. An idle applet should not require a polling timer.
- Use a non-grabbing Wayland popup (`grab = false`) so the cheatsheet stays open
  while the user interacts with other windows.
- Match close and keyboard-dismiss events to the correct surface ID, ignoring old
  popup events. Pointer entry/exit/motion must not trigger popup actions.

### Shortcut configuration

- Use upstream `cosmic-settings-config` binding/action types and `cosmic-config`
  lookup rules for `com.system76.CosmicSettings.Shortcuts/v1`.
- Read `defaults`, then merge `custom` **by binding**, honoring `Disable` and
  reassigned bindings. Treat the user's shortcut configuration as read-only.
- Identify preferred apps through `System(Terminal)`, `System(WebBrowser)`, and
  `System(HomeFolder)`. A custom `Spawn` binding counts only when its command
  matches the configured system action command; do not guess from substrings.
- Display unbound actions as **Not set**. Preserve alternatives with deterministic
  ordering and make their relationship to the modifiers clear.
- Collapse workspaces to `1–9` only when all nine bindings consistently match;
  otherwise show individual rows, including missing bindings.
- Refresh on user configuration notifications, including atomic replacement and
  deletion. Retain the last valid snapshot with a notice if a reload fails.
- System-default changes from package upgrades currently require an applet restart.

## Development and verification

Keep Rust code formatted and consistent with nearby modules. Preserve the pinned
COSMIC-compatible Git revisions and lockfile unless dependency changes are part of
the task. The double-slash Git URLs in `Cargo.toml` are intentional: Cargo uses them
to distinguish the sources used in transitive dependency patches.

For Rust changes, run the relevant existing tests and checks:

```sh
cargo fmt --all -- --check
cargo test --locked --all-targets
cargo clippy --locked --all-targets -- -D warnings
```

Use `cargo fmt --all` to apply formatting. Add focused tests for meaningful binding,
configuration, or interaction changes. Configuration tests must use temporary
directories rather than the real desktop settings. Documentation-only changes do
not need a Rust build; icon-only changes need SVG validation and a rebuild because
the icon is embedded.

```sh
cargo run --locked -- --print-shortcuts
cargo run --locked -- --preview
cargo build --release --locked
```

Use the native preview to inspect visual changes. It shows the same content in a
regular window; actual click-toggle behavior requires the panel applet. Keep temporary
capture helpers, screenshots, and build artifacts out of the maintained source.

## Installation and applying updates

```sh
python3 scripts/install.py             # build and install
python3 scripts/install.py --no-build  # install an existing release build
python3 scripts/install.py --uninstall
```

By default, the binary goes to `~/.local/bin`; the desktop entry and icon go under
`${XDG_DATA_HOME:-~/.local/share}`. `--prefix PATH` supports another prefix.
The desktop entry uses an absolute executable path, and the installer atomically
replaces the binary so a running instance can finish using the old executable.

Installation and live panel refresh are separate operations. Confirm a live
COSMIC Panel refresh with the user, since it reloads the other panel applets too.
Discover the actual service or session supervisor rather than assuming a systemd
unit exists or reusing a saved PID. Directly terminating an individual applet is
not a reliable reload mechanism on this COSMIC stack. After an approved refresh,
verify that the applet belongs to the refreshed panel and uses the newly installed
executable, not merely that a process with the expected name exists.
