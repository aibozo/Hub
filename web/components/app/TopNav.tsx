"use client";
import Link from 'next/link';
import { usePathname } from 'next/navigation';
import { HealthBadge } from './HealthBadge';
import { useUi } from '@/lib/store';

const tabs = [
  { href: '/', label: 'Chat' },
  { href: '/agents', label: 'Agents' },
  { href: '/research', label: 'Research' },
  { href: '/settings', label: 'Settings' },
];

export function TopNav() {
  const pathname = usePathname();
  const { toggleRight } = useUi();
  return (
    <header className="sticky top-0 z-10 flex h-14 items-center justify-between border-b border-border bg-bg px-4">
      <div className="flex items-center gap-6">
        <div className="text-sm font-semibold">Hub</div>
        <nav className="flex items-center gap-4">
          {tabs.map((t) => {
            const active = pathname === t.href;
            return (
              <Link key={t.href} href={t.href} className={`text-sm ${active ? 'text-text' : 'text-text-dim hover:text-text'}`}>
                {t.label}
                {active && <span className="block h-0.5 w-full bg-accent-500" />}
              </Link>
            );
          })}
        </nav>
      </div>
      <div className="flex items-center gap-3">
        <HealthBadge />
        <button className="rounded-md bg-surface px-3 py-1 text-sm hover:bg-bg-1" onClick={toggleRight}>Activity</button>
      </div>
    </header>
  );
}
