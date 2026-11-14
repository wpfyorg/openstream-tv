<script>
  import { onMount } from 'svelte'
  import { CalendarClock, History, ListVideo, Play, Search, Settings, Tv, Volume2 } from 'lucide-svelte'

  import WindowFrame from '../components/WindowFrame.svelte'
  import { hero } from '../content.js'
  import { cn } from '../lib/ui.js'

  const MOCK_W = 1018
  const MOCK_H = 600

  const appNav = [
    { label: 'Live TV', icon: Tv, active: true },
    { label: 'Guide', icon: CalendarClock },
    { label: 'Catch-up', icon: History },
    { label: 'Playlists', icon: ListVideo },
    { label: 'Settings', icon: Settings },
  ]

  const onNow = [
    { time: '20:00', title: 'Matchday Live', channel: 'Arena Sports 1', progress: '62%' },
    { time: '21:00', title: 'The Evening Bulletin', channel: 'Metro News', progress: '18%' },
    { time: '21:30', title: 'Late Night Film', channel: 'Cinema One', progress: '0%' },
  ]

  const channels = [
    ['101', 'Arena Sports 1', 'Sport'],
    ['102', 'Metro News', 'News'],
    ['103', 'Cinema One', 'Film'],
    ['104', 'Nature HD', 'Documentary'],
    ['105', 'Kids Club', 'Family'],
    ['106', 'Music Box', 'Music'],
  ]

  let wrap = $state(null)
  let scale = $state(1)
  let tilt = $state({ rotateX: 22, scale: 0.9, y: 60 })

  /*
    The window tilts up and settles as it scrolls into place, and scales down
    with the viewport so the fixed 1018px design always fits. Both are skipped
    when the reader prefers reduced motion, and following the media query keeps
    that true when the setting changes while the page is open.
  */
  onMount(() => {
    const motion = window.matchMedia('(prefers-reduced-motion: reduce)')
    let reduce = motion.matches
    let frame = 0

    const fit = () => {
      if (!wrap) return
      scale = Math.min(1, wrap.clientWidth / MOCK_W)
    }

    const update = () => {
      frame = 0
      if (!wrap) return
      fit()
      if (reduce) {
        tilt = { rotateX: 0, scale: 1, y: 0 }
        return
      }
      const rect = wrap.getBoundingClientRect()
      const viewport = window.innerHeight
      const span = viewport - viewport * 0.66
      const progress = span <= 0 ? 1 : Math.min(1, Math.max(0, (viewport - rect.top) / span))
      tilt = {
        rotateX: 22 * (1 - progress),
        scale: 0.9 + 0.1 * progress,
        y: 60 * (1 - progress),
      }
    }

    const onScroll = () => {
      if (frame) return
      frame = requestAnimationFrame(update)
    }

    const onMotionChange = (event) => {
      reduce = event.matches
      if (reduce) tilt = { rotateX: 0, scale: 1, y: 0 }
      update()
    }

    update()
    const observer = new ResizeObserver(fit)
    if (wrap) observer.observe(wrap)
    window.addEventListener('scroll', onScroll, { passive: true })
    window.addEventListener('resize', onScroll)
    motion.addEventListener('change', onMotionChange)

    return () => {
      if (frame) cancelAnimationFrame(frame)
      observer.disconnect()
      window.removeEventListener('scroll', onScroll)
      window.removeEventListener('resize', onScroll)
      motion.removeEventListener('change', onMotionChange)
    }
  })
</script>

<div
  bind:this={wrap}
  class="relative mx-auto w-full max-w-[1018px] [perspective:1600px]"
  style="height: {MOCK_H * scale}px"
