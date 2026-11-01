# CutCraft — External References, Third-Party Software and Provenance Policy

> Scope: how this repository handles external references, third-party products, open-source projects and licensing while building an **independent native** Rust editor. This document is an engineering policy, **not legal advice**. Follow applicable law, license terms, and any specific user constraints.
>
> **RULE 000 overrides this document.** Product-specific reverse engineering, compatibility analysis and private fixtures are not developed, stored, or maintained here (`AGENTS.md` §0 and §9). Where older text in this file or elsewhere seems to authorize that work, RULE 000 wins and the conflict must be reported.

## 1. Separate facts, hypotheses and design

Technical notes about an external product use three evidence labels, and they belong in the owner-authorized research workspace, not here:

- **CONFIRMED** — directly supported by identified source, a specification, or a reproducible test or trace; specify the scope and branch conditions. A static call edge does not prove the path ran at runtime.
- **INFERRED** — a plausible conclusion based on matching names, offsets, interfaces or correlated behavior but lacking a needed direct edge.
- **UNKNOWN** — required evidence is absent or the analysis boundary is opaque. This does not prove the behavior is absent.

Keep a fourth distinct category: **CUTCRAFT DESIGN** — a proposed independent implementation, not a claim about another product's internals. Never promote INFERRED/UNKNOWN into CONFIRMED in code comments, plans or documentation.

## 2. Boundaries when studying third-party software

- Observe **publicly documented behavior, published specifications and the running product as an ordinary user**. Do not decompile, patch, hook, inject, instrument or bypass application protections, and do not copy proprietary instructions, assets or private headers into this repository.
- Do not commit another company's executables, DLLs, disassembly dumps, decompiled source, artwork, project files or screenshots.
- Do not commit local absolute paths, machine-specific locations, tokens, or personally identifying project data. Store any research material outside this repository under an access policy.
- Third-party trademarks belong to their owners and appear here only to identify a format or product being discussed at a high level. See [`THIRD_PARTY_NOTICES.md`](../THIRD_PARTY_NOTICES.md).

A bounded search with no matches means **no match in the scanned corpus**, not whole-product absence.

## 3. What may inform the CutCraft implementation

**Allowed engineering inputs, subject to legal review:** publicly documented user behavior, media container/codec standards, ordinary editing UX requirements, high-level architecture ideas, measurements obtained legitimately, and original tests/fixtures.

**Do not port or copy:** proprietary binary instructions or decompiler pseudocode, literal internal tables, private assets, disassembled algorithms expressed as code, cryptographic/security bypasses, proprietary project branding, or unlicensed media.

Treat another product's internal identifiers as research labels, **not** names that must appear in CutCraft's public APIs. A CutCraft `FrameScheduler` is a design choice, not proof that a similarly named class elsewhere behaves the same way.

### Clean-room terminology

If developers have reviewed proprietary internals, do not claim a strict information-isolated clean-room implementation unless the project actually established and followed that separation process. Accurate wording is **independent implementation informed by high-level research and public specifications**. For sensitive components, consider separate researchers and implementers, neutral behavioral specifications, and documented provenance.

## 4. Research-to-implementation lifecycle

```text
Question → public specification / lawful source → bounded observation
         → claim classification → independent design proposal
         → CutCraft implementation → tests / benchmarks
```

Design notes that inform code are summarized in the ADR that records the decision. The ADR's Context cites lawful sources; the ADR's Decision must explain CutCraft's own tradeoffs, alternatives and tests. Raw research material stays outside this repository.

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

## 7. Interchange formats

Native `.cutcraft` documents are the only project format this repository writes ([ADR-0003](ADR/0003-native-project-persistence.md)). Import of any third-party project format requires an owner-approved compatibility design developed **outside** this repository, field-by-field validation against sanitized examples, and clear unsupported-field behavior before it can be claimed here. No such compatibility is implemented or verified.

- Import must validate before changing native project state.
- Never claim a lossless round trip from a few fixtures; make a distinct claim per version and feature actually tested.
- Never copy a user's private project files or media into a public test suite.

## 8. Stop rule for open questions

Pursue an external question only while a **specific unresolved point** has a plausible lawful probe with meaningful information gain. When evidence ends at an undocumented boundary, record that boundary and move effort to independent engineering experiments and public specifications.

## 9. Provenance checklist

Before implementing a component that was informed by an external reference:

- [ ] Is the behavior described as an implementation-neutral requirement?
- [ ] Are confirmed findings separated from inference and unknowns?
- [ ] Can the implementation be justified by public specs, original design or independent measurements?
- [ ] Have proprietary code, data and assets been excluded?
- [ ] Are API names, algorithms and tests independently authored?
- [ ] Is license and redistribution review complete for new dependencies?
- [ ] Are test fixtures lawful, sanitized and reproducible?
- [ ] Do documentation and final reports avoid overstated compatibility/performance claims?
