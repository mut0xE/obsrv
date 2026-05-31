"use client";

import { PageAnalyze } from "@/components/PageAnalyze";

// Home renders the Analyze page directly so `/` always resolves, regardless
// of how the hosting platform handles server-side `redirect()` calls.
export default function Home() {
  return <PageAnalyze />;
}
