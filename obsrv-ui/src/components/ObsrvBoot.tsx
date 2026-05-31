"use client";

import { useEffect } from "react";
import { useObsrv } from "@/lib/store";
import { onAuthChange, readWalletAuth } from "@/lib/wallet-auth";

// Mounts once at the app root, kicks off the single websocket connection,
// and pulls the monitor list. Survives page navigation so the live feed and
// the monitor list don't reset every time the user switches tabs.
export function ObsrvBoot() {
  const startWs = useObsrv((s) => s.startWs);
  const fetchMonitorList = useObsrv((s) => s.fetchMonitorList);

  useEffect(() => {
    startWs();
    if (readWalletAuth()) fetchMonitorList();

    // When the user connects or disconnects a wallet the per-user filter
    // changes, so we re-pull /monitor/list (or clear it on logout).
    return onAuthChange(() => {
      const next = useObsrv.getState();
      if (readWalletAuth()) {
        fetchMonitorList();
      } else {
        // Logged out: drop the per-user lists and any feed events that were
        // matched against the previous session.
        next.fetchMonitorList; // keep tree-shaker happy
        useObsrv.setState({
          wallets: [],
          programs: [],
          walletFeed: [],
          programFeed: [],
        });
      }
    });
  }, [startWs, fetchMonitorList]);

  return null;
}
