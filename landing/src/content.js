/*
  Every string on the landing page.
*/

export const brand = {
  name: 'OpenStream TV',
  tagline: 'Your channels, one server',
}

const repo = 'https://github.com/wpfyorg/openstream-tv'
const docs = 'https://wpfyorg.github.io/openstream-tv/docs/'

export const links = {
  repo,
  docs,
  releases: `${repo}/releases/latest`,
  discord: 'https://discord.gg/Xn3bUsBCZU',
}

export const nav = {
  links: [
    { label: 'Features', href: '#features' },
    { label: 'Providers', href: '#providers' },
    { label: 'Install', href: '#install' },
    { label: 'FAQ', href: '#faq' },
  ],
  secondary: { label: 'Documentation', href: docs },
  cta: { label: 'Install', href: '#install' },
}

export const hero = {
  headline: 'All your channels,',
  highlight: 'one server',
  sub: 'OpenStream TV is a lightweight Rust server for live TV, catch-up and EPG, with a web player and an IPTV playlist for the clients you already own. Install it once and watch from anywhere on your network.',
  cta: { label: 'Install OpenStream TV', href: '#install' },
  secondary: { label: 'Read the documentation', href: docs },
  discord: { label: 'Join the Discord', href: links.discord },
  mockTitle: 'OpenStream TV — Live',
}

export const logos = ['JioTV', 'JioTV+', 'Fastway', 'Airtel Xstream', 'Tata Play', 'PlayboxTV']

export const stats = [
  { value: '1', label: 'Rust binary, no runtime to install' },
  { value: '6', label: 'providers, stable and beta' },
  { value: 'Web UI', label: 'guide, player and settings' },
  { value: 'M3U + EPG', label: 'for VLC, Kodi and TiviMate' },
]

export const features = {
  eyebrow: 'Everything in one place',
  title: 'A TV server that stays out of the way',
  sub: 'One binary serves the guide, the player, the playlist and the guide data. No database to run, no browser extension, no second box.',
  items: [
    {
      icon: 'tv',
      tone: 'brand',
      title: 'Live channels',
      body: 'Channels from your provider, proxied through your own server behind a protected local URL.',
    },
    {
      icon: 'guide',
      tone: 'teal',
      title: 'Guide and catch-up',
      body: 'A seven-day EPG and catch-up where the provider offers it, searchable from the web UI.',
    },
    {
      icon: 'playlist',
      tone: 'violet',
      title: 'IPTV playlist',
      body: 'A ready-made M3U playlist and XMLTV guide, so any IPTV client on your network can tune in.',
    },
    {
      icon: 'shield',
      tone: 'amber',
      title: 'Protected playback',
      body: 'DASH and Widevine handling for protected streams, with playback errors surfaced instead of swallowed.',
    },
    {
      icon: 'monitor',
      tone: 'slate',
      title: 'Web interface',
      body: 'A Svelte UI served from the same binary: browse the guide, play a channel, manage the account.',
    },
    {
      icon: 'server',
      tone: 'teal',
      title: 'Runs anywhere',
      body: 'Docker, OpenWrt, Android and Termux, a Raspberry Pi or the laptop in front of you.',
    },
  ],
}

export const steps = {
  eyebrow: 'How it works',
  title: 'Watching in about a minute',
  items: [
    {
      title: 'Install the server',
      body: 'One command detects your CPU, drops the binary in place and sets up a background service.',
    },
    {
      title: 'Sign in to your provider',
      body: 'Choose JioTV, JioTV+, Fastway or a beta provider in the web UI. Credentials stay on your machine.',
    },
    {
      title: 'Start watching',
      body: 'Open the built-in web player, or point an IPTV client at the playlist and guide URLs.',
    },
  ],
}

export const providers = {
  eyebrow: 'Providers',
  title: 'Built around the providers you already pay for',
  sub: 'JioTV, JioTV+ and Fastway are the stable integrations. Airtel Xstream, Tata Play Smart, PlayboxTV and VZY are in beta while their remaining runtime contracts are validated.',
  items: [
    {
      name: 'JioTV',
      status: 'Stable',
      body: 'Live channels, catch-up and EPG through the JioTV flow, with OTP sign-in and account switching.',
    },
    {
      name: 'JioTV+',
      status: 'Stable',
      body: 'The optional JioTV+ source, connected alongside your main provider and streamed the same way.',
    },
    {
      name: 'Fastway',
      status: 'Stable',
      body: 'Cable live TV, guide and catch-up from a Fastway account, with its own login flow.',
    },
    {
      name: 'Airtel Xstream',
      status: 'Beta',
      body: 'Live channels and guide while the remaining runtime contracts are validated.',
    },
    {
      name: 'Tata Play Smart',
      status: 'Beta',
      body: 'Live TV from a Tata Play Smart account, available to try in the beta line.',
    },
    {
      name: 'PlayboxTV',
      status: 'Beta',
      body: 'PlayboxTV channels through the same guide, player and playlist plumbing.',
    },
  ],
}

