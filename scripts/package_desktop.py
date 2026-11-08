"""Stage and finalize 3-platform desktop installers, portable bundles, CLI archives, and baselines."""

from __future__ import annotations

import argparse
import hashlib
import io
import json
import os
from pathlib import Path
import platform
import re
import stat
import subprocess
import tarfile
import zipfile


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        while True:
            chunk = handle.read(1024 * 1024)
            if not chunk:
                break
            digest.update(chunk)
    return digest.hexdigest()


def verified_version(root: Path) -> str:
    tauri_conf = json.loads(
        (root / "apps/web/src-tauri/tauri.conf.json").read_text(encoding="utf-8")
    )
    web_pkg = json.loads((root / "apps/web/package.json").read_text(encoding="utf-8"))
    cargo_root = (root / "Cargo.toml").read_text(encoding="utf-8")
    cargo_desktop = (root / "apps/web/src-tauri/Cargo.toml").read_text(encoding="utf-8")
    m_root = re.search(r'^\[workspace\.package\][\s\S]*?^version\s*=\s*"([^"]+)"', cargo_root, re.M)
    m_desk = re.search(r'^\[package\][\s\S]*?^version\s*=\s*"([^"]+)"', cargo_desktop, re.M)
    version = tauri_conf.get("version", "")
    if not re.fullmatch(r"\d+\.\d+\.\d+", version):
        raise ValueError("release_version_invalid")
    if (
        web_pkg.get("version") != version
        or not m_root
        or m_root.group(1) != version
        or not m_desk
        or m_desk.group(1) != version
    ):
        raise ValueError("version_mismatch_across_manifests")
    return version


def copy_exclusive(src: Path, dst: Path) -> None:
    if src.is_symlink() or not src.is_file() or src.stat().st_size == 0:
        raise ValueError(f"invalid_artifact_source:{src}")
    with src.open("rb") as reader, dst.open("xb") as writer:
        while True:
            chunk = reader.read(1024 * 1024)
            if not chunk:
                break
            writer.write(chunk)


def single_match(directory: Path, pattern: str) -> Path:
    matches = sorted(p for p in directory.glob(pattern) if not p.is_symlink())
    if len(matches) != 1:
        raise ValueError(f"expected_single_match:{directory}/{pattern}:found_{len(matches)}")
    return matches[0]


def write_zip_exclusive(archive: Path, entries: dict[str, tuple[bytes, int]]) -> None:
    with zipfile.ZipFile(archive, "x", compression=zipfile.ZIP_DEFLATED) as zf:
        for name, (data, mode) in entries.items():
            info = zipfile.ZipInfo(name, date_time=(2026, 1, 1, 0, 0, 0))
            info.create_system = 3
            info.external_attr = (stat.S_IFREG | mode) << 16
            info.compress_type = zipfile.ZIP_DEFLATED
            zf.writestr(info, data)
    with zipfile.ZipFile(archive, "r") as zf:
        if zf.testzip() is not None or set(zf.namelist()) != set(entries):
            raise ValueError(f"zip_verification_failed:{archive.name}")
        for name, (data, _) in entries.items():
            if zf.read(name) != data:
                raise ValueError(f"zip_bytes_mismatch:{archive.name}:{name}")


def write_tar_gz_files_exclusive(archive: Path, entries: dict[str, tuple[bytes, int]]) -> None:
    with archive.open("xb") as raw_out:
        with tarfile.open(fileobj=raw_out, mode="w:gz", format=tarfile.PAX_FORMAT) as tf:
            for name, (data, mode) in entries.items():
                info = tarfile.TarInfo(name=name)
                info.size = len(data)
                info.mtime = 1767225600
                info.mode = mode
                info.type = tarfile.REGTYPE
                tf.addfile(info, io.BytesIO(data))
    with tarfile.open(archive, mode="r:gz") as tf:
        members = {m.name: m for m in tf.getmembers()}
        if set(members) != set(entries):
            raise ValueError(f"tar_verification_failed:{archive.name}")
        for name, (data, mode) in entries.items():
            extracted = tf.extractfile(members[name])
            if extracted is None or extracted.read() != data:
                raise ValueError(f"tar_bytes_mismatch:{archive.name}:{name}")
            if (mode & 0o111) and not (members[name].mode & 0o111):
                raise ValueError(f"tar_exec_bit_missing:{archive.name}:{name}")


