# Shipping and improving French proofing

> **Last reviewed:** 2026-10-20 · **Last updated:** 2026-10-20 · **Change:** minor (reviewed French baseline: dictionary recovery, regional labels, historical compatibility, pinned CI and French pattern triggers); previously minor (personal-cleft agreement and optional finite morphology; baseline based on #1660)

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

General subject–verb agreement, gender/number agreement and contextual homophones are not
checked. Conjugation coverage is limited to the first cases below. No automatic document-wide
correction is applied.

## First grammar and conjugation increment

This follow-up is kept separate from installation and the baseline's integration review.
It adds two small rules, in both `fr-FR` and `fr-CA`:

- `si il` / `si ils` → `s’il` / `s’ils`. `si elle`, `si elles`, `si on`, names and words separated
  by punctuation remain unchanged. This follows the [OQLF rule on obligatory elision](https://vitrinelinguistique.oqlf.gouv.qc.ca/21737/lorthographe/elision-et-apostrophe/elision-obligatoire).
- `J’ai manger` → `J’ai mangé`, and equivalent subject + present `avoir` beginnings such as
  `Tu as écouter` → `Tu as écouté`. The first rule covers only a paragraph's initial phrase and
  24 explicitly listed first-group verbs; it does not guess from an arbitrary `-er` ending.
  `Je vais manger`, `J’ai entendu chanter`, preceding objects, quotations, code, tables and
  detected metalinguistic examples remain outside this rule. The [OQLF distinction between
  infinitive and participle](https://vitrinelinguistique.oqlf.gouv.qc.ca/21614/la-grammaire/le-verbe/modes/infinitif/distinction-entre-linfinitif-et-le-participe-passe)
  explains why replacing every infinitive after a verb would be incorrect. The simple
  auxiliary forms were checked against the [Académie's avoir conjugation](https://www.dictionnaire-academie.fr/conjuguer/A9A3461).

The finite tables and synthetic sentences are original; no third-party corpus or dictionary
prose is imported. This is an initial, deliberately narrow check, not general conjugation or
participle agreement. It can miss mistakes in quotations, later clauses and other verbs.

`crates/proof/tests/french_grammar.rs` separates original errors, correct text and unsupported
errors. The last category is not counted as correct-text precision data. Every targeted error
must have the expected byte range and suggestion; each suggested correction must stop triggering
the new rule. Correct-text examples must produce no new alerts. These are small synthetic
regressions, not an estimate of a real-world false-positive rate. The table test separately
checks every supported verb and subject combination and its corrected counterpart.

The engine integration test exercises F7, Change, Undo and **Mark grammar errors as you type**
off/on using the same commands as the desktop UI. Installation, spelling and typing behaviour
are unchanged by this increment.

## Personal-cleft agreement and optional morphology

`C’est moi qui est` suggests `suis`; `C’est nous qui ont` suggests `avons`.
The check covers only a paragraph-initial `C’est` + personal antecedent + `qui` +
adjacent finite verb, or `Ce sont eux/elles qui`. It excludes quotations, code,
tables, intervening clitics/negation and detected metalinguistic examples.
The [OQLF agreement rule](https://vitrinelinguistique.oqlf.gouv.qc.ca/23397/la-grammaire/le-verbe/accord-du-verbe-avec-le-sujet/sujet-qui-est-un-pronom-relatif/accord-du-verbe-avec-le-pronom-relatif-qui-precede-dun-attribut)
also describes variable agreement with noun/demonstrative attributes. Accordingly,
`Je suis celle qui est` and `Nous sommes ceux qui ont` are outside this rule.

Four original present-tense paradigms (être, avoir, suivre, sommer) work without
downloads. All finite readings are retained: `C’est toi qui suis cette piste`
and `C’est toi qui sommes le débiteur de payer` must not be flagged merely because
`suis` and `sommes` also have être readings.

For additional verbs and seven finite tenses/moods, place the unchanged
`lexique-grammalecte-fr-v7.7.txt` beside the spelling dictionaries in the configured
**Dictionaries folder**, or in its `wordcraft-french-grammar-7.7-bdccf206` subdirectory.
The direct file takes precedence. Reload the folder setting or restart after changing it.
This optional data is **not installed by the spelling-dictionary button**, and is never
bundled in the repository or application.

The [published 7.7 lexicon](https://www.grammalecte.net/dic/lexique-grammalecte-fr-v7.7.zip)
contains linguistic TSV data under **MPL 2.0**, confirmed in its own README and header.
Archive SHA-256: `bdccf2065e252010253171000a38de84581407896b6bc60aca7d4d69dab7e4d2`.
Retain `README_lexique.txt` and its licence notice with the data. The original Rust
reader does not use Grammalecte grammar code. The real-data gate currently reads
274,427 finite analyses; nouns, infinitives, imperatives and participles are not
treated as finite readings. File size, line length, row count and index fan-out are bounded.
An unreadable optional file records a diagnostic through `fr::morphology::load_error()`
and leaves the builtin rules available. A successful reload clears the diagnostic.

A form can have several lemmas or moods. If **any** finite reading agrees with the
antecedent, no alert is produced. Otherwise suggestions retain a possible lemma and
tense/mood; the reader does not infer the intended mood from the sentence. For example,
`manges` has indicative and subjunctive readings, so `nous` can yield `mangeons` and
`mangions`. This conservative scope can miss errors; it is not general subject agreement,
participle agreement or a real-world precision claim.

The independent runtime test covers corrupt data, manual-file precedence, fallback,
reload and cleared errors. The pinned real-data test covers seven finite tenses/moods,
homographs and both French regions. F7, Change and Undo also cover the builtin cleft rule.

```sh
# Synthetic tests need no network/data download.
cargo test --locked -p wordcraft-proof --test french_morphology
# Explicit data gate: absence is a failure, never a silent skip.
WORDCRAFT_FRENCH_LEXICON=/path/to/lexique-grammalecte-fr-v7.7.txt cargo test --locked -p wordcraft-proof --test french_morphology -- --ignored
```

## Reproduce the checks

The synthetic suite runs without downloads:

```sh
cargo test --locked -p wordcraft-proof
cargo test --locked -p wordcraft-proof --test french_grammar
cargo test --locked -p wordcraft-engine --lib french_proofing_through_the_review_commands
cargo test --locked -p wordcraft-engine --test french_grammar
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
| 2026-10-20 | minor | French dictionary replacement clears stale load errors only after successful parsing; Canadian French keeps its own Options label |
| 2026-10-20 | trivial | French hyphenation pattern asset included in both workflow path filters so pattern-only changes run the proofing regressions |
| 2026-10-20 | minor | Frozen pre-French affix outputs and legacy compact dictionaries agree; French CI pins Rust 1.98.1 and checks formatting on all three desktop systems |
| 2026-10-20 | minor | Personal-cleft agreement, ambiguity-aware optional MPL-2.0 finite morphology, bounded reader, fallback/reload diagnostics and explicit pinned real-data coverage |
| 2026-10-20 | minor | Bounded avoir + first-group participle suggestions, si il/ils elision, original error/quiet/unsupported regressions and command-level apply/undo tests |
| 2026-10-20 | major | Integration checklist, explicit real-dictionary gate, feedback format and staged French follow-ups |

A successful call to the public French dictionary installation API clears an earlier
load diagnostic; an invalid replacement preserves the current dictionary and diagnostic.
The regression exercises unreadable disk data, a failed replacement, a valid replacement
and a second failed replacement. Options labels keep France and Canada distinct.
