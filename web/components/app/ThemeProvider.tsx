"use client";
import { createContext, useContext, useEffect, useMemo, useState } from 'react';

type Accent = 'teal' | 'blue' | 'purple' | 'lime';
type Density = 'comfortable' | 'compact';
type FontSize = 'small' | 'default' | 'large';

type Settings = {
  accent: Accent;
  density: Density;
  fontSize: FontSize;
  codeLigatures: boolean;
  inlineToolLogs: boolean;
};

const defaultSettings: Settings = {
  accent: 'teal',
  density: 'comfortable',
  fontSize: 'default',
  codeLigatures: false,
  inlineToolLogs: true,
};

type Ctx = {
  settings: Settings;
  set: <K extends keyof Settings>(key: K, value: Settings[K]) => void;
  reset: () => void;
};

const Ctx = createContext<Ctx>({ settings: defaultSettings, set: () => {}, reset: () => {} });

export function useThemeSettings() { return useContext(Ctx); }

const STORAGE_KEY = 'hub_ui_settings_v1';

export function ThemeProvider({ children }: { children: React.ReactNode }) {
  const [settings, setSettings] = useState<Settings>(() => {
    try { const raw = localStorage.getItem(STORAGE_KEY); if (raw) return { ...defaultSettings, ...JSON.parse(raw) }; } catch {}
    return defaultSettings;
  });

  useEffect(() => { try { localStorage.setItem(STORAGE_KEY, JSON.stringify(settings)); } catch {} }, [settings]);
  useEffect(() => { applyTheme(settings); }, [settings]);

  const api = useMemo<Ctx>(() => ({
    settings,
    set: (k, v) => setSettings((s) => ({ ...s, [k]: v })),
    reset: () => setSettings(defaultSettings),
  }), [settings]);

  return <Ctx.Provider value={api}>{children}</Ctx.Provider>;
}

function applyTheme(s: Settings) {
  const el = document.documentElement;
  el.setAttribute('data-theme', 'dark');
  el.setAttribute('data-density', s.density);
  el.style.setProperty('--font-size-base', s.fontSize === 'small' ? '14px' : s.fontSize === 'large' ? '17px' : '15px');
  el.style.setProperty('--code-ligatures', s.codeLigatures ? 'normal' : 'none');
  // Accent palettes
  const palettes: Record<Accent, Record<string, string>> = {
    teal: {50:'#083e3b',100:'#0a4c48',200:'#0f615b',300:'#12756e',400:'#128a81',500:'#14b8a6',600:'#0ea89a',700:'#0b8c82',800:'#0a6f67',900:'#095953'},
    blue: {50:'#0a2a43',100:'#0e3860',200:'#11477d',300:'#15579b',400:'#1967b8',500:'#1e80e0',600:'#1a72c8',700:'#165fa6',800:'#114c84',900:'#0d3c67'},
    purple: {50:'#2b0a43',100:'#3a0e60',200:'#4a117d',300:'#5c159b',400:'#6f19b8',500:'#8a1ee0',600:'#7a1ac8',700:'#6516a6',800:'#501184',900:'#3f0d67'},
    lime: {50:'#243b0a',100:'#36580f',200:'#4a7514',300:'#5f9319',400:'#76b81f',500:'#99e01e',600:'#86c81a',700:'#6fa616',800:'#588411',900:'#47670d'},
  };
  const p = palettes[s.accent];
  Object.entries(p).forEach(([k, v]) => el.style.setProperty(`--accent-${k}`, v));
}

