import type { Metadata } from "next";
import { Geist, Geist_Mono } from "next/font/google";
import Link from "next/link";
import "./globals.css";
import { Providers } from "./providers";
import { HealthBadge } from "@/components/health-badge";

const geistSans = Geist({ variable: "--font-geist-sans", subsets: ["latin"] });
const geistMono = Geist_Mono({ variable: "--font-geist-mono", subsets: ["latin"] });

export const metadata: Metadata = {
  title: "Blind Bazaar",
  description: "Sealed-bid auctions where every bid is a shielded Zcash transaction.",
};

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en" className={`${geistSans.variable} ${geistMono.variable} h-full antialiased`}>
      <body className="min-h-full flex flex-col">
        <Providers>
          <header className="border-b border-border">
            <div className="mx-auto flex max-w-4xl items-center justify-between gap-4 px-4 py-4">
              <Link
                href="/"
                className="rounded font-semibold tracking-tight focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background"
              >
                Blind Bazaar
              </Link>
              <nav className="flex items-center gap-4 text-sm text-muted-foreground">
                <Link
                  href="/embed"
                  className="rounded py-2 hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
                >
                  Go Dark
                </Link>
                <HealthBadge />
              </nav>
            </div>
          </header>
          <main className="mx-auto w-full max-w-4xl flex-1 px-4 py-8">{children}</main>
          <footer className="border-t border-border">
            <p className="mx-auto max-w-4xl px-4 py-6 text-xs text-muted-foreground">
              Every bid is a shielded Zcash transaction. Losing bids are never revealed. MIT licensed.
            </p>
          </footer>
        </Providers>
      </body>
    </html>
  );
}
