"""Controlled same-VM comparison of two checksum-pinned, prebuilt release runners."""
import hashlib, io, json, os, platform, resource, statistics, subprocess, time, zipfile
from pathlib import Path
from urllib.error import HTTPError
from urllib.request import Request, build_opener, HTTPRedirectHandler, urlopen

ROOT = Path("performance-results")
ROOT.mkdir(exist_ok=True)
SOURCE_DIR = Path(__file__).resolve().parent
ENGINES = {
    "before": {
        "source_sha": "694a8e76079453de3fa68321d3ce11f49dcf9f39",
        "run": 40362973260, "artifact_id": 12668188325,
        "zip_sha256": "5e1889d9023b0696497768748e1727d996fe0703f839b040fff7387e70d69cff",
    },
    "after": {
        "source_sha": os.environ["GITHUB_SHA"],
        "run": int(os.environ["GITHUB_RUN_ID"]),
        "local_binary": "tools/proofing-benchmark/target/release/wordcraft-proofing-benchmark",
    },
}
def require(ok, message):
    if not ok:
        raise RuntimeError(message)
def digest(data):
    return hashlib.sha256(data).hexdigest()
class NoRedirect(HTTPRedirectHandler):
    def redirect_request(self, req, fp, code, msg, headers, newurl):
        return None
api = build_opener(NoRedirect())
def authenticated(url):
    return Request(url, headers={
        "Authorization": "Bearer " + os.environ["GH_TOKEN"],
        "Accept": "application/vnd.github+json",
        "X-GitHub-Api-Version": "2022-11-28",
        "User-Agent": "WordCraft-controlled-affix-performance",
    })
def download_runner(label, engine):
    base = "https://api.github.com/repos/totoz64/wordcraft/actions/artifacts/" + str(engine["artifact_id"])
    with api.open(authenticated(base), timeout=60) as response:
        metadata = json.load(response)
    require(metadata["workflow_run"]["head_sha"] == engine["source_sha"], "Artifact source SHA mismatch")
    require(metadata["workflow_run"]["id"] == engine["run"], "Artifact workflow run mismatch")
    require(metadata["digest"] == "sha256:" + engine["zip_sha256"] and not metadata["expired"], "Artifact digest/expiry mismatch")
    # The bearer token is sent to api.github.com only. A signed storage redirect is
    # followed with a new unauthenticated request; no credential reaches that host.
    try:
        with api.open(authenticated(base + "/zip"), timeout=60) as response:
            data = response.read(32 * 1024 * 1024 + 1)
    except HTTPError as error:
        if error.code != 302:
            raise RuntimeError("Artifact API download failed with status " + str(error.code)) from None
        location = error.headers.get("Location", "")
        require(location.startswith("https://"), "Unsafe artifact redirect")
        with urlopen(Request(location, headers={"User-Agent": "WordCraft-controlled-affix-performance"}), timeout=90) as response:
            data = response.read(32 * 1024 * 1024 + 1)
    require(digest(data) == engine["zip_sha256"], "Downloaded ZIP digest mismatch")
    (ROOT / (label + "-runner.zip")).write_bytes(data)
    with zipfile.ZipFile(io.BytesIO(data)) as archive:
        names = [info.filename for info in archive.infolist() if not info.is_dir()]
        require(names == ["wordcraft-proofing-benchmark"], "Unexpected artifact ZIP contents")
        info = archive.getinfo(names[0])
        require(info.file_size <= 32 * 1024 * 1024, "Runner artifact too large")
        binary = archive.read(names[0])
    destination = ROOT / (label + "-runner")
    destination.write_bytes(binary)
    destination.chmod(0o700)
    engine["binary_sha256"] = digest(binary)
    engine["binary_bytes"] = len(binary)
    engine["artifact_digest_verified"] = True
    return destination.resolve()

