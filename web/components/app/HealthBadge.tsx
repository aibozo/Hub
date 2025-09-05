"use client";
import { useEffect, useState } from 'react';
import { api } from '@/lib/api';

export function HealthBadge() {
  const [ok, setOk] = useState<boolean | null>(null);

  useEffect(() => {
    let mounted = true;
    let id: any;
    const tick = async () => {
      const v = await api.ready();
      if (mounted) setOk(v);
    };
    tick();
    id = setInterval(tick, 1000);
    return () => { mounted = false; clearInterval(id); };
  }, []);

  const color = ok == null ? 'bg-muted' : ok ? 'bg-[color:var(--ok)]' : 'bg-[color:var(--err)]';
  const label = ok == null ? 'Checking…' : ok ? 'Core connected' : 'Core offline';

  return (
    <div className="inline-flex items-center gap-2 rounded-full border border-border px-2 py-1 text-xs text-text-dim">
      <span className={`h-2.5 w-2.5 rounded-full ${color}`} />
      <span>{label}</span>
    </div>
  );
}

