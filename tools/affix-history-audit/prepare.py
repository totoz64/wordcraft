"""Fetch only WordCraft's own pinned public source, never Hunspell implementation."""
import hashlib
import json
import pathlib
import urllib.request

ROOT = pathlib.Path(__file__).parent
SOURCES = {
    "historical": ("7e9bc805c8db7707d1015d6f9680063877295af7", "b16cacaba92b898690a1276a40be50e4b4a1deaf"),
    "current": ("c394110777924eca9f900941dba4cc040f526573", "af49dbb167cde84b78a432f4fd8e4e54ee9f13d8"),
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
