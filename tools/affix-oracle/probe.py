"""Original black-box Hunspell probes; no reference implementation source is read."""
import json, os, subprocess
from pathlib import Path
root = Path(os.environ["RUNNER_TEMP"]) / "wordcraft-affix-oracle"
root.mkdir(exist_ok=True)
observations = []
for zero in (False, True):
    for pcross in ("Y", "N"):
        for scross in ("Y", "N"):
            for special in ("", "NEEDAFFIX N", "NOSUGGEST N", "FORBIDDENWORD N"):
                name = f"{'zero' if zero else 'pre'}-{pcross}-{scross}-{special.split(' ')[0] or 'plain'}"
                directory = root / name
                directory.mkdir(exist_ok=True)
                prefix = "0" if zero else "pre"
                cont = "S" + ("N" if special else "")
                aff = f"SET UTF-8\n{special}\nPFX P {pcross} 1\nPFX P 0 {prefix}/{cont} .\nSFX S {scross} 1\nSFX S 0 s .\n"
                dic = "1\nroot/P\n"
                base = directory / "probe"
                base.with_suffix(".aff").write_text(aff, encoding="utf-8")
                base.with_suffix(".dic").write_text(dic, encoding="utf-8")
                words = ["root", "roots", "preroot", "preroots", "prepreroots"]
                result = subprocess.run(["hunspell", "-l", "-d", str(base)], input="\n".join(words)+"\n", text=True, capture_output=True, check=True)
                rejected = set(result.stdout.splitlines())
                expected_derived = pcross == "Y" and scross == "Y"
                derived = "roots" if zero else "preroots"
                if (derived not in rejected) != expected_derived:
                    raise RuntimeError(f"cross/continuation mismatch: {name}: {result.stdout!r}")
                if "prepreroots" not in rejected:
                    raise RuntimeError(f"unexpected stacked prefix: {name}")
                observations.append({"fixture":name,"aff":aff,"dic":dic,"accepted":{w:w not in rejected for w in words},"stderr":result.stderr})
version = subprocess.run(["hunspell", "-v"], text=True, capture_output=True, check=True).stdout
output = Path("affix-oracle-observations.json")
output.write_text(json.dumps({"hunspell_version":version,"method":"CLI only, original fixtures, no implementation source read","observations":observations}, indent=2),encoding="utf-8")
print(output.read_text(encoding="utf-8"))
