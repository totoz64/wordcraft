# Shipping and improving French proofing

> **Last reviewed:** 2026-10-20 · **Last updated:** 2026-10-20 · **Change:** minor (first bounded conjugation checks and si elision; baseline based on #1660)

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
| 2026-10-20 | minor | Bounded avoir + first-group participle suggestions, si il/ils elision, original error/quiet/unsupported regressions and command-level apply/undo tests |
| 2026-10-20 | major | Integration checklist, explicit real-dictionary gate, feedback format and staged French follow-ups |
