# /// script
# requires-python = ">=3.12"
# dependencies = ["zstandard==0.25.0"]
# ///
"""Install the optional, separately licensed local explainshell matcher/data.
Run: uv run scripts/setup-explainshell.py
No recorded commands are read or uploaded. Restart Linger after setup.
"""
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import urllib.request
import zstandard

COMMIT = "827ebe38da9822a10e89946551e377330a9de35e"
ASSET = "explainshell-2026-06-22-063435.db.zst"
DIGEST = "9c6286d533f7f52e07b2420555b9cff93cf4754380c379747166e1f20374ebc6"
URL = f"https://github.com/idank/explainshell/releases/download/db-latest/{ASSET}"

def main():
    data = Path(os.environ.get("XDG_DATA_HOME", str(Path.home() / ".local/share")))
    root = Path(os.environ.get("LINGER_EXPLAINSHELL_DIR", str(data / "linger/explainshell")))
    root.mkdir(parents=True, exist_ok=True)
    source = root / "source"
    if not source.exists():
        subprocess.run(["git", "clone", "--filter=blob:none", "--no-checkout", "https://github.com/idank/explainshell.git", str(source)], check=True)
        subprocess.run(["git", "-C", str(source), "checkout", "--detach", COMMIT], check=True)
    actual = subprocess.check_output(["git", "-C", str(source), "rev-parse", "HEAD"], text=True).strip()
    dirty = subprocess.check_output(["git", "-C", str(source), "status", "--porcelain"], text=True).strip()
    if actual != COMMIT or dirty:
        raise SystemExit("Existing backend differs from the pinned source; choose a fresh LINGER_EXPLAINSHELL_DIR.")
    python = root / ".venv/bin/python"
    if not python.exists():
        subprocess.run(["uv", "venv", str(root / ".venv")], check=True)
    subprocess.run(["uv", "pip", "install", "--python", str(python), "bashlex==0.18", "pydantic==2.12.5"], check=True)
    db = root / "manpages.db"
    receipt = root / "receipt.json"
    if not db.exists():
        archive = root / (ASSET + ".partial")
        print("Downloading the pinned manpage pack (220 MiB)…", flush=True)
        with urllib.request.urlopen(URL, timeout=60) as response, archive.open("wb") as output:
            shutil.copyfileobj(response, output)
        with archive.open("rb") as stream:
            digest = hashlib.file_digest(stream, "sha256").hexdigest()
        if digest != DIGEST:
            raise SystemExit("Manpage archive checksum mismatch; installation stopped.")
        temporary = root / "manpages.db.partial"
        with archive.open("rb") as compressed, temporary.open("wb") as output:
            zstandard.ZstdDecompressor().copy_stream(compressed, output)
        temporary.replace(db)
        archive.unlink()
        receipt.write_text(json.dumps({"source_commit": COMMIT, "database": ASSET, "archive_sha256": DIGEST, "database_sha256": hashlib.file_digest(db.open("rb"), "sha256").hexdigest(), "source_license": "GPL-3.0; see source/LICENSE", "dependencies": {"bashlex": "0.18", "pydantic": "2.12.5"}}, indent=2) + "\n")
    else:
        if not receipt.exists():
            raise SystemExit("Existing database has no installation receipt; choose a fresh LINGER_EXPLAINSHELL_DIR.")
        recorded = json.loads(receipt.read_text())
        if recorded.get("source_commit") != COMMIT or recorded.get("archive_sha256") != DIGEST:
            raise SystemExit("Existing database receipt differs from this setup version; choose a fresh LINGER_EXPLAINSHELL_DIR.")
        with db.open("rb") as stream:
            actual = hashlib.file_digest(stream, "sha256").hexdigest()
        if actual != recorded.get("database_sha256"):
            raise SystemExit("Existing database checksum differs from its receipt; installation retained for inspection.")
    print(f"Local explainshell backend ready: {root}\nRestart Linger, open a call, press 3, then h/l to explore.")

if __name__ == "__main__":
    main()
