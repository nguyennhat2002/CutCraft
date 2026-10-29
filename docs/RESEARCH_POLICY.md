# CutCraft — External Research, Reverse Engineering and Provenance Policy

> Scope: research used to inform the independent CutCraft Rust editor, including static analysis of CapCut Desktop binaries and third-party open-source projects. This document is an engineering policy, **not legal advice**. Follow applicable law, license terms, and any specific user constraints.

## 1. Separate facts, hypotheses and design

All technical research notes use three evidence labels:

- **CONFIRMED** — directly supported by identified disassembly/data, source, a specification, a reproducible test or a trace; specify the scope and branch conditions. A static call edge does not prove the path ran at runtime.
- **INFERRED** — a plausible conclusion based on matching type names, RTTI, offsets, interfaces, symbols or correlated behavior but lacking a needed direct edge.
- **UNKNOWN** — required evidence is absent or the analysis boundary is opaque. This does not prove the behavior is absent.

Keep a fourth distinct category: **CUTCRAFT DESIGN** — a proposed independent implementation, not a claim about CapCut internals. Never promote INFERRED/UNKNOWN into CONFIRMED in code comments, plans or documentation.

## 2. Static reverse-engineering constraints

For the current CapCut 9.6.0.4097 REA investigation, the established scope is **static analysis only**. Do not run CapCut or load its DLLs for inspection; do not hook, inject, patch, instrument or bypass app protections. A change in research scope requires a separate explicit user decision and a compliance review.

For each claim, when available record:

- Module, version, file hash, image base, VA and RVA (never show an unqualified VA across DLLs).
- Target function, callsite, vtable/slot/RTTI and evidence mechanism.
- Direct dataflow/pointer identity/ownership evidence versus a mere type-name or offset match.
- Branch condition, static-only scope, limitations of decompiler output and bounded search coverage.
- Links to the original research artifact, date and investigator/tool version where possible.

A bounded scan with no matches means **no match in the scanned corpus**, not whole-program absence. Separate the identities of `PlayerWin`, `DraftPlayerImpl`, `TEPreviewGraph`, `TERecorderGraph`, pool members, and map values unless a direct object identity edge connects them.

## 3. What may inform the CutCraft implementation

**Allowed engineering inputs, subject to legal review:** publicly documented user behavior, media container/codec standards, ordinary editing UX requirements, high-level architecture ideas, measurements obtained legitimately, and original tests/fixtures.

**Do not port or copy:** proprietary binary instructions or decompiler pseudocode, literal internal tables, private assets, disassembled algorithms expressed as code, cryptographic/security bypasses, proprietary project branding, or unlicensed media. Do not mechanically translate Ghidra output into Rust.

Treat CapCut-specific internal identifiers as research labels, **not** names that must appear in CutCraft's public APIs. A CutCraft `FrameScheduler` is a design choice, not proof that a named CapCut class has the same behavior.

### Clean-room terminology

If developers have reviewed proprietary internals, do not claim a strict information-isolated clean-room implementation unless the project actually established and followed that separation process. Accurate wording is **independent implementation informed by high-level research and public specifications**. For sensitive components, consider separate researchers and implementers, neutral behavioral specifications, and documented provenance.

## 4. Research evidence lifecycle

```text
Question → source selection → bounded inspection → evidence ledger
         → claim classification → independent design proposal
         → CutCraft implementation → tests / benchmarks
```

Record research in non-production notes, not scattered across program source. When useful, attach a source citation to an ADR's Context section. The ADR's Decision must explain CutCraft's own tradeoffs, alternatives and tests.

Do not commit:

- CapCut executables or DLLs, disassembly dumps, decompiled source, proprietary artwork or private SDK headers.
- Local absolute paths (for example a researcher's machine drive paths), tokens or personally identifying project data.
- Copyrighted sample media without authorization and the right to redistribute.

If confidential or licensed research artifacts must be retained, store them outside the public repository under an access policy. Prefer a neutral summary of observed behavior and a precise pointer to lawful source material rather than embedding protected material.

## 5. FilmCraft and other open-source projects

CutCraft may study FilmCraft's **workflow ideas**: authority of `AGENTS.md`, session instructions, roadmaps, layered architecture, tests, asset tracking, and measurable Definition of Done.

- Write CutCraft's own original policy prose, tailored to its actual scaffold and intended Windows-first use.
- Respect each repository's license before reusing code or detailed expressive documentation. Link and attribute inspiration where appropriate; don't bulk-copy source or assets.
- Audit dependency licenses separately from licenses of standalone project documentation.
- Avoid adopting design choices merely because FilmCraft uses them. FilmCraft's pure-Rust codec strategy and restrictions on FFmpeg are **not** automatically CutCraft's policy.

## 6. Codec libraries and external specifications

- Prefer documented container, codec and graphics APIs when implementing interoperability.
- Evaluate FFmpeg, Rust bindings, Media Foundation, Direct3D and other backend choices independently for performance, deployment and licensing.
- Do not mix the presence of an import or symbol with a proven decoded-frame-to-display flow.
- Track source licenses, binary dependency versions, optional codecs, linking method and redistribution obligations. LGPL and GPL obligations depend on concrete build/distribution choices; obtain appropriate review before release.
- Keep source/original PTS, frame rates, color metadata and memory ownership explicitly represented regardless of backend.

## 7. CapCut draft format compatibility

`capcut-format` is currently a schema scaffold. Compatibility is **unverified**. Any support must be validated against user-authorized/sanitized examples, with field-by-field tests and clear unsupported-field behavior.

- Import should validate before changing native project state.
- Export must not silently omit meaningful data while claiming lossless round-tripping.
- Avoid copying confidential project files or media into public test suites.
- Keep native CutCraft project semantics independent from undocumented CapCut assumptions where practicable.
- Make a distinct compatibility claim for each version/feature tested; don't claim broad project support after a few fixtures.

## 8. Research stop rule

A research branch should continue only when a **specific unresolved edge** has a plausible static probe with meaningful information gain. If repeated vtable/RTTI scans produce only names and compatible offsets, record the stop boundary and move effort to the independent prototype or public spec research.

For the preview-pipeline study, do not assert a proven decoder → graph → GPU presentation pipeline unless direct call/pointer/ownership evidence actually connects its stages. Where evidence ends at an indirect `display` child or unresolved property object, document that boundary and use independent engineering experiments to build CutCraft.

## 9. Research-to-code checklist

Before implementing a research-informed component:

- [ ] Is the behavior described as an implementation-neutral requirement?
- [ ] Are confirmed findings separated from inference and unknowns?
- [ ] Can implementation be justified by public specs, original design or independent measurements?
- [ ] Have proprietary code/data/assets been excluded?
- [ ] Are API names, algorithms and tests independently authored?
- [ ] Is license and redistribution review complete for new dependencies?
- [ ] Are test fixtures lawful, sanitized and reproducible?
- [ ] Do documentation and final reports avoid overstated compatibility/performance claims?