def archive_macos_app_bundle(app_dir: Path, archive: Path) -> None:
    if not (app_dir / "Contents/Info.plist").is_file():
        raise ValueError("macos_app_info_plist_missing")
    macos_bins = [
        p for p in (app_dir / "Contents/MacOS").iterdir() if p.is_file() and p.stat().st_size > 0
    ]
    if not macos_bins:
        raise ValueError("macos_app_executable_missing")
    with archive.open("xb") as raw_out:
        with tarfile.open(
            fileobj=raw_out, mode="w:gz", format=tarfile.PAX_FORMAT, dereference=False
        ) as tf:
            tf.add(app_dir, arcname=app_dir.name, recursive=True)
    with tarfile.open(archive, mode="r:gz") as tf:
        names = tf.getnames()
        if f"{app_dir.name}/Contents/Info.plist" not in names:
            raise ValueError("macos_app_archive_incomplete")


def run_capture(cmd: list[str]) -> str:
    try:
        res = subprocess.run(
            cmd,
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            timeout=15,
            check=False,
        )
        return (res.stdout or res.stderr).strip()
    except Exception as exc:  # noqa: BLE001
        return f"unavailable ({exc})"


def collect_host_baseline(platform_id: str, desktop_bin: Path, deb_path: Path | None) -> dict[str, object]:
    info: dict[str, object] = {
        "platform": platform_id,
        "system": platform.system(),
        "release": platform.release(),
        "version": platform.version(),
        "machine": platform.machine(),
        "rustc": run_capture(["rustc", "--version"]),
        "node": run_capture(["node", "--version"]),
    }
    if platform_id == "linux-x64":
        info["osRelease"] = (
            Path("/etc/os-release").read_text(encoding="utf-8", errors="replace")
            if Path("/etc/os-release").is_file()
            else ""
        )
        info["lddVersion"] = run_capture(["ldd", "--version"]).splitlines()[0]
        if deb_path is not None:
            info["debDepends"] = run_capture(["dpkg-deb", "-f", str(deb_path), "Depends"])
    elif platform_id.startswith("macos-"):
        info["swVers"] = run_capture(["sw_vers"])
        info["otoolLibraries"] = run_capture(["otool", "-L", str(desktop_bin)])
    elif platform_id == "windows-x64":
        info["windowsProduct"] = run_capture(
            [
                "powershell",
                "-NoProfile",
                "-Command",
                "(Get-CimInstance Win32_OperatingSystem | Select-Object Caption,Version,BuildNumber | ConvertTo-Json -Compress)",
            ]
        )
    return info


