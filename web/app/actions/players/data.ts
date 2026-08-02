import * as s from 'remix/data-schema'
import type { InferOutput } from 'remix/data-schema'

const playerSchema = s.object({
  id: s.string(),
  pdn_code: s.string(),
  name: s.string(),
  created_at: s
    .string()
    .refine((value) => !Number.isNaN(Date.parse(value)), 'Expected a valid date'),
  mode: s.enum_(['unassigned', 'hunter', 'bounty']),
})

const playersResponseSchema = s.object({
  players: s.array(playerSchema),
  pagination: s.object({
    page: s.number(),
    per_page: s.number(),
  }),
})

export type Player = InferOutput<typeof playerSchema>
export type PlayersResponse = InferOutput<typeof playersResponseSchema>

export async function getPlayers(
  apiOrigin: URL,
  signal?: AbortSignal,
  fetcher: typeof fetch = fetch,
): Promise<PlayersResponse> {
  let url = new URL('/players', apiOrigin)
  let response = await fetcher(url, {
    headers: { Accept: 'application/json' },
    signal,
  })

  if (!response.ok) {
    throw new Error(`Players API returned ${response.status} ${response.statusText}`)
  }

  return s.parse(playersResponseSchema, await response.json())
}
