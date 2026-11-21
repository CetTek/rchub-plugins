#!/usr/bin/env python3
"""Build a Windows x86-64 native power plugin package."""
import json
import platform
import subprocess
import sys
import zipfile
from pathlib import Path

HERE = Path(__file__).resolve().parent


def main():
    if platform.system() != "Windows" or platform.machine().lower() not in ("x86_64", "amd64"):
        raise SystemExit("This plugin supports Windows x86-64 only.")
    target_info = subprocess.check_output(["rustc", "-vV"], text=True)
    host = next((line[6:] for line in target_info.splitlines() if line.startswith("host: ")), None)
    if not host or not host.startswith("x86_64-") or "windows" not in host:
        raise SystemExit("The Rust host target must be Windows x86-64.")
    target_dir = HERE / "target"
    subprocess.run(["cargo", "build", "--release", "--target", host, "--target-dir", str(target_dir), "--manifest-path", str(HERE / "Cargo.toml")], check=True)
    executable = "rchub-power-plugin.exe"
    entrypoint = "bin/" + executable
    manifest = {"id": "rchub.power", "name": "Power Options", "localizedNames": {"tr": "Güç Seçenekleri", "en": "Power Options"}, "version": "0.2.1", "protocol": 2, "entrypoints": {"windows-x86_64": entrypoint}, "ui": {"desktop": "ui/desktop/index.html", "android": "ui/android/index.html"}}
    destination = HERE / "power-plugin-windows-x86_64.zip"
    with zipfile.ZipFile(destination, "w", compression=zipfile.ZIP_DEFLATED) as archive:
        archive.writestr("manifest.json", json.dumps(manifest, ensure_ascii=False))
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
