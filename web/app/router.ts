import { createRouter, type MiddlewareContext } from 'remix/router'
import { staticFiles } from 'remix/middleware/static'

import controller from './actions/controller.tsx'
import playersController from './actions/players/controller.tsx'
import { apiOrigin } from './middleware/api-origin.ts'
import { render } from './middleware/render.tsx'
import { routes } from './routes.ts'

type AppContext = MiddlewareContext<[ReturnType<typeof apiOrigin>, ReturnType<typeof render>]>

declare module 'remix/router' {
  interface RouterTypes {
    context: AppContext
  }
}

export const router = createRouter<AppContext>({
  middleware: [staticFiles('./public', { index: false }), apiOrigin(), render()],
})

router.map(routes, controller)
router.map(routes.players, playersController)
