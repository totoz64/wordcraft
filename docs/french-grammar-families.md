# French grammar families

> **Last reviewed:** 2026-10-22 · **Last updated:** 2026-10-22 · **Change:** minor (bounded expanded French subjects and recap coordination); previously minor (initial bounded grammar-family catalogue)

This catalogue groups checks by grammatical dependency rather than individual example sentences. It records recognised constructions, required lexical facts, reasons to decline a suggestion and the evidence needed to widen a family. It is neither an exhaustive French grammar nor a comparison with Word. Implementation, local validation, native validation, hosted validation and publication are separate states; see [French proofing](french-proofing.md) for the delivery record.

## Existing contribution stack

| Family | Recognised dependency | Lexical and context requirements | Deliberate limits |
|---|---|---|---|
| `FR-NOMINAL` | Determiner, common-noun head and at most one adjective at a recognised sentence beginning | Recorded correlated gender/number readings and same-lemma target forms; all deciding words and gaps belong to one French region | Preserve compatible readings; do not guess compound colours, positional masculine adjective alternatives, nominal labels or complex noun groups |
| `FR-SUBJECT-NOMINAL` | A number-compatible determiner/common noun, optionally one adjective, controls a following finite verb | Recorded subject number and finite person/tense/mood alternatives; bounded complete context for a nonverbal homograph | No general parsing of coordination, relatives, inversion, intervening complements or ambiguous collective/quantity agreement |
| `FR-SUBJECT-NEGATIVE` | The same subject followed by `ne`/`n’`/`n'`, a recorded finite form and one immediate `pas`, `plus`, `jamais`, `guère` or `point` | Preserve recorded finite alternatives and render the complete negative prefix with correct elision; include the marker and any deciding homograph tail in the French context | No omitted `ne`, restriction with `que`, combined markers, arbitrary skipped adverbs or general negative-clause analysis; existing nominal ambiguity guards remain |

Personal-subject/personal-cleft agreement, bounded avoir–participle checks and reflexive auxiliary selection are separate existing families documented in [French proofing](french-proofing.md). This catalogue does not merge their different parsing or auxiliary policies merely because they share morphological data.

## This increment — prepared in the contribution stack

| Addition | Intended gain | Required boundary |
|---|---|---|
| `FR-SUBJECT-ONE-CLITIC` | Reuse the nominal-subject family through exactly one non-reflexive `lui`, `leur`, `y` or `en`, including the existing explicit negative bridge | Require an attested finite source without a nonverbal reading in this path. `en` can be a preposition and `leur` a possessive determiner. Exclude other clitic forms, reflexive constructions and chains; preserve the complete prefix and same-region context |
| Recorded-paradigm construction sweep | Exercise the construction over deterministically sampled singular/plural pairs from several recorded tense/mood classes, both French regions, prefix variants and generated homograph declines | Report distinct generated cases and their source classes from actual execution. The oracle is lexical agreement self-consistency: it does not establish independent linguistic accuracy, clitic valency or general French coverage |

Retain independently authored examples alongside generated cases. A large generated set must not conceal a missing manual test for a linguistic distinction. Correct examples, erroneous examples and unsupported/ambiguous examples have different denominators; remaining quiet on unsupported input is not evidence of correct-text precision.

## Next prepared subject families

| Family | Recognised dependency | Deliberate boundary |
|---|---|---|
| Plural head with a determined complement | The plural head controls a following pure finite form through one short determined nominal complement | No singular collective/quantity heads, bare complements, nesting or free verb complements; the complete bounded tail is deciding context |
| Nominal two-clitic chain | Exactly `lui en` or `leur en`, with the existing explicit negative bridge | Reuse the typed order/person representation; no other pair or reflexive/participle analysis |
| Explicit personal recap coordination | Two disjunctive personal forms joined by `et`, comma and compatible `nous`/`vous`/`ils`/`elles` before the pure finite form | Unrecapped titles remain unmarked; every prefix word and gap is deciding context; no general clause parser |

## Shared constraint contract

A family consumes original UTF-8 token spans, retained lexical readings and explicit context dependencies. It may propose only attested replacements consistent with its recognised dependency; a compatible reading remains unmarked. Unknown or incomplete facts stay undecided. Preserve correlated nominal readings instead of independently combining gender and number sets. Reuse the existing lexical APIs rather than spelling-based conjugation guesses.

Each emitted suggestion carries the replacement span and every positively deciding word or gap. Context acceptance precedes spelling/elision priority, and priority stays within the accepted replacement. Rejected syntax must leave independent local diagnostics intact. Applying a correction and undoing it must preserve the document text and proofing languages.

A reusable bounded representation can support these contracts. This increment does not claim to implement a general dependency parser or to identify a sentence’s intended meaning.

## Future families — not implemented by this increment