>
  <div
    class="absolute top-0 left-0 origin-top-left"
    style="width: {MOCK_W}px; transform: scale({scale})"
  >
    <div
      style="transform: perspective(1600px) rotateX({tilt.rotateX}deg) scale({tilt.scale}) translateY({tilt.y}px); transform-origin: 50% 0%"
    >
      <WindowFrame title={hero.mockTitle} class="w-[1018px]">
        <div inert aria-hidden="true" class="flex h-[554px] bg-[#0a0e1c] text-white">
          <!-- Sidebar -->
          <div class="flex w-[190px] shrink-0 flex-col border-r border-white/10 px-3 py-4">
            <div class="px-2 pb-5">
              <span class="block text-[13px] font-bold tracking-[0.02em] text-[#ff6a3d]">OPENSTREAM TV</span>
            </div>
            <div class="grid gap-1">
              {#each appNav as item (item.label)}
                {@const Icon = item.icon}
                <span
                  class={cn(
                    'flex h-8 items-center gap-2.5 rounded-sm px-2.5 text-[13px]',
                    item.active ? 'bg-white/10 font-medium text-white' : 'text-[#9aa3b8]',
                  )}
                >
                  <Icon size={15} strokeWidth={1.75} />
                  {item.label}
                </span>
              {/each}
            </div>
            <div class="mt-auto flex items-center gap-2 rounded-sm border border-white/10 px-2.5 py-2">
              <span class="size-1.5 rounded-full bg-[#3ddc97]"></span>
              <span class="text-[11px] text-[#b4bacb]">JioTV · signed in</span>
            </div>
          </div>

          <!-- Main column -->
          <div class="flex min-w-0 flex-1 flex-col px-6 pt-5">
            <div class="flex items-center gap-3">
              <p class="text-[17px] font-bold tracking-[-0.02em]">Live TV</p>
              <div class="ml-auto flex items-center gap-2">
                <span
                  class="flex h-8 w-[190px] items-center gap-2 rounded-sm border border-white/10 bg-white/5 px-2.5 text-[12px] text-[#7d879e]"
                >
                  <Search size={14} />
                  Search channels
                </span>
                <span class="grid size-8 place-items-center rounded-full bg-[#f24e1e] text-[11px] font-bold">L</span>
              </div>
            </div>

            <!-- Player -->
            <div class="relative mt-4 overflow-hidden rounded-md bg-gradient-to-br from-[#1b2540] via-[#111a30] to-[#251a2c]">
              <div
                class="absolute inset-0 opacity-[0.35] [background-image:radial-gradient(circle_at_30%_20%,#f24e1e44,transparent_55%),radial-gradient(circle_at_75%_75%,#149e9344,transparent_55%)]"
              ></div>
              <div class="relative flex h-[252px] flex-col justify-between p-4">
                <div class="flex items-center gap-2">
                  <span
                    class="flex items-center gap-1.5 rounded-xs bg-[#f24e1e] px-1.5 py-0.5 text-[10px] font-bold tracking-wide"
                  >
                    <span class="size-1.5 rounded-full bg-white"></span> LIVE
                  </span>
                  <span class="rounded-xs bg-black/40 px-1.5 py-0.5 text-[10px] font-medium text-[#d6dae6]">HD</span>
                </div>
                <div class="grid place-items-center">
                  <span
                    class="grid size-14 place-items-center rounded-full bg-white/15 ring-1 ring-white/25 backdrop-blur"
                  >
                    <Play size={24} class="ml-0.5 fill-white text-white" />
                  </span>
                </div>
                <div>
                  <p class="text-[19px] font-bold tracking-[-0.02em]">Arena Sports 1</p>
                  <p class="mt-0.5 text-[12px] text-[#c4c9d6]">Matchday Live · 20:00 – 22:30</p>
                  <div class="mt-3 flex items-center gap-3">
                    <span class="relative h-1 flex-1 overflow-hidden rounded-full bg-white/20">
                      <span class="absolute inset-y-0 left-0 w-[62%] rounded-full bg-[#f24e1e]"></span>
                    </span>
                    <span class="text-[10px] text-[#9aa3b8] tabular-nums">20:42</span>
                    <Volume2 size={14} class="text-[#9aa3b8]" />
                  </div>
                </div>
              </div>
            </div>

            <!-- On now -->
            <p class="mt-5 text-[11px] font-semibold tracking-wider text-[#7d879e] uppercase">On now</p>
            <div class="mt-2.5 grid grid-cols-3 gap-3">
              {#each onNow as programme (programme.title)}
                <div class="rounded-sm border border-white/10 bg-white/[0.04] p-3">
                  <p class="text-[10px] text-[#7d879e] tabular-nums">{programme.time}</p>
                  <p class="mt-1 truncate text-[13px] font-semibold">{programme.title}</p>
                  <p class="truncate text-[11px] text-[#9aa3b8]">{programme.channel}</p>
                  <span class="mt-2.5 block h-0.5 overflow-hidden rounded-full bg-white/15">
                    <span class="block h-full rounded-full bg-[#3ddc97]" style="width: {programme.progress}"></span>
                  </span>
                </div>
              {/each}
            </div>
          </div>

          <!-- Channel rail -->
          <div class="ml-5 flex w-[236px] shrink-0 flex-col border-l border-white/10 px-4 pt-5 pb-4">
            <div class="flex items-center justify-between">
              <p class="text-[12px] font-semibold">Channels</p>
              <span class="text-[10px] text-[#7d879e]">1–6 of 248</span>
            </div>
            <div class="mt-3 grid gap-1">
              {#each channels as [num, name, genre], i (num)}
                <div
                  class={cn(
                    'flex items-center gap-2.5 rounded-sm px-2 py-2',
                    i === 0 ? 'bg-[#f24e1e]/15 ring-1 ring-[#f24e1e]/40' : 'hover:bg-white/5',
                  )}
                >
                  <span class="w-6 text-[11px] text-[#7d879e] tabular-nums">{num}</span>
                  <span class="min-w-0">
                    <span class={cn('block truncate text-[12px]', i === 0 ? 'font-semibold text-white' : 'text-[#d6dae6]')}>
                      {name}
                    </span>
                    <span class="block text-[10px] text-[#7d879e]">{genre}</span>
                  </span>
                </div>
              {/each}
            </div>
            <p class="mt-auto text-[10px] leading-4 text-[#7d879e]">M3U and XMLTV ready for VLC, Kodi and TiviMate.</p>
          </div>
        </div>
      </WindowFrame>
    </div>
  </div>
</div>
