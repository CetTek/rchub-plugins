#!/usr/bin/env python3
"""Build a Windows x86-64 clipboard plugin package."""
import json
import platform
import subprocess
import sys
import zipfile
from pathlib import Path

HERE = Path(__file__).resolve().parent


def main():
    if platform.system() != "Windows" or platform.machine().lower() not in ("amd64", "x86_64"):
        raise SystemExit("Only Windows x86-64 hosts are supported.")
    info = subprocess.check_output(["rustc", "-vV"], text=True)
    host = next((line[6:] for line in info.splitlines() if line.startswith("host: ")), None)
    if not host or not host.startswith("x86_64-") or "windows" not in host:
        raise SystemExit("The Rust host must target Windows x86-64.")
    target = HERE / "target"
    subprocess.run(["cargo", "build", "--release", "--target", host, "--target-dir", str(target), "--manifest-path", str(HERE / "Cargo.toml")], check=True)
    entry = "bin/rchub-clipboard-plugin.exe"
    manifest = {"id": "rchub.clipboard", "name": "Clipboard", "localizedNames": {"tr": "Pano", "en": "Clipboard"}, "version": "0.2.3", "icon": "ui/icon.svg", "protocol": 2, "entrypoints": {"windows-x86_64": entry}, "ui": {"desktop": "ui/desktop/index.html", "android": "ui/android/index.html"}}
    destination = HERE / "clipboard-plugin-windows-x86_64.zip"
    with zipfile.ZipFile(destination, "w", compression=zipfile.ZIP_DEFLATED) as archive:
        archive.writestr("manifest.json", json.dumps(manifest, ensure_ascii=False))
        archive.write(HERE / "ui" / "icon.svg", "ui/icon.svg")
        archive.write(target / host / "release" / "rchub-clipboard-plugin.exe", entry)
        for surface in ("desktop", "android"):
            archive.write(HERE / "ui" / "index.html", f"ui/{surface}/index.html")
    print(destination)


if __name__ == "__main__":
    try:
        main()
    except (OSError, subprocess.CalledProcessError, zipfile.BadZipFile) as error:
        print(f"Packaging failed: {error}", file=sys.stderr)
        sys.exit(1)
