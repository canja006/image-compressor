import { getThumbnail } from './tauri'

// Module-level cache so thumbnails survive row remounts/re-sorts and are fetched at most once.
const cache = new Map<string, string>()
const pending = new Map<string, Promise<string | null>>()

/** Synchronously read an already-loaded thumbnail, if any. */
export function cachedThumbnail(path: string): string | null {
  return cache.get(path) ?? null
}

/** Load (and cache) a thumbnail data URL for a path, de-duplicating concurrent requests. */
export function loadThumbnail(path: string, max = 128): Promise<string | null> {
  const cached = cache.get(path)
  if (cached) return Promise.resolve(cached)

  const inflight = pending.get(path)
  if (inflight) return inflight

  const promise = getThumbnail(path, max)
    .then((url) => {
      if (url) cache.set(path, url)
      pending.delete(path)
      return url
    })
    .catch(() => {
      pending.delete(path)
      return null
    })
  pending.set(path, promise)
  return promise
}
