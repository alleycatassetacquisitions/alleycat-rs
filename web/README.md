# Alleycat web UI

This directory contains the server-rendered operator UI. A Node HTTP server
runs Remix, calls the Rust API from the server, validates each API response,
and returns HTML to the browser.

Current pages are:

- `/` — placeholder Remix starter home
- `/players` — first page of recently registered players
- `/device-logs` — first page of recent device crash logs

For the full-system view, see the repository's
[architecture guide](../docs/architecture.md). For setup and deployment, see
the [developer guide](../docs/development.md) and
[operations runbook](../docs/operations.md).

## Run natively

Requires Node.js `>=24.3.0` and a Rust API at `http://localhost:8000`:

```bash
npm ci
npm run dev
```

The native server defaults to <http://localhost:44100>. Use
`PORT=3000 npm run dev` to match the Compose port.

Other commands:

```bash
npm run start
npm test
npm run typecheck
```

## Runtime configuration

- `PORT` is the Node listen port. Native runs default to `44100`; the container
  sets `3000`.
- `API_ORIGIN` is the origin the Node server uses for Rust API calls. It
  defaults to `http://localhost:8000`; Compose supplies `http://app:8000`, and
  DigitalOcean supplies the API component's private URL.

`API_ORIGIN` must be an HTTP(S) origin only—no credentials, path, query, or
fragment. It is server-side; browsers call the UI.

## Code map

| Path | Responsibility |
| --- | --- |
| `server.ts` | Node HTTP listener and signal-triggered shutdown |
| `app/routes.ts` | Named route contract |
| `app/router.ts` | Global middleware and controller wiring |
| `app/actions/controller.tsx` | Home and asset actions |
| `app/actions/<feature>/controller.tsx` | Feature request handling and error responses |
| `app/actions/<feature>/data.ts` | API request plus runtime response validation |
| `app/actions/<feature>/show-page.tsx` | Server-rendered feature page |
| `app/middleware/api-origin.ts` | Validated request-scoped API origin |
| `app/middleware/render.tsx` | Request-scoped renderer |
| `app/ui/` | Shared document shell and UI |
| `app/assets.ts` | Server-side asset pipeline |
| `public/` | Static files served at the application root |

## Add a page

1. Add its path to `app/routes.ts`.
2. Add a controller, data module, and page under `app/actions/<feature>/`.
3. Map the controller in `app/router.ts`.
4. Validate API data at the `data.ts` boundary rather than passing untrusted
   JSON into the UI.
5. Add tests, then run `npm test` and `npm run typecheck`.
