import type { Metadata } from 'next'
import { LanguageProvider } from '@isy/shared-i18n'

import './globals.css'

export const metadata: Metadata = {
  metadataBase: new URL('https://isy.chat'),
  title: 'ISY® — I Seek You, reborn.',
  description:
    'ISY® is a modern, EU-built, end-to-end encrypted messenger. Your UIN is yours forever.',
  icons: {
    icon: '/favicon.svg',
  },
  robots: {
    index: false,
    follow: false,
  },
}

export default function RootLayout({
  children,
}: {
  children: React.ReactNode
}) {
  return (
    <html lang="en">
      <body>
        <LanguageProvider>{children}</LanguageProvider>
      </body>
    </html>
  )
}
