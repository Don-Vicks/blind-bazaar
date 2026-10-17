"use client";

import { useEffect, useState } from "react";

const KEY = "bb.sellerKey";

/** Seller key kept in sessionStorage only; cleared when the tab closes. */
export function useSellerKey(): [string, (k: string) => void] {
  const [key, setKeyState] = useState("");
  useEffect(() => {
    try {
      setKeyState(sessionStorage.getItem(KEY) ?? "");
    } catch {
      /* no storage */
    }
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
  return [key, setKey];
}
