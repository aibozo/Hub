import '../styles/globals.css';
import { QueryProvider } from '@/lib/query';
import { TopNav } from '@/components/app/TopNav';
import { Sidebar } from '@/components/app/Sidebar';
import { RightDrawer } from '@/components/app/RightDrawer';
import { ThemeProvider } from '@/components/app/ThemeProvider';
import { CommandPalette } from '@/components/app/CommandPalette';
import { Toaster } from '@/lib/toast';

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en" data-theme="dark">
      <body className="bg-bg text-text">
        <QueryProvider>
          <ThemeProvider>
          <div className="grid h-dvh grid-rows-[56px_1fr]">
            <TopNav />
            <div className="grid grid-cols-[280px_1fr]">
              <Sidebar />
              <main className="relative">{children}</main>
            </div>
          </div>
          <RightDrawer />
          <CommandPalette />
          <Toaster />
          </ThemeProvider>
        </QueryProvider>
      </body>
    </html>
  );
}