def stage(args: argparse.Namespace) -> None:
    root = Path(__file__).resolve().parent.parent
    version = verified_version(root)
    out: Path = args.output
    out.mkdir(parents=True, exist_ok=True)
    bundle_dir: Path = args.bundle_dir
    desktop_bin: Path = args.desktop_bin
    cli_bin: Path = args.cli_bin

    cli_readme = (
        f"Skill Hub v{version} CLI ({args.platform})\n\n"
        "1. 先使用 --home <隔离绝对目录> 验证 agents list / import / diff / apply / restore。\n"
        "2. 每次写入均需预览后显式确认（--yes），无后台同步。\n"
        "3. 详细说明见 CLI.md、MANUAL-VALIDATION.md 与 SAFETY.md。\n"
    ).encode("utf-8")
    desktop_readme = (
        f"Skill Hub v{version} Desktop ({args.platform})\n\n"
        "1. 支持 --home <隔离绝对目录> 启动独立验收环境。\n"
        "2. 导入、编辑只改本地库；应用到 Agent 与历史恢复均需弹窗预览后手动确认。\n"
        "3. 平台最低系统基线与真实 Agent 加载核对清单见 DESKTOP.md、MANUAL-VALIDATION.md、SAFETY.md。\n"
    ).encode("utf-8")

    common_cli_docs = {
        "CLI.md": ((root / "docs/CLI.md").read_bytes(), 0o644),
        "MANUAL-VALIDATION.md": ((root / "docs/MANUAL-VALIDATION.md").read_bytes(), 0o644),
        "SAFETY.md": ((root / "docs/SAFETY.md").read_bytes(), 0o644),
        "README.md": (cli_readme, 0o644),
    }

    deb_target: Path | None = None
    if args.platform == "windows-x64":
        nsis = single_match(bundle_dir / "nsis", "*-setup.exe")
        msi = single_match(bundle_dir / "msi", "*.msi")
        copy_exclusive(nsis, out / f"skill-hub-desktop-v{version}-windows-x64-setup.exe")
        copy_exclusive(msi, out / f"skill-hub-desktop-v{version}-windows-x64.msi")
        write_zip_exclusive(
            out / f"skill-hub-desktop-v{version}-windows-x64-portable.zip",
            {
                "skill-hub-desktop.exe": (desktop_bin.read_bytes(), 0o644),
                "DESKTOP.md": ((root / "docs/DESKTOP.md").read_bytes(), 0o644),
                "MANUAL-VALIDATION.md": ((root / "docs/MANUAL-VALIDATION.md").read_bytes(), 0o644),
                "SAFETY.md": ((root / "docs/SAFETY.md").read_bytes(), 0o644),
                "README.md": (desktop_readme, 0o644),
            },
        )
        write_zip_exclusive(
            out / f"skill-hub-cli-v{version}-windows-x64.zip",
            {
                "skill-hub.exe": (cli_bin.read_bytes(), 0o644),
                **common_cli_docs,
            },
        )
    elif args.platform in ("macos-arm64", "macos-x64"):
        dmg = single_match(bundle_dir / "dmg", "*.dmg")
        app_dir = single_match(bundle_dir / "macos", "*.app")
        copy_exclusive(dmg, out / f"skill-hub-desktop-v{version}-{args.platform}.dmg")
        archive_macos_app_bundle(
            app_dir, out / f"skill-hub-desktop-v{version}-{args.platform}.app.tar.gz"
        )
        write_tar_gz_files_exclusive(
            out / f"skill-hub-cli-v{version}-{args.platform}.tar.gz",
            {
                "skill-hub": (cli_bin.read_bytes(), 0o755),
                **common_cli_docs,
            },
        )
    elif args.platform == "linux-x64":
        appimage = single_match(bundle_dir / "appimage", "*.AppImage")
        deb = single_match(bundle_dir / "deb", "*.deb")
        with appimage.open("rb") as f:
            if f.read(4) != b"\x7fELF":
                raise ValueError("appimage_elf_magic_invalid")
        with deb.open("rb") as f:
            if f.read(8) != b"!<arch>\n":
                raise ValueError("deb_archive_magic_invalid")
        appimage_dst = out / f"skill-hub-desktop-v{version}-linux-x64.AppImage"
        deb_target = out / f"skill-hub-desktop-v{version}-linux-x64.deb"
        copy_exclusive(appimage, appimage_dst)
        if os.name != "nt":
            appimage_dst.chmod(0o755)
        copy_exclusive(deb, deb_target)
        write_tar_gz_files_exclusive(
            out / f"skill-hub-cli-v{version}-linux-x64.tar.gz",
            {
                "skill-hub": (cli_bin.read_bytes(), 0o755),
                **common_cli_docs,
            },
        )
    else:
        raise ValueError(f"unsupported_platform:{args.platform}")

    copy_exclusive(args.smoke_report, out / f"smoke-report-{args.platform}.json")
    baseline = collect_host_baseline(args.platform, desktop_bin, deb_target)
    with (out / f"baseline-{args.platform}.json").open("x", encoding="utf-8", newline="\n") as f:
        json.dump(baseline, f, ensure_ascii=False, indent=2)
        f.write("\n")


