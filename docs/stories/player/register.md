## Register a player

As an event operator,
I want to register a player quickly,
so that I can get people into matches without slowing down the line.

### Acceptance criteria

- [ ] Can create a player with display name
- [ ] Rejects duplicate active player names
- [ ] Returns the created player as JSON
- [ ] Returns a 4 digit code for pdn registration
- [ ] Shows a useful error message on failure
- [ ] Covered by at least one API test
- [ ] Some codes are reserved TBD

### Notes

- Route: POST /api/players
- DB table: players
- UI later: admin registration form
