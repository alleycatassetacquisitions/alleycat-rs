import { createController } from 'remix/router'

import { ApiOrigin } from '../../middleware/api-origin.ts'
import { routes } from '../../routes.ts'
import { getPlayers } from './data.ts'
import { PlayersPage } from './show-page.tsx'

export default createController(routes.players, {
  actions: {
    async show(context) {
      try {
        let { players, pagination } = await getPlayers(
          context.get(ApiOrigin),
          context.request.signal,
        )
        return context.render(<PlayersPage players={players} pagination={pagination} />)
      } catch (error) {
        if (context.request.signal.aborted) throw error

        console.error('Failed to retrieve players from the API', error)
        return new Response('Unable to retrieve players', { status: 502 })
      }
    },
  },
})
