"use client";
import { useThemeSettings } from '@/components/app/ThemeProvider';

export default function SettingsPage() {
  const { settings, set, reset } = useThemeSettings();
  return (
    <div className="h-full p-6">
      <h1 className="mb-4 text-xl font-semibold">Settings</h1>
      <div className="space-y-4">
        <Card title="Appearance">
          <Row label="Accent color">
            <select className="rounded-md bg-surface px-2 py-1 text-sm" value={settings.accent} onChange={(e) => set('accent', e.target.value as any)}>
              <option value="teal">Teal</option>
              <option value="blue">Blue</option>
              <option value="purple">Purple</option>
              <option value="lime">Lime</option>
            </select>
          </Row>
          <Row label="UI density">
            <select className="rounded-md bg-surface px-2 py-1 text-sm" value={settings.density} onChange={(e) => set('density', e.target.value as any)}>
              <option value="comfortable">Comfortable</option>
              <option value="compact">Compact</option>
            </select>
          </Row>
          <Row label="Font size">
            <select className="rounded-md bg-surface px-2 py-1 text-sm" value={settings.fontSize} onChange={(e) => set('fontSize', e.target.value as any)}>
              <option value="small">Small</option>
              <option value="default">Default</option>
              <option value="large">Large</option>
            </select>
          </Row>
          <Row label="Code ligatures">
            <Switch checked={settings.codeLigatures} onChange={(v) => set('codeLigatures', v)} />
          </Row>
        </Card>
        <Card title="Chat">
          <Row label="Inline tool logs">
            <Switch checked={settings.inlineToolLogs} onChange={(v) => set('inlineToolLogs', v)} />
          </Row>
        </Card>
        <div>
          <button className="rounded-md bg-surface px-3 py-1 text-sm hover:bg-bg-1" onClick={reset}>Reset to defaults</button>
        </div>
      </div>
    </div>
  );
}

function Card({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <div className="rounded-md bg-bg-1 p-4 shadow-card">
      <div className="mb-3 text-sm font-semibold text-text">{title}</div>
      <div className="space-y-3">{children}</div>
    </div>
  );
}

function Row({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div className="flex items-center gap-3">
      <div className="w-48 text-sm text-text-dim">{label}</div>
      <div className="flex-1">{children}</div>
    </div>
  );
}

function Switch({ checked, onChange }: { checked: boolean; onChange: (v: boolean) => void }) {
  return (
    <button
      role="switch"
      aria-checked={checked}
      className={`inline-flex items-center rounded-full px-1 py-1 ${checked ? 'bg-accent-500' : 'bg-surface'}`}
      onClick={() => onChange(!checked)}
    >
      <span className={`h-4 w-4 rounded-full bg-black transition-transform ${checked ? 'translate-x-4' : ''}`} />
    </button>
  );
}