export const install = {
  eyebrow: 'Installation',
  title: 'One command, one binary',
  sub: 'The installers detect your CPU, fetch the right release asset and set up a service. Configuration is a set of OPENSTREAM_* environment variables; the legacy JIOTV_* names still work for migration.',
  note: 'Docker, docker-compose, building from source and the slim build are covered in the docs.',
  docs: { label: 'Read the installation guide', href: docs },
  options: [
    {
      name: 'Linux, macOS, Android',
      badge: 'Recommended',
      body: 'Termux counts as Android. Installs to the OpenStream data directory and starts a background service.',
      command: 'curl -fsSL https://raw.githubusercontent.com/wpfyorg/openstream-tv/main/scripts/install.sh | sh',
    },
    {
      name: 'Windows',
      badge: 'PowerShell',
      body: 'PowerShell 5 or newer. The same release assets, unpacked into your user profile.',
      command: 'irm https://raw.githubusercontent.com/wpfyorg/openstream-tv/main/scripts/install.ps1 | iex',
    },
    {
      name: 'OpenWrt',
      badge: 'Router',
      body: 'A package with a procd service and a UCI config, for routers and small always-on boxes.',
      command: 'wget -qO- https://raw.githubusercontent.com/wpfyorg/openstream-tv/main/scripts/install.sh | sh',
    },
  ],
}

export const faq = {
  title: 'Questions, answered',
  items: [
    {
      q: 'Do I need my own provider account?',
      a: 'Yes. OpenStream TV is a server for accounts you already hold: sign in to JioTV, JioTV+, Fastway or a supported beta provider from the web interface, the same way you would in the provider app.',
    },
    {
      q: 'Where are my credentials stored?',
      a: 'In the OpenStream data directory — normally $HOME/.openstream-tv, or /etc/openstream on OpenWrt. Sessions and tokens stay on the machine running the server and are never sent anywhere else.',
    },
    {
      q: 'Will my TV or IPTV app work with it?',
      a: 'Yes. The server publishes an M3U playlist and an XMLTV guide, so VLC, Kodi, TiviMate and most IPTV clients on your network can use it. The built-in web player needs nothing installed.',
    },
    {
      q: 'What can I run it on?',
      a: '64-bit Linux, macOS, Windows, OpenWrt and Android via Termux, plus Docker and docker-compose. A Raspberry Pi or a spare mini PC is plenty — the server is a single Rust binary.',
    },
    {
      q: 'Is it free?',
      a: 'OpenStream TV is free to use and its source is published under Creative Commons Attribution 4.0. Providers may still require their own subscription.',
    },
  ],
}

export const cta = {
  title: 'Your channels, on your server',
  sub: 'Install in a minute. No cloud account, nothing to subscribe to beyond your provider.',
  button: 'Install OpenStream TV',
  href: '#install',
  secondary: { label: 'Star on GitHub', href: repo },
}

export const footer = {
  blurb: 'A lightweight Rust server for live TV, catch-up, EPG and IPTV playback.',
  columns: [
    {
      title: 'Product',
      links: [
        { label: 'Features', href: '#features' },
        { label: 'Providers', href: '#providers' },
        { label: 'Install', href: '#install' },
        { label: 'FAQ', href: '#faq' },
      ],
    },
    {
      title: 'Project',
      links: [
        { label: 'Documentation', href: docs },
        { label: 'GitHub', href: repo },
        { label: 'Releases', href: `${repo}/releases/latest` },
        { label: 'Discord', href: 'https://discord.gg/Xn3bUsBCZU' },
      ],
    },
  ],
  quickstart: {
    title: 'Quick start',
    command: 'curl -fsSL https://raw.githubusercontent.com/wpfyorg/openstream-tv/main/scripts/install.sh | sh',
    note: 'Then open the web interface and sign in to your provider.',
    docs: { label: 'Docker and OpenWrt', href: docs },
  },
  legal: '© 2026 OpenStream TV. Source available under CC BY 4.0.',
}
