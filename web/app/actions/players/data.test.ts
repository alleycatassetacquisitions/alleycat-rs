import assert from 'node:assert/strict'
import { describe, it } from 'node:test'

import { getPlayers } from './data.ts'

describe('getPlayers', () => {
  it('retrieves and validates players from the API', async () => {
    let result = await getPlayers(new URL('http://api.example.test:8000'), undefined, async (input) => {
      assert.equal(input.toString(), 'http://api.example.test:8000/players')

      return Response.json({
        players: [
          {
            id: '7f3df51e-0e66-4e70-98ee-80d775adf76f',
            pdn_code: 'ABC',
            name: 'Martha Wells',
            created_at: '2026-08-01T12:00:00Z',
            mode: 'unassigned',
          },
        ],
        pagination: { page: 1, per_page: 20 },
      })
    })

    assert.equal(result.players[0]?.name, 'Martha Wells')
    assert.deepEqual(result.pagination, { page: 1, per_page: 20 })
  })

  it('rejects a response that does not match the API contract', async () => {
    await assert.rejects(
      getPlayers(new URL('http://api.example.test:8000'), undefined, async () =>
        Response.json({ players: 'not-an-array', pagination: {} }),
      ),
    )
  })
})
