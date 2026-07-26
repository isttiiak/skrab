import type { AppInfo, AppSettings } from '@skrab/ipc-types';
import { ArrowLeft, FolderOpen, ShieldCheck, Trash2 } from 'lucide-react';
import { useCallback, useEffect, useState } from 'react';
import { toast } from 'sonner';
import { getAppInfo, getSettings, openDataDir, saveSettings } from '@/lib/tauri';
import { cn } from '@/lib/utils';
import { useClipboardStore } from '@/stores/clipboardStore';

const RETENTION_CHOICES = [
  { value: 7, label: '7 days' },
  { value: 30, label: '30 days' },
  { value: 90, label: '90 days' },
  { value: 0, label: 'Forever' },
];

export function SettingsPanel({ onBack }: { onBack: () => void }) {
  const [settings, setSettings] = useState<AppSettings | null>(null);
  const [info, setInfo] = useState<AppInfo | null>(null);
  const clearAll = useClipboardStore((s) => s.clearAll);

  useEffect(() => {
    void getSettings()
      .then(setSettings)
      .catch(() => toast.error('Could not load settings'));
    void getAppInfo()
      .then(setInfo)
      .catch(() => undefined);
  }, []);

  const patch = useCallback(
    async (changes: Partial<AppSettings>) => {
      if (!settings) return;
      const next = { ...settings, ...changes };
      // Optimistic so toggles feel instant; the backend sanitizes and returns the
      // authoritative copy, which we then adopt.
      setSettings(next);
      try {
        setSettings(await saveSettings(next));
      } catch {
        toast.error('Could not save settings');
        setSettings(settings);
      }
    },
    [settings],
  );

  if (!settings) {
    return <p className="text-muted-foreground p-4 text-sm">Loading…</p>;
  }

  return (
    <div className="flex h-full flex-col">
      <header className="border-border flex shrink-0 items-center gap-2 border-b px-3 py-2.5">
        <button
          type="button"
          onClick={onBack}
          aria-label="Back to clipboard"
          className="text-muted-foreground hover:bg-surface-muted hover:text-foreground rounded-md p-1.5 transition-colors"
        >
          <ArrowLeft size={15} />
        </button>
        <h1 className="text-sm font-semibold tracking-tight">Settings</h1>
      </header>

      <div className="min-h-0 flex-1 space-y-5 overflow-y-auto px-3 py-3">
        <Section title="Capture">
          <Toggle
            label="Watch the clipboard"
            hint="Turn off to stop recording without quitting Skrab."
            checked={settings.monitoringEnabled}
            onChange={(v) => void patch({ monitoringEnabled: v })}
          />
          <Field label="Check every" hint="Lower feels snappier and costs a little more.">
            <select
              value={settings.pollIntervalMs}
              onChange={(e) => void patch({ pollIntervalMs: Number(e.target.value) })}
              className="border-border bg-surface rounded-md border px-2 py-1 text-xs"
            >
              <option value={150}>150 ms</option>
              <option value={250}>250 ms</option>
              <option value={500}>500 ms</option>
              <option value={1000}>1 s</option>
            </select>
          </Field>
        </Section>

        <Section title="Privacy" icon={<ShieldCheck size={13} className="text-primary" />}>
          <p className="text-muted-foreground bg-primary-soft/40 rounded-lg px-2.5 py-2 text-[11px] leading-relaxed">
            Skrab always skips content that your password manager marks as concealed. Everything
            stays in an encrypted database on this machine.
          </p>
          <Toggle
            label="Also skip things that look like secrets"
            hint="API keys, tokens and private keys, even without an OS marker."
            checked={settings.skipSecretPatterns}
            onChange={(v) => void patch({ skipSecretPatterns: v })}
          />
          <Field label="Never record from" hint="One app name per line, matched loosely.">
            <textarea
              rows={3}
              defaultValue={settings.blockedApps.join('\n')}
              onBlur={(e) =>
                void patch({
                  blockedApps: e.target.value
                    .split('\n')
                    .map((s) => s.trim())
                    .filter(Boolean),
                })
              }
              placeholder="1Password&#10;Bitwarden"
              className="border-border bg-surface w-full rounded-md border px-2 py-1.5 text-xs"
            />
          </Field>
        </Section>

        <Section title="History">
          <Field label="Keep clips for">
            <div className="flex gap-1">
              {RETENTION_CHOICES.map((choice) => (
                <button
                  key={choice.value}
                  type="button"
                  onClick={() => void patch({ retentionDays: choice.value })}
                  className={cn(
                    'rounded-full px-2 py-1 text-[11px] font-medium transition-colors',
                    settings.retentionDays === choice.value
                      ? 'gradient-brand text-white'
                      : 'bg-surface-muted text-muted-foreground hover:text-foreground',
                  )}
                >
                  {choice.label}
                </button>
              ))}
            </div>
          </Field>
          <Field label="Maximum clips" hint="Starred and pinned items are never removed.">
            <input
              type="number"
              min={50}
              max={100000}
              defaultValue={settings.maxItems}
              onBlur={(e) => void patch({ maxItems: Number(e.target.value) })}
              className="border-border bg-surface w-24 rounded-md border px-2 py-1 text-xs"
            />
          </Field>
        </Section>

        <Section title="General">
          <Toggle
            label="Launch at login"
            checked={settings.launchAtLogin}
            onChange={(v) => void patch({ launchAtLogin: v })}
          />
          <Field label="Theme">
            <div className="flex gap-1">
              {['system', 'light', 'dark'].map((theme) => (
                <button
                  key={theme}
                  type="button"
                  onClick={() => void patch({ theme })}
                  className={cn(
                    'rounded-full px-2.5 py-1 text-[11px] font-medium capitalize transition-colors',
                    settings.theme === theme
                      ? 'gradient-brand text-white'
                      : 'bg-surface-muted text-muted-foreground hover:text-foreground',
                  )}
                >
                  {theme}
                </button>
              ))}
            </div>
          </Field>
        </Section>

        <Section title="Data">
          <button
            type="button"
            onClick={() => void openDataDir()}
            className="border-border hover:bg-surface-muted flex w-full items-center gap-2 rounded-lg border px-2.5 py-2 text-xs transition-colors"
          >
            <FolderOpen size={14} className="text-muted-foreground" />
            Open data folder
          </button>
          <button
            type="button"
            onClick={() => {
              void clearAll();
              toast.success('History cleared (starred and pinned kept)');
            }}
            className="border-destructive/30 text-destructive hover:bg-destructive-soft flex w-full items-center gap-2 rounded-lg border px-2.5 py-2 text-xs transition-colors"
          >
            <Trash2 size={14} />
            Clear history
          </button>
        </Section>

        {info && (
          <p className="text-muted-foreground pt-1 text-center text-[10px]">
            Skrab {info.version} · Tauri {info.tauriVersion} · {info.os}
          </p>
        )}
      </div>
    </div>
  );
}

