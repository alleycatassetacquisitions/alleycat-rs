import assert from 'node:assert/strict'
import { describe, it } from 'node:test'

import { parseApiOrigin } from './api-origin.ts'

describe('parseApiOrigin', () => {
  it('accepts an HTTP origin', () => {
    assert.equal(parseApiOrigin('http://localhost:8000').href, 'http://localhost:8000/')
  })

  it('defaults to the local API', () => {
    assert.equal(parseApiOrigin(undefined).href, 'http://localhost:8000/')
  })

  it('rejects a URL with a path', () => {
    assert.throws(
      () => parseApiOrigin('https://api.example.com/v1'),
      /only the scheme, host, and optional port/,
    )
  })
})
