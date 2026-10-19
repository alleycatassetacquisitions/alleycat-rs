# Architecture decision records

ADRs capture why we chose an architecture, the alternatives considered, and the
tradeoffs we accepted. They help future changes preserve the intent behind the
code. An accepted decision can be implemented in stages; its status does not
mean every related feature has shipped.

## When to write one

Write an ADR for a significant choice about data ownership, persistence,
consistency, system boundaries, or behavior that would be costly or surprising
to change. Bug fixes, routine refactors, and small implementation details usually
do not need one. Keep each record focused on one decision or a closely related
set of decisions.

Use OpenAPI for current API contracts, the
[generated database reference](../database/schema/README.md) for schema details, and the
[project README](../../readme.md) for operating instructions. Link to those sources
and the implementation where useful.

## Creating and maintaining a record

1. Copy [template.md](template.md) to `NNNN-short-decision-title.md`, using the
   next unused four-digit number. Never reuse or renumber an existing record.
2. Set the date to the decision date in `YYYY-MM-DD` format. Use `Proposed` while
   evaluating a choice and `Accepted` once agreed. Decisions already agreed
   during the work can be recorded directly as accepted.
3. Describe the problem, chosen approach, real alternatives, and consequences.
   Note important deferred work without turning the record into a task list.
4. Add the record to the index below and include it with the relevant change.

Use `Rejected` for a proposal that was considered but not adopted. If an accepted
decision is replaced, create a new record, mark the old one `Superseded`, and link
both records using their status lines. Update this index too. Preserve the old
context and rationale; typo fixes, clarifications, and repaired links are fine.

## Index

| ADR | Status | Date |
| --- | --- | --- |
| [0001: Quickdraw results and scoring periods](0001-quickdraw-results-and-scoring-periods.md) | Partially superseded by 0003 | 2026-10-11 |
| [0002: Quickdraw device ingestion](0002-quickdraw-device-ingestion.md) | Partially superseded by 0009 and 0010 | 2026-10-11 |
| [0003: Reversible delayed scoring](0003-reversible-delayed-scoring.md) | Accepted | 2026-10-15 |
| [0004: Player lifecycle and device lookups](0004-player-lifecycle-and-device-lookups.md) | Partially superseded by 0009 | 2026-10-15 |
| [0005: Extension-free scoring-period constraints](0005-extension-free-scoring-periods.md) | Accepted | 2026-10-16 |
| [0006: Event affiliations](0006-event-affiliations.md) | Partially superseded by 0008 | 2026-10-17 |
| [0007: Use the active event for routine API operations](0007-active-event-api-scope.md) | Partially superseded by 0009 | 2026-10-18 |
| [0008: Include affiliations in device player lookups](0008-device-player-affiliations.md) | Accepted | 2026-10-18 |
| [0009: Accept match uploads only for the active event](0009-active-event-match-uploads.md) | Partially superseded by 0010 | 2026-10-18 |
| [0010: Accept valid matches from partially invalid batches](0010-partial-match-batch-acceptance.md) | Accepted | 2026-10-18 |
| [0011: Player scoreboard](0011-player-scoreboard.md) | Accepted | 2026-10-18 |
