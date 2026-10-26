# French grammatical architecture: bounded morphology and agreement

> **Last reviewed:** 2026-10-26 · **Last updated:** 2026-10-26 · **Change:** major (first isolated morphological analysis and explicit-subject agreement increment)

## Current implementation

The spelling dictionary answers whether a form exists. It is not a grammatical tagger:
the pinned French Hunspell input has no morphological fields, and the existing affix
reader ignores such fields. This increment does not reinterpret dictionary flags.

The new French-only pipeline separates four responsibilities:

1. Original closed lexical paradigms generate inflected forms and their readings.
2. A surface lookup retains every supported lemma/person/mood/tense reading.
3. An explicit subject frame supplies a required grammatical person.
4. A repair is offered only when no applicable reading agrees and every applicable
   reading generates exactly the same replacement.

The seed covers 27 explicitly named first-group verbs and original complete simple
paradigms for être, avoir and suivre, with seven indicative,
conditional and subjunctive paradigms, and the three imperative persons. Soft g/c
spelling changes are generated for manger/ranger and commencer. Imperative readings
remain in morphological analysis; the explicit subject frame excludes them. Past-participle
readings are retained separately; a finite/participle collision such as eus or suivis
causes abstention. The shared suis surface retains both être and suivre. These
are independently authored lexical facts and generative rules, not third-party
source code or a bundled grammatical dictionary.

The lexical index is built once from a fixed, bounded input. Unknown forms receive
no guessed lemma. Case-sensitive composed lexical copies are looked up without
rewriting the source. Issue ranges and replacements use the original UTF-8 bounds.

## First consumer and abstentions

Only a real paragraph-initial je/tu/il/elle/on/ils/elles followed immediately by
a known finite form is considered, together with initial j'/j’ and a known vowel
form. Adjacent horizontal spaces are allowed; punctuation, quotations and line
breaks are barriers. The subject and verb must be in the same complete French
proofing-language region. A formatting boundary of the same region may merge.
A French run inside another paragraph is not a new paragraph or subject frame.

Nous/vous are deferred: even initial ones can be objects before an inverted subject.
Intervening clitics, relative clauses, coordinated subjects, noun/adjective agreement,
compound tenses and unlisted/alternating verbs are outside this first consumer.
This is not a complete French parser or a claim of complete lexical ambiguity coverage.

A compatible reading means silence. Different possible targets mean abstention:
for example an imperfect and a present-subjunctive reading may require different
repairs. The rule does not choose a tense by frequency or change suggestion ranking.
Spelling and previous French rules are preserved. No automatic whole-document
correction is applied.

## Evaluation and the 59,048 observations

The historical comparison contains 49,048 synthetic variants and 10,000 source
controls. It is silver data; sources are not certified correct and variants are
not all independently certified mistakes. Word object-model observations do not
expose the complete Editor interface.

An exploratory JavaScript model with only the 27 first-group lemmas found zero
candidates in all 59,048 inputs. Of 5,238 initial unambiguous-pronoun inputs, only
156 had one of those finite verbs and all 156 were compatible. Adding the three
irregular paradigms produced 24 exploratory candidates before adjudication.
These are not Rust results, certified improvements or a blind accuracy score.
The corpus exposes a further ambiguity: Il ont can reflect a verb error or a
missing s in Ils. The first consumer assumes that the explicit subject is the
intended one; its finite-form repair need not restore the intended source.
Corrections are suggestions only, and these competing hypotheses must be audited
before any production integration. Do not widen the rule to force a positive number.

Validation uses original grammatical errors, paired controls, ambiguity and scope
abstentions, both French regions, mixed-language boundaries, original NFC/NFD bounds,
and the actual review-command correction/undo path. An independently authored
50-case challenge was frozen before its author's source review. Once inspected it
is exposed development evidence, not a blind accuracy score. Historical observations
remain unchanged and separately attributed to their actually measured commits.

A useful future evaluation must report detection at the erroneous span, correct
suggestions, newly introduced alerts on adjudicated controls, abstentions and coverage.
Measure runtime on the same host before drawing performance conclusions. The existing
top-suggestion ranking gap and entity/foreign-language alerts are separate projects;
this grammatical increment does not claim to solve them.

## Architecture direction

Later consumers can reuse the lexical readings for other proven subject frames and
agreement constraints. Adding lexical coverage requires a documented, independently
reviewed data source or original entries, explicit paradigms and homograph coverage.
Adding syntactic coverage requires correct counterexamples and region/boundary tests.
Do not add suffix guesses or erase ambiguous readings to inflate coverage.

Public descriptions consulted, without copying implementation:

- [OQLF: identify the subject before agreeing the verb](https://vitrinelinguistique.oqlf.gouv.qc.ca/25319/la-syntaxe/le-sujet-dans-la-phrase/reperage-du-sujet-pour-eviter-les-erreurs-daccord-du-verbe)
- [LanguageTool: contextual disambiguation](https://dev.languagetool.org/developing-a-disambiguator.html)
- [LanguageTool: agreement features](https://dev.languagetool.org/using-unification.html)
- [Grammalecte: public description](https://www.grammalecte.net/?lang=fr_fr)
- [Académie: être](https://www.dictionnaire-academie.fr/conjuguer/A9E2986),
  [avoir](https://www.dictionnaire-academie.fr/conjuguer/A9A3461),
  [suivre](https://www.dictionnaire-academie.fr/conjuguer/A9S3347)

## Validation status

Local cargo xtask ci was attempted and could not run because cargo is absent.
Hosted formatting, proofing, real-dictionary, engine and full workspace checks must
be linked to the exact published head before this increment is called validated.
A successful Sonar workflow without a token is not a completed analysis or quality gate.

## Revision history

| Date | Change | Summary |
|---|---|---|
| 2026-10-26 | major | Separate original bounded verb paradigms, ambiguous readings and explicit-subject agreement; preserve language regions, original ranges and independent challenge evidence |
