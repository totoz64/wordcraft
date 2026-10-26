# Shipping and improving French proofing

> **Last reviewed:** 2026-10-20 · **Last updated:** 2026-10-26 · **Change:** minor (bounded contextual suggestions under benchmark validation); previously minor (French uppercase/number proofing options and review-record corrections); previously minor (integration with current main, shared NFC lookup and external interface catalogs); previously minor (French spelling keeps attached combining marks and checks compound boundaries); previously minor (French proofing region parser respects extension boundaries); previously minor (French dictionary recovery and regional labels); previously trivial (French hyphenation CI path coverage); previously minor (historical affix compatibility gates and pinned French CI toolchain); previously major (French integration and regression gates, based on #1660)

## Shared-engine compatibility and reproducible CI

The [historical affix compatibility gate](affix-compatibility.md) compares seven
original non-French fixtures against actual upstream e6c5a70 outputs and old
WCAFF1 bytes. The full proofing suite includes these tests. The French workflow
pins both its action revision and Rust 1.98.1, prints the compiler version, and
runs `cargo fmt --all -- --check` on Linux, Windows and macOS. Real dictionary
inputs retain their revision and byte checksums. Changes to
`assets/hyphenation/fr.pat` trigger this workflow on pull requests and main pushes,
including changes to that pattern file alone.

## Baseline for review

This series rebases the French work in [#1660](https://github.com/storytold/wordcraft/pull/1660)
onto current main, retaining the original author's commits. It preserves Indonesian proofing
and the newer dictionary error diagnostics. French works with the installed Dicollecte Hunspell
dictionary; the dictionary is an external runtime input and is never bundled in this repository.
Install it using [spelling-dictionaries.md](spelling-dictionaries.md).

The baseline includes spelling, accent and apostrophe suggestions, conservative French typography
and grammar, and hyphenation. `fr-CA` shares spelling and uses Quebec punctuation conventions.
New documents under the French interface take the French system region. Missing and unreadable
dictionaries are reported through the existing status-bar and Options diagnostics.

French spelling follows the Proofing choices for ignoring uppercase words and words
containing numbers. Turning either choice off checks those words against the installed
dictionary in both French regions; pure numbers and the no-dictionary fallback remain
unmarked. The direct spelling APIs retain their default ignore choices.

Document proofing reads the Canadian region only in its region position, after an optional
script (`fr-CA`, `fr-Latn-CA`). Extension data such as `fr-u-ca-gregory` and `fr-x-ca`
does not select Quebec typography.

General subject–verb agreement, gender/number agreement and conjugation remain outside this baseline.
A separately evaluated follow-up recognizes a short list of paragraph-initial subject/avoir/first-group
infinitive forms, a/à in a few initial avoir expressions, and là où. It preserves the original token
ranges and offers suggestions only; no automatic document-wide correction is applied.

These new rules are intentionally narrow: no intervening object or relative clause, no punctuation
or line break between matched words, no arbitrary -er word stemming, and no full participle-agreement
claim. Correct and ambiguous counterexamples are required, including hier, cher, personal names,
relative clauses, avoir à manger, and correct infinitives. A valid article ou/où is not elided;
la où is offered as là où, preserving case. The full comparative corpus remains silver data and
must not be treated as certified grammar truth. See the [isolated observation protocol](https://github.com/totoz64/wordcraft/blob/694a8e76079453de3fa68321d3ce11f49dcf9f39/docs/proofing-comparison.md).

## Reproduce the checks

The synthetic suite runs without downloads:

```sh
cargo test --locked -p wordcraft-proof
cargo test --locked -p wordcraft-engine --lib french_proofing_through_the_review_commands
cargo test --locked -p wordcraft-ui-egui --lib a_french_interface_writes_new_documents_in_french
cargo xtask ci
```

For the real dictionary, set `WORDCRAFT_DICTIONARIES` to its folder, then run:

```sh
cargo test --locked -p wordcraft-proof --test french_dictionary -- --ignored
```

This explicit gate fails if the dictionary is absent, unreadable, unexpectedly small, or found
outside the requested folder. It checks real words, accents, elisions, suggestions and the
fr-FR/fr-CA dispatch. A separate process tests unreadable-dictionary diagnostics and recovery,
so synthetic unit tests cannot replace the dictionary being checked.

The **French proofing** workflow runs these dictionary and review-command checks on Windows,
macOS and Linux. It downloads a pinned LibreOffice revision of Dicollecte 7.7 and verifies both
SHA-256 hashes. The existing UI workflow and `cargo xtask ci` remain required validation;
declaring a workflow here does not mean its remote runs have passed.

## Small follow-ups

| Order | Deliverable | Acceptance evidence |
|---|---|---|
| 1 | Integrate this baseline and its installation guide | Current-main diff, local CI, real dictionary gate, native UI smoke test; maintainer review and remote checks before merge |
| 2 | Add French AutoFormat for spaces around `; : ! ?` and `« »` | Typing-command regression cases for fr-FR and fr-CA, ordinary/non-breaking spaces, URLs, times, undo and pasted text; update `typing-parity.md` |
| 3 | Expand the regression corpus from reported errors | One short synthetic sentence per failure, expected suggestions or explicit quiet result, region and dictionary version; no personal documents or unreviewed data committed |
| 4 | Add agreement and homophone rules with linguistic data | A documented data/licence choice, positive and quiet cases, bounded runtime, and false-positive measurements before each rule is enabled |

## Reporting a French correction problem

Provide the WordCraft commit/version, the dictionary version, the document's proofing language,
one short non-sensitive example, the observed result and the expected correction. Distinguish a
misspelled word, an unsuitable suggestion, and a grammatical error outside the baseline. Include
a correct sentence that must stay unmarked when a proposed rule could be ambiguous.

Use synthetic examples in code and in public reports. Add the smallest regression case to
`crates/proof/src/fr/` or the engine review-command test, and run the required real-dictionary gate
before proposing a suggestion/ranking change. Keep follow-ups independently reviewable.

## Revision history

| Date | Change | Summary |
|---|---|---|
| 2026-10-26 | minor | Isolated benchmark follow-up: fix invalid article elision before ou/où, suggest là où and selected paragraph-initial avoir forms; regressions and comparative revalidation required |
| 2026-10-26 | minor | French dispatch honours the uppercase and number ignore options; regressions cover both regions, independent choices and original UTF-8 issue ranges; progress records aligned with the current integration |
| 2026-10-25 | minor | Integration with current main preserves shared NFC lookup, other proofing languages, dictionary diagnostics and external interface catalogs |
| 2026-10-20 | minor | Attached Unicode marks keep their original spelling ranges; French compound fallback rejects empty internal components while preserving explicit dictionary entries |
| 2026-10-20 | minor | French proofing reads Canada only from the region after the language/optional script; Unicode calendar and private-use extension subtags do not select Quebec typography; production-parser regressions |
| 2026-10-20 | minor | French dictionary replacement clears stale load errors only after successful parsing; Canadian French keeps its own Options label |
| 2026-10-20 | trivial | French hyphenation pattern asset included in both workflow path filters so pattern-only changes run the proofing regressions |
| 2026-10-20 | minor | Frozen pre-French affix outputs and legacy compact dictionaries agree; French CI pins Rust 1.98.1 and checks formatting on all three desktop systems |
| 2026-10-20 | major | Integration checklist, explicit real-dictionary gate, feedback format and staged French follow-ups |

A successful call to the public French dictionary installation API clears an earlier
load diagnostic; an invalid replacement preserves the current dictionary and diagnostic.
The regression exercises unreadable disk data, a failed replacement, a valid replacement
and a second failed replacement. Options labels keep France and Canada distinct.

## Combining marks and French compound boundaries

The proofing tokenizer keeps a Unicode Mark attached to its preceding letter or
continuing mark sequence. It does not attach an orphan mark after punctuation or
digits. This avoids splitting a decomposed spelling such as `e` + U+0301 + `cole`
before spelling lookup. Current main composes lookup copies to NFC before dictionary input conversions. The original
UTF-8 text and issue offsets are retained; the document is not normalized or
rewritten. See [Unicode canonical equivalence](https://unicode.org/reports/tr15/#Canonical_And_Compatibility_Equivalence).

French punctuation trimming also retains a final attached mark, as in `cafe` +
U+0301. The shared NFC lookup introduced on main is preserved for French and
other languages; recognition then uses the installed dictionary and its input
conversions. Only the lookup copy is composed; source text and byte ranges stay unchanged.

The French compound fallback requires every internal hyphen-separated component
to be nonempty. `porte--monnaie` and `va--t-il` can therefore be reported instead
of bypassing spelling checks. Existing valid compounds and inversions remain
accepted. Explicit full-word dictionary entries and user-dictionary choices are
checked before this fallback and retain priority.

Process-isolated synthetic regressions cover original byte ranges, both French
regions, orphan marks, another dictionary-backed language and explicit user
entries. The existing required real French dictionary gate covers both forms of
accented words and malformed versus valid compounds with pinned Dicollecte data.