function Section({
  title,
  icon,
  children,
}: {
  title: string;
  icon?: React.ReactNode;
  children: React.ReactNode;
}) {
  return (
    <section className="space-y-2.5">
      <h2 className="text-muted-foreground flex items-center gap-1.5 text-[11px] font-semibold tracking-wide uppercase">
        {icon}
        {title}
      </h2>
      {children}
    </section>
  );
}

function Field({
  label,
  hint,
  children,
}: {
  label: string;
  hint?: string;
  children: React.ReactNode;
}) {
  return (
    <div className="space-y-1">
      <div className="flex items-center justify-between gap-3">
        <span className="text-xs font-medium">{label}</span>
        {children}
      </div>
      {hint && <p className="text-muted-foreground text-[10px]">{hint}</p>}
    </div>
  );
}

function Toggle({
  label,
  hint,
  checked,
  onChange,
}: {
  label: string;
  hint?: string;
  checked: boolean;
  onChange: (value: boolean) => void;
}) {
  return (
    <div className="space-y-1">
      <label className="flex cursor-pointer items-center justify-between gap-3">
        <span className="text-xs font-medium">{label}</span>
        <input
          type="checkbox"
          checked={checked}
          onChange={(e) => onChange(e.target.checked)}
          className="peer sr-only"
        />
        <span
          aria-hidden
          className={cn(
            'relative h-5 w-9 shrink-0 rounded-full transition-colors',
            checked ? 'gradient-brand' : 'bg-surface-muted border-border border',
          )}
        >
          <span
            className={cn(
              'absolute top-0.5 h-4 w-4 rounded-full bg-white shadow transition-transform',
              checked ? 'translate-x-[1.15rem]' : 'translate-x-0.5',
            )}
          />
        </span>
      </label>
      {hint && <p className="text-muted-foreground text-[10px]">{hint}</p>}
    </div>
  );
}
