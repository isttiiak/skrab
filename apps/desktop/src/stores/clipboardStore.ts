import type { ClipItem, ClipType } from '@skrab/ipc-types';
import { create } from 'zustand';
import * as api from '@/lib/tauri';

/** How many clips one page of the history list holds. */
const PAGE_SIZE = 60;

export type TypeFilter = ClipType | 'all' | 'favorites';

type ClipboardState = {
  clips: ClipItem[];
  search: string;
  filter: TypeFilter;
  selectedIndex: number;
  loading: boolean;
  error: string | null;

  setSearch: (value: string) => void;
  setFilter: (value: TypeFilter) => void;
  setSelectedIndex: (index: number) => void;
  moveSelection: (delta: number) => void;

  refresh: () => Promise<void>;
  copy: (id: string) => Promise<void>;
  toggleFavorite: (id: string) => Promise<void>;
  togglePinned: (id: string) => Promise<void>;
  remove: (id: string) => Promise<void>;
  clearAll: () => Promise<void>;
};

/** Translates the UI's single filter chip into backend query fields. */
function queryFor(search: string, filter: TypeFilter) {
  const trimmed = search.trim();
  return {
    search: trimmed.length > 0 ? trimmed : null,
    clipType: filter === 'all' || filter === 'favorites' ? null : filter,
    favoritesOnly: filter === 'favorites',
    pinnedOnly: null,
    limit: PAGE_SIZE,
    offset: 0,
  };
}

function describe(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

export const useClipboardStore = create<ClipboardState>((set, get) => ({
  clips: [],
  search: '',
  filter: 'all',
  selectedIndex: 0,
  loading: false,
  error: null,

  setSearch(value) {
    // Reset the cursor: keeping index 3 while the result set changes underneath
    // means Enter pastes something the user never looked at.
    set({ search: value, selectedIndex: 0 });
    void get().refresh();
  },

  setFilter(value) {
    set({ filter: value, selectedIndex: 0 });
    void get().refresh();
  },

  setSelectedIndex(index) {
    set({ selectedIndex: index });
  },

  moveSelection(delta) {
    const { clips, selectedIndex } = get();
    if (clips.length === 0) return;
    const next = Math.min(Math.max(selectedIndex + delta, 0), clips.length - 1);
    set({ selectedIndex: next });
  },

  async refresh() {
    const { search, filter } = get();
    set({ loading: true });
    try {
      const clips = await api.listClips(queryFor(search, filter));
      set((state) => ({
        clips,
        loading: false,
        error: null,
        // Clamp rather than reset, so a background refresh (a new clip arriving)
        // doesn't yank the cursor away from whatever the user is on.
        selectedIndex: Math.min(state.selectedIndex, Math.max(clips.length - 1, 0)),
      }));
    } catch (error) {
      set({ loading: false, error: describe(error) });
    }
  },

  async copy(id) {
    try {
      await api.copyClip(id);
    } catch (error) {
      set({ error: describe(error) });
      throw error;
    }
  },

  async toggleFavorite(id) {
    const clip = get().clips.find((c) => c.id === id);
    if (!clip) return;
    // Optimistic: the round trip is fast, and a star that lags feels broken.
    set((state) => ({
      clips: state.clips.map((c) => (c.id === id ? { ...c, isFavorite: !c.isFavorite } : c)),
    }));
    try {
      await api.setClipFavorite(id, !clip.isFavorite);
    } catch (error) {
      set({ error: describe(error) });
      await get().refresh();
    }
  },

  async togglePinned(id) {
    const clip = get().clips.find((c) => c.id === id);
    if (!clip) return;
    try {
      await api.setClipPinned(id, !clip.isPinned);
      // Pinning reorders the list, so re-query rather than patching in place.
      await get().refresh();
    } catch (error) {
      set({ error: describe(error) });
    }
  },

  async remove(id) {
    set((state) => ({ clips: state.clips.filter((c) => c.id !== id) }));
    try {
      await api.deleteClip(id);
    } catch (error) {
      set({ error: describe(error) });
      await get().refresh();
    }
  },

  async clearAll() {
    try {
      await api.clearHistory();
      await get().refresh();
    } catch (error) {
      set({ error: describe(error) });
    }
  },
}));
