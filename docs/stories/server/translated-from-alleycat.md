# Server Stories Translated From Alleycat

> **Status:** Backlog; not current server behavior. See
> [currently implemented routes](../../architecture.md#http-surface).

These stories come from the separate `alleycat` implementation, mainly its
`main` branch.

## Register a Player

As an event operator,
I want to create a player profile with name, role, allegiance, faction, and Neo ID,
so that a new asset can enter the game quickly.

### Acceptance criteria

- [ ] Creates a player with a reserved-safe four digit ID.
- [ ] Stores display name, Hunter/Bounty role, allegiance, faction, and Neo ID.
- [ ] Rejects duplicate player names case-insensitively.
- [ ] Returns or redirects to the created player on success.
- [ ] Returns a useful error when creation fails.

### Previous implementation notes

- Source behavior exists in `alleycat/main/routes/api/players.ts`.
- Player IDs are generated from the `available_ids` table and skip reserved values.

## Look Up a Player

As an event operator,
I want to look up a player by ID,
so that I can confirm their profile during check-in or support.

### Acceptance criteria

- [ ] Accepts a short numeric ID and pads it to four digits.
- [ ] Returns the player profile when found.
- [ ] Returns a clear not-found error when the player does not exist.
- [ ] Includes role, allegiance, faction, Neo ID, and FDN/box progress in the response.

### Previous implementation notes

- Source behavior exists in `alleycat/main/routes/api/players.ts` and `alleycat/main/db/queries.ts`.

## Update a Player Profile

As an event operator,
I want to update a player profile,
so that check-in corrections and support fixes can be made without creating duplicate players.

### Acceptance criteria

- [ ] Updates display name, allegiance, faction, and Neo ID.
- [ ] Preserves the player's existing Hunter/Bounty role.
- [ ] Redirects or returns success for a valid update.
- [ ] Returns the submitted data and errors when update fails.

### Previous implementation notes

- Source behavior exists in `alleycat/main/routes/api/players.ts`.
- The previous update form does not update the `hunter` field.

## Bind Neo ID to a Player

As an event operator,
I want to store a scanned Neo ID on a player profile,
so that Neoband or card scans can identify the right player later.

### Acceptance criteria

- [ ] Accepts a scanned Neo ID value on player create or update.
- [ ] Stores the Neo ID with the player profile.
- [ ] Allows later lookup by Neo ID for mission or support workflows.
- [ ] Handles empty or missing Neo ID values consistently with player forms.

### Previous implementation notes

- Source behavior exists through `neoId` on players and `getPlayerByNeoId` in `alleycat/main/db/queries.ts`.
- The update page includes a scanner-assisted Neo ID field.

## Record Quickdraw Matches

As a PDN or operator system,
I want to submit completed duel results,
so that match history and scoring can be calculated.

### Acceptance criteria

- [ ] Accepts a JSON payload containing one or more matches.
- [ ] Stores match ID, Hunter player ID, Bounty player ID, winner role, Hunter time, and Bounty time.
- [ ] Ignores duplicate match IDs.
- [ ] Rejects invalid match payloads with a clear error.
- [ ] Allows matches to be listed for review or debugging.

### Previous implementation notes

- Source behavior exists in `alleycat/main/routes/api/matches.ts`.

## Hunter and Bounty Leaderboards

As a player,
I want to see ranked Hunter and Bounty win totals,
so that I know who is leading the core game.

### Acceptance criteria

- [ ] Calculates Hunter wins from stored match results.
- [ ] Calculates Bounty wins from stored match results.
- [ ] Sorts each leaderboard by wins descending.
- [ ] Includes player name, allegiance, faction, and FDN/box progress in each score row.
- [ ] Supports pagination for API consumers.

### Previous implementation notes

- Source behavior exists in `alleycat/main/routes/api/scores.ts` and `alleycat/main/db/queries.ts`.

## Average Draw Time Leaderboard

As a player,
I want to see the fastest average draw times,
so that speed-based performance can be recognized.

### Acceptance criteria

- [ ] Calculates average draw time for players in the Hunter role.
- [ ] Calculates average draw time for players in the Bounty role.
- [ ] Labels each score row with the role used for the average.
- [ ] Sorts fastest averages first.
- [ ] Supports pagination for API consumers.

### Previous implementation notes

- Source behavior exists in `alleycat/main/routes/api/scores.ts` and `alleycat/main/db/queries.ts`.

## Track FDN Box State

As the game system,
I want to track which FDN boxes a player has hacked,
so that minigame progress can persist across scans and page loads.

### Acceptance criteria

- [ ] Reads a player's FDN/box state as a boolean array.
- [ ] Sets one box state to hacked.
- [ ] Clears one box state.
- [ ] Stores state compactly on the player record.
- [ ] Rejects missing player IDs, invalid box IDs, and unknown players.

### Previous implementation notes

- Source behavior exists in `alleycat/main/routes/api/boxes.ts`, `alleycat/main/lib/boxes.ts`, and `alleycat/main/lib/boxes-service.ts`.
- The previous implementation supports box IDs `0` through `30`.

## Stream Reader Scan Events

As an operator system,
I want uFR reader scan events streamed from the server,
so that Neoband and reader activity can drive live gameplay.

### Acceptance criteria

- [ ] Accepts uFR reader WebSocket connections.
- [ ] Parses incoming reader frames from supported raw formats.
- [ ] Publishes valid holder-present scan events.
- [ ] Maps card UID to user ID.
- [ ] Maps reader OSN to reader ID.
- [ ] Streams parsed read events to server-sent event subscribers.

### Previous implementation notes

- Source behavior exists in `alleycat/main/routes/api.ts`, `alleycat/main/lib/ufr/ufr-bus.ts`, and `alleycat/main/lib/ufr/ws-manager.ts`.

## Control Readers From Operator UI

As an operator UI,
I want to bind to a reader, read cards, and configure card data,
so that check-in and support stations can manage physical devices.

### Acceptance criteria

- [ ] Requires browser clients to connect with a station ID.
- [ ] Lists connected uFR readers.
- [ ] Binds an operator client to a selected reader.
- [ ] Supports card read requests.
- [ ] Supports linear card configuration requests.
- [ ] Sends clear socket errors for invalid commands or missing bindings.

### Previous implementation notes

- Source behavior exists in `alleycat/main/lib/ufr/bun-handlers.ts`, `alleycat/main/lib/ufr/ws-manager.ts`, and `alleycat/main/lib/ufr/ws-types.ts`.

## Manage Event Databases

As an admin,
I want to create, list, select, and delete event databases,
so that each event can run against the correct data set.

### Acceptance criteria

- [ ] Lists available database files and identifies the active one.
- [ ] Creates a new seeded database.
- [ ] Selects the active database.
- [ ] Deletes inactive databases.
- [ ] Prevents deletion of the active database.
- [ ] Returns validation errors for missing names, invalid names, and provider failures.

### Previous implementation notes

- Source behavior exists in `alleycat/main/routes/api/dbs.ts` and `alleycat/main/utils/db-admin.ts`.

## Manage Quest Givers

As an affiliate or mission operator,
I want to create and access a quest giver profile with a passcode,
so that only authorized staff can manage their missions.

### Acceptance criteria

- [ ] Creates quest givers with name and passcode.
- [ ] Enforces unique passcodes.
- [ ] Allows lookup by ID.
- [ ] Allows access by passcode.
- [ ] Rejects invalid or missing passcodes.

### Previous implementation notes

- Source behavior exists in `alleycat/main/routes/site/affiliate.tsx` and the `quest_givers` table.

## Build Missions

As an affiliate or mission operator,
I want to create missions with type-specific configuration,
so that players can be given basic, timed, or multi-part objectives.

### Acceptance criteria

- [ ] Creates a mission for a quest giver.
- [ ] Stores label, description, mission type, options, and win condition.
- [ ] Supports basic missions.
- [ ] Supports timed missions with hours and minutes converted to duration seconds.
- [ ] Supports multi-part missions with at least two parts.
- [ ] Validates required fields and type-specific options.

### Previous implementation notes

- Source behavior exists in `alleycat/main/routes/site/affiliate.tsx`, `alleycat/main/lib/missions.ts`, and the `missions` table.

## Manage Player Mission Status

As an affiliate or mission operator,
I want to start, update, complete, reset, and clear a player's mission status,
so that mission progress can be tracked during the event.

### Acceptance criteria

- [ ] Looks up a player by Neo ID or player ID.
- [ ] Lists missions with that player's current status.
- [ ] Starts a mission and records `started_at`.
- [ ] Updates status notes.
- [ ] Updates completed parts for multi-part missions.
- [ ] Marks a mission complete and records `completed_at`.
- [ ] Resets the start time for timed missions.
- [ ] Deletes a mission status when progress needs to be cleared.

### Previous implementation notes

- Source behavior exists in `alleycat/main/routes/site/affiliate.tsx`, `alleycat/main/db/queries.ts`, and `alleycat/main/db/mutations.ts`.
