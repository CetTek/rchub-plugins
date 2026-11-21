#!/usr/bin/env python3
"""Build and package the Windows x86-64 media plugin."""
import json
import platform
import subprocess
import sys
import zipfile
from pathlib import Path

HERE = Path(__file__).resolve().parent


def main():
    if platform.system() != "Windows" or platform.machine().lower() not in ("amd64", "x86_64"):
        raise SystemExit("Packaging requires Windows x86-64.")
    info = subprocess.check_output(["rustc", "-vV"], text=True)
    host = next((line[6:] for line in info.splitlines() if line.startswith("host: ")), "")
    if not host.startswith("x86_64-") or "windows" not in host:
        raise SystemExit("The Rust host target must be Windows x86-64.")
    target = HERE / "target"
    subprocess.run(["cargo", "build", "--release", "--target", host, "--target-dir", str(target), "--manifest-path", str(HERE / "Cargo.toml")], check=True)
    exe = "rchub-media-plugin.exe"
    manifest = {"id": "rchub.media", "name": "Media Controls", "localizedNames": {"tr": "Medya Kontrolleri", "en": "Media Controls"}, "version": "0.2.1", "protocol": 2, "entrypoints": {"windows-x86_64": "bin/" + exe}, "ui": {"desktop": "ui/desktop/index.html", "android": "ui/android/index.html"}}
    destination = HERE / "media-plugin-windows-x86_64.zip"
    with zipfile.ZipFile(destination, "w", compression=zipfile.ZIP_DEFLATED) as archive:
        archive.writestr("manifest.json", json.dumps(manifest, ensure_ascii=False))
        archive.write(target / host / "release" / exe, "bin/" + exe)
        for surface in ("desktop", "android"):
            archive.write(HERE / "ui" / "index.html", f"ui/{surface}/index.html")
    print(destination)


if __name__ == "__main__":
    try:
        main()
    except (OSError, subprocess.CalledProcessError, zipfile.BadZipFile) as error:
        print(f"Packaging failed: {error}", file=sys.stderr)
        sys.exit(1)
