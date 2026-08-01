# Web

A minimal Remix application starter with a home page.

## Starter Shape

- `app/actions/controller.tsx` owns the top-level route actions.
- `app/routes.ts` defines the route contract.
- `app/router.ts` wires routes to handlers.
- `app/middleware/render.tsx` installs the request-scoped renderer used by actions.
- `app/ui/` holds the shared document shell and home page UI.
- `app/assets.ts` owns the server-side asset pipeline used by the asset route and renderer.
- `public/` contains static files served from the app root.

## Growing The App

- Put top-level route actions in `app/actions/controller.tsx`.
- Add `app/actions/<route-key>/controller.tsx` when a nested route map needs its own actions or middleware.
- Add directories like `app/data/` or `test/` when the app actually needs them.
- Move shared UI into `app/ui/` once more than one route needs it.

## Commands

```sh
npm i
npm run dev
npm run start
npm test
npm run typecheck
```

`API_ORIGIN` is the URL the UI server uses to reach the Rust API and defaults to
`http://localhost:8000`. Override it when needed, for example with
`API_ORIGIN=https://api.example.com npm run start`. It is only read by the UI
server; browsers continue to make same-origin requests to the UI. Docker
Compose supplies `http://app:8000` automatically.
