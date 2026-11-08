"""Run native desktop launch smoke and zero-recompilation fifth-agent checks in GitHub Actions."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import shutil
import signal
import stat
import subprocess
import sys
import tempfile
import time


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        while True:
            chunk = handle.read(1024 * 1024)
            if not chunk:
                break
            digest.update(chunk)
    return digest.hexdigest()


def run_cli(cli_bin: Path, home: Path, args: list[str]) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [str(cli_bin), "--home", str(home), *args],
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
        timeout=30,
        check=False,
    )


def terminate_process_tree(proc: subprocess.Popen[bytes]) -> None:
    if proc.poll() is not None:
        return
    if os.name != "nt":
        try:
            os.killpg(proc.pid, signal.SIGTERM)
        except ProcessLookupError:
            return
    else:
        proc.terminate()
    try:
        proc.wait(timeout=8)
    except subprocess.TimeoutExpired:
        if os.name != "nt":
            try:
                os.killpg(proc.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
        else:
            proc.kill()
        proc.wait(timeout=5)


def launch_desktop_alive(desktop_bin: Path, target_home: Path, seconds: float = 3.0) -> str:
    cmd = [str(desktop_bin), "--home", str(target_home)]
    env = os.environ.copy()
    launch_mode = "native-display"

    if sys.platform.startswith("linux") and not env.get("DISPLAY") and not env.get("WAYLAND_DISPLAY"):
        xvfb = shutil.which("xvfb-run")
        if not xvfb:
            raise RuntimeError("xvfb-run is required on headless Linux runner")
        cmd = [xvfb, "-a", *cmd]
        env["WEBKIT_DISABLE_COMPOSITING_MODE"] = "1"
        env["LIBGL_ALWAYS_SOFTWARE"] = "1"
        launch_mode = "xvfb-run"
    elif os.name == "nt":
        wv2_dir = Path(tempfile.mkdtemp(prefix="skill-hub-wv2-")).resolve()
        env["WEBVIEW2_USER_DATA_FOLDER"] = str(wv2_dir)
        launch_mode = "win32-webview2"

    popen_kwargs: dict[str, object] = {
        "stdout": subprocess.DEVNULL,
        "stderr": subprocess.PIPE,
        "env": env,
    }
    if os.name != "nt":
        popen_kwargs["start_new_session"] = True

    proc = subprocess.Popen(cmd, **popen_kwargs)  # type: ignore[arg-type]
    try:
        deadline = time.monotonic() + seconds
        while time.monotonic() < deadline:
            exit_code = proc.poll()
            if exit_code is not None:
                stderr_bytes = proc.stderr.read() if proc.stderr else b""
                stderr_text = stderr_bytes.decode("utf-8", errors="replace")
                raise RuntimeError(
                    f"desktop exited prematurely with code {exit_code}: {stderr_text}"
                )
            time.sleep(0.25)
    finally:
        terminate_process_tree(proc)
        if proc.stderr:
            proc.stderr.close()

    return launch_mode


def verify_desktop_smoke(desktop_bin: Path) -> dict[str, object]:
    if not desktop_bin.is_file() or desktop_bin.stat().st_size == 0:
        raise RuntimeError(f"missing desktop binary: {desktop_bin}")

    # 1. Relative --home must be rejected cleanly with exit code 1 before starting UI.
    bad = subprocess.run(
        [str(desktop_bin), "--home", "relative-non-absolute-home"],
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
        timeout=20,
        check=False,
    )
    if bad.returncode != 1 or "desktop_environment_unavailable" not in bad.stderr:
        raise RuntimeError(
            f"unexpected relative --home result: code={bad.returncode}, stderr={bad.stderr!r}"
        )

    # 2. Launch native desktop process with isolated absolute --home and verify it stays alive
    #    without crashing and without creating .skill-hub state on startup (A12).
    smoke_home = Path(tempfile.mkdtemp(prefix="skill-hub-desktop-smoke-")).resolve()
    launch_mode = launch_desktop_alive(desktop_bin, smoke_home, seconds=3.0)
    if (smoke_home / ".skill-hub").exists():
        raise RuntimeError("desktop created .skill-hub during read-only startup")

    return {
        "relativeHomeRejectedExit1": True,
        "processAliveSeconds": 3.0,
        "zeroStartupWritesVerified": True,
        "launchMode": launch_mode,
    }


def verify_fifth_agent_zero_recompile(
    desktop_bin: Path, cli_bin: Path, repo_root: Path
) -> dict[str, object]:
    if not cli_bin.is_file() or cli_bin.stat().st_size == 0:
        raise RuntimeError(f"missing cli binary: {cli_bin}")

    home = Path(tempfile.mkdtemp(prefix="skill-hub-fifth-smoke-")).resolve()
    fifth_fixture = repo_root / "tests/fixtures/agents/custom.fifth.json"
    if not fifth_fixture.is_file():
        raise RuntimeError(f"missing fixture: {fifth_fixture}")

    # Negative 1: invalid custom profile rejected.
    bad_profile = home / "invalid.json"
    bad_profile.write_text('{"schemaVersion":1,"id":"../bad"}', encoding="utf-8")
    res_bad = run_cli(cli_bin, home, ["agents", "validate", "--file", str(bad_profile)])
    if res_bad.returncode != 1:
        raise RuntimeError(f"expected exit 1 for invalid profile, got {res_bad.returncode}")

    # Seed valid Claude skill with UTF-8 Chinese, CRLF, binary file, and Unix executable script.
    claude_skill = home / ".claude/skills/code-review"
    (claude_skill / "scripts").mkdir(parents=True, exist_ok=True)
    skill_bytes = (
        b"---\r\nname: code-review\r\n"
        b"description: \xe7\xac\xac\xe4\xba\x94\xe4\xb8\xaa Agent \xe9\x9b\xb6\xe9\x87\x8d\xe7\xbc\x96\xe8\xaf\x91\xe9\xaa\x8c\xe8\xaf\x81\r\n"
        b"---\r\nKeep CRLF.\r\n"
    )
    (claude_skill / "SKILL.md").write_bytes(skill_bytes)
    (claude_skill / "asset.bin").write_bytes(bytes([0x00, 0xFE, 0xFF]))
    script_path = claude_skill / "scripts/check.sh"
    script_path.write_bytes(b"#!/bin/sh\nexit 0\n")
    if os.name != "nt":
        script_path.chmod(0o755)

    imp = run_cli(
        cli_bin,
        home,
        ["import", "--source", str(home / ".claude/skills"), "--new", "--yes"],
    )
    if imp.returncode != 0:
        raise RuntimeError(f"import failed: {imp.stderr}")
    records = json.loads(imp.stdout)
    listed = json.loads(run_cli(cli_bin, home, ["list"]).stdout)
    pkg_id = listed["records"][0]["id"]

    # Negative 2: unapproved custom.fifth.json on disk stays pending and refuses apply.
    agents_dir = home / ".skill-hub/agents"
    agents_dir.mkdir(parents=True, exist_ok=True)
    (agents_dir / "custom.fifth.json").write_bytes(fifth_fixture.read_bytes())
    unapproved = run_cli(cli_bin, home, ["apply", "--pair", f"{pkg_id}=custom.fifth", "--yes"])
    if unapproved.returncode != 1 or (home / ".fifth/skills").exists():
        raise RuntimeError("unapproved fifth agent must be rejected without writing")

    # Approve & enable custom.fifth with boolean config probe enabled.
    (home / ".fifth").mkdir(parents=True, exist_ok=True)
    (home / ".fifth/config.json").write_text(
        '{"compat":{"shared":{"skills":true}}}', encoding="utf-8"
    )
    saved = run_cli(
        cli_bin,
        home,
        ["agents", "save", "--file", str(fifth_fixture), "--enable", "--yes"],
    )
    if saved.returncode != 0:
        raise RuntimeError(f"agents save failed: {saved.stderr}")

    # Shared root visibility dynamically includes custom.fifth on codex (.agents/skills).
    diff_codex = run_cli(cli_bin, home, ["diff", "--pair", f"{pkg_id}=codex"])
    if diff_codex.returncode != 0:
        raise RuntimeError(f"diff codex failed: {diff_codex.stderr}")
    readers = json.loads(diff_codex.stdout)["groups"][0]["possibleReaders"]
    if not any(
        r.get("agent") == "custom.fifth" and r.get("evidence") == "configEnabled"
        for r in readers
    ):
        raise RuntimeError(f"expected custom.fifth in possibleReaders, got {readers!r}")

    # Flip boolean config probe to false and verify dynamic reader evidence updates to configDisabled.
    (home / ".fifth/config.json").write_text(
        '{"compat":{"shared":{"skills":false}}}', encoding="utf-8"
    )
    diff_disabled = run_cli(cli_bin, home, ["diff", "--pair", f"{pkg_id}=codex"])
    if diff_disabled.returncode != 0:
        raise RuntimeError(f"diff codex (probe disabled) failed: {diff_disabled.stderr}")
    disabled_readers = json.loads(diff_disabled.stdout)["groups"][0]["possibleReaders"]
    if not any(
        r.get("agent") == "custom.fifth" and r.get("evidence") == "configDisabled"
        for r in disabled_readers
    ):
        raise RuntimeError(
            f"expected custom.fifth configDisabled after probe flip, got {disabled_readers!r}"
        )
    (home / ".fifth/config.json").write_text(
        '{"compat":{"shared":{"skills":true}}}', encoding="utf-8"
    )

    # Apply to custom.fifth, verify exact bytes & Unix execute bit, launch packaged desktop on fifth home, then restore.
    applied = run_cli(cli_bin, home, ["apply", "--pair", f"{pkg_id}=custom.fifth", "--yes"])
    if applied.returncode != 0:
        raise RuntimeError(f"apply custom.fifth failed: {applied.stderr}")
    applied_json = json.loads(applied.stdout)
    target_skill = home / ".fifth/skills/code-review"
    if (target_skill / "SKILL.md").read_bytes() != skill_bytes:
        raise RuntimeError("CRLF/UTF-8 bytes changed after fifth-agent apply")
    if (target_skill / "asset.bin").read_bytes() != bytes([0x00, 0xFE, 0xFF]):
        raise RuntimeError("binary bytes changed after fifth-agent apply")
    if os.name != "nt":
        mode = (target_skill / "scripts/check.sh").stat().st_mode
        if not (mode & stat.S_IXUSR):
            raise RuntimeError("Unix execute bit lost on fifth-agent apply")

    launch_desktop_alive(desktop_bin, home, seconds=2.5)

    restored = run_cli(cli_bin, home, ["restore", applied_json["operationId"], "--yes"])
    if restored.returncode != 0:
        raise RuntimeError(f"restore custom.fifth failed: {restored.stderr}")
    if target_skill.exists():
        raise RuntimeError("target directory still present in load root after restore")

    return {
        "invalidProfileRejected": True,
        "unapprovedProfileRejected": True,
        "dynamicSharedReaderVerified": True,
        "probeFlipVerified": True,
        "desktopBootWithFifthAgentVerified": True,
        "applyAndRestoreVerified": True,
        "importOperationId": records["operationId"],
        "applyOperationId": applied_json["operationId"],
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--desktop-bin", type=Path, required=True)
    parser.add_argument("--cli-bin", type=Path, required=True)
    parser.add_argument("--platform", type=str, required=True)
    parser.add_argument("--report", type=Path, required=True)
    args = parser.parse_args()

    repo_root = Path(__file__).resolve().parent.parent
    desktop_bin = args.desktop_bin.resolve()
    cli_bin = args.cli_bin.resolve()

    desktop_sha_before = sha256_file(desktop_bin)
    cli_sha_before = sha256_file(cli_bin)

    desktop_result = verify_desktop_smoke(desktop_bin)
    fifth_result = verify_fifth_agent_zero_recompile(desktop_bin, cli_bin, repo_root)

    desktop_sha_after = sha256_file(desktop_bin)
    cli_sha_after = sha256_file(cli_bin)
    if desktop_sha_before != desktop_sha_after or cli_sha_before != cli_sha_after:
        raise RuntimeError("binary SHA-256 changed during smoke/fifth-agent verification")

    report = {
        "platform": args.platform,
        "hostSystem": platform.system(),
        "hostRelease": platform.release(),
        "hostVersion": platform.version(),
        "hostMachine": platform.machine(),
        "pythonVersion": platform.python_version(),
        "desktopSha256": desktop_sha_after,
        "cliSha256": cli_sha_after,
        "binaryUnmodifiedBeforeAfter": True,
        "desktopSmoke": desktop_result,
        "fifthAgentZeroRecompile": fifth_result,
    }
    args.report.parent.mkdir(parents=True, exist_ok=True)
    with args.report.open("x", encoding="utf-8", newline="\n") as handle:
        json.dump(report, handle, ensure_ascii=False, indent=2)
        handle.write("\n")
    print(f"Smoke and zero-recompile fifth-agent verified for {args.platform}")


if __name__ == "__main__":
    main()
