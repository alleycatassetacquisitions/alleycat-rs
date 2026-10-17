# 0006: Event affiliations

- Status: Accepted
- Date: 2026-10-17

## Context

We need a way to describe which corporation or faction a player belongs to,
without building those particular categories or their values into the server.
Each event should be able to define its own.

We initially called these roles, but that suggests what a player does or what
they're allowed to do. Affiliation better describes the relationship we're
storing. The existing staff, courier, and miniboss values were unused
placeholders, not a gameplay model we need to preserve.

## Decision

Use affiliation categories, affiliations within those categories, and player
affiliations. Categories and affiliations belong to an event. A player can have
zero or one affiliation in each category, and it must belong to the same event
as the player. Assigning another affiliation in that category replaces the
previous choice. Clearing it leaves the player unassigned in that category.
Every category is optional for a player; registration doesn't require an
affiliation.

The database enforces the event boundaries and the one-per-category rule.
Names are unique within their parent: categories within an event, affiliations
within a category. Renaming keeps the same identity and assignments.
An affiliation stays in the category where it was created. The API won't offer
deletion of categories or affiliations; clearing a player's assignment is still
allowed.

The schema has no predefined categories or affiliations. The optional development
seed supplies example Allegiance and Faction choices for its own event; creating
an event does not automatically add them. We replace the unused fixed-role table
and enum in a new migration; published migration files keep their original names
and contents.

Affiliations describe membership only. They don't grant permissions, alter
scoring, or replace teams. If gameplay roles need concrete rules later, we'll
design those separately instead of giving special meaning to affiliation names.

The management JSON API will expose these assignments. Device responses stay
unchanged for now. This record captures the agreed model; API implementation
follows separately from the schema.

## Alternatives considered

- Separate corporation and faction fields would make each new category a server
  change and leave names difficult to manage consistently.
- Keeping the fixed role enum would require a migration for each new value.
- Multiple affiliations per category would add choices we don't currently need.
- Sharing categories across events would couple events that should be able to
  choose and rename their own affiliations independently.

## Consequences

An organizer can define the affiliations their event needs without changing
code. Different events can reuse the same names without sharing assignments.

Assignments describe current membership; we aren't keeping a history of
affiliation changes or adding match-time affiliation snapshots for scoring.
Any future score grouping by affiliation can use current assignments, so
reassigning a player can change which affiliation's total includes their points.
Concrete gameplay behavior and device exposure remain separate decisions.

## References

- [Affiliation migration](../../migrations/20261017080000_create_event_affiliations.sql)
- [Generated database reference](../database/schema/README.md)
- [Player lifecycle](0004-player-lifecycle-and-device-lookups.md)
