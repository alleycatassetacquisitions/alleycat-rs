import type { Handle } from 'remix/ui'
import { css } from 'remix/ui'

import { Document } from '../../ui/document.tsx'
import type { Player, PlayersResponse } from './data.ts'

interface PlayersPageProps {
  players: Player[]
  pagination: PlayersResponse['pagination']
}

export function PlayersPage(handle: Handle<PlayersPageProps>) {
  return () => {
    let { players, pagination } = handle.props

    return (
      <Document title="Players">
        <main
          mix={css({
            maxWidth: '56rem',
            margin: '0 auto',
            padding: '4rem 1.5rem',
            fontFamily: 'system-ui, sans-serif',
          })}
        >
          <h1 mix={css({ margin: '0 0 2rem' })}>Players</h1>
          {players.length === 0 ? (
            <p>No players have registered yet.</p>
          ) : (
            <ul
              mix={css({
                listStyle: 'none',
                margin: 0,
                padding: 0,
                display: 'grid',
                gap: '1rem',
              })}
            >
              {players.map((player) => (
                <li
                  key={player.id}
                  mix={css({
                    border: '1px solid #d1d5db',
                    borderRadius: '0.5rem',
                    padding: '1rem',
                  })}
                >
                  <strong>{player.name}</strong>
                  <dl
                    mix={css({
                      display: 'grid',
                      gridTemplateColumns: 'max-content 1fr',
                      gap: '0.25rem 0.75rem',
                      margin: '0.75rem 0 0',
                    })}
                  >
                    <dt>PDN code</dt>
                    <dd mix={css({ margin: 0 })}>{player.pdn_code}</dd>
                    <dt>Mode</dt>
                    <dd mix={css({ margin: 0 })}>{player.mode}</dd>
                    <dt>Registered</dt>
                    <dd mix={css({ margin: 0 })}>
                      <time dateTime={player.created_at}>
                        {new Date(player.created_at).toLocaleString('en-US')}
                      </time>
                    </dd>
                  </dl>
                </li>
              ))}
            </ul>
          )}
          <p mix={css({ marginTop: '2rem', color: '#4b5563' })}>
            Page {pagination.page} · Up to {pagination.per_page} players per page
          </p>
        </main>
      </Document>
    )
  }
}
