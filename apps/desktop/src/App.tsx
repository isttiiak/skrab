import { useEffect, useState } from 'react';
import { Toaster } from 'sonner';
import { ClipboardPanel } from '@/components/clipboard/ClipboardPanel';
import { EditorView } from '@/components/editor/EditorView';
import { CaptureOverlay } from '@/components/screenshot/CaptureOverlay';
import { SettingsPanel } from '@/components/settings/SettingsPanel';
import { getSettings } from '@/lib/tauri';

type View = 'clips' | 'settings';

/** Applies the stored theme preference, following the OS when set to `system`. */
function useTheme() {
  useEffect(() => {
    const media = window.matchMedia('(prefers-color-scheme: dark)');

    let preference = 'system';
    const apply = () => {
      const dark = preference === 'dark' || (preference === 'system' && media.matches);
      document.documentElement.classList.toggle('dark', dark);
    };

    let active = true;
    void getSettings()
      .then((settings) => {
        if (!active) return;
        preference = settings.theme;
        apply();
      })
      .catch(() => apply());

    apply();
    media.addEventListener('change', apply);
    return () => {
      active = false;
      media.removeEventListener('change', apply);
    };
  }, []);
}

export function App() {
  const [view, setView] = useState<View>('clips');
  useTheme();

  // The pinned widget is a second window loading the same bundle; the query string
  // decides which surface to render rather than pulling in a router for two views.
  const params = new URLSearchParams(window.location.search);
  const surface = params.get('view');
  if (surface === 'overlay') return <CaptureOverlay />;
  if (surface === 'editor') {
    const id = params.get('id');
    return id ? <EditorView clipId={id} /> : <p className="p-4 text-sm">No image selected.</p>;
  }

  return (
    <main className="h-full">
      {view === 'clips' ? (
        <ClipboardPanel onOpenSettings={() => setView('settings')} />
      ) : (
        <SettingsPanel onBack={() => setView('clips')} />
      )}
      <Toaster position="bottom-center" richColors toastOptions={{ duration: 1800 }} />
    </main>
  );
}
