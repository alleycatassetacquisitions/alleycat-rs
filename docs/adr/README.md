# Architecture decision records

ADRs explain decisions whose reasons are not evident in the code. They help
people and agents understand those reasons without the original conversation.
Each topic has one current document; Git keeps its history.
Our ADRs are maintained in place, without supersession chains or amendment logs.

## What to record

Record significant choices about product behavior, ownership, persistence,
consistency, or system boundaries that would be costly or surprising to change.
Explain what a reader cannot reliably learn from code: why this approach fits,
which real alternatives were rejected, and what costs we accept. Routine fixes,
refactors, and implementation details usually do not need a record.

Keep documents concise and human-readable. A sentence or two is enough when it
explains the decision and its reason. Explain each decision alongside its reason.
Use only the template sections that help; combine or omit sections as appropriate.
Split independently useful topics when a document grows; do not split a topic
just because its decision changed. State each decision once in its owning document
and link to it elsewhere. Preserve useful reasons for rejected alternatives, not
the transcript of the discussion. Flag unknown rationale rather than invent it.

Keep HTTP contracts in OpenAPI, device contracts in
[protobuf comments](../../proto/device_api.proto), schema details in the
[database reference](../database/schema/README.md), and operating instructions in
the [project README](../../readme.md). Link to them rather than duplicate fields,
validation rules, endpoint inventories, or implementation walkthroughs.

## Creating and maintaining documents

1. Read the topic index and relevant documents before changing behavior. Update
   the existing owner when the topic is already covered.
2. For a new topic, copy [template.md](template.md) to a descriptive filename such
   as `quickdraw-ingestion.md`. Use stable topic names without numbers, dates,
   version suffixes, or titles describing a one-time change.
3. Use `Proposed` for a choice awaiting agreement and `Accepted` once agreed.
   Proposed text is non-binding. Acceptance does not mean implementation is
   complete; briefly identify any implementation gap. Do not replace an accepted
   decision with an unagreed proposal; discuss the proposal in the change review.
4. Update the decision, why, and tradeoffs together in the same change as the
   implementation. Remove obsolete statements and completed follow-ups. Git is
   the changelog; do not add revision files, superseding records, or amendment logs.
5. Keep the index and links current. When merging or retiring topics, preserve
   still-relevant rationale in the remaining owner and remove obsolete files.
   Historical versions and rejected proposals remain available in Git or review
   discussion rather than competing with current guidance.

Accepted decisions guide work but can be questioned. When a task conflicts with
one, flag the conflict and explain the proposed change instead of forcing an
obsolete design to fit. An explicit user instruction changing the decision is
authorization to update it; otherwise resolve the conflict before changing it.
Do not silently rewrite a decision merely because the implementation differs.

Before finishing, check that a reader can find the topic by name and understand
the current choice without following its history. Check links, conflicting rules,
and whether still-relevant rationale was lost during consolidation.

## Topic index

| Topic | Question it answers |
| --- | --- |
| [Device inventory and inspections](device-inventory-and-inspections.md) | Why keep device identities and typed inspection snapshots across events? |
| [Device player lookups](device-player-lookups.md) | Why give PDNs a separate, filtered response and convert display text at the boundary? |
| [Event affiliations](event-affiliations.md) | Why let each event define optional membership categories independently of gameplay roles? |
| [Event API scope](event-api-scope.md) | Which operations follow the active event, and why are there exceptions? |
| [Player lifecycle](player-lifecycle.md) | Why retain inactive registrations and preserve their match history? |
| [Player scoreboard](player-scoreboard.md) | Why calculate rankings from scoring matches and share ranks for equal scores? |
| [Quickdraw ingestion](quickdraw-ingestion.md) | Why accept partial batches, trust match-time snapshots, and preserve the first completed result? |
| [Quickdraw raw reports and diagnostics](quickdraw-raw-reports.md) | Why retain duplicate and rejected submissions without reconciling results? |
| [Quickdraw scoring](quickdraw-scoring.md) | Why use scoring periods and let operators control late points without rewriting results? |
| [Scoring-period database constraints](scoring-period-constraints.md) | Why enforce non-overlap with UUID ranges instead of a database extension? |

## Foundations

- [Coding Agents Love Decision Records](https://www.oreilly.com/radar/coding-agents-love-decision-records/): durable intent, readable current decisions, and history in Git.
- [Diátaxis: Explanation](https://diataxis.fr/explanation/): bounded topics explaining reasons and alternatives separately from reference material.
- [Software Engineering at Google: Documentation](https://abseil.io/resources/swe-book/html/ch10.html): canonical documents maintained alongside code.
- [Design Docs at Google](https://www.industrialempathy.com/posts/design-docs-at-google/): tradeoffs and consistent documents rather than accumulated amendments.
