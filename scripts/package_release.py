"""Package CI-tested binaries using only Python's standard library, in GitHub."""

import argparse
import hashlib
from pathlib import Path
import re
import stat
import tomllib
import zipfile


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--input", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    root = Path(__file__).resolve().parent.parent
    version = tomllib.loads((root / "Cargo.toml").read_text(encoding="utf-8"))[
        "workspace"
    ]["package"]["version"]
    if not re.fullmatch(r"\d+\.\d+\.\d+", version):
        raise ValueError("release_version_invalid")
    platforms = [
        ("ubuntu-24.04-X64", "linux-x64", "skill-hub"),
        ("windows-2022-X64", "windows-x64", "skill-hub.exe"),
        ("macos-15-ARM64", "macos-arm64", "skill-hub"),
        ("macos-15-intel-X64", "macos-x64", "skill-hub"),
    ]
    args.output.mkdir(parents=True, exist_ok=True)
    checksums = []
    for artifact, platform, executable in platforms:
        binary = args.input / f"skill-hub-{artifact}" / executable
        if binary.is_symlink() or not binary.is_file() or binary.stat().st_size == 0:
            raise ValueError("native_binary_missing")
        entries = {
            executable: binary.read_bytes(),
            "CLI.md": (root / "docs/CLI.md").read_bytes(),
            "SAFETY.md": (root / "docs/SAFETY.md").read_bytes(),
            "README.md": (
                f"Skill Hub v{version} CLI ({platform})\n\n"
                "解压后运行 skill-hub --help（Windows 使用 skill-hub.exe）。\n"
                "Linux/macOS 如需执行权限，可执行 chmod +x ./skill-hub。\n"
                "首次建议使用 --home 指向临时目录；详见 CLI.md 与 SAFETY.md。\n"
                "每次预览后手动应用，不自动同步；桌面安装包与手工验收清单见同版本发行制品。\n"
            ).encode("utf-8"),
        }
        archive = args.output / f"skill-hub-v{version}-{platform}.zip"
        # Refuse to overwrite an earlier candidate in this runner's output.
        with zipfile.ZipFile(archive, "x", compression=zipfile.ZIP_DEFLATED) as z:
            for name, data in entries.items():
                info = zipfile.ZipInfo(name, date_time=(2026, 1, 1, 0, 0, 0))
                info.create_system = 3
                executable_mode = name == executable and not name.endswith(".exe")
                mode = 0o755 if executable_mode else 0o644
                info.external_attr = (stat.S_IFREG | mode) << 16
                info.compress_type = zipfile.ZIP_DEFLATED
                z.writestr(info, data)
        with zipfile.ZipFile(archive) as z:
            if z.testzip() is not None or set(z.namelist()) != set(entries):
                raise ValueError("release_archive_invalid")
            for name, data in entries.items():
                if z.read(name) != data:
                    raise ValueError("release_bytes_changed")
            if platform != "windows-x64":
                if not (z.getinfo(executable).external_attr >> 16) & stat.S_IXUSR:
                    raise ValueError("release_execute_bit_missing")
        digest = hashlib.sha256(archive.read_bytes()).hexdigest()
        checksums.append(f"{digest}  {archive.name}\n")
        print(f"Verified {archive.name}")
    with (args.output / "SHA256SUMS.txt").open("x", encoding="utf-8", newline="\n") as f:
        f.writelines(checksums)


if __name__ == "__main__":
    main()
