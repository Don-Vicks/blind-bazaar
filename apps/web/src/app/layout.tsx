import type { Metadata } from "next";
import { Geist, Geist_Mono, Instrument_Serif } from "next/font/google";
import Link from "next/link";
import "./globals.css";
import { Providers } from "./providers";
import { HealthBadge } from "@/components/health-badge";

const geistSans = Geist({ variable: "--font-geist-sans", subsets: ["latin"] });
const geistMono = Geist_Mono({ variable: "--font-geist-mono", subsets: ["latin"] });
const instrument = Instrument_Serif({ variable: "--font-instrument-serif", subsets: ["latin"], weight: "400", style: ["normal", "italic"] });

export const metadata: Metadata = {
  title: "Blind Bazaar",
  description: "Sealed-bid auctions where every bid is a shielded Zcash transaction.",
};

const nav =
  "whitespace-nowrap rounded-[2px] py-2 text-[13px] text-muted-foreground transition-colors duration-150 hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent";

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en" className={`${geistSans.variable} ${geistMono.variable} ${instrument.variable} h-full antialiased`}>
      <body className="min-h-full flex flex-col">
        <Providers>
          <header className="hairline-b">
            <div className="mx-auto flex max-w-[1120px] flex-wrap items-center justify-between gap-x-6 gap-y-2 px-6 py-5">
              <Link
                href="/"
                className="serif rounded-[2px] text-[22px] leading-none focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent"
              >
                Blind Bazaar
              </Link>
              <nav className="flex flex-wrap items-center gap-x-4 gap-y-1 sm:gap-x-6">
                <Link href="/" className={nav}>
                  Lots
                </Link>
                <Link href="/how" className={nav}>
                  How it works
                </Link>
                <Link href="/bids" className={nav}>
                  My bids
                </Link>
                <Link href="/dashboard" className={nav}>
                  Sell
                </Link>
                <Link href="/embed" className={nav}>
                  Pay button
                </Link>
                <HealthBadge />
              </nav>
            </div>
          </header>
          <main className="mx-auto w-full max-w-[1120px] flex-1 px-6 pb-24 pt-10 sm:pt-14">{children}</main>
          <footer className="hairline-t">
            <div className="mx-auto flex max-w-[1120px] flex-col gap-2 px-6 py-8 text-[13px] text-muted-foreground sm:flex-row sm:items-center sm:justify-between">
              <p>Private auctions on Zcash. Bids are encrypted payments; losing bids are never revealed; results are provable.</p>
              <p className="num text-[12px]">MIT · open source</p>
            </div>
          </footer>
        </Providers>
      </body>
    </html>
  );
}
