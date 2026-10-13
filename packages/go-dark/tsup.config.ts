import { defineConfig } from "tsup";

export default defineConfig([
  {
    entry: { index: "src/index.ts" },
    format: ["esm"],
    dts: true,
    clean: true,
    external: ["react", "react-dom"],
  },
  {
    entry: { "go-dark": "src/iife.tsx" },
    format: ["iife"],
    globalName: "GoDark",
    minify: true,
    noExternal: [/.*/],
    platform: "browser",
    outExtension: () => ({ js: ".iife.js" }),
  },
]);
