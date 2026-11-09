# COSMIC Shortcut Cheatsheet

A native Rust/libcosmic panel applet for a quick look at your **actual COSMIC shortcuts**.

- **Click** the reference-card icon to open the cheatsheet; click it again to close.
- The cheatsheet stays open while you work in other windows. Hover does not open
  or close it, and there is no **×** button.
- Rounded sections with accent-colored group icons and headers, individual shortcut
  keycaps, and a muted footer—all using native COSMIC theme colors; no main title.
- The popup is scaled up **25%** for readability, including text, icons, keycaps,
  and spacing. Its **575 × 900 px** size shows all included default shortcuts
  without scrolling.

## Included shortcuts

| Group | Actions |
| --- | --- |
| Open apps | Terminal, browser, file manager, launcher, screenshot |
| Window management | Maximize, close, float/tile, swap positions, window switcher |
| Workspace management | Workspaces 1–9, next workspace, previous workspace, workspace overview |

Modifiers and keys appear in separate keycaps. Alternatives with the same modifiers
share the final keycap: `Super + Ctrl + → / ↓ / J / L` means hold **Super + Ctrl**,
then press **one** of those keys. Separate lines are alternative bindings.
Unbound actions show **Not set**.

The Close window row omits the `Alt+F4` alternative, leaving `Super+Q` with COSMIC's
defaults. Custom close bindings still follow live settings. This is a display
preference; the applet reads your shortcut configuration without editing it.

Workspaces collapse to `Super + 1–9` only when all nine bindings follow that pattern.
Customized or missing bindings expand into individual workspace rows. Long custom
lists scroll; the panel icon always toggles the popup.

## Build and install

Requires a COSMIC Wayland session, Rust **1.93+**, Cargo, Git, Python 3 (installer),
pkg-config, a C/C++ toolchain, and the development libraries for Wayland,
xkbcommon, fontconfig, FreeType, libinput, udev, and OpenGL/EGL.
On Arch/CachyOS the library packages include `wayland`, `libxkbcommon`, `fontconfig`,
`freetype2`, `libinput`, `systemd-libs`, and `libglvnd`; use `base-devel` for build tools.

```sh
python3 scripts/install.py
```

Then open **COSMIC Settings → Desktop → Panel → Configure panel applets**, choose
**Add applet**, and select **Shortcut Cheatsheet**. Reopen the settings page if it
was already open during installation. The applet can also be added to the dock.

The installer places the executable in `~/.local/bin`, and the desktop entry/icon
under `${XDG_DATA_HOME:-~/.local/share}`. It embeds an absolute executable path in
the desktop entry, so `~/.local/bin` need not be on the panel's PATH.

```sh
# Install an already compiled release build
python3 scripts/install.py --no-build

# Remove installed files (also remove the applet in Panel settings)
python3 scripts/install.py --uninstall
```

`--prefix PATH` supports another installation prefix. Git dependencies and
`Cargo.lock` are pinned to a dependency set compatible with COSMIC 1.11.

## Preview and diagnostics

```sh
cargo run --locked -- --preview
cargo run --locked -- --print-shortcuts
cargo test --locked --all-targets
cargo clippy --locked --all-targets -- -D warnings
cargo fmt --all -- --check
```

The preview shows the same themed content in a regular window; press Escape to close
it. Click-toggle behavior is exercised by the panel applet. `--print-shortcuts` reads
the real configuration without opening a window. `RUST_LOG=debug` enables detailed logging.

## Live settings

The applet uses COSMIC's `cosmic-settings-config` action/binding types and
`cosmic-config` lookup rules for `com.system76.CosmicSettings.Shortcuts/v1`:

1. Read `defaults` through the normal user/system XDG lookup.
2. Merge `custom` by key binding, respecting `Disable` and reassigned bindings.
3. Resolve app actions (`Terminal`, `WebBrowser`, `HomeFolder`) independently of
   which preferred application they launch. Custom `Spawn` shortcuts also count
   when their command exactly matches the configured system action command.

Filesystem notifications refresh the sheet when user settings change, including
while open. Opening uses the already-loaded sheet, and an idle applet
has no polling timer. If a settings file is temporarily malformed, the last
successfully read bindings remain visible with a refresh-error notice.

System defaults usually live in `/usr/share/cosmic/`; user overrides live under
`${XDG_CONFIG_HOME:-~/.config}/cosmic/`. Following a package upgrade that changes
system defaults, restart the applet to reload them.

## Implementation

- `src/app.rs`: native panel icon, non-grabbing Wayland popup, themed view.
- `src/popup.rs`: click-toggle lifecycle with stale-surface event protection.
- `src/shortcuts.rs`: configuration loading, live updates, compact binding display.
- `data/`: panel discovery desktop entry and embedded symbolic reference-card icon.
- `scripts/install.py`: user-local install/uninstall.

Licensed under **MPL-2.0**. Dependencies retain their own licenses.
