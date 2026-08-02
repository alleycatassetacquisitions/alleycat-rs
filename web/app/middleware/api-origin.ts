import { createContextKey, type Middleware } from 'remix/router'

export const ApiOrigin = createContextKey<URL>()
export const DEFAULT_API_ORIGIN = 'http://localhost:8000'

export function apiOrigin(value = process.env.API_ORIGIN): Middleware<{
  key: typeof ApiOrigin
  value: URL
}> {
  let origin = parseApiOrigin(value)

  return async (context, next) => {
    context.set(ApiOrigin, origin)
    return next()
  }
}

export function parseApiOrigin(value: string | undefined): URL {
  let candidate = value ?? DEFAULT_API_ORIGIN

  let url: URL
  try {
    url = new URL(candidate)
  } catch {
    throw new Error(`API_ORIGIN must be a valid URL; received ${JSON.stringify(candidate)}`)
  }

  if (url.protocol !== 'http:' && url.protocol !== 'https:') {
    throw new Error('API_ORIGIN must use http or https')
  }

  if (url.username || url.password || url.search || url.hash || url.pathname !== '/') {
    throw new Error('API_ORIGIN must contain only the scheme, host, and optional port')
  }

  return url
}
