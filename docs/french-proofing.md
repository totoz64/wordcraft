# Shipping and improving French proofing

> **Last reviewed:** 2026-10-20 · **Last updated:** 2026-10-20 · **Change:** minor (French spelling keeps attached combining marks and checks compound boundaries); previously minor (French proofing region parser respects extension boundaries); previously minor (French dictionary recovery and regional labels); previously trivial (French hyphenation CI path coverage); previously minor (historical affix compatibility gates and pinned French CI toolchain); previously major (French integration and regression gates, based on #1660); previously minor (reviewed French baseline: dictionary recovery, regional labels, historical compatibility, pinned CI and French pattern triggers); previously minor (desktop dictionary installation and activation; baseline based on #1660); previously minor (first bounded conjugation checks and si elision; baseline based on #1660); previously minor (reviewed French baseline: dictionary recovery, regional labels, historical compatibility, pinned CI and French pattern triggers); previously minor (dictionary installation, personal-cleft agreement and optional finite morphology; baseline based on #1660); previously minor (personal-cleft agreement and optional finite morphology; baseline based on #1660); previously minor (reviewed French baseline: dictionary recovery, regional labels, historical compatibility, pinned CI and French pattern triggers); previously minor (explicit French verb-form data installation; baseline #1660); previously minor (French irregular participles and five avoir tenses; baseline #1660)

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

Desktop builds offer **File › Options › Proofing › Install French dictionary**. An explicit
click downloads pinned, hash-verified files in the background, publishes a complete versioned
directory, then persists and reloads the dictionaries folder without restarting. Existing manual
dictionaries take precedence. Errors permit retry and a folder changed during installation is
never replaced by a stale result. There is no startup download or browser installer.

The baseline includes spelling, accent and apostrophe suggestions, conservative French typography
and grammar, and hyphenation. `fr-CA` shares spelling and uses Quebec punctuation conventions.
New documents under the French interface take the French system region. Missing and unreadable
dictionaries are reported through the existing status-bar and Options diagnostics.

Document proofing reads the Canadian region only in its region position, after an optional
script (`fr-CA`, `fr-Latn-CA`). Extension data such as `fr-u-ca-gregory` and `fr-x-ca`
does not select Quebec typography.

General subject–verb agreement, gender/number agreement and contextual homophones are not
checked. Conjugation coverage is limited to the first cases below. No automatic document-wide
correction is applied.

## Infinitive/participle and elision checks

Both `fr-FR` and `fr-CA` support these bounded suggestions:

- `si il` / `si ils` → `s’il` / `s’ils`. `si elle`, `si elles`, `si on`, names and words separated
  by punctuation remain unchanged. This follows the [OQLF rule on obligatory elision](https://vitrinelinguistique.oqlf.gouv.qc.ca/21737/lorthographe/elision-et-apostrophe/elision-obligatoire).
- A paragraph-initial subject followed by correctly matched **avoir** can suggest a participle:
  `J’ai manger` → `J’ai mangé`, `J’avais écrire` → `J’avais écrit`,
  `Nous aurions prendre` → `Nous aurions pris`. Supported auxiliary tenses are present,
  imperfect, future, conditional and simple past, checked against the
  [Académie conjugation](https://www.dictionnaire-academie.fr/conjuguer/A9A3461).
  One separated dialogue/list dash (`—`, `–`, `-`) may precede the subject.

There are 66 original fallback infinitive/participle pairs, including irregular verbs. Optional
data supplies additional recorded forms; no arbitrary ending is transformed. A valid recorded
participle reading stays quiet. Verbs requiring être, unknown auxiliary choices and unavailable
forms are declined. Data indicating a transitive active use permits avoir, following the
[OQLF auxiliary table](https://vitrinelinguistique.oqlf.gouv.qc.ca/24584/la-grammaire/le-verbe/auxiliaires/tableau-synthese-de-lemploi-des-auxiliaires-avoir-et-etre).
For mixed/uncertain transitive–intransitive uses, an adjacent object determiner is required;
impersonal verbs require explicit `il`. Bare `pouvoir`/`devoir` can be nouns, so those two
modal readings additionally require an adjacent known infinitive complement.

`Je vais manger`, `J’ai entendu chanter`, preceding objects/clitics, negation, intervening
adverbs, quotations, code, tables, legacy `--` dialogue prefixes and detected metalinguistic
examples remain outside this rule. It does not choose an auxiliary or resolve participle
agreement. The [OQLF distinction between infinitive and participle](https://vitrinelinguistique.oqlf.gouv.qc.ca/21614/la-grammaire/le-verbe/modes/infinitif/distinction-entre-linfinitif-et-le-participe-passe)
explains why replacing every infinitive after a verb would be incorrect.

`french_grammar.rs` and `french_participles.rs` separate original errors, correct text and
ambiguous/unsupported constructions. Unsupported errors do not count as correct-text precision
data. Every targeted error checks exact byte ranges and suggestions, then verifies stable
corrected text. An additional table checks 3,300 subject/tense/verb combinations and their
3,300 corrected counterparts. The explicit real-data test exercises the whole grammar path for
6,057 supported simple infinitives in the defined first-person context (an infinitive complement
is supplied for the ambiguous modals), including canonical suggested forms and stable results.
These are synthetic/data coverage checks, not a real-world precision estimate.

An optional `french_prose` test uses external French prose, never bundled in this repository.
The recorded local run used 9,486 original paragraphs from four books, with 42 auxiliary
beginnings, plus 2,889 separately labelled variants replacing only a leading `--` by `— `,
with 47 auxiliary beginnings. There were 79 unquoted eligible beginnings and no new participle
alerts. Counts include overlapping variants and are a smoke check, not statistical precision
or broad grammar coverage. See [source metadata and normalization](french-prose-sources.json).
Input format is UTF-8 `source<TAB>paragraph` per line, bounded to 8 MB; set
`WORDCRAFT_FRENCH_PROSE` and `WORDCRAFT_FRENCH_LEXICON`, then run:

```sh
cargo test --locked -p wordcraft-proof --test french_prose -- --ignored --nocapture
```

Engine integration exercises F7, Change, Undo and **Mark grammar errors as you type** off/on.
It also fixes `Je avais écrire` in two steps, verifying recomputed UTF-8 ranges after elision
and two undos restoring the original text. Suggestions never rewrite the document automatically.

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

For additional verbs and seven finite tenses/moods, desktop users can open
**File › Options › Proofing** and click **Add French verb forms** (French:
**Ajouter les formes verbales françaises**). The roughly 12 MB download installs
the external text and its licence/readme in a separate versioned subdirectory,
then reloads the selected folder immediately. No document is edited and no restart is needed.
An existing complete installation is verified and reused without another download.
The grammar and spelling installers cannot run at the same time; this prevents
competing folder activation. Startup and status inspection never download data.
The command `ui.frenchGrammarData` takes `{install?: bool}`; inspection without
`install: true` only reports availability, busy/ready state and errors. The web
has no native installation host and does not show the download button.
Readiness is based on the validated data actually loaded, so it remains visible after
restart. The command also reports `analyses` and `dataError`; an unreadable file is
diagnosed even without starting an installation. Selecting another folder reloads this state.

The original native host checks the pinned ZIP hash before decompression and then
checks the text and README hashes. Only those two named, size-bounded members are read,
without extracting arbitrary paths. A complete parsed index is staged before atomic
publication. Interrupted downloads, unreadable manual files and incomplete or corrupt
managed installations are reported and kept unchanged; retry is possible after the
underlying problem is resolved. Changing the selected folder during installation keeps
the newer preference and asks for a retry. Installation progress and errors are localized.

Alternatively, place the unchanged
`lexique-grammalecte-fr-v7.7.txt` beside the spelling dictionaries in the configured
**Dictionaries folder**, or in its `wordcraft-french-grammar-7.7-bdccf206` subdirectory.
The direct file takes precedence. Reload the folder setting or restart after changing it.
This optional data uses its own button, separate from the spelling dictionary. It is never
bundled in the repository or application.

The [published 7.7 lexicon](https://www.grammalecte.net/dic/lexique-grammalecte-fr-v7.7.zip)
contains linguistic TSV data under **MPL 2.0**, confirmed in its own README and header.
Archive SHA-256: `bdccf2065e252010253171000a38de84581407896b6bc60aca7d4d69dab7e4d2`.
Retain `README_lexique.txt` and its licence notice with the data. The original Rust
reader does not use Grammalecte grammar code. The real-data gate currently reads
274,427 finite analyses, kept separate from the new infinitive/canonical-participle index;
nouns and imperatives are not retained as verbal readings. Canonical participles are masculine
singular or invariant forms, not guessed agreement variants. File size, line length, retained
row count and index fan-out are bounded across finite and nonfinite records. The public finite
analysis count and installer readiness remain unchanged.
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
cargo test --locked -p wordcraft french_dictionary -- --include-ignored
cargo test --locked -p wordcraft-ui-egui --test french_dictionary_setup
```

This explicit gate fails if the dictionary is absent, unreadable, unexpectedly small, or found
outside the requested folder. It checks real words, accents, elisions, suggestions and the
fr-FR/fr-CA dispatch. A separate process tests unreadable-dictionary diagnostics and recovery,
so synthetic unit tests cannot replace the dictionary being checked.

The **French proofing** workflow runs these dictionary and review-command checks on Windows,
macOS and Linux. It downloads a pinned LibreOffice revision of Dicollecte 7.7 and verifies all three
SHA-256 hashes (including the dictionary readme). It also checks atomic installation, interrupted
downloads, existing dictionaries, retry, immediate activation and preference persistence. The
existing UI workflow and `cargo xtask ci` remain required validation;
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
| 2026-10-20 | minor | Attached Unicode marks keep their original spelling ranges; French compound fallback rejects empty internal components while preserving explicit dictionary entries |
| 2026-10-20 | minor | French proofing reads Canada only from the region after the language/optional script; Unicode calendar and private-use extension subtags do not select Quebec typography; production-parser regressions |
| 2026-10-20 | minor | French dictionary replacement clears stale load errors only after successful parsing; Canadian French keeps its own Options label |
| 2026-10-20 | trivial | French hyphenation pattern asset included in both workflow path filters so pattern-only changes run the proofing regressions |
| 2026-10-20 | minor | Frozen pre-French affix outputs and legacy compact dictionaries agree; French CI pins Rust 1.98.1 and checks formatting on all three desktop systems |
| 2026-10-20 | minor | French irregular participles: five avoir tenses, 66 original fallback pairs, bounded optional lexical coverage and separate prose smoke gate; no general agreement claim |
| 2026-10-20 | minor | Explicit desktop French verb-form data installation: pinned MPL-2.0 archive, bounded ZIP members, atomic activation, retained manual files, shared installation interlock and live reload; no general-grammar or readiness-estimate claim |
| 2026-10-20 | minor | Personal-cleft agreement, ambiguity-aware optional MPL-2.0 finite morphology, bounded reader, fallback/reload diagnostics and explicit pinned real-data coverage |
| 2026-10-20 | minor | Bounded avoir + first-group participle suggestions, si il/ils elision, original error/quiet/unsupported regressions and command-level apply/undo tests |
| 2026-10-20 | minor | Explicit desktop download button, verified atomic installation, live activation and saved folder; existing dictionaries retained |
| 2026-10-20 | major | Integration checklist, explicit real-dictionary gate, feedback format and staged French follow-ups |

A successful call to the public French dictionary installation API clears an earlier
load diagnostic; an invalid replacement preserves the current dictionary and diagnostic.
The regression exercises unreadable disk data, a failed replacement, a valid replacement
and a second failed replacement. Options labels keep France and Canada distinct.

## Combining marks and French compound boundaries

The proofing tokenizer keeps a Unicode Mark attached to its preceding letter or
continuing mark sequence. It does not attach an orphan mark after punctuation or
digits. This avoids splitting a decomposed spelling such as `e` + U+0301 + `cole`
before the dictionary can apply its existing input conversions. The original
UTF-8 text and issue offsets are retained; the document is not normalized or
rewritten. See [Unicode canonical equivalence](https://unicode.org/reports/tr15/#Canonical_And_Compatibility_Equivalence).

French punctuation trimming also retains a final attached mark, as in `cafe` +
U+0301. Recognition still depends on the installed dictionary and its input
conversions; this change does not add a general normalization engine.

The French compound fallback requires every internal hyphen-separated component
to be nonempty. `porte--monnaie` and `va--t-il` can therefore be reported instead
of bypassing spelling checks. Existing valid compounds and inversions remain
accepted. Explicit full-word dictionary entries and user-dictionary choices are
checked before this fallback and retain priority.

Process-isolated synthetic regressions cover original byte ranges, both French
regions, orphan marks, another dictionary-backed language and explicit user
entries. The existing required real French dictionary gate covers both forms of
accented words and malformed versus valid compounds with pinned Dicollecte data.
