# OpenStream TV

OpenStream TV is a lightweight Rust server for live TV, catch-up, EPG, IPTV playlists, and supported on-demand playback, with an optional Svelte web interface. It currently supports JioTV and JioTV+ sources, with the project structured to support additional providers.

## Installation

Pick your platform and run one command. The installer detects your CPU automatically.

**Linux, macOS, Android/Termux**

```sh
curl -fsSL https://raw.githubusercontent.com/wpfyorg/openstream-tv/main/scripts/install.sh | sh
```

**Windows PowerShell**

```powershell
irm https://raw.githubusercontent.com/wpfyorg/openstream-tv/main/scripts/install.ps1 | iex
```

**OpenWrt**

```sh
wget -qO- https://raw.githubusercontent.com/wpfyorg/openstream-tv/main/scripts/install.sh | sh
```

For Docker and step-by-step setup, see the [OpenStream TV documentation](https://wpfyorg.github.io/openstream-tv/).

Configuration uses `OPENSTREAM_*` environment variables. Legacy `JIOTV_*` names remain accepted for migration compatibility; new OpenStream names take precedence.

## Community

For support and development discussion, join the [OpenStream TV Discord](https://discord.gg/VWDvwqTjY).
