"use client";

import { useEffect, useState } from "react";

const KEY = "bb.sellerKey";

/** Seller key kept in sessionStorage only; cleared when the tab closes. */
export function useSellerKey(): [string, (k: string) => void, boolean] {
  const [key, setKeyState] = useState("");
  const [ready, setReady] = useState(false);
  useEffect(() => {
    try {
      setKeyState(sessionStorage.getItem(KEY) ?? "");
    } catch {
      /* no storage */
    }
    setReady(true);
  }, []);
  const setKey = (k: string) => {
    setKeyState(k);
    try {
      if (k) sessionStorage.setItem(KEY, k);
      else sessionStorage.removeItem(KEY);
    } catch {
      /* no storage */
    }
  };
  return [key, setKey, ready];
}
