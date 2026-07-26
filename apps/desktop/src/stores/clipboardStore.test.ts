import type { ClipItem } from '@skrab/ipc-types';
import { beforeEach, describe, expect, it, vi } from 'vitest';

const api = vi.hoisted(() => ({
  listClips: vi.fn(),
  copyClip: vi.fn(),
  setClipFavorite: vi.fn(),
  setClipPinned: vi.fn(),
  deleteClip: vi.fn(),
  clearHistory: vi.fn(),
}));
vi.mock('@/lib/tauri', () => api);

const { useClipboardStore } = await import('./clipboardStore.ts');

function clip(id: string, preview = id): ClipItem {
  return {
    id,
    clipType: 'text',
    preview,
    thumb: null,
    imagePath: null,
    sizeBytes: preview.length,
    sourceApp: 'Test',
    isPinned: false,
    isFavorite: false,
    category: null,
    createdAt: 1_700_000_000_000,
    accessedAt: 1_700_000_000_000,
  };
}

describe('clipboardStore', () => {
  beforeEach(() => {
    for (const fn of Object.values(api)) fn.mockReset();
    useClipboardStore.setState({
      clips: [],
      search: '',
      filter: 'all',
      selectedIndex: 0,
      loading: false,
      error: null,
    });
  });

  it('maps the filter chip onto backend query fields', async () => {
    api.listClips.mockResolvedValue([]);
    useClipboardStore.setState({ filter: 'favorites', search: '  hello  ' });

    await useClipboardStore.getState().refresh();

    expect(api.listClips).toHaveBeenCalledWith(
      expect.objectContaining({ search: 'hello', favoritesOnly: true, clipType: null }),
    );
  });

  it('sends no search term when the box is blank', async () => {
    api.listClips.mockResolvedValue([]);
    useClipboardStore.setState({ search: '   ' });

    await useClipboardStore.getState().refresh();

    expect(api.listClips).toHaveBeenCalledWith(expect.objectContaining({ search: null }));
  });

  it('clamps the selection when a refresh returns fewer clips', async () => {
    // A background purge can shrink the list under the cursor; the index must not
    // dangle past the end or Enter would copy nothing.
    useClipboardStore.setState({ clips: [clip('a'), clip('b'), clip('c')], selectedIndex: 2 });
    api.listClips.mockResolvedValue([clip('a')]);

    await useClipboardStore.getState().refresh();

    expect(useClipboardStore.getState().selectedIndex).toBe(0);
  });

  it('does not blank the list when a copy fails', async () => {
    // `error` state replaces the whole history with a red message, so a failed copy
    // must not set it — the caller shows a toast instead.
    useClipboardStore.setState({ clips: [clip('a')] });
    api.copyClip.mockRejectedValue(new Error('clipboard unavailable'));

    await expect(useClipboardStore.getState().copy('a')).rejects.toThrow('clipboard unavailable');

    const state = useClipboardStore.getState();
    expect(state.error).toBeNull();
    expect(state.clips).toHaveLength(1);
  });

  it('rolls the list back when a delete fails', async () => {
    useClipboardStore.setState({ clips: [clip('a'), clip('b')] });
    api.deleteClip.mockRejectedValue(new Error('locked'));
    api.listClips.mockResolvedValue([clip('a'), clip('b')]);

    await useClipboardStore.getState().remove('a');

    expect(useClipboardStore.getState().clips).toHaveLength(2);
  });

  it('keeps the selection inside the list bounds', () => {
    useClipboardStore.setState({ clips: [clip('a'), clip('b')], selectedIndex: 0 });
    const { moveSelection } = useClipboardStore.getState();

    moveSelection(-1);
    expect(useClipboardStore.getState().selectedIndex).toBe(0);

    moveSelection(5);
    expect(useClipboardStore.getState().selectedIndex).toBe(1);
  });
});
