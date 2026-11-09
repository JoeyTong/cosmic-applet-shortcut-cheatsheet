#!/usr/bin/env python3
"""Build/install the applet for the current user, or uninstall its files."""

import argparse
import os
from pathlib import Path
import shutil
import subprocess
import sys


APP_ID = "io.github.joeytong.cosmic-applet-shortcut-cheatsheet"
PREVIOUS_APP_ID = "io.github.cosmic_utils.ShortcutCheatsheet"
BINARY = "cosmic-applet-shortcut-cheatsheet"
ICON = "shortcut-cheatsheet-symbolic.svg"
ROOT = Path(__file__).resolve().parent.parent


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--no-build", action="store_true", help="install an existing release build")
    parser.add_argument("--uninstall", action="store_true", help="remove installed applet files")
    parser.add_argument("--prefix", type=Path, help="install prefix (default: ~/.local)")
    args = parser.parse_args()

    prefix = (args.prefix or Path.home() / ".local").expanduser().resolve()
    data = (
        prefix / "share"
        if args.prefix
        else Path(os.environ.get("XDG_DATA_HOME", prefix / "share")).expanduser().resolve()
    )
    executable = prefix / "bin" / BINARY
    desktop = data / "applications" / f"{APP_ID}.desktop"
    previous_desktop = data / "applications" / f"{PREVIOUS_APP_ID}.desktop"
    icon = data / "icons" / "hicolor" / "scalable" / "apps" / ICON

    if args.uninstall:
        for path in [executable, desktop, previous_desktop, icon]:
            path.unlink(missing_ok=True)
        print("Uninstalled Shortcut Cheatsheet. Remove its entry in Panel settings if present.")
    else:
        if not args.no_build:
            subprocess.run(["cargo", "build", "--release", "--locked"], cwd=ROOT, check=True)
        target = Path(os.environ.get("CARGO_TARGET_DIR", ROOT / "target"))
        if not target.is_absolute():
            target = ROOT / target
        built = target / "release" / BINARY
        if not built.is_file():
            raise FileNotFoundError(f"No release binary at {built}; run without --no-build first")

        for path in [executable, desktop, icon]:
            path.parent.mkdir(parents=True, exist_ok=True)

        # Atomic replacement also supports upgrading a currently running applet.
        staging = executable.with_name(executable.name + ".new")
        shutil.copyfile(built, staging)
        staging.chmod(0o755)
        staging.replace(executable)

        # Desktop Exec values have their own escaping rules, independent of sh.
        command = str(executable).replace("\\", "\\\\\\\\")
        for character in ['"', "`", "$"]:
            command = command.replace(character, "\\\\" + character)
        command = command.replace("%", "%%")
        entry = (ROOT / "data" / f"{APP_ID}.desktop").read_text()
        entry = entry.replace(f"Exec={BINARY}", f'Exec="{command}"')
        desktop.write_text(entry)
        desktop.chmod(0o644)
        shutil.copyfile(ROOT / "data" / ICON, icon)
        icon.chmod(0o644)
        previous_desktop.unlink(missing_ok=True)

        if shutil.which("desktop-file-validate"):
            subprocess.run(["desktop-file-validate", str(desktop)], check=True)
        print(f"Installed: {executable}")
        print("Add ‘Shortcut Cheatsheet’ in COSMIC Settings → Desktop → Panel → Configure panel applets.")

    if shutil.which("update-desktop-database") and desktop.parent.exists():
        subprocess.run(["update-desktop-database", str(desktop.parent)], check=True)


if __name__ == "__main__":
    try:
        main()
    except (OSError, subprocess.CalledProcessError) as error:
        print(f"Install failed: {error}", file=sys.stderr)
        sys.exit(1)
