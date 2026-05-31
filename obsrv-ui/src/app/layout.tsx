import type { Metadata } from "next";
import "./globals.css";
import { AlertContainer } from "@/components/ErrorAlert";

export const metadata: Metadata = {
  title: "obsrv — see before you sign",
  description: "Real-time Solana transaction monitor with AI-powered risk analysis",
};

export default function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <html lang="en">
      <head>
        <meta charSet="UTF-8" />
        <meta name="viewport" content="width=device-width, initial-scale=1.0" />
        <link rel="preconnect" href="https://fonts.googleapis.com" />
        <link rel="preconnect" href="https://fonts.gstatic.com" crossOrigin="anonymous" />
        <link
          href="https://fonts.googleapis.com/css2?family=JetBrains+Mono:wght@400;500;600&family=IBM+Plex+Mono:wght@300;400;500&family=DM+Sans:wght@400;500;600;700&family=Fira+Code:wght@400;500&display=swap"
          rel="stylesheet"
        />
      </head>
      <body>
        <div id="app-root">{children}</div>
        <AlertContainer />
      </body>
    </html>
  );
}
