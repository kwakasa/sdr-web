import type { Metadata } from "next";
import "./globals.css";

export const metadata: Metadata = {
  title: "SDR-Web",
  description: "Software Defined Radio in the browser",
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="en">
      <body className="bg-gray-950 text-gray-100 antialiased">{children}</body>
    </html>
  );
}
