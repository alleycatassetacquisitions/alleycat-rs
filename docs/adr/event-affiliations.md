# Event affiliations

- Status: Accepted

Let each event define optional affiliation categories, such as corporation or
faction, so organizers can manage membership without code changes or migrations.
Keep catalogs event-local so one event can rename its choices without affecting
another. One assignment per category meets the current need; multiple assignments
would add choices we do not need yet.

Affiliations describe belonging. They do not grant permissions, replace teams, or
imply gameplay rules based on their names. This keeps organizer-defined labels
from acquiring unintended behavior.

Keep current assignments without historical snapshots. Consequently, future
statistics grouped by current affiliation could move a player's earned points
between groups after reassignment.

## References

- [Catalog API](../../src/routes/affiliations.rs)
- [Player assignment API](../../src/routes/players/update.rs)
- [Database reference](../database/schema.sql)
- [Event API scope](event-api-scope.md)
