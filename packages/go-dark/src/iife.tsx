import { createRoot } from "react-dom/client";
import { GoDark } from "./widget";
import type { GoDarkOptions } from "./types";

/** `GoDark.mount(el, opts)` for plain <script> embeds. */
export function mount(el: HTMLElement, opts: GoDarkOptions) {
  const root = createRoot(el);
  root.render(<GoDark {...opts} />);
  return { unmount: () => root.unmount() };
}
