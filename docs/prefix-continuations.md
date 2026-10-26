# Bounded prefix-to-suffix continuations

> **Last reviewed:** 2026-10-26 · **Last updated:** 2026-10-26 · **Change:** minor (experimental shared-affix follow-up)

A prefix can grant a suffix flag that is absent from the dictionary stem.
The French runtime dictionary uses this for metric-unit plurals; a zero-length
prefix can grant the same class. The original parser already retained these
continuation flags, but recognition and generation did not follow this direction.

The new path accepts exactly one prefix followed by one suffix, with both cross
flags enabled and each strip/condition checked on the corresponding form.
It does not alter the historical suffix-chain walk, add stacked prefixes, or
attempt complete Hunspell compatibility. Recognition rejects overlong input and
the reconstructed stem is bounded by MAX_WORD_BYTES. Generation shares the
existing MAX_FORMS/MAX_FORMS_WORK limits and filters newly generated overlong forms.
Stem FORBIDDENWORD, NOSUGGEST, KEEPCASE and NEEDAFFIX restrictions remain effective.

The original fixtures in prefix_continuations.rs cover both cross flags, zero
and nonempty prefixes, no duplicated prefixes or extra suffixes, compact-format
round trips, stem restrictions, prefix NEEDAFFIX, Unicode strip/conditions and
length bounds. The frozen historical affix_compatibility expectations are not
regenerated. A separate ignored regression requires the checksum-pinned French
runtime dictionary and checks metric-unit plurals and nearby invalid spellings.

## Reference observations

The [public Hunspell dictionary format manual](https://github.com/hunspell/hunspell/blob/master/man/hunspell.5)
defines continuation classes and prefix/suffix cross permission.
The dedicated workflow executes the installed Hunspell CLI against original
synthetic fixtures; no Hunspell implementation source is read or copied.
The initial oracle run 40369355677 used Hunspell 1.7.2 and observed 32 fixtures.
An additional four partner-affix fixtures in run 40370987011 confirmed that a
NEEDAFFIX suffix accepts an ordinary prefix, but two affixes both carrying
NEEDAFFIX cannot satisfy each other. Recognition and generation preserve this
distinction, including compact-format round trips.
Both cross flags are required, including a zero prefix. Prefix NEEDAFFIX is
satisfied by the suffix. FORBIDDENWORD written in a prefix continuation was
ignored by that CLI observation; this change preserves existing directive scope
rather than inventing a different interpretation. The CLI spelling probe does
not establish suggestion filtering behavior for directives placed on affixes.

Hosted full proofing, historical compact compatibility, Clippy and real-dictionary
gates must pass on the actual branch head before any integration claim.
Cargo is absent on the development Windows machine; no local Rust success is claimed.
The previously inspected corpus units are development diagnostics, not a blind
held-out evaluation for this change. External dictionaries are runtime inputs and
are not committed or bundled.

## Revision history

| Date | Change | Summary |
|---|---|---|
| 2026-10-26 | minor | Independent one-prefix/one-suffix continuation follow-up and CLI oracle |
