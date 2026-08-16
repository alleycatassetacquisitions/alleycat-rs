# New Server Stories From Alleycat User Stories

> **Status:** Backlog; not current server behavior. See
> [currently implemented routes](../../architecture.md#http-surface).

These stories come from the Google Doc narrative and have no direct equivalent
in the separate `alleycat` implementation.

## Returning Asset Reissue

As an event operator,
I want to identify a returning asset by previous card, PIN, or old system name,
so that I can reissue their ID without creating a duplicate profile.

### Acceptance criteria

- [ ] Searches by previous card number.
- [ ] Searches by PIN.
- [ ] Supports fallback lookup by previous display name.
- [ ] Shows enough matched profile details for staff verification.
- [ ] Reissues the current player code for the new ID card.
- [ ] Avoids creating a duplicate player unless staff explicitly chooses to.

## Training Certification

As an event operator,
I want to record that a player completed required training videos,
so that only certified players enter active gameplay.

### Acceptance criteria

- [ ] Tracks completion of the Quickdraw Guide.
- [ ] Tracks completion of the Multiplayer Modes Guide.
- [ ] Tracks completion of the Courier Mode Guide.
- [ ] Marks a player certified when all required videos are complete.
- [ ] Exposes certification state for check-in and support flows.
- [ ] Prevents or flags gameplay activation for uncertified players.

## Expanded Alleycat Screen Leaderboards

As a player,
I want Alleycat screens to show multiple live leaderboards,
so that different play styles are visible and competitive.

### Acceptance criteria

- [ ] Supports Highest Win Streak.
- [ ] Supports Most Improved Player.
- [ ] Supports Fastest Players.
- [ ] Supports Daily Scoreboard.
- [ ] Supports Most Unique Encounters.
- [ ] Supports NeoCorp leaderboard.
- [ ] Supports Largest Posse leaderboard.
- [ ] Supports Cat Coin leaderboard.
- [ ] Exposes leaderboard data in a display-friendly format.

## Awards Snapshot

As an event operator,
I want to freeze final leaderboard standings for the Toppee Awards,
so that winners can be announced from a stable result set.

### Acceptance criteria

- [ ] Captures final standings at a chosen cutoff time.
- [ ] Stores winner snapshots by award category.
- [ ] Prevents later gameplay from changing announced winners.
- [ ] Exposes award winners for staff review.
- [ ] Exposes award winners for display screens or announcement tooling.

## Posse Tracking

As a player,
I want group support in duels to count toward posse achievements,
so that large social encounters are rewarded.

### Acceptance criteria

- [ ] Records which players supported a duel.
- [ ] Associates posse support with the underlying duel or encounter.
- [ ] Tracks largest posse in a single duel.
- [ ] Tracks total posse instances.
- [ ] Supports posse-based awards and leaderboard rows.

## Shootout Events

As an event operator,
I want to run scheduled shootout events,
so that groups can compete in public bracket-style showdowns.

### Acceptance criteria

- [ ] Creates a shootout event.
- [ ] Tracks participating players or teams.
- [ ] Records shootout rounds and winners.
- [ ] Tracks total shootouts entered or hosted.
- [ ] Tracks most shootout wins.
- [ ] Supports shootout award categories.

## Data Courier Missions

As a player,
I want to pick up, carry, deliver, or intercept Data Crypts,
so that stealth movement and duels can score points.

### Acceptance criteria

- [ ] Creates available Data Crypt pickups at FDNs or equivalent devices.
- [ ] Assigns a Data Crypt to a carrying player.
- [ ] Tracks pickup, carrying, delivery, interception, and completion state.
- [ ] Awards points for successful transport.
- [ ] Awards points for successful interception according to final game rules.
- [ ] Defines whether an interceptor must finish delivery to keep the points.

## Cat Coin Economy

As a player,
I want to earn, transfer, and spend Cat Coin,
so that gameplay connects to prizes and trading.

### Acceptance criteria

- [ ] Tracks Cat Coin balance per player.
- [ ] Awards Cat Coin from configured game actions.
- [ ] Supports prize purchases or redemptions.
- [ ] Supports optional player-to-player transfers.
- [ ] Records transaction history for audit and debugging.
- [ ] Exposes Cat Coin totals for leaderboard display.

## Real Hunter Bounty Selection

As a roleplay-focused Hunter,
I want to browse and activate special bounty targets,
so that I can pursue a more narrative mission.

### Acceptance criteria

- [ ] Lists available real bounty targets.
- [ ] Shows wanted-for text.
- [ ] Shows warnings about the target.
- [ ] Shows pre-dual and post-dual scene requests.
- [ ] Shows optional bonus or gift details.
- [ ] Allows a Hunter to select one target.
- [ ] Marks the selected bounty mission active.
- [ ] Notifies the selected Bounty.

## Real Bounty Submission

As a roleplay-focused Bounty,
I want to submit myself as a special target,
so that Hunters can opt into a scene with me.

### Acceptance criteria

- [ ] Captures what the player is wanted for.
- [ ] Captures warnings about the player.
- [ ] Captures pre-dual scene requests.
- [ ] Captures post-dual scene requests.
- [ ] Captures optional bonus or gift details.
- [ ] Makes the player available for Hunter selection.
- [ ] Allows the player or staff to remove the listing.

## Bounty Activation Notifications

As a selected Bounty,
I want my PDN to notify me when a Hunter activates my mission,
so that I know the scene is live.

### Acceptance criteria

- [ ] Sends a text notification to the target PDN.
- [ ] Triggers haptic behavior when supported.
- [ ] Triggers light behavior when supported.
- [ ] Records when the activation notification was sent.
- [ ] Handles missing, unbound, or offline PDNs gracefully.

## Special Quest Rules

As a game designer,
I want configurable rules for special bounty missions,
so that timers, exclusivity, and normal duel behavior are explicit.

### Acceptance criteria

- [ ] Supports optional mission timers.
- [ ] Defines whether a Hunter can duel others while pursuing a real bounty.
- [ ] Defines whether a Bounty can be pursued by multiple Hunters at once.
- [ ] Defines mission success outcomes.
- [ ] Defines mission failure or expiration outcomes.
- [ ] Stores rule configuration with the mission or mission type.

## Global PDN Alerts

As an event operator,
I want to broadcast messages to all PDNs,
so that large live events can pull players into the same moment.

### Acceptance criteria

- [ ] Sends a text alert to all active PDNs.
- [ ] Supports haptic behavior when available.
- [ ] Supports light behavior when available.
- [ ] Allows staff to trigger predefined messages.
- [ ] Allows staff to compose a custom message if permitted.
- [ ] Records when a broadcast was sent.

## Mini-Boss Encounters

As an event operator,
I want to run mini-boss posse encounters,
so that players can join sides or gang up in a public showdown.

### Acceptance criteria

- [ ] Creates a mini-boss encounter.
- [ ] Tracks one-boss or two-side encounter modes.
- [ ] Tracks participating players and posse membership.
- [ ] Records wins or contribution toward the encounter.
- [ ] Determines the winning side or boss outcome.
- [ ] Exposes encounter state for displays or staff tools.

## Elite Boss Health

As a player,
I want group wins to reduce an elite boss health bar,
so that the whole event can collaborate on defeating a major target.

### Acceptance criteria

- [ ] Creates an elite boss with configurable health.
- [ ] Reduces boss health after qualifying wins.
- [ ] Supports multiple posses contributing over time.
- [ ] Exposes current boss health for display or device output.
- [ ] Records final defeat.
- [ ] Supports related awards or announcements.