def finalize(args: argparse.Namespace) -> None:
    root = Path(__file__).resolve().parent.parent
    version = verified_version(root)
    inp: Path = args.input
    out: Path = args.output
    out.mkdir(parents=True, exist_ok=True)
    evidence_dir = out / "evidence"
    evidence_dir.mkdir(parents=True, exist_ok=True)

    expected = [
        f"skill-hub-desktop-v{version}-windows-x64-setup.exe",
        f"skill-hub-desktop-v{version}-windows-x64.msi",
        f"skill-hub-desktop-v{version}-windows-x64-portable.zip",
        f"skill-hub-cli-v{version}-windows-x64.zip",
        f"skill-hub-desktop-v{version}-macos-arm64.dmg",
        f"skill-hub-desktop-v{version}-macos-arm64.app.tar.gz",
        f"skill-hub-cli-v{version}-macos-arm64.tar.gz",
        f"skill-hub-desktop-v{version}-macos-x64.dmg",
        f"skill-hub-desktop-v{version}-macos-x64.app.tar.gz",
        f"skill-hub-cli-v{version}-macos-x64.tar.gz",
        f"skill-hub-desktop-v{version}-linux-x64.AppImage",
        f"skill-hub-desktop-v{version}-linux-x64.deb",
        f"skill-hub-cli-v{version}-linux-x64.tar.gz",
    ]
    platforms = ["linux-x64", "windows-x64", "macos-arm64", "macos-x64"]

    discovered = {p.name: p for p in inp.rglob("*") if p.is_file()}
    checksums: list[str] = []
    for name in sorted(expected):
        if name not in discovered:
            raise ValueError(f"missing_expected_release_artifact:{name}")
        dst = out / name
        copy_exclusive(discovered[name], dst)
        digest = sha256_file(dst)
        checksums.append(f"{digest}  {name}\n")

    baselines: list[dict[str, object]] = []
    for plat in platforms:
        for prefix in ("smoke-report-", "baseline-"):
            fname = f"{prefix}{plat}.json"
            if fname not in discovered:
                raise ValueError(f"missing_expected_evidence:{fname}")
            copy_exclusive(discovered[fname], evidence_dir / fname)
        baselines.append(
            json.loads((evidence_dir / f"baseline-{plat}.json").read_text(encoding="utf-8"))
        )

    with (out / "SHA256SUMS.txt").open("x", encoding="utf-8", newline="\n") as f:
        f.writelines(checksums)

    summary_lines = [
        f"# Skill Hub v{version} 构建基线与校验摘要\n\n",
        "本文件由 GitHub Actions 自动生成，仅记录本次候选构建的实际 runner 环境与产物摘要；未自动打标签或创建 Release。\n\n",
        "## 平台构建基线\n\n",
    ]
    for b in baselines:
        summary_lines.append(
            f"- **{b['platform']}**: `{b['system']} {b['release']} ({b['machine']})`, `{b['rustc']}`, `{b['node']}`"
        )
        if b.get("lddVersion"):
            summary_lines.append(f", `{b['lddVersion']}`")
        if b.get("debDepends"):
            summary_lines.append(f", `.deb Depends`: `{b['debDepends']}`")
        summary_lines.append("\n")
    summary_lines.append("\n## SHA-256\n\n```text\n")
    summary_lines.extend(checksums)
    summary_lines.append("```\n")

    with (out / "BUILD-BASELINE.md").open("x", encoding="utf-8", newline="\n") as f:
        f.writelines(summary_lines)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="cmd", required=True)

    p_stage = sub.add_parser("stage")
    p_stage.add_argument("--platform", required=True)
    p_stage.add_argument("--bundle-dir", type=Path, required=True)
    p_stage.add_argument("--desktop-bin", type=Path, required=True)
    p_stage.add_argument("--cli-bin", type=Path, required=True)
    p_stage.add_argument("--smoke-report", type=Path, required=True)
    p_stage.add_argument("--output", type=Path, required=True)

    p_fin = sub.add_parser("finalize")
    p_fin.add_argument("--input", type=Path, required=True)
    p_fin.add_argument("--output", type=Path, required=True)

    args = parser.parse_args()
    if args.cmd == "stage":
        stage(args)
    else:
        finalize(args)


if __name__ == "__main__":
    main()