selection = json.loads((SOURCE_DIR / "selected-ids.json").read_text(encoding="utf-8"))
all_rows = [json.loads(line) for line in Path("benchmark-data/news-all.jsonl").read_text(encoding="utf-8").splitlines()]
require(len(all_rows) == 59048, "Unexpected full input size")
full_fingerprint = digest("".join(r["id"] + "\0" + r["language"] + "\0" + r["input"] + "\n" for r in all_rows).encode())
require(full_fingerprint == selection["full_corpus_canonical_sha256"], "Full corpus canonical mismatch")
by_id = {row["id"]: row for row in all_rows}
require(len(by_id) == 59048, "Duplicated full input IDs")
rows = [by_id[case_id] for case_id in selection["case_ids"]]
require(len(rows) == 192 and len(set(selection["case_ids"])) == 192, "Unexpected sample size")
fingerprint = digest("".join(r["id"] + "\0" + r["language"] + "\0" + r["input"] + "\n" for r in rows).encode())
require(fingerprint == selection["canonical_cases_sha256"], "Selected input mismatch")
input_path = ROOT / "inputs.jsonl"
warmup_path = ROOT / "warmup.jsonl"
for file, values in ((input_path, rows), (warmup_path, rows[:48])):
    file.write_text("".join(json.dumps(r, ensure_ascii=False, separators=(",", ":")) + "\n" for r in values), encoding="utf-8")
binaries = {"before": download_runner("before", ENGINES["before"])}
local_binary = Path(ENGINES["after"]["local_binary"]).resolve()
binary_bytes = local_binary.read_bytes()
ENGINES["after"]["binary_sha256"] = digest(binary_bytes)
ENGINES["after"]["binary_bytes"] = len(binary_bytes)
ENGINES["after"]["source_verification"] = "release binary built from this workflow checkout after full proofing gates"
binaries["after"] = local_binary
def machine_information():
    result = {
        "platform": platform.platform(), "python": platform.python_version(),
        "logical_cpus": os.cpu_count(), "github_run_id": os.environ.get("GITHUB_RUN_ID"),
        "workflow_source_sha": os.environ.get("GITHUB_SHA"),
    }
    proc = subprocess.run(["lscpu", "--json"], text=True, capture_output=True, check=True, timeout=10)
    result["lscpu"] = json.loads(proc.stdout)
    result["memtotal"] = next((line for line in Path("/proc/meminfo").read_text().splitlines() if line.startswith("MemTotal:")), None)
    governor = Path("/sys/devices/system/cpu/cpu0/cpufreq/scaling_governor")
    result["cpu_governor"] = governor.read_text().strip() if governor.exists() else "unavailable"
    return result
report = {
    "status": "running", "engines": ENGINES, "machine": machine_information(),
    "selection": selection, "warmup_cases_per_engine": 48, "repetitions": 4,
    "measurement_order": [["before", "after"], ["after", "before"], ["before", "after"], ["after", "before"]],
    "protocol": {
        "same_job_same_machine": True, "sequential_processes": True, "same_dictionary_and_options": True,
        "before_prebuilt_release_after_built_release": True, "fresh_process_per_invocation": True,
        "source_sha_child_environment": "GITHUB_SHA set to the verified artifact engine source SHA; workflow SHA recorded separately",
        "decision_heuristic": "Consistent overhead if all four wall ratios exceed 1.05 and median exceeds 1.10; diagnostic threshold, not a statistical confidence interval",
    },
    "measurements": [], "limits": [
        "192 fixed development cases; not a general-language speed estimate or accuracy score.",
        "One hosted VM and four paired repetitions limit generalization; CPU scheduling/frequency and file cache may still vary.",
        "Separate fresh runner processes model the same public proofing API surface; internal process state is not shared.",
        "The measured before engine is the pre-prefix baseline694. Equality of observations on this fixed sample is enforced; unit corrections are validated separately. The optimized after runner is built from this workflow source SHA.",
        "No Word timing is compared."
    ],
}
def write_report():
    (ROOT / "performance-report.json").write_text(json.dumps(report, indent=2), encoding="utf-8")
