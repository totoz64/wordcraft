# Bounded prefix-to-suffix continuations

> **Last reviewed:** 2026-10-26 · **Last updated:** 2026-10-26 · **Change:** minor (indexed metric-unit continuation lookup)

A prefix can grant a suffix flag that is absent from the dictionary stem.
The French runtime dictionary uses this for metric-unit plurals, including
a zero-length prefix. Recognition and generation follow one prefix and one
suffix, with both cross permissions enabled and each strip/condition checked
on the corresponding form. This does not add stacked prefixes or change the
existing suffix-continuation walk.

Stem FORBIDDENWORD, NOSUGGEST, KEEPCASE and NEEDAFFIX restrictions remain
effective. Either affix may require a partner through NEEDAFFIX, but two
affixes both requiring another affix cannot satisfy each other. Recognition
checks word/stem bounds; generation retains MAX_FORMS/MAX_FORMS_WORK limits
and filters newly generated overlong forms.

## Indexed lookup

The derived index records which suffix classes any cross prefix can grant.
Prefixes are grouped by granted suffix classes, NEEDAFFIX requirement and
first appended Unicode character. Empty prefixes remain candidates for every
word. Each relevant prefix index is stored once, including multi-class
continuations. Each suffix group also retains only its structurally eligible
cross suffix rules; conditions are still checked on the actual candidate.

This avoids scans for impossible classes and mismatched prefix initials
without allocating candidate lists during lookup. Index storage grows
linearly with the prefix and suffix rules. The WCAFF1 format is unchanged;
the reader rebuilds these indexes from the same stored rules. Generation
order and the existing suffix-chain algorithm remain unchanged.

## Evidence and regression gates

Original regressions cover both cross permissions, zero/nonempty prefixes,
NEEDAFFIX partners, stem restrictions, Unicode strip/conditions and initial
characters, length limits, multi-class continuations, linear index storage
and compact round trips. The frozen historical affix-compatibility snapshots
and old compact inputs are not regenerated. The existing checksum-pinned
French dictionary gate checks metric-unit plurals and nearby misspellings
on each desktop OS.

The [public Hunspell dictionary-format manual](https://github.com/hunspell/hunspell/blob/master/man/hunspell.5)
defines continuation classes and cross permission. Reference CLI Hunspell 1.7.2
was observed with 32 original fixtures in run 40369355677 and four partner-affix
fixtures in run 40370987011. No implementation source was read or copied.
FORBIDDENWORD in a prefix continuation was ignored in that CLI observation;
this follow-up preserves the existing directive scope rather than assigning
new semantics to such flags.

Experimental source a1a4e17a38df438090f3c0ebb18ec65ce91c68a7 passed formatting,
the full proofing suite, frozen compatibility, real French/metric-unit gates
and Clippy in run 40378675995. An independent differential audit compiled the
actual old 75c58ac and indexed a1a4 sources: 128 original fixtures plus a
high-numeric-flag case, 256624 word observations across four lookup APIs,
generated forms and both compact directions had no divergence
(run 40379165718). These are bounded checks, not complete Hunspell compatibility.
A clean follow-up still requires its own head-specific hosted checks.

## Performance observations

The initial unindexed path added about 37% median CPU/wall time on 192 fixed
development cases, measured on one VM with warmup and four balanced pairs
(run 40374259717). Operation profiling on eight original probes counted
26998277 prefix-rule visits. The indexed path reduced these to 107920, with
the same 60681 stem lookups, accepted-word count and suggestion count.

A direct same-VM comparison against pre-prefix baseline 694a8e7 measured
the final indexed source at about 1.15% higher median CPU/wall time on the
same 192-case sample, with identical observations across all four pairs
(run 40378675995). This addresses the large observed regression; it is not
proof of zero overhead or a general performance guarantee. Word timing is
not compared.

The earlier full corpus run on 68266aaf (40370360081) observed 59048 identical
inputs before the later NEEDAFFIX partner refinement. That marker has no
affix continuation in the pinned French dictionary. The corpus therefore
documents the measured stage but does not validate a newer source SHA.
Previously inspected units are development diagnostics, not blind held-out
evidence or a global accuracy score. External dictionaries and source
sentences remain runtime inputs and are not committed or bundled.

Cargo is unavailable on the development Windows machine; no local Rust
success is claimed.

## Revision history

| Date | Change | Summary |
|---|---|---|
| 2026-10-26 | minor | Independent bounded continuation follow-up and CLI oracle |
| 2026-10-26 | minor | Linear prefix/suffix indexes, differential audit and controlled performance checks |
