// Anna host connection + per-user storage, shared by the main panel and the
// floating pet mode (both run from index.html; see app.js main()).
import { unwrap } from './core.js';

export const host = { anna: null };
let connecting;

/** Connect once per window; later calls get the same runtime (or null outside Anna). */
export function connectHost() {
  connecting ||= (async () => {
    try {
      const { AnnaAppRuntime } = await import('/static/anna-apps/_sdk/latest/index.js');
      host.anna = await AnnaAppRuntime.connect();
    } catch (e) {
      console.warn('[companion] no Anna host:', e);
      host.anna = null;
    }
    return host.anna;
  })();
  return connecting;
}

export async function load(key, fallback) {
  if (!host.anna) return fallback;
  try {
    const r = unwrap(await host.anna.storage.get({ key }));
    return r && r.exists !== false && r.value != null ? r.value : fallback;
  } catch (e) {
    console.warn('[companion] storage.get', key, e);
    return fallback;
  }
}

export async function save(key, value) {
  if (!host.anna) return;
  try {
    unwrap(await host.anna.storage.set({ key, value }));
  } catch (e) {
    console.warn('[companion] storage.set', key, e);
  }
}
