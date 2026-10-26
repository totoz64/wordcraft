"""Fetch only WordCraft's own pinned public source, never Hunspell implementation."""
import hashlib
import json
import pathlib
import urllib.request

ROOT = pathlib.Path(__file__).parent
SOURCES = {
    "historical": ("75c58ac46bb281d43281b0bf675442848d10b221", "cc5a88615c0d5cf4157b2f231b7c03ecb386bb4d"),
    "current": ("a1a4e17a38df438090f3c0ebb18ec65ce91c68a7", "46bf02d996b969bd10d4b36817c57a19caffc223"),
}
provenance = {}
for name, (commit, blob) in SOURCES.items():
    url = f"https://raw.githubusercontent.com/totoz64/wordcraft/{commit}/crates/proof/src/affix.rs"
    data = urllib.request.urlopen(url, timeout=60).read()
    actual = hashlib.sha1(b"blob " + str(len(data)).encode() + b"\0" + data).hexdigest()
    if actual != blob:
        raise RuntimeError(f"Git blob mismatch for {name}: {actual}")
    text = data.decode("utf-8")
    marker = "#[cfg(test)]\npub(crate) mod tests {"
    if text.count(marker) != 1:
        raise RuntimeError("Expected exactly one trailing test module")
    # Preserve the complete production module byte-for-byte. Its trailing test module
    # references the full parent crate; our independent harness supplies its own tests.
    production = text.split(marker)[0]
    (ROOT / f"{name}.rs").write_text(production, encoding="utf-8")
    provenance[name] = {"commit": commit, "blob": blob, "source_sha256": hashlib.sha256(data).hexdigest(), "production_sha256": hashlib.sha256(production.encode()).hexdigest(), "url": url}
(ROOT / "source-provenance.json").write_text(json.dumps(provenance, indent=2) + "\n")
print(json.dumps(provenance, indent=2))
