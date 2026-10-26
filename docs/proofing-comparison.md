# Proofing comparison observations

> Last updated: 2026-10-26. Status: benchmark preparation, no comparative quality score claimed.

This isolated fork branch starts from French proofing c394110777924eca9f900941dba4cc040f526573. It does not merge the independent Proofing pane proposal. It records the existing public proofing API on 117 original pilot examples and 59048 runtime-only synthetic news cases (49048 distinct altered variants and 10000 matching source controls).

The corpus, dictionary and source revision are pinned; input acquisition verifies byte hashes and the canonical digest of every case ID, explicit language and input text. Neither downloaded dictionaries nor news sentences are repository assets. Results contain IDs, ranges, original WordCraft rule messages and ordered suggestions; source sentences are not uploaded.

Word observations remain outside this repository and use its documented object model. Its grammar API reports sentence ranges without grammar suggestions, and parity with every cloud Editor feature is not established. Compare combined detection and accessible spelling ranking without claiming complete Editor UI equivalence. Both runners cap suggestions at six. Word uses UTF-16 offsets; the Rust runner converts its UTF-8 offsets to UTF-16 and retains both for audit. Word keeps existing private dictionaries; the custom-dictionary pilot case is excluded from the shared score.

Synthetic targets are silver annotations, not linguistic gold. Valid changes to numbers or words and alternative good corrections require adjudication. Source control sentences also need review. Report correlated source families and spelling/grammar separately; the held-out split must not be used to tune rules. Runtime measurements use different hosts and cannot prove relative application speed.

Run tools/proofing-benchmark/prepare_inputs.py, set WORDCRAFT_DICTIONARIES to benchmark-data/dictionary, then cargo run --release --manifest-path tools/proofing-benchmark/Cargo.toml -- input.jsonl output.jsonl [shard] [shard_count]. Cargo is absent on the preparation PC; hosted formatting, Clippy and observations are required.

## Sources

- Corpus: https://huggingface.co/datasets/fdemelo/spelling-correction-french-news (MIT declared by its card; underlying news provenance retained, redistribution not assumed).
- Dictionary: Dicollecte 7.7 at LibreOffice/dictionaries 67032ee957631908098a72edf48dc81cac27e369; external runtime input, MPL 2.0.
- Original pilot: synthetic project examples; expectations remain provisional and actual outputs are evaluated separately.

## History

- 2026-10-26: add isolated, bounded observation runner and pinned runtime inputs; no production behavior changed and no benchmark winner claimed.