write_report()
def observe(label, repetition, source, measured):
    output = ROOT / (("measured-" if measured else "warmup-") + str(repetition) + "-" + label + ".jsonl")
    env = dict(os.environ)
    env["GITHUB_SHA"] = ENGINES[label]["source_sha"]
    # The runner never needs access to the job token.
    env.pop("GH_TOKEN", None)
    usage_before = resource.getrusage(resource.RUSAGE_CHILDREN)
    started = time.perf_counter_ns()
    process = subprocess.run([str(binaries[label]), str(source), str(output)], env=env, text=True, capture_output=True, timeout=180)
    elapsed = (time.perf_counter_ns() - started) / 1e9
    usage_after = resource.getrusage(resource.RUSAGE_CHILDREN)
    require(process.returncode == 0, "Runner failed: " + label + " status " + str(process.returncode))
    records = [json.loads(line) for line in output.read_text(encoding="utf-8").splitlines()]
    expected_rows = rows if measured else rows[:48]
    require([r["id"] for r in records] == [r["id"] for r in expected_rows], "Output IDs/order mismatch")
    require(all(r["status"] == "ok" and r["range_unit"] == "utf16" for r in records), "Observation status mismatch")
    metadata = json.loads(Path(str(output) + ".metadata.json").read_text(encoding="utf-8"))
    require(metadata["source_sha"] == ENGINES[label]["source_sha"] and metadata["status"] == "completed" and metadata["cases"] == len(expected_rows), "Runner metadata mismatch")
    require(metadata["options"] == {"ignore_uppercase": True, "ignore_numbers": True, "ignore_internet": True}, "Unexpected runner options")
    require(metadata["suggestion_limit"] == 6 and metadata["grammar_checked"], "Unexpected runner surface")
    observable = [{"id": r["id"], "status": r["status"], "spelling": r["spelling"], "grammar": r["grammar"]} for r in records]
    result = {
        "engine": label, "repetition": repetition, "measured": measured,
        "wall_seconds": elapsed, "runner_seconds": metadata["duration_seconds"],
        "child_user_seconds": usage_after.ru_utime - usage_before.ru_utime,
        "child_system_seconds": usage_after.ru_stime - usage_before.ru_stime,
        "cases": len(records), "status_ok": len(records),
        "spelling_issues": sum(len(r["spelling"]) for r in records),
        "grammar_issues": sum(len(r["grammar"]) for r in records),
        "observable_sha256": digest(json.dumps(observable, sort_keys=True, separators=(",", ":")).encode()),
    }
    report["measurements"].append(result)
    write_report()
    print(json.dumps(result), flush=True)
    return result
observe("before", 0, warmup_path, False)
observe("after", 0, warmup_path, False)
pairs = []
for repetition, order in enumerate(report["measurement_order"], 1):
    pair = {label: observe(label, repetition, input_path, True) for label in order}
    cpu_before = pair["before"]["child_user_seconds"] + pair["before"]["child_system_seconds"]
    cpu_after = pair["after"]["child_user_seconds"] + pair["after"]["child_system_seconds"]
    pairs.append({
        "repetition": repetition, "order": order,
        "wall_ratio_after_over_before": pair["after"]["wall_seconds"] / pair["before"]["wall_seconds"],
        "runner_ratio_after_over_before": pair["after"]["runner_seconds"] / pair["before"]["runner_seconds"],
        "cpu_ratio_after_over_before": cpu_after / cpu_before,
    })
ratios = [p["wall_ratio_after_over_before"] for p in pairs]
consistent = all(r > 1.05 for r in ratios) and statistics.median(ratios) > 1.10
report["paired_ratios"] = pairs
report["median_wall_ratio_after_over_before"] = statistics.median(ratios)
report["median_cpu_ratio_after_over_before"] = statistics.median(p["cpu_ratio_after_over_before"] for p in pairs)
report["result_consistency"] = {
    label: len(set(m["observable_sha256"] for m in report["measurements"] if m["measured"] and m["engine"] == label)) == 1
    for label in ENGINES
}
require(report["result_consistency"]["before"] and report["result_consistency"]["after"], "Nondeterministic per-engine observations")
fingerprints = {m["observable_sha256"] for m in report["measurements"] if m["measured"]}
require(len(fingerprints) == 1, "Before/after observable results differ on controlled sample")
report["before_after_observations_identical"] = True
report["status"] = "completed"
speedup = all(r < 0.95 for r in ratios) and statistics.median(ratios) < 0.90
report["conclusion"] = "consistent_speedup_on_this_development_sample" if speedup else ("consistent_overhead_on_this_development_sample" if consistent else "mixed_or_small_difference_on_this_development_sample")
write_report()
print(json.dumps({key: report[key] for key in ["status", "conclusion", "median_wall_ratio_after_over_before", "median_cpu_ratio_after_over_before", "result_consistency", "paired_ratios"]}, indent=2), flush=True)
