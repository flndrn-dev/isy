'use client'

import { useLanguage } from '@isy/shared-i18n'

export default function SplashPage() {
  const { t } = useLanguage()

  return (
    <main
      className="flex min-h-dvh flex-col items-center justify-center gap-4 px-6 text-center"
      style={{
        backgroundImage:
          'radial-gradient(ellipse at center, rgba(142,111,212,0.18) 0%, rgba(15,14,24,1) 65%)',
      }}
    >
      <img
        src="/logo.svg"
        alt={t('common.appName')}
        className="h-40 w-auto md:h-48"
      />
      <p className="text-base text-isy-bg-light/60 md:text-lg">
        {t('common.tagline')}
      </p>
      <p className="text-xs uppercase tracking-widest text-isy-bg-light/35">
        {t('common.domain')}
      </p>
    </main>
  )
}
