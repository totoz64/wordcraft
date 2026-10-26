# Runtime-only input acquisition: source sentences and dictionary files are not repository assets.
import csv
import hashlib
import io
import json
import os
from pathlib import Path
from urllib.request import urlopen

root = Path(os.environ.get('BENCHMARK_DATA', 'benchmark-data'))
root.mkdir(parents=True, exist_ok=True)
dictionary = root / 'dictionary'
dictionary.mkdir(exist_ok=True)
base = 'https://raw.githubusercontent.com/LibreOffice/dictionaries/67032ee957631908098a72edf48dc81cac27e369/fr_FR/dictionaries/'
for name, expected in {
    'fr.aff': 'c176610cd5dc4846806a65ddd029f422d87978bf58f224aa44222662a16a2de5',
    'fr.dic': 'b78a868e31dd6e373b6c3217969afb898a9acde828a5e7ef97308da42218c88c',
}.items():
    with urlopen(base + name, timeout=90) as response:
        data = response.read(64 * 1024 * 1024 + 1)
    if hashlib.sha256(data).hexdigest() != expected:
        raise ValueError('Dictionary checksum mismatch: ' + name)
    (dictionary / name).write_bytes(data)
if os.environ.get('BENCHMARK_DICTIONARY_ONLY') == '1':
    raise SystemExit(0)
url = 'https://huggingface.co/datasets/fdemelo/spelling-correction-french-news/resolve/e43d812d5b37f784dfbe9aebb36fe5b6648a4ea5/data/dataset/dataset.csv?download=true'
with urlopen(url, timeout=90) as response:
    data = response.read(20 * 1024 * 1024 + 1)
if hashlib.sha256(data).hexdigest() != '568adf975baad0f56b2a7e366ddfbb66e012cb2fab59de67d326019e2eae2091':
    raise ValueError('Corpus checksum mismatch')
reader = csv.DictReader(io.StringIO(data.decode('utf-8'), newline=''))
families, seen, rows = {}, set(), []
for index, row in enumerate(reader, 1):
    if not row['input'].startswith('grammaire: '):
        raise ValueError('Unexpected prefix')
    source, target = row['input'][11:], row['target']
    key = (source, target)
    if key in seen:
        continue
    seen.add(key)
    rows.append({'id': f'news-error-{index:05}', 'language': 'fr-FR', 'input': source})
    families.setdefault(target, 'news-' + hashlib.sha256(target.encode()).hexdigest()[:20])
rows.extend({'id': family + '-control', 'language': 'fr-FR', 'input': text} for text, family in families.items())
digest = hashlib.sha256()
with (root / 'news-all.jsonl').open('w', encoding='utf-8', newline='\n') as output:
    for row in rows:
        digest.update((row['id'] + '\0' + row['language'] + '\0' + row['input'] + '\n').encode('utf-8'))
        output.write(json.dumps(row, ensure_ascii=False, separators=(',', ':')) + '\n')
expected_digest = '28bf8b73aa4ae14e72e1c76c3254fd16f5b713c8a0e22019850140f2e3030f0d'
if len(rows) != 59048 or len(families) != 10000 or digest.hexdigest() != expected_digest:
    raise ValueError('Canonical case-set mismatch')
(root / 'input-provenance.json').write_text(json.dumps({'case_count': len(rows), 'canonical_cases_sha256': digest.hexdigest(), 'source_url': url, 'license': 'MIT according to dataset card; do not redistribute source sentences from this runtime directory'}, indent=2), encoding='utf-8')
print(json.dumps({'cases': len(rows), 'canonical_cases_sha256': digest.hexdigest()}))