1. **Widen short prepositional expansions.** The new plural-head determined-complement path leaves singular collective/quantity heads and richer groups undecided. Further support needs positive semantic facts and independent ambiguity evidence.
2. **Richer clitic and predicate structure.** Add other clitic forms/reflexive chains and subject attributes only with their own ambiguity, order and agreement policies.
3. **Broader syntax.** Unrecapped coordination, relative clauses, participle agreement and context-sensitive homophones require distinct analysis and independent corpus evidence. They are not implied by installing more conjugated forms.

For each widening, record the source-defined construction, examples and counterexamples, required traits, recognised context, explicit decline conditions, current tests and unresolved cases before changing the parser.

## Evidence to report per family

- Recognised constructions divided by eligible constructions in a fixed annotated evaluation set, with the eligibility rule stated.
- Detected and correctly repaired injected errors divided by the injected errors within the supported scope; keep generated self-consistency separate from independent examples.
- False alerts among the correct sentences actually reviewed, and adjudicated precision among emitted alerts. Do not treat a zero-alert smoke run as a population-wide estimate.
- Stable corrections, grammar-disabled behaviour, foreign words and gaps, regional boundaries, UTF-8 ranges, F7/Apply/Undo and unchanged unrelated diagnostics.
- Time and memory measured on the same paragraph lengths before and after a change. Keep a held-out set distinct from construction and tuning cases.

No overall French-completion percentage follows from these measures.

## Linguistic and data references

- OQLF: [coordinated subjects of different grammatical persons](https://vitrinelinguistique.oqlf.gouv.qc.ca/25402/la-grammaire/le-verbe/accord-du-verbe-avec-le-sujet/sujets-joints-par-une-conjonction/sujets-coordonnes-de-personnes-differentes) and [subjects joined by et](https://vitrinelinguistique.oqlf.gouv.qc.ca/24542/la-grammaire/le-verbe/accord-du-verbe-avec-le-sujet/sujets-joints-par-une-conjonction/accord-du-verbe-avec-des-sujets-unis-par-et). Person agreement and the possibility of titles motivate the explicit-recap boundary.
- Académie française: [lui](https://www.dictionnaire-academie.fr/article/A9L1331) and [UOC pronoun-order teaching table](https://aprenentatge.recursos.uoc.edu/continguts/PID_00221043/def29.html) support the admitted lui/leur + en pattern; other pairs remain unsupported here rather than declared universally incorrect.

- OQLF: [agreement with a subject containing a collective noun](https://vitrinelinguistique.oqlf.gouv.qc.ca/21443/la-grammaire/le-verbe/accord-du-verbe-avec-le-sujet/sujet-qui-est-un-nom-collectif/accord-du-verbe-avec-groupe-foule-troupe). Morphological singular/plural facts alone do not classify collective meanings; the determined-complement extension leaves singular heads undecided.

- OQLF: [the noun and nominal group](https://vitrinelinguistique.oqlf.gouv.qc.ca/24321/la-grammaire/la-grammaire-actuelle/les-classes-de-mots-et-les-groupes/le-nom-et-le-groupe-nominal) and [subject–verb agreement index](https://vitrinelinguistique.oqlf.gouv.qc.ca/la-grammaire/le-verbe/accord-du-verbe-avec-le-sujet). These provide linguistic facts and distinguish head, expansions and special subject classes; examples and implementation here are original.
- OQLF: [two-part negation](https://vitrinelinguistique.oqlf.gouv.qc.ca/23905/la-syntaxe/la-negation-et-la-restriction/generalites-sur-la-negation) and [obligatory elision](https://vitrinelinguistique.oqlf.gouv.qc.ca/21737/lorthographe/elision-et-apostrophe/elision-obligatoire).
- The installed Dicollecte/Grammalecte 7.7 lexicon has its own MPL-2.0 licence and provenance; retain these with the data. Its forms are inputs, not a count of grammatical rules or measured accuracy.
- [UD French GSD documentation](https://universaldependencies.org/treebanks/fr_gsd/) and [provider README](https://github.com/UniversalDependencies/UD_French-GSD/blob/master/README.md) describe an optional future evaluation source. Annotation licensing and underlying-text rights are distinct; the corpus is not an unquestioned gold oracle and no raw corpus import is implied by this catalogue.

No GPL/LGPL/AGPL grammar code or rule files are used for this proposal. A reference to a public grammar description does not assert permission to copy its articles or examples wholesale.

## Revision history

| Date | Change | Summary |
|---|---|---|
| 2026-10-22 | minor | Bounded plural subjects with one determined complement, nominal lui en/leur en chains, and explicitly recapped personal coordination; retain lexical ambiguity, complete same-region dependencies and separate delivery states |
| 2026-10-22 | minor | Separate existing checks, one-clitic nominal subjects and future families; define lexical/context contracts and independent versus generated evidence |
