"use client";
import { useEffect, useRef, useState } from 'react';
import { useThemeSettings } from '@/components/app/ThemeProvider';

type Monaco = typeof import('monaco-editor');

export function CodeBlock({ value, language }: { value: string; language?: string }) {
  const preRef = useRef<HTMLPreElement | null>(null);
  const divRef = useRef<HTMLDivElement | null>(null);
  const editorRef = useRef<import('monaco-editor').editor.IStandaloneCodeEditor | null>(null);
  const modelRef = useRef<import('monaco-editor').editor.ITextModel | null>(null);
  const [copied, setCopied] = useState(false);
  const [ready, setReady] = useState(false);
  const { settings } = useThemeSettings();

  // Render fallback immediately
  useEffect(() => {
    if (preRef.current) preRef.current.textContent = value;
  }, [value]);

  // Lazy load Monaco on client
  useEffect(() => {
    let cancelled = false;
    (async () => {
      if (typeof window === 'undefined') return;
      try {
        const monaco: Monaco = await import('monaco-editor');
        if (cancelled || !divRef.current) return;
        // Create model and editor
        const lang = language || detectLang(value);
        const model = monaco.editor.createModel(value, lang);
        modelRef.current = model;
        const editor = monaco.editor.create(divRef.current, {
          model,
          automaticLayout: true,
          minimap: { enabled: false },
          readOnly: true,
          scrollBeyondLastLine: false,
          fontLigatures: settings.codeLigatures,
          wordWrap: 'on',
          theme: 'vs-dark',
        });
        editorRef.current = editor;
        // Size heuristics
        const lines = Math.max(4, Math.min(30, value.split('\n').length));
        const height = 20 * lines + 20; // approx line height
        divRef.current.style.height = `${height}px`;
        setReady(true);
      } catch {
        // keep fallback
      }
    })();
    return () => {
      cancelled = true;
      try {
        editorRef.current?.dispose();
        modelRef.current?.dispose();
      } catch {}
    };
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Update content/language on change
  useEffect(() => {
    (async () => {
      if (!editorRef.current || !modelRef.current) return;
      const monaco: Monaco = await import('monaco-editor');
      if (modelRef.current.getValue() !== value) modelRef.current.setValue(value);
      const lang = language || detectLang(value);
      monaco.editor.setModelLanguage(modelRef.current, lang || 'plaintext');
    })();
  }, [value, language]);

  return (
    <div className="relative">
      <div ref={divRef} className="w-full overflow-hidden rounded-md border border-border" aria-label={`Code block ${language ?? ''}`} />
      {!ready && (
        <pre ref={preRef} className="max-h-96 overflow-auto rounded-md bg-[color:var(--bg-1)] p-3 text-xs" aria-hidden={ready} />
      )}
      <button
        className="absolute right-2 top-2 rounded-md bg-surface px-2 py-1 text-2xs text-text-dim hover:bg-bg-1"
        onClick={async () => { try { await navigator.clipboard.writeText(value); setCopied(true); setTimeout(() => setCopied(false), 1200); } catch {} }}
      >
        {copied ? 'Copied' : 'Copy'}
      </button>
    </div>
  );
}

function detectLang(s: string): string | undefined {
  const head = s.trim().slice(0, 200).toLowerCase();
  if (head.startsWith('{') || head.startsWith('[')) return 'json';
  if (head.includes('fn ') || head.includes('pub ') || head.includes('crate::')) return 'rust';
  if (head.includes('import ') || head.includes('export ') || head.includes('console.log')) return 'typescript';
  if (head.includes('#include') || head.includes('int main(')) return 'cpp';
  if (head.includes('def ') || head.includes('import ') && head.includes(' as ')) return 'python';
  if (head.includes('<html') || head.includes('<div') || head.includes('</')) return 'html';
  return undefined;
}
