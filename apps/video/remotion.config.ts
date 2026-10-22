import { Config } from "@remotion/cli/config";

// Pages are embedded live from the running web app, so one tab renders every frame in order
// (a fresh tab mid-scene would show a page still loading).
Config.setVideoImageFormat("jpeg");
Config.setJpegQuality(92);
Config.setConcurrency(1);
Config.setDelayRenderTimeoutInMilliseconds(120_000);
Config.setOverwriteOutput(true);
Config.setEntryPoint("./src/index.ts");
// Ports 3000/3001/8080/8787/8797/8801-8803/9067 belong to the running app; keep off them.
Config.setRendererPort(4397);
Config.setStudioPort(4398);
