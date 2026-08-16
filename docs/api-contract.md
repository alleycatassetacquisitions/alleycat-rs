# Legacy Alleycat API Contract

> [!WARNING]
> Historical porting reference—not the current Rust API. See
> [Device API](device-api.md) and
> [currently implemented routes](architecture.md#http-surface).

Source: the earlier Hono/Bun server in the separate `alleycat` repository,
primarily its `main` branch.

This document describes the API portion to recreate in this repo. It excludes
HTML site routes such as `/players`, `/matches`, `/scoreboard`, `/admin`, and
`/affiliate`, except where those pages submit to `/api/*`.

## General conventions

- Base path for HTTP JSON/form APIs: `/api`.
- Most JSON endpoints return `{ "data": ..., "errors": [] }`.
- Error envelopes usually return `{ "data": null, "errors": ["message"] }`.
- Some form endpoints return plain text on failure and redirects on success.
- Pagination query params are supported on list endpoints:
  - `page`: positive integer, default `1`.
  - `rows`: positive integer, default `10`.
  - Invalid, missing, zero, or negative values fall back to defaults.
- Player ids are four digit strings. Lookups pad ids with leading zeroes before
  querying the database, so `/api/players/7` searches for player `0007`.
- `Allegiance` is stored as a number:
  - `0`: None
  - `1`: Endline
  - `2`: Helix
  - `3`: Reboot

## Shared types

```ts
type ApiResponse<T> = {
  data: T;
  errors: string[];
};

type Player = {
  id: string;
  name: string;
  hunter: boolean | 0 | 1;
  allegiance: 0 | 1 | 2 | 3;
  faction: string;
  neoId: string;
  hacked_boxes_bits: number;
};

type Match = {
  match_id: string;
  hunter: string;
  bounty: string;
  winner_is_hunter: boolean | 0 | 1;
  hunter_time: number;
  bounty_time: number;
};

type ScoreRow = {
  id: string;
  name: string;
  allegiance: 0 | 1 | 2 | 3;
  faction: string;
  hacked_boxes_bits: number;
  wins: number;
};

type AverageDrawTimeRow = {
  name: string;
  average_time: number;
  role: "hunter" | "bounty";
};

type DbFileInfo = {
  name: string;
  path: string;
  active: boolean;
  sizeBytes: number;
};
```

## Health and time

### `GET /api/health-check`

Returns the server health envelope.

Response `200`:

```json
{
  "data": "All good",
  "errors": []
}
```

### `GET /api/time`

Returns current Unix time in seconds. The source server asks SQLite for
`strftime('%s','now')` and falls back to process time if needed.

Response formats:

- If `HX-Request` header is present: `text/plain`, body is the seconds value.
- If `Accept` contains `text/plain`: `text/plain`, body is the seconds value.
- Otherwise: JSON `{ "now": number }`.

All responses set `Cache-Control: no-store`.

## Players

### `GET /api/players`

Lists players.

Query params:

- `page`
- `rows`

Response `200`:

```ts
ApiResponse<Player[]>
```

### `GET /api/players/:id`

Fetches one player by id. The id is left-padded to 4 characters before lookup.

Response `200`:

```ts
ApiResponse<Player>
```

Response `404`:

```json
{
  "data": null,
  "errors": ["Player 'does-not-exist' not found"]
}
```

### `POST /api/players`

Creates a player from form data. This endpoint is form-oriented, not JSON.

Accepted body: `application/x-www-form-urlencoded` or `multipart/form-data`.

Fields:

- `name`: string.
- `hunter`: string; only exactly `"true"` becomes `true`, all other values
  become `false`.
- `allegiance`: string parsed as base-10 integer.
- `faction`: string, defaults to `""`.
- `neoId`: string.

Duplicate name handling:

- Names are compared case-insensitively across all players.
- Duplicate response: status `400`, plain text body
  `Player name is already taken`.

Success response:

- Redirect to `/players/{id}`.
- The source route uses Hono's default redirect status, `302`.

Generic failure response:

- Status `400`, plain text body `Failed to create player`.

### `POST /api/players/:id`

Updates an existing player from form data. This endpoint is form-oriented, not
JSON.

Accepted body: `application/x-www-form-urlencoded` or `multipart/form-data`.

Fields:

- `name`: string.
- `allegiance`: string parsed as base-10 integer.
- `faction`: string, defaults to `""`.
- `neoId`: string.

The `hunter` flag is not updated by this endpoint.

Success response:

- Redirect to `/players/{id}`.
- The source route uses Hono's default redirect status, `302`.

Failure response is JSON with the default `200` status from the source route:

```ts
{
  errors: unknown[];
  data: {
    id: string;
    name: string;
    allegiance: number;
    faction: string;
    neoId: string;
  };
}
```

## Matches

### `GET /api/matches`

Lists matches.

Query params:

- `page`
- `rows`

Response `200`:

```ts
ApiResponse<Match[]>
```

### `PUT /api/matches`

Adds matches. Existing `match_id` values are ignored by the database insert.

Request body: JSON.

```json
{
  "matches": [
    {
      "match_id": "match-1",
      "hunter": "0001",
      "bounty": "0002",
      "winner_is_hunter": true,
      "hunter_time": 123,
      "bounty_time": 456
    }
  ]
}
```

Validation requires:

- Top-level `matches` array.
- Each item has string `match_id`, `hunter`, and `bounty`.
- Each item has boolean `winner_is_hunter`.
- Each item has number `hunter_time` and `bounty_time`.

Response `200`:

```json
{
  "data": "1 Matches added",
  "errors": []
}
```

Response `400`:

```json
{
  "errors": ["Invalid format for matches"]
}
```

Note: the invalid response does not include a `data` field in the source server.

## Scores

### `GET /api/score/hunters`

Lists hunter wins, ordered by `wins` descending.

Query params:

- `page`
- `rows`

Response `200`:

```ts
ApiResponse<ScoreRow[]>
```

Rows include only players with at least one hunter win.

### `GET /api/score/bounties`

Lists bounty wins, ordered by `wins` descending.

Query params:

- `page`
- `rows`

Response `200`:

```ts
ApiResponse<ScoreRow[]>
```

Rows include only players with at least one bounty win.

### `GET /api/score/draw/average`

Lists average draw times across hunter and bounty roles, ordered by
`average_time` ascending.

Query params:

- `page`
- `rows`

Response `200`:

```ts
ApiResponse<AverageDrawTimeRow[]>
```

## Boxes

Box hack state is stored on `players.hacked_boxes_bits` as a bitset. Valid box
ids are integers `0` through `30`, inclusive. API responses expose the expanded
state as a boolean array with 31 entries.

### `GET /api/boxes`

Reads a player's box hack states.

Query params:

- `playerId`: player id.
- `userId`: accepted alias if `playerId` is absent.

Response `200`:

```ts
ApiResponse<boolean[]>
```

Response `400` when neither `playerId` nor `userId` is supplied:

```json
{
  "data": null,
  "errors": ["Expected playerId query parameter"]
}
```

Response `404` when the player is missing:

```json
{
  "data": null,
  "errors": ["Player '0001' not found"]
}
```

### `POST /api/boxes`

Sets or clears one player's box hack state.

Accepted body: JSON or form body.

Fields:

- `playerId`: string.
- `userId`: accepted alias if `playerId` is absent.
- `boxId`: integer or integer string.
- `hacked`: boolean, or string `"true"`, `"1"`, `"false"`, or `"0"`.

Response `200`:

```ts
ApiResponse<boolean[]>
```

Response `400` for missing or unparseable required fields:

```json
{
  "data": null,
  "errors": ["Expected playerId, numeric boxId, and hacked boolean"]
}
```

Response `400` for out-of-range box id:

```json
{
  "data": null,
  "errors": ["Unknown box id: 31"]
}
```

Response `404` when the player is missing:

```json
{
  "data": null,
  "errors": ["Player '0001' not found"]
}
```

## Database admin

These endpoints manage SQLite database files through the configured
`DbProvider`.

### `GET /api/dbs`

Lists available databases.

Response `200`:

```ts
ApiResponse<DbFileInfo[]>
```

### `POST /api/dbs`

Creates and seeds a new database.

Accepted body: JSON or form body.

Fields:

- `dbName`: string.
- `name`: accepted alias if `dbName` is absent.
- `playerCount`: string parsed as positive integer.
- `hunterName`: required string.
- `bountyName`: required string.

Success response `200`:

```ts
ApiResponse<DbFileInfo[]>
```

Failure response `400`:

```ts
ApiResponse<null>
```

Known validation errors:

- `Database name is required`
- `Player count must be a positive integer`
- `Hunter name is required`
- `Bounty name is required`

Provider errors are surfaced in `errors`, for example:

- `A database named 'event-1' already exists`
- `Database name must contain letters or numbers`

### `POST /api/dbs/select`

Selects the active database.

Accepted body: JSON or form body.

Accepted payload shapes:

```json
{ "name": "event-1" }
```

```json
{ "dbName": "event-1" }
```

The raw body value is also accepted by the parser if it can be parsed as a
request body by the framework.

Success response `200`:

```ts
ApiResponse<DbFileInfo[]>
```

Failure response `400`:

```ts
ApiResponse<null>
```

Known validation/provider errors:

- `Name is required`
- `Database file does not exist`
- `Invalid database name`
- `Failed to select database`

### `DELETE /api/dbs`

Deletes a database.

Accepted body: JSON or form body.

Accepted payload shapes are the same as `POST /api/dbs/select`.

Success response `200`:

```ts
ApiResponse<DbFileInfo[]>
```

Failure response `400`:

```ts
ApiResponse<null>
```

Known validation/provider errors:

- `Name is required`
- `Cannot delete the active database`
- `Invalid database name`
- `Failed to delete database`

## Server-sent events

### `GET /api/read-events`

Streams uFR read events as Server-Sent Events.

Event data is JSON stringified:

```ts
type UfrRead = {
  userId: string;
  readerId: string;
};
```

The source bus only publishes an event when the incoming uFR frame parses as a
query string with `holder_state=1`. It maps:

- `UID` to `userId`.
- `OSN` to `readerId`.

## WebSocket endpoints

These are outside `/api`, but they are server API surfaces in the source app.

### `GET /ufr/ws`

WebSocket upgrade endpoint for uFR readers.

Connection behavior:

- Accepts `/ufr/ws` and `/ufr/ws/`.
- On connect, the server records the reader device IP from forwarded headers or
  Bun request IP.
- The server attempts `POST http://{deviceIp}/info` with Basic auth from server
  config to discover `OSN` and `SN`.
- Incoming reader frames may be JSON, query-string text, or concatenated
  query-string events.
- Incoming frames are also fed to `/api/read-events` subscribers.

The server emits no JSON protocol to the reader. It responds with uFR COM command
response strings generated by `UfrMasterMode`.

Failed upgrade response:

- Status `400`, body `WebSocket upgrade failed`.

### `GET /ui/ws?stationId=...`

WebSocket upgrade endpoint for browser/operator clients that observe and control
uFR readers.

Required query param:

- `stationId`: string. Missing value returns status `401`, body
  `missing station id`.

Failed upgrade response:

- Status `400`, body `WebSocket upgrade failed`.

Client-to-server JSON messages:

```ts
type UiRequest =
  | { type: "list" }
  | { type: "bind"; onlineSerialNumber: string }
  | {
      type: "linear.set";
      onlineSerialNumber?: string;
      absBlock: number; // 0..255
      size: number; // 1..192
      authMode?: number; // default 96
      keyHex?: string; // normalized to 12 hex chars, default FFFFFFFFFFFF
    }
  | { type: "card.read" };
```

Server-to-client JSON messages:

```ts
type UiEvent =
  | { type: "hello"; clientId: string; stationId: string }
  | {
      type: "ufr.list";
      devices: Array<{
        onlineSerialNumber: string;
        nfcSerialNumber?: string;
        deviceIp?: string;
      }>;
    }
  | {
      type: "ufr.connected";
      deviceIp?: string;
      onlineSerialNumber?: string;
      nfcSerialNumber?: string;
    }
  | { type: "ufr.bound"; clientId: string; onlineSerialNumber: string }
  | {
      type: "ufr.linear.configured";
      onlineSerialNumber: string;
      changed: boolean;
      beginBytes: number;
      absBlock?: number;
      size: number;
      authMode: number;
      keyHex: string;
    }
  | { type: "ufr.linear.error"; onlineSerialNumber?: string; message: string }
  | {
      type: "ufr.event";
      deviceIp?: string;
      onlineSerialNumber?: string;
      nfcSerialNumber?: string;
      reader?: number | string;
      uid?: string;
      ctrlInfo?: string;
      data?: string;
      linearHex?: string;
      linearText?: string;
      linearBeginBytes?: number;
      linearAbsBlock?: number;
      linearSize?: number;
      alias?: string;
      aliasSource?: "ws";
      holderState?: string;
      online?: string;
      sequence?: number;
      raw?: string;
      rawFormat?: "json" | "querystring" | "unknown";
    }
  | {
      type: "scan-event.profile";
      osn: string;
      uid: string;
      seenAtMs: number;
      alias: string;
      allegiance: string;
      faction: string;
    }
  | {
      type: "scan-event.removed";
      osn: string;
      uid: string;
      seenAtMs: number;
    }
  | {
      type: "card.read.result";
      profile: Extract<UiEvent, { type: "scan-event.profile" }>;
    }
  | { type: "error"; message: string };
```

UI socket behavior:

- On open, server sends `hello`, then `ufr.list`.
- `list` sends `ufr.list`.
- `bind` records the selected reader and sends `ufr.bound`; if the reader is
  currently connected, it also sends `ufr.list` and `ufr.connected`.
- `card.read` requires a bound OSN. Without one, server sends
  `{ "type": "error", "message": "Not bound to an OSN (bind first)" }`.
- `linear.set` requires either `onlineSerialNumber` or a previous bind.
  Validation errors are sent as `ufr.linear.error`.
- Unknown or invalid JSON messages are answered with `error`.

## Source route map

The source app mounts these API routers:

- `/api/health-check`
- `/api/time`
- `/api/read-events`
- `/api/boxes`
- `/api/players`
- `/api/matches`
- `/api/score`
- `/api/dbs`
- `/ufr/ws`
- `/ui/ws`
