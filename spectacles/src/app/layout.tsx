import { PT_Sans, PT_Sans_Caption, PT_Sans_Narrow } from "next/font/google";
import Link from "next/link";
import "@/styles/base.css";
import styles from "./layout.module.css";

const PTSans = PT_Sans({
  subsets: ["latin"],
  weight: ["400", "700"],
  variable: "--sans-serif",
});
const PTSansCaption = PT_Sans_Caption({
  subsets: ["latin"],
  weight: ["400", "700"],
  variable: "--expanded-sans-serif",
});
const PTSansNarrow = PT_Sans_Narrow({
  subsets: ["latin"],
  weight: ["400", "700"],
  variable: "--condensed-sans-serif",
});

const iconPath =
  process.env.NODE_ENV === "development" ? "/favicon_dev.png" : "/favicon.png";
export const metadata = {
  title: "spectacles",
  description: "extrasensory perception into your 200mb chat client",
  icons: { icon: iconPath },
};

export default function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <html lang="en">
      <body
        className={`${PTSans.variable} ${PTSansCaption.variable} ${PTSansNarrow.variable}`}
      >
        <header className={styles.mainHeader}>
          <div className={styles.brand}>spectacles</div>
          <Link href="/">home</Link>
          <Link href="/builds">builds</Link>
          <Link href="/manage">manage</Link>
        </header>
        <main>{children}</main>
      </body>
    </html>
  );
}
