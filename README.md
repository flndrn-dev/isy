# ISY

**A modern end-to-end encrypted messenger with permanent 9-digit UIN identity, built in the EU.**

---

## What is ISY?

ISY is a consumer messenger where:

- **Your identity is a UIN** — a 9-digit number that is yours for life. Not tied to a phone number. Not tied to an email address. Yours.
- **Every message is end-to-end encrypted** via MLS (IETF RFC 9420). flndrn cannot read your messages, cannot produce them in response to a subpoena, cannot scan them for any purpose. The architecture makes it impossible.
- **You find people by who they are**, not who you already know — a searchable directory filtered by interest, country, and language. Opt-in only.
- **The product is free forever.** No ads, no subscriptions, no data sale, no paid tiers, no feature paywalls.

If you want a memorable UIN — short, palindromic, sequential — there is a marketplace for that. Everything else is free for everyone, forever.

## Status

ISY is in active development. Early private beta targeted for 2026. Public launch will be announced on [isy.chat](https://isy.chat) and in EU privacy communities. This repository contains the code; the product is not yet available to end users.

## Why ISY

Every other messenger makes a trade-off we don't:

| Concern | Most messengers | ISY |
|---|---|---|
| Where is message content stored? | On their servers | Only as ciphertext we cannot decrypt |
| How is it funded? | Ads, subscriptions, data sale | Optional custom UIN marketplace only |
| Who owns your identity? | The provider (tied to phone or email) | You (your UIN, permanent and portable) |
| Jurisdiction? | Varies | Republic of Cyprus (EU, GDPR-native) |
| Distribution? | Through mobile app stores | Direct from the website (no store intermediaries) |

## Distribution

Users never go through an app store:

- **Web** — at [isy.chat](https://isy.chat)
- **Desktop** — signed Windows, macOS, and Linux binaries, downloaded directly, auto-updating
- **Android** — direct APK download
- **iOS** — installable as a Progressive Web App from Safari

## Getting started (developers)

Requires Node 22+ and [pnpm](https://pnpm.io) 9+.

```bash
pnpm install
pnpm dev          # run the web app at http://localhost:3000
pnpm typecheck    # full monorepo typecheck
pnpm test         # run the test suite
```

## Repository layout

```
apps/
  web/             Next.js 16 + React 19 + Tailwind v4 web application
packages/
  shared/
    i18n/          Custom React LanguageProvider (no third-party i18n library)
convex/            Backend: schema, mutations, and queries
locales/           Translation files (English, more languages coming)
```

## Technology

- **Frontend** — Next.js 16, React 19, Tailwind v4, shadcn/ui, Framer Motion
- **Backend** — Convex
- **Authentication** — Better Auth
- **Encryption** — OpenMLS (MIT), MLS per IETF RFC 9420
- **Desktop packaging** — Tauri v2
- **Mobile packaging** — Capacitor (Android, iOS)
- **Edge and delivery** — Cloudflare, a dedicated Rust WebSocket service
- **Language** — TypeScript, with Rust for performance-critical components

## Contributing

ISY is primarily developed by a single maintainer at this stage. Bug reports, small fixes, and documentation improvements are welcome via pull request. Larger proposals should start as an issue so we can discuss scope before code is written.

## Security

If you believe you have found a security vulnerability, please **do not** open a public issue. Email `security@isy.chat` with a description and, if applicable, a proof of concept. We will acknowledge receipt within 72 hours.

## License

Source code is released under the [MIT License](LICENSE).

## Contact

Operated by **flndrn Limited**, registered in the Republic of Cyprus.

- General enquiries: `hello@isy.chat`
- Legal and data protection: `legal@isy.chat`
- Security disclosures: `security@isy.chat`
