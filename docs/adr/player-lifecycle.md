# Player lifecycle

- Status: Accepted

Retain inactive registrations so operators can reactivate players without losing
match history or changing the identity referenced by saved results. Inactive
registrations continue to reserve their name and PIN within the event.

Current status and team membership cannot establish a player's eligibility or
team when an offline match occurred. Changing them therefore does not invalidate
completed matches or rewrite earned credit. Enforcing eligibility at match time
would require agreed gameplay rules and a history of status changes.

## References

- [Player routes and contracts](../../src/routes/players/)
- [SQL schema](../database/schema.sql)
- [Event affiliations](event-affiliations.md)
