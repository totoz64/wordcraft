# Shipping and improving French proofing

> **Last reviewed:** 2026-10-20 · **Last updated:** 2026-10-22 · **Change:** minor (bounded expanded French subjects and recap coordination); previously minor (French nominal subjects with one non-reflexive clitic); previously minor (bounded French negative common-noun subjects); previously minor (bounded French nominal agreement and common-noun subjects); previously minor (explicit French word mentions and local protection); previously minor (French local grammar respects language boundaries); previously minor (bounded French grammar deciding-language context); previously minor (French spelling keeps attached combining marks and checks compound boundaries); previously minor (French proofing region parser respects extension boundaries); previously minor (French dictionary recovery and regional labels); previously trivial (French hyphenation CI path coverage); previously minor (historical affix compatibility gates and pinned French CI toolchain); previously major (French integration and regression gates, based on #1660); previously minor (reviewed French baseline: dictionary recovery, regional labels, historical compatibility, pinned CI and French pattern triggers); previously minor (desktop dictionary installation and activation; baseline based on #1660); previously minor (first bounded conjugation checks and si elision; baseline based on #1660); previously minor (reviewed French baseline: dictionary recovery, regional labels, historical compatibility, pinned CI and French pattern triggers); previously minor (dictionary installation, personal-cleft agreement and optional finite morphology; baseline based on #1660); previously minor (personal-cleft agreement and optional finite morphology; baseline based on #1660); previously minor (reviewed French baseline: dictionary recovery, regional labels, historical compatibility, pinned CI and French pattern triggers); previously minor (explicit French verb-form data installation; baseline #1660); previously minor (French irregular participles and five avoir tenses; baseline #1660); previously minor (French negative/adverb participle contexts; baseline #1660); previously minor (French personal-subject agreement; baseline #1660); previously trivial (preserve explicit written-pronoun examples); previously trivial (French suggestion priority after language filtering); previously minor (French reflexive auxiliary correction); previously minor (reviewed French baseline: dictionary recovery, regional labels, historical compatibility, pinned CI and French pattern triggers); previously minor (French agreement through object clitics; baseline #1660)

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

Subject–verb agreement is limited to the initial personal-subject and personal-cleft cases
below, plus simple determiner-led common-noun subjects at clear sentence starts. A bounded
nominal rule also suggests some determiner/noun/adjective agreements. These additions do not
analyse arbitrary later clauses, coordinated subjects, general gender/number agreement or
contextual homophones. No automatic document-wide correction is applied.

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

Explicit negative auxiliaries are supported too: `Je n’ai pas écrire` → `Je n’ai pas écrit`,
`Nous n’avions plus jamais lire` → `Nous n’avions plus jamais lu`, including straight apostrophes.
Negation requires a matched `n’` + auxiliary and `pas`, `plus`, `jamais`, `guère` or `point`;
the additional combinations `plus jamais`, `jamais plus`, `pas plus`, `guère plus` and
`pas du tout` are explicit. This follows the [OQLF placement of negation around an auxiliary](https://vitrinelinguistique.oqlf.gouv.qc.ca/23909/la-syntaxe/la-negation-et-la-restriction/place-du-verbe-avec-ne-pas-et-ne-pas).
Negation is not added, removed or otherwise rewritten.

A short explicit list also allows `J’ai déjà écrire` → `J’ai déjà écrit`: `aussi`, `assez`,
`beaucoup`, `bien`, `bientôt`, `déjà`, `encore`, `finalement`, `mal`, `mieux`, `peu`, `presque`,
`probablement`, `rarement`, `seulement`, `souvent`, `toujours`, `trop`, `vraiment`. At most
three intervening **words**, counting negative words and `du tout`, are accepted; every gap
must be horizontal whitespace. The rule never skips an existing participle, an object clitic,
`à`, restrictive `que` or `beau`. Thus `J’ai déjà fait relire`, `J’ai beau écrire`,
`Je n’ai pas à écrire` and `Je les avais déjà écrites` remain unmarked by this check.

There are 66 original fallback infinitive/participle pairs, including irregular verbs. Optional
data supplies additional recorded forms; no arbitrary ending is transformed. A valid recorded
participle reading stays quiet. Verbs requiring être, unknown auxiliary choices and unavailable
forms are declined. Data indicating a transitive active use permits avoir, following the
[OQLF auxiliary table](https://vitrinelinguistique.oqlf.gouv.qc.ca/24584/la-grammaire/le-verbe/auxiliaires/tableau-synthese-de-lemploi-des-auxiliaires-avoir-et-etre).
For mixed/uncertain transitive–intransitive uses, an adjacent object determiner is required;
impersonal verbs require explicit `il`. Bare `pouvoir`/`devoir` can be nouns, so those two
modal readings additionally require an adjacent known infinitive complement.

`Je vais manger`, `J’ai entendu chanter`, preceding objects/clitics, omitted/bare negation, unknown or excessive
intervening words, quotations, code, tables, legacy `--` dialogue prefixes and detected metalinguistic
examples remain outside this rule. It does not choose an auxiliary or resolve participle
agreement. The [OQLF distinction between infinitive and participle](https://vitrinelinguistique.oqlf.gouv.qc.ca/21614/la-grammaire/le-verbe/modes/infinitif/distinction-entre-linfinitif-et-le-participe-passe)
explains why replacing every infinitive after a verb would be incorrect.

`french_grammar.rs`, `french_participles.rs` and `french_negative_participles.rs` separate original errors, correct text and
ambiguous/unsupported constructions. Unsupported errors do not count as correct-text precision
data. Every targeted error checks exact byte ranges and suggestions, then verifies stable
corrected text. An additional table checks 3,300 subject/tense/verb combinations and their
3,300 corrected counterparts. The explicit real-data test exercises the whole grammar path for
6,057 supported simple infinitives in three defined first-person contexts (positive, explicit
negative and intervening adverb: 18,171 constructions; an infinitive complement is supplied for
the ambiguous modals), including canonical suggested forms and stable results.
These are synthetic/data coverage checks, not a real-world precision estimate.

An optional `french_prose` test uses external French prose, never bundled in this repository.
The recorded local run used 9,486 original paragraphs from four books, with 51 auxiliary
beginnings, plus 2,889 separately labelled variants replacing only a leading `--` by `— `,
with 60 auxiliary beginnings. There were 99 unquoted auxiliary beginnings and no new participle
alerts. Counts include overlapping variants and are a smoke check, not statistical precision
or broad grammar coverage. See [source metadata and normalization](french-prose-sources.json).
Input format is UTF-8 `source<TAB>paragraph` per line, bounded to 8 MB; set
`WORDCRAFT_FRENCH_PROSE` and `WORDCRAFT_FRENCH_LEXICON`, then run:

```sh
cargo test --locked -p wordcraft-proof --test french_prose -- --ignored --nocapture
```

Engine integration exercises F7, Change, Undo and **Mark grammar errors as you type** off/on.
It also fixes `Je avais déjà écrire` in two steps, verifying recomputed UTF-8 ranges after elision
and two undos restoring the original text. Suggestions never rewrite the document automatically.

## Simple nominal groups and common-noun subjects

Both French regions use the same optional external lexical file for these additions. An
existing installation through **Add French verb forms** is reused: there is no new download,
button, startup request or bundled data. The original reader retains a separate bounded
common-noun/adjective index with lemma, gender and number. `nom adj` combinations, epicene
genders and invariant numbers retain all their readings; an unknown reading stays undecided.
Verb-tagged participial adjectives are outside this initial nominal index. The pinned file's
274,427 finite analyses remain a separate measure; an independent index count is 166,897
nominal analyses, not a count of rules, corrected errors or grammar accuracy.

The parser accepts a determiner plus a known common noun, optionally with one adjective before
or after it, at the paragraph beginning or a clear later sentence beginning. Abbreviations,
ellipses, dialogue, quotations, code and uncertain boundaries stay outside its scope. It does
not skip arbitrary clitics, coordination, relative clauses or intervening complements. Subject agreement
also admits the explicit bounded negative pattern and single-clitic construction described below. This
bounded use follows the OQLF descriptions of the [noun and nominal group](https://vitrinelinguistique.oqlf.gouv.qc.ca/24321/la-grammaire/la-grammaire-actuelle/les-classes-de-mots-et-les-groupes/le-nom-et-le-groupe-nominal)
and [agreement within a nominal group](https://www.oqlf.gouv.qc.ca/francisation/ordres_prof/capsules-grammaticales/accord-groupe-nominal.aspx).
Examples here are original applications of these principles, not borrowed grammar code.

For nominal agreement:

- `Les petite maisons` can suggest `petites`, and `Une petit maison` can suggest `petite`.
  The determiner and head noun must already share a valid gender/number reading before an
  adjective correction is offered. Suggestions come from recorded forms of the same lemma
  and part of speech; there is no guessed ending. Any compatible current reading stays quiet.
- `Les maison` can suggest `maisons` while retaining the noun's gender. Here the written
  determiner supplies number: this is one possible repair, not a claim to know whether the
  writer intended a singular or plural group. Suggestions never change text automatically.
- `Un maison` can suggest `Une`, and `La jardin` can suggest `Le`, only for supported fixed
  singular determiner paradigms and a uniquely recorded noun gender/number. Generating an
  elided article, guessing an unfamiliar h, or changing a possessive such as `mon amie` is
  declined. The `ce/cet/cette` alternatives use the immediate following word's sound,
  including a preposed adjective, with the existing explicit onset safeguards.

Compound colours (`Les robes bleu clair`, `Les robes bleu Zora`), positional/adverbial
adjectives such as `demi`, `nu` and `feu`, variable-gender `gens`/`amours`/`délices`/`orgues`,
unknown forms, unmatched group parses and ambiguous punctuation are deliberately declined.
This is not general noun-phrase, participle or adjective analysis.
Multiple recorded adjective alternatives and generated masculine-singular positional forms
such as `beau/bel`, `vieux/vieil`, `nouveau/nouvel`, `fou/fol` and `mou/mol` are declined.
The rule does not infer these position-dependent spellings, even from an incomplete custom
file; supported unambiguous suggestions such as `Une beau maison` → `belle` remain available.

An already compatible noun followed by a possible finite verb keeps its short-subject
interpretation (`La vieille dit bonjour`). Unparsed bare noun/noun continuations stay quiet.
With plural determiners, the five explicit forms `cent`, `mil`, `mille`, `neuf` and `zéro`
retain their possible cardinal reading (`Les neuf petites`, `Ces mille maisons`), rather
than being inflected as ordinary nouns or adjectives.

For subject agreement, `Les enfants joue` can suggest `jouent`. The determiner and head noun
must agree on one number, with at most one recorded, number-compatible adjective. The finite
verb must immediately follow that simple group or its bounded negative prefix below.
Suggestions preserve a recorded lemma and
possible tense/mood; they do not select an intended tense. A valid finite reading or a matching
nominal/adjectival interpretation remains unmarked, so labels such as `Les enfants guides.`
are not forced into clauses. When an otherwise mismatching finite form also has a nonverbal
reading, only a bare verb or a complete tail of at most eight words from an original explicit
11-adverb list is admitted: `dehors`, `ici`, `là`, `hier`, `demain`, `bientôt`, `souvent`,
`toujours`, `rarement`, `vite`, `encore`. An unfamiliar complement stays quiet. This narrow
exception supports `Les enfants joue dehors` without interpreting `Les voitures modèle sport`
as a verb error; it is not a general part-of-speech parser.
A simple nominal subject can also be followed by `ne` or an ASCII/typographic
`n'`/`n’`, then one recorded finite verb and immediately `pas`, `plus`, `jamais`,
`guère` or `point`. For example, `Les enfants ne joue pas` can suggest `ne jouent`,
replacing the complete `ne joue` span. Each suggestion renders the negative prefix
with the recorded replacement, so `n’est` can become `ne sont` and conversely.
The pattern follows the OQLF descriptions of [two-part negation](https://vitrinelinguistique.oqlf.gouv.qc.ca/23905/la-syntaxe/la-negation-et-la-restriction/generalites-sur-la-negation)
and [obligatory elision](https://vitrinelinguistique.oqlf.gouv.qc.ca/21737/lorthographe/elision-et-apostrophe/elision-obligatoire). The exact
replacement and all deciding words/gaps stay within one French region. Compatible
finite readings remain unmarked; nominal ambiguity guards are retained.
After its complete French context is accepted, a negative nominal-subject correction
also supersedes covered spelling and conflicting elision suggestions only inside
the replaced prefix. Thus `Les enfants ne est pas ici` offers `ne sont` directly.
If a foreign subject or marker rejects that correction, an independently French
`ne est` still retains its local elision and spelling diagnostics. Existing
bare-finite nominal priority and personal-subject context reconstruction are unchanged.

This addition does not infer a missing `ne`, insert a missing second negative word,
skip object/reflexive clitics or intervening adverbs, or parse `ne ... que`,
`ni ... ni` or a general negative clause. Combined following markers such as
`pas plus` and `plus jamais` stay outside this addition. Homographs retain the
complete bounded adverb-tail requirement, so `Les enfants n’est pas contents`
remains unsupported with the pinned lexical data; the noun reading of `a` also
keeps `Les enfants n’a jamais raison` undecided. Other following complements are
not interpreted. It does not choose an intended
tense or analyse participle agreement. Self-contained proof/Review regressions and
the separate pinned `french_nominal_negative_real` gate describe these boundaries;
their existence does not establish native, hosted or release validation.

### Three bounded subject extensions

The contribution stack prepares three additional constructions, using the existing
optional MPL morphology. This describes code scope; it does not establish hosted,
native or official-release validation.

- A plural nominal head can control a verb through one short determined complement:
  `Les enfants de la voisine parlait` suggests `parlaient`. Only `du`, `des`, or `de`
  with a supported determiner/`l’` and one number-compatible recorded noun are admitted.
  Singular heads are declined because the lexicon does not classify collective or
  quantity meanings. Bare `de` plus a noun, nested groups and complement modifiers
  are outside this path. Existing word-mention guards protect the head; the new complement path also
  declines explicit word/title labels such as `terme`, `nom` and `titre`, whose
  following finite-looking word may be an unquoted mention. This bounded label
  class does not classify arbitrary meaning or recover absent style metadata. The source verb must have no nonverbal reading; a bounded
  complete adverb-only tail is required and included in the deciding French context.
- After a simple nominal subject, exactly `lui en` or `leur en` can bridge to the
  finite form, also inside the existing explicit negative pattern. Reuse the typed
  clitic order/person checks. Other pairs, direct-object/reflexive chains and
  participle agreement are outside this increment.
- Two personal subjects coordinated with `et` can precede a comma and an explicit
  compatible recap pronoun: `Toi et moi, nous parlait` suggests `parlions`.
  An unrecapped phrase remains undecided because it may be a title. The recap must
  agree with the grammatical person of the coordinated subjects. This path admits
  only an immediate pure finite form, with no clitics, negation or auxiliary analysis.
  All coordinated words, the comma/gaps, recap and verb are deciding dependencies.

These bounded constructions are separate families, not general French sentence
analysis. The `french_structured_subjects_real` gate separates expected errors from
correct/ambiguous/unsupported controls. `french_structured_subjects_review` checks
actual Review, marks, Apply, Undo and language preservation. See the
[family catalogue](french-grammar-families.md) for source references and future scope.

### One non-reflexive clitic after a simple nominal subject

The same subject may precede exactly one `lui`, `leur`, `y` or `en`, then a recorded
finite form: `Les enfants lui parlait` suggests `lui parlaient`. The existing explicit
negative bridge also admits this clitic: `Les enfants n’y allait pas` suggests `n’y allaient`.
Negative elision follows the clitic (`ne lui`, `ne leur`, `n’y`, `n’en`), preserving
all recorded finite alternatives and the final negative marker. The full subject,
clitic, gaps and any negative marker must belong to one French region before the
replacement takes precedence over covered spelling or conflicting elision.

This path requires the source finite form to have no nonverbal reading in the installed
lexicon. Thus `Les robes en voile`, `Le mur en briques` and possessive `leur` fragments
remain undecided; the check does not infer a verb merely from a possible conjugation.
Other clitic forms (`le`, `la`, `les`, `l’`), reflexive constructions, chains,
arbitrary intervening words and general
negative-clause parsing are outside this addition. An immediately adjacent bare
negative marker does not establish the explicit negative bridge. Unparsed tails
are not a claim to recognise all colloquial omissions of `ne`.

The isolated `french_nominal_clitic_review` integration test exercises actual Review,
Apply, Undo, marks, options and language preservation. The pinned
`french_nominal_clitic_real` gate now includes the two admitted chains as well: 22 independently listed errors,
16 correct examples, 30 unsupported/ambiguous examples, 1,484 distinct generated erroneous
constructions and 96 generated homograph declines, each in both French regions.
The construction sweep samples eight admissible third-person singular/plural pairs
per recorded tense/mood class across seven classes; identical constructions are
deduplicated. Its oracle is lexical self-consistency, not independently measured
linguistic accuracy, clitic valency or parity with Word. Independently expected
repairs and the stability of every offered alternative are checked separately.

See the [family catalogue](french-grammar-families.md) for the structured development
strategy and [OQLF description of en](https://vitrinelinguistique.oqlf.gouv.qc.ca/23513/la-grammaire/les-pronoms/pronoms-personnels/le-pronom-personnel-en)
for its pronominal uses. No new linguistic download or bundled data is introduced.

The four common nominal epithets `type`, `modèle`, `pilote` and `cadre`, including their
plurals, do not license a verb correction. This preserves an ambiguous nominal reading even
when its own written agreement is wrong; it is not a claim that every declined label is
correct French. Primary descriptions: [Académie, type](https://www.dictionnaire-academie.fr/article/A9T2786),
[Académie, modèle](https://www.dictionnaire-academie.fr/article/A9M2435),
[Clés de la rédaction, pilote](https://nos-langues.canada.ca/fr/cles-de-la-redaction/pilote),
and [Juridictionnaire, cadre](https://nos-langues.canada.ca/fr/juridictionnaire/cadre).

Each suggestion carries the actual determiner/head/modifier or subject/verb context it uses,
including any deciding homograph-tail adverbs. A postposed adjective with a following word
also needs that word's finite reading; it and the intervening gap belong to the deciding
context. These words must all belong to one French region
before the suggestion is accepted. Adjacent runs in the same region merge; French/English,
uncovered and fr-FR/fr-CA boundaries cannot supply deciding words. Original UTF-8 text and byte
ranges remain intact, unrelated errors remain available, and disabling grammar leaves spelling
behavior intact. Regression requirements cover synthetic ambiguous/quiet cases, the pinned real
file, both French regions and Review/Change/Undo. These requirements do not establish native,
three-system, maintainer, merge or release validation; those results must name their source.

## Direct personal-subject agreement

At a paragraph's beginning, `je`, `tu`, `il`, `elle`, `on`, `nous`, `vous`, `ils` and `elles`
can suggest same-lemma finite forms agreeing with the pronoun: `Nous manges` can offer
`mangeons` and `mangions`, `Vous a` offers `avez`, and `Je est` offers `suis`.
The pronouns' [grammatical persons](https://vitrinelinguistique.oqlf.gouv.qc.ca/fiche-gdt/fiche/26558057/pronom-personnel)
determine the person/number; polite singular vous still takes the plural verb, as the
[Académie explains](https://www.dictionnaire-academie.fr/article/QDL059).
An optional separated dialogue dash and immediate `ne` or contracted `n’`/`n'` are supported.
Fused `J’est`/`J'est` becomes `Je suis`; `Je n’est` becomes `Je ne suis`.

Object/reflexive clitics can now intervene: `Je les voit` → `Je les vois`,
`Nous lui dis` → `Nous lui disons`, `Il se demandent` → `Il se demande`.
The bounded prefix accepts at most three clitics in increasing pre-verbal groups:
me/te/se/nous/vous → le/la/les → lui/leur → y → en, with ne/n’ before them.
Same-group repetitions, reverse order, excessive prefixes and punctuation/line breaks are
declined. This is finite agreement before the verb, not imperative/post-verbal reordering.
See the [OQLF pronominal complement table](https://vitrinelinguistique.oqlf.gouv.qc.ca/24134/la-grammaire/les-pronoms/pronoms-personnels/la-reprise-ou-lomission-du-pronom-personnel-complement)
and its [pre-verbal/negative examples and elisions](https://vitrinelinguistique.oqlf.gouv.qc.ca/24206/la-grammaire/les-pronoms/pronoms-personnels/pronoms-personnels-employes-avec-un-verbe-a-limperatif).

Apostrophe-linked prefixes, straight or curly, are parsed too: `Je n’en voit` → `Je n’en vois`,
`J’y vont` → `J’y vais`, `Je m’est préparé` → `Je me suis préparé` and
`Je le avons lu` → `Je l’ai lu`. Suggestions rebuild the bounded prefix with every object
and ne/n’ retained, rather than dropping them or producing J’l’ai / Je le ai. New grammatical
elisions use the existing curly-apostrophe convention. An elided l’ has lost le/la gender;
when correction would require uneliding it, the rule declines instead of guessing. The
existing unknown-h policy is retained when an elision decision needs pronunciation data.

Se must match a third-person subject. Reflexive clitics are recognized by their person;
a person-only correction of avoir in that construction is declined, since
[pronominal compound verbs require être](https://vitrinelinguistique.oqlf.gouv.qc.ca/22939/la-grammaire/le-verbe/verbes-pronominaux/emploi-de-la-forme-pronominale).
The following bounded extension selects être only with an independently recorded
participle of a supported common pronominal lemma; missing/wrong clitic persons remain unsupported.
Personal me/te/se/nous/vous/lui establish a finite context; determiner-like le/la/les/leur
and en/y retain the nominal ambiguity guards. `Nous les guides de montagne…` and
`Nous en juges impartiaux…` therefore remain quiet.

The person correction leaves following participles unchanged: `Je les ont confiés` →
`Je les ai confiés`, `Nous les as offertes` → `Nous les avons offertes`.
The separate infinitive/participle rule is not extended past preposed objects, whose
gender/number agreement needs additional analysis. A later infinitive after a valid finite
verb, such as `Je veux vous remercier`, is not reinterpreted as this prefix.

The existing four original present paradigms work without optional data. With the data,
all supported finite tenses/moods remain available. Any valid person reading stays quiet:
`Tu suis cette piste` and `Tu sommes le débiteur de payer` retain their suivre/sommer readings.
The rule preserves overlapping moods and can offer several alternatives; it does not infer
the intended tense/mood from the whole sentence or turn an infinitive into a guessed tense.

For this personal-subject rule, nonverbal rows and adjectival-participle readings remain
bounded homograph facts separate from the finite index. The newer nominal rules also retain
common-noun/adjective morphology in their own index. They do not increase the public 274,427 finite-analysis
count. Positive plural subjects followed by ambiguous non-auxiliary forms are declined:
`Nous guides de montagne accompagnons ce groupe`, `Vous élèves de ce collège savez la réponse`
and `Nous abasourdis restons sans voix` must not become guidons, élevez or abasourdissons.
For an être/avoir homograph, a bounded following context protects nominal appositions such as
`Nous as du volant sommes prudents`; direct auxiliary errors such as `Vous a raison` remain
correctable. This is not general syntactic or semantic disambiguation and can miss errors
whose forms have a nominal/participial reading.

Agreement takes precedence over a conflicting apostrophe-only fix, so `Je est` never offers
the invalid `J’est`. When necessary the suggestion replaces je plus the verb as one range:
`Je avons` → `J’ai`. Negative suggestions choose ne/n’ consistently. Unknown h pronunciation
is declined when generating je/ne elision, using the existing explicit mute-h table rather
than guessing. Unrelated typography/elision issues are retained.

With grammar enabled, a known French person correction supersedes only covered French spelling
marks for that same contraction. Thus real-dictionary `J’est` has one useful grammatical
suggestion rather than duplicate spelling/grammar guesses. Spelling-only mode still reports
the misspelling, other misspelled words remain, and a whole-subject replacement cannot cross
a proofing-language boundary. Quotations, code, tables, metalinguistic examples, intervening
adverbs, invalid/excessive clitic prefixes and later clauses remain outside the new rule.

Independent synthetic and pinned-data tests cover exact ranges, six persons, all seven
finite tense/mood categories, negative/fused contractions, nominal and participial appositions,
unrelated errors, language boundaries and stable suggestions in both French regions. Unsupported
errors and ambiguous fragments are kept separate from correct-text evidence. The external prose
gate additionally observes 1,247 known finite personal-subject beginnings, including 1,064 unquoted
beginnings (375 clitic beginnings, 316 unquoted), with no new subject/participle alerts across the same four books. These are prefix
counts, include overlapping dialogue variants, and are not a precision estimate.

F7/Change/Undo and grammar marks off/on use the same commands as the UI. The engine regression
fixes `Nous a déjà écrire` in two stages: a → avons shifts the later byte range and enables
the participle check, then écrire → écrit; two undos restore the initial issue. The existing
elision → participle sequence remains covered.
The clitic regression also fixes ASCII `s'es` → curly `s’est` in `Il s'es trompé:`:
the added UTF-8 bytes move the later colon; F7 must then select its new range for the
French non-breaking-space correction, in both regions. New pinned-data cases verify
ordered/contracted clitics, retained participles, negative prefixes and stable alternatives.

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
row count and index fan-out are bounded across finite, nonfinite and separate nominal records;
nominal feature lists exceeding 32 entries are rejected rather than silently truncated. The
separate nonverbal-form set is bounded too and keeps only finite homographs after parsing. The public finite
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
cargo test --locked -p wordcraft-engine --test french_nominal_clitic_review
cargo test --locked -p wordcraft-engine --test french_structured_subjects_review
cargo test --locked -p wordcraft-ui-egui --lib a_french_interface_writes_new_documents_in_french
cargo xtask ci
```

For the real dictionary, set `WORDCRAFT_DICTIONARIES` to its folder, then run:

```sh
cargo test --locked -p wordcraft-proof --test french_dictionary -- --ignored
cargo test --locked -p wordcraft-proof --test french_nominal_real -- --ignored
cargo test --locked -p wordcraft-proof --test french_nominal_negative_real -- --ignored
cargo test --locked -p wordcraft-proof --test french_nominal_clitic_real -- --ignored --nocapture
cargo test --locked -p wordcraft-proof --test french_structured_subjects_real -- --ignored --nocapture
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
| 4 | Expand beyond the bounded nominal/subject rules and add contextual homophones | A documented data/licence choice, positive and quiet cases, bounded runtime, and false-positive measurements before each rule is enabled |

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
| 2026-10-22 | minor | Bounded plural subjects with one determined complement, nominal lui en/leur en chains, and explicitly recapped personal coordination; retain lexical ambiguity, complete same-region dependencies and separate delivery states |
| 2026-10-22 | minor | French simple nominal subjects through one non-reflexive lui/leur/y/en; recorded paradigms, conservative nonverbal guards, exact language context and Review/Undo regressions; family catalogue and bounded generated lexical checks |
| 2026-10-21 | minor | Bounded French nominal subjects through explicit ne/n’/n' + finite verb + pas/plus/jamais/guère/point; recorded forms and minimal elision repair retain exact same-region dependencies; clitics, omitted ne and general negative-clause parsing remain unsupported |
| 2026-10-21 | minor | Bounded French determiner/noun/adjective agreement and simple common-noun subjects at clear sentence starts; reuse optional licensed data, preserve ambiguous readings and exact language contexts; no general-grammar or readiness-estimate claim |
| 2026-10-21 | minor | Explicit French un/le/ce mot mentions retain their written pronoun; the validated same-region marker protects only the initial copula, leaving unrelated local elisions available; existing broad candidate guards and general-grammar limits remain unchanged |
| 2026-10-20 | minor | French elision, repeated-word and quoted-text replacements require their own complete language context; local French checks after a foreign prefix remain available; original UTF-8 text and broader grammar limits retained |
| 2026-10-20 | minor | Require one French-region deciding context for existing personal-subject, personal-cleft and avoir suggestions; retain unrelated foreign tails, original UTF-8 ranges and independent spelling/elision precedence; no general-grammar claim |
| 2026-10-20 | minor | Attached Unicode marks keep their original spelling ranges; French compound fallback rejects empty internal components while preserving explicit dictionary entries |
| 2026-10-20 | minor | French proofing reads Canada only from the region after the language/optional script; Unicode calendar and private-use extension subtags do not select Quebec typography; production-parser regressions |
| 2026-10-20 | trivial | Explicit written-pronoun contexts retain their text: third-person copulas with pronoun labels or typographic cues no longer receive personal agreement or apostrophe corrections; ordinary prose remains checked |
| 2026-10-20 | trivial | French elision conflicts are resolved only after grammatical context passes exact-language filtering; a rejected reflexive auxiliary retains independent spelling and typography |
| 2026-10-20 | minor | French reflexive auxiliary correction with recorded participles, bounded clitics/negation, exact-language context and retained participle endings; no general participle agreement |
| 2026-10-20 | minor | French dictionary replacement clears stale load errors only after successful parsing; Canadian French keeps its own Options label |
| 2026-10-20 | trivial | French hyphenation pattern asset included in both workflow path filters so pattern-only changes run the proofing regressions |
| 2026-10-20 | minor | Frozen pre-French affix outputs and legacy compact dictionaries agree; French CI pins Rust 1.98.1 and checks formatting on all three desktop systems |
| 2026-10-20 | minor | French initial finite agreement through bounded ordered clitics: retained objects/negation/elision, reflexive-avoir and unknown-gender guards, preposed participles untouched, real-data/prose and UTF-8 Review regressions |
| 2026-10-20 | minor | French initial personal-subject agreement: preserved finite/nominal/participial readings, scoped elision and spelling precedence, negative contractions, mixed-language protection and real-data/prose regressions |
| 2026-10-20 | minor | French participles through explicit negation and 19 common adverbs; three-word bound, idiom/clitic guards, exact-range stable corrections and expanded real-data/prose checks |
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

## Bounded reflexive auxiliary correction

With optional recorded morphology, initial personal subjects now support `Je m’ai trompée`
→ `Je me suis trompée`, `Nous nous avons parlé` → `Nous nous sommes parlé` and
`Elle se les a offertes` → `Elle se les est offertes`. The rule follows the
[OQLF description of pronominal verbs](https://vitrinelinguistique.oqlf.gouv.qc.ca/22939/la-grammaire/le-verbe/verbes-pronominaux/emploi-de-la-forme-pronominale):
the reflected clitic has the subject’s person and compound pronominal verbs take être.
Ordinary objects referring to someone else, such as `Nous vous avons écrit`, stay quiet.

The same bounded ordered-clitic parser and at most three explicit negative/adverb words
are used. A separate recorded-participle bit includes all written genders/numbers without
changing the canonical participles used by the earlier avoir + infinitive rule. The exact
written participle is retained: `parlé`, `écrit`, `rendu compte`, `fait entendre` and
preposed-object forms are never given an invented agreement ending.

Scope is deliberately limited to 35 original common-lemma choices: aider, apercevoir,
appeler, arrêter, blesser, cacher, confier, coucher, demander, donner, écrire, enfuir,
expliquer, faire, habiller, inscrire, laver, lever, mettre, occuper, offrir, parler,
préparer, présenter, procurer, promener, rappeler, raser, regarder, rendre, réveiller,
sentir, souvenir, taire and tromper. A written past participle must have only supported
participle readings and a matching finite paradigm. All written avoir mood/tense readings
must be mapped to recorded être forms for the subject. Unsupported/incomplete data,
lost le/la gender on unelision, quotations/code and excessive prefixes remain undecided.
This does not claim full French grammar or participle agreement.

The whole deciding context, from subject through the participle, must belong to one
French region. A foreign or other-region subject, negative word, adverb or participle
therefore cannot determine the correction. Only the smaller replaced prefix suppresses
overlapping spelling guesses; spelling on the unchanged participle remains visible.
Spelling-only mode retains its own marks.
A conflicting elision is suppressed only after the subject/auxiliary correction passes
that language check. If a foreign or other-region participle rejects the auxiliary,
the independent French elision and prefix spelling marks remain available. A small
process-isolated synthetic regression runs without optional external data; the real
lexicon gate checks the same precedence in both French regions.

Original regressions exercise seven lexical mood/tense mappings, recorded participle
genders/numbers, incomplete and ambiguous data, excessive word bounds, both regions,
correct objects, unsupported errors, mixed-language contexts and spelling priority.
Command-level tests cover F7/Change, grammar-mark options, separate Undo and an ASCII
prefix expansion before a colon whose UTF-8 range must be recomputed. Real-data and
external-prose results are reported separately from these synthetic regressions.

## Explicit references to written personal pronouns

`Je est un pronom personnel` and `Nous est écrit en gras sur cette affiche`
describe the written word, which has third-person singular agreement. The
[OQLF description of autonyms](https://vitrinelinguistique.oqlf.gouv.qc.ca/24379/la-typographie/mise-en-relief/italique/emploi-de-litalique-pour-les-autonymes-et-les-mots-de-registres-differents)
allows typographic emphasis instead of quotation marks; its
[determiner guidance](https://vitrinelinguistique.oqlf.gouv.qc.ca/24686/la-grammaire/les-determinants/determinants-definis/determinant-defini-devant-un-autonyme)
also describes direct autonymous use.

A bounded guard recognizes an initial written personal pronoun, a third-person
singular copula (including avoir + été), and an immediate predicative pronoun label,
an explicit `un mot`, `le mot` or `ce mot` label, or a nearby
`en gras`, `en italique(s)`, `en majuscules` or `en minuscules` cue. The deciding
subject-to-marker range must belong to one French region. Initial conjugation/agreement
suggestions are withheld there; only elisions overlapping the initial subject/copula
are withheld, so an unrelated later elision remains available. An intervening comma
or sentence punctuation still ends the marker search.
Spelling and punctuation checks remain available. Ordinary `Je est responsable`
still proposes `Je suis responsable`, and a formatting cue alone cannot disable
an ordinary non-copular personal agreement correction.

This is not a general interpretation of unquoted autonyms, literary grammar,
formatting or user intent. The text-only proofing API does not see italics/bold.
Self-contained synthetic and pinned real-data regressions cover both French
regions, ordinary-error controls and spelling-only behavior.

## French grammar across language runs

Existing paragraph-initial personal-subject agreement, personal-cleft agreement
and avoir + infinitive/participle suggestions require the words actually used to
decide the correction to belong to one French region (`fr-FR` or `fr-CA`). A
foreign or other-region subject, cleft antecedent, auxiliary, required modal
complement or deciding object cue cannot determine a French suggestion. An
unrelated following object does not disable an otherwise unambiguous recorded
participle suggestion.

The original UTF-8 text, issue positions and smaller replacement spans are kept.
Context validation precedes the existing suppression of overlapping spelling
guesses or conflicting elision; rejecting the grammatical suggestion retains
independent spelling and elision. The grammar-mark option and explicit Review/F7
behavior are unchanged. These remain incomplete, bounded rules, without a new
grammatical family, general multilingual parser or document normalization.

Self-contained cases live in `french_mixed_context.rs` and
`french_mixed_cleft_context.rs` under `crates/proof/tests`. The French workflow
also requires the pinned-data `french_mixed_context_real` gate and the engine
`fr_mixed_context` Review test. These test requirements do not establish native,
remote or release readiness; those results must identify their actual source.

## Local grammar and language boundaries

French elision, repeated-word removal and replacements spanning quoted text must
stay within one French region over their complete replacement or deciding range.
For example, French `le` followed by an English-tagged `ami` does not become
`l’ami`, and `test test` is not shortened across French/English runs. A quotation
replacement encompassing foreign-language text is withheld conservatively too.
Adjacent runs in the same French region remain compatible; France and Canada
retain their distinct contexts.

An unrelated foreign prefix does not disable later local French checks whose
own ranges are entirely French. The existing wider subject/auxiliary context
guards, spelling precedence, grammar-mark option and explicit Review/F7 behavior
are retained. Original UTF-8 text and positions are not normalized or rewritten.
These remain bounded checks. General French grammar and multilingual parsing
remain outside this change’s scope.

## Explicit word mentions and local protection

`Je est un mot`, `Je était le mot retenu` and `Nous a été ce mot` explicitly
refer to a written personal-pronoun form. The additional label is the singular
word `mot` immediately after `un`, `le` or `ce`; it does not recognize `mots`,
`motard`, an intervening modifier or an arbitrary later occurrence of `mot`.

The guard separates the words needed to establish the mention from the smaller
initial subject/copula it protects. Its marker cannot be supplied by an English,
unsupported-language or other-French-region run. In `Je est un pronom. Il refuse
de avoir peur.`, the mention stays intact while `de avoir` can still offer `d’avoir`.

The older personal-subject candidate exclusions for nearby `mot`/`mots` and
other metalinguistic words remain unchanged. Rejecting the new guard is not a
promise that those existing exclusions will allow a personal-agreement suggestion.
Spelling, punctuation, original UTF-8 positions and independent Review behavior
are retained. This does not add a general autonym parser, grammar coverage or
native/remote/release validation claim.
