import { beforeEach, describe, expect, it, vi } from 'vitest';

const invoke = vi.hoisted(() => vi.fn());
vi.mock('@tauri-apps/api/core', () => ({ invoke }));

const { getAppInfo, listClips, setClipFavorite } = await import('./tauri.ts');

describe('tauri command wrappers', () => {
  // Block body on purpose: `mockReset()` returns the mock, and a function returned
  // from beforeEach is treated by Vitest as a teardown callback — which would then
  // call the still-rejecting mock after every test.
  beforeEach(() => {
    invoke.mockReset();
  });

  it('passes the resolved value through unchanged', async () => {
    const info = { name: 'Skrab', version: '0.1.0', tauriVersion: '2.11.5', os: 'macos' };
    invoke.mockResolvedValue(info);

    await expect(getAppInfo()).resolves.toEqual(info);
    expect(invoke).toHaveBeenCalledWith('get_app_info', undefined);
  });

  it('forwards arguments under the names the Rust commands expect', async () => {
    invoke.mockResolvedValue(undefined);

    await setClipFavorite('clip-1', true);

    // Tauri matches these keys to the Rust parameter names; a rename on either
    // side has to break here rather than silently pass `undefined`.
    expect(invoke).toHaveBeenCalledWith('set_clip_favorite', { id: 'clip-1', value: true });
  });

  it('wraps the query object so it arrives as a single `query` parameter', async () => {
    invoke.mockResolvedValue([]);
    const query = {
      search: 'foo',
      clipType: null,
      favoritesOnly: false,
      pinnedOnly: null,
      limit: 60,
      offset: 0,
    };

    await listClips(query);

    expect(invoke).toHaveBeenCalledWith('list_clips', { query });
  });

  it('turns a Rust string rejection into an Error naming the command', async () => {
    // Tauri rejects with the serialized `Error`, which is a bare string — not an
    // Error instance. Without wrapping, `e.message` in the UI would be undefined.
    invoke.mockRejectedValue('database is locked');

    await expect(getAppInfo()).rejects.toThrow('get_app_info failed: database is locked');
  });

  it('preserves the original rejection as the error cause', async () => {
    invoke.mockRejectedValue('disk full');

    const error = await getAppInfo().catch((e: unknown) => e);

    expect(error).toBeInstanceOf(Error);
    expect((error as Error).cause).toBe('disk full');
  });
});
