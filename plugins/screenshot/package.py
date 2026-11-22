#!/usr/bin/env python3
"""Build and package the screenshot plugin for the current host."""
import json
import platform
import subprocess
import sys
import zipfile
from pathlib import Path

HERE = Path(__file__).resolve().parent


def main():
    system = platform.system()
    machine = platform.machine().lower()
    if system not in ("Windows", "Linux") or machine not in ("x86_64", "amd64"):
        raise SystemExit("Only Windows and Linux x86-64 hosts are supported.")
    target_info = subprocess.check_output(["rustc", "-vV"], text=True)
    host = next((line[6:] for line in target_info.splitlines() if line.startswith("host: ")), None)
    if not host or not host.startswith("x86_64-") or (system == "Windows" and "windows" not in host) or (system == "Linux" and "linux" not in host):
        raise SystemExit("The Rust host target must match the current x86-64 operating system.")
    target_dir = HERE / "target"
    subprocess.run(["cargo", "build", "--release", "--target", host, "--target-dir", str(target_dir), "--manifest-path", str(HERE / "Cargo.toml")], check=True)
    executable = "rchub-screenshot-plugin.exe" if system == "Windows" else "rchub-screenshot-plugin"
    key = "windows-x86_64" if system == "Windows" else "linux-x86_64"
    entrypoint = "bin/" + executable
    manifest = {"id": "rchub.screenshot", "name": "Screenshot", "localizedNames": {"tr": "Ekran Görüntüsü", "en": "Screenshot"}, "version": "0.1.4", "icon": "ui/icon.svg", "protocol": 2, "entrypoints": {key: entrypoint}, "ui": {"desktop": "ui/desktop/index.html", "android": "ui/android/index.html"}}
    destination = HERE / ("screenshot-plugin-" + key + ".zip")
    with zipfile.ZipFile(destination, "w", compression=zipfile.ZIP_DEFLATED) as archive:
        archive.writestr("manifest.json", json.dumps(manifest, ensure_ascii=False))
        archive.write(HERE / "ui" / "icon.svg", "ui/icon.svg")
        archive.write(target_dir / host / "release" / executable, entrypoint)
        for surface in ("desktop", "android"):
            archive.write(HERE / "ui" / "index.html", f"ui/{surface}/index.html")
    print(destination)


if __name__ == "__main__":
    try:
        main()
    except (OSError, subprocess.CalledProcessError, zipfile.BadZipFile) as error:
        print(f"Packaging failed: {error}", file=sys.stderr)
        sys.exit(1)
