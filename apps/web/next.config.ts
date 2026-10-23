import type { NextConfig } from "next";

// Same-origin API: with NEXT_PUBLIC_BB_API=/bb the browser calls this app, which forwards to the auction
// service (BB_API_UPSTREAM, default the local service). One public hostname is then enough for a demo
// behind a tunnel; the Caddy deployment does the same at the proxy.
const upstream = (process.env.BB_API_UPSTREAM ?? "http://127.0.0.1:8787").replace(/\/$/, "");

const nextConfig: NextConfig = {
  async rewrites() {
    return [{ source: "/bb/:path*", destination: `${upstream}/:path*` }];
  },
  cacheComponents: true,
  partialPrefetching: true,
  turbopack: {
    rules: {
      "*.css": {
        loaders: ["@tailwindcss/turbopack"],
        as: "*.css",
      },
    },
  },
};

export default nextConfig;
