"""Render the original French acceptance case with the existing ui_shot API.
This is headless UI evidence, never a native Windows GUI/click claim.
"""
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import urllib.request

root = Path(__file__).resolve().parents[2]
out = root / "dist" / "french-preview-visuals"
out.mkdir(parents=True, exist_ok=False)
sha = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=root, text=True).strip()
if sha != os.environ["GITHUB_SHA"]:
    raise SystemExit("Visual checkout SHA mismatch")
revision = "67032ee957631908098a72edf48dc81cac27e369"
base = f"https://raw.githubusercontent.com/LibreOffice/dictionaries/{revision}/fr_FR/dictionaries"
dictionary = Path(os.environ["RUNNER_TEMP"]) / f"french-preview-dictionary-{sha[:8]}"
dictionary.mkdir(parents=True, exist_ok=False)
hashes = {
    "fr.aff": "c176610cd5dc4846806a65ddd029f422d87978bf58f224aa44222662a16a2de5",
    "fr.dic": "b78a868e31dd6e373b6c3217969afb898a9acde828a5e7ef97308da42218c88c",
}
for name, expected_hash in hashes.items():
    data = urllib.request.urlopen(f"{base}/{name}", timeout=60).read()
    if hashlib.sha256(data).hexdigest() != expected_hash:
        raise SystemExit(f"Dictionary hash mismatch: {name}")
    (dictionary / name).write_bytes(data)
env = dict(os.environ, WORDCRAFT_DICTIONARIES=str(dictionary),
           WORDCRAFT_PROFILE_DIR=str(out / "headless-profile"), WORDCRAFT_LANGUAGE="fr",
           WORDCRAFT_REGION="fr-FR", UI_SHOT_FULL="1")
spec = json.loads((root / "packaging/french-preview/acceptance.json").read_text(encoding="utf-8"))
case = next(c for c in spec["cases"] if c["id"] == "dictionary")
def command(name, params=None):
    return {"method": "engine.execute", "params": {"command": name, "params": params or {}}}
lines = [
    {"empty": True, "width": 1440, "height": 900},
    command("ui.language", {"value": "fr"}),
    command("text.insert", {"text": case["text"]}),
    command("select.all"),
    command("review.language", {"lang": "fr-FR", "noProof": False}),
    command("review.proofing", {"value": True}),
    command("caret.docStart"),
    {"method": "ui.key", "params": {"key": "F7"}},
    {"steps": 12},
]
for theme in ["light", "dark"]:
    lines += [
        command("ui.theme", {"value": theme}), {"steps": 8},
        {"method": "ui.inspect"},
        {"shot": str(out / f"french-proofing-{theme}.png")},
    ]
script = out / "original-french-ui-script.jsonl"
script.write_text("".join(json.dumps(line, ensure_ascii=False) + "\n" for line in lines), encoding="utf-8")
proc = subprocess.run(
    ["cargo", "run", "--locked", "-p", "wordcraft-ui-egui", "--example", "ui_shot", "--", str(script)],
    cwd=root, env=env, text=True, encoding="utf-8", capture_output=True
)
(out / "ui-shot.stdout.txt").write_text(proc.stdout, encoding="utf-8")
(out / "ui-shot.stderr.txt").write_text(proc.stderr, encoding="utf-8")
if proc.returncode:
    raise SystemExit(f"ui_shot exited {proc.returncode}; see captured stderr")
inspections = []
for line in proc.stdout.splitlines():
    if ": " not in line:
        continue
    method, payload = line.split(": ", 1)
    if method not in {"engine.execute", "ui.key", "ui.inspect"}:
        continue
    reply = json.loads(payload)
    if reply.get("ok") is not True:
        raise SystemExit(f"Headless command failed: {line}")
    if method == "ui.inspect":
        inspections.append(reply["result"])
if len(inspections) != 2:
    raise SystemExit("Missing light/dark UI inspection replies")
for state in inspections:
    proofing = state.get("proofing")
    if not proofing or proofing.get("issue", {}).get("text") != case["expect"]["issueText"]:
        raise SystemExit("F7 did not open the expected French spelling pane")
    if case["expect"]["suggestion"] not in proofing["issue"].get("suggestions", []):
        raise SystemExit("French spelling pane did not offer the expected suggestion")
    if proofing["issue"].get("lang") != "fr":
        raise SystemExit("French spelling pane issue language family mismatch")
images = {}
for theme in ["light", "dark"]:
    image = out / f"french-proofing-{theme}.png"
    data = image.read_bytes()
    if not data.startswith(b"\x89PNG\r\n\x1a\n") or len(data) < 1000:
        raise SystemExit(f"Invalid/missing {theme} PNG")
    images[image.name] = hashlib.sha256(data).hexdigest()
legacy = subprocess.run(
    ["cargo", "test", "--locked", "-p", "wordcraft-ui-egui", "--lib",
     "spelling_review_dialog_preview", "--", "--ignored", "--nocapture"],
    cwd=root, env=env, text=True, encoding="utf-8", capture_output=True
)
(out / "existing-preview.stdout.txt").write_text(legacy.stdout, encoding="utf-8")
(out / "existing-preview.stderr.txt").write_text(legacy.stderr, encoding="utf-8")
if legacy.returncode:
    raise SystemExit("Existing light/dark preview test failed")
for theme in ["light", "dark"]:
    src = root / "crates" / "ui-egui" / f"spelling-review-{theme}.png"
    if not src.is_file():
        raise SystemExit(f"Existing preview did not produce {src}")
    shutil.copy2(src, out / f"existing-english-{theme}.png")
report = {
    "protocol": "wordcraft-french-headless-visual-v1", "source_sha": sha,
    "status": "headless_f7_and_french_suggestions_verified_visual_inspection_pending",
    "acceptance_case": case, "requested_document_language": case["lang"],
    "observed_issue_language_families": [s["proofing"]["issue"]["lang"] for s in inspections],
    "dictionary_revision": revision, "dictionary_sha256": hashes,
    "images_sha256": images, "inspections": inspections,
    "native_windows_gui_run": False, "native_mouse_click_tested": False,
    "limitations": ["Existing ui_shot offscreen harness, not a Windows desktop launch.",
                    "F7 passed through synthetic egui input; image composition still requires inspection.",
                    "Existing English light/dark fixture retained separately."]
}
(out / "visual-acceptance.json").write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
print(f"Saved exact-SHA French headless proofing images: {sha}")
