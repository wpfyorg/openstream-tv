import { motion, useReducedMotion, useScroll, useTransform } from 'motion/react'
import { CalendarClock, History, ListVideo, Play, Search, Settings, Tv, Volume2 } from 'lucide-react'
import { useEffect, useRef, useState } from 'react'

import { WindowFrame } from '../components/WindowFrame'
import { hero } from '../content'
import { duration, ease } from '../lib/motion'
import { cn } from '../lib/cn'

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

/**
  The framed app window in the hero: a still of the OpenStream TV web player,
  scaled to fit and inert (a picture you can't tab into). It tilts up into
  place as you scroll.
*/
export function HeroMock() {
  const wrap = useRef<HTMLDivElement>(null)
  const [scale, setScale] = useState(1)
  const reduce = useReducedMotion()
  const { scrollYProgress } = useScroll({ target: wrap, offset: ['start end', 'start 0.66'] })
  const rotateX = useTransform(scrollYProgress, [0, 1], [reduce ? 0 : 22, 0])
  const scaleIn = useTransform(scrollYProgress, [0, 1], [reduce ? 1 : 0.9, 1])
  const y = useTransform(scrollYProgress, [0, 1], [reduce ? 0 : 60, 0])

  useEffect(() => {
    const el = wrap.current
    if (!el) return
    const ro = new ResizeObserver(([e]) => setScale(Math.min(1, e.contentRect.width / MOCK_W)))
    ro.observe(el)
    return () => ro.disconnect()
  }, [])

  return (
    <div ref={wrap} className="relative mx-auto w-full max-w-[1018px] [perspective:1600px]" style={{ height: MOCK_H * scale }}>
      <motion.div
        initial={reduce ? false : { opacity: 0, y: 40 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: duration.reveal, delay: 0.35, ease }}
        className="absolute top-0 left-0 origin-top-left"
        style={{ width: MOCK_W, scale }}
      >
        <motion.div style={{ rotateX, scale: scaleIn, y, transformOrigin: '50% 0%' }}>
          <WindowFrame title={hero.mockTitle} className="w-[1018px]">
            <div inert aria-hidden className="flex h-[554px] bg-[#0a0e1c] text-white">
              {/* Sidebar */}
              <div className="flex w-[190px] shrink-0 flex-col border-r border-white/10 px-3 py-4">
                <div className="px-2 pb-5">
                  <span className="block text-[13px] font-bold tracking-[0.02em] text-[#ff6a3d]">OPENSTREAM TV</span>
                </div>
                <div className="grid gap-1">
                  {appNav.map((item) => (
                    <span
                      key={item.label}
                      className={cn(
                        'flex h-8 items-center gap-2.5 rounded-sm px-2.5 text-[13px]',
                        item.active ? 'bg-white/10 font-medium text-white' : 'text-[#9aa3b8]',
                      )}
                    >
                      <item.icon className="size-[15px]" strokeWidth={1.75} />
                      {item.label}
                    </span>
                  ))}
                </div>
                <div className="mt-auto flex items-center gap-2 rounded-sm border border-white/10 px-2.5 py-2">
                  <span className="size-1.5 rounded-full bg-[#3ddc97]" />
                  <span className="text-[11px] text-[#b4bacb]">JioTV · signed in</span>
                </div>
              </div>

              {/* Main column */}
              <div className="flex min-w-0 flex-1 flex-col px-6 pt-5">
                <div className="flex items-center gap-3">
                  <p className="text-[17px] font-bold tracking-[-0.02em]">Live TV</p>
                  <div className="ml-auto flex items-center gap-2">
                    <span className="flex h-8 w-[190px] items-center gap-2 rounded-sm border border-white/10 bg-white/5 px-2.5 text-[12px] text-[#7d879e]">
                      <Search className="size-3.5" />
                      Search channels
                    </span>
                    <span className="grid size-8 place-items-center rounded-full bg-[#f24e1e] text-[11px] font-bold">L</span>
                  </div>
                </div>

                {/* Player */}
                <div className="relative mt-4 overflow-hidden rounded-md bg-gradient-to-br from-[#1b2540] via-[#111a30] to-[#251a2c]">
                  <div className="absolute inset-0 opacity-[0.35] [background-image:radial-gradient(circle_at_30%_20%,#f24e1e44,transparent_55%),radial-gradient(circle_at_75%_75%,#149e9344,transparent_55%)]" />
                  <div className="relative flex h-[252px] flex-col justify-between p-4">
                    <div className="flex items-center gap-2">
                      <span className="flex items-center gap-1.5 rounded-xs bg-[#f24e1e] px-1.5 py-0.5 text-[10px] font-bold tracking-wide">
                        <span className="size-1.5 rounded-full bg-white" /> LIVE
                      </span>
                      <span className="rounded-xs bg-black/40 px-1.5 py-0.5 text-[10px] font-medium text-[#d6dae6]">HD</span>
                    </div>
                    <div className="grid place-items-center">
                      <span className="grid size-14 place-items-center rounded-full bg-white/15 ring-1 ring-white/25 backdrop-blur">
                        <Play className="ml-0.5 size-6 fill-white text-white" />
                      </span>
                    </div>
                    <div>
                      <p className="text-[19px] font-bold tracking-[-0.02em]">Arena Sports 1</p>
                      <p className="mt-0.5 text-[12px] text-[#c4c9d6]">Matchday Live · 20:00 – 22:30</p>
                      <div className="mt-3 flex items-center gap-3">
                        <span className="relative h-1 flex-1 overflow-hidden rounded-full bg-white/20">
                          <span className="absolute inset-y-0 left-0 w-[62%] rounded-full bg-[#f24e1e]" />
                        </span>
                        <span className="text-[10px] text-[#9aa3b8] tabular-nums">20:42</span>
                        <Volume2 className="size-3.5 text-[#9aa3b8]" />
                      </div>
                    </div>
                  </div>
                </div>

                {/* On now */}
                <p className="mt-5 text-[11px] font-semibold tracking-wider text-[#7d879e] uppercase">On now</p>
                <div className="mt-2.5 grid grid-cols-3 gap-3">
                  {onNow.map((p) => (
                    <div key={p.title} className="rounded-sm border border-white/10 bg-white/[0.04] p-3">
                      <p className="text-[10px] text-[#7d879e] tabular-nums">{p.time}</p>
                      <p className="mt-1 truncate text-[13px] font-semibold">{p.title}</p>
                      <p className="truncate text-[11px] text-[#9aa3b8]">{p.channel}</p>
                      <span className="mt-2.5 block h-0.5 overflow-hidden rounded-full bg-white/15">
                        <span className="block h-full rounded-full bg-[#3ddc97]" style={{ width: p.progress }} />
                      </span>
                    </div>
                  ))}
                </div>
              </div>

              {/* Channel rail */}
              <div className="ml-5 flex w-[236px] shrink-0 flex-col border-l border-white/10 px-4 pt-5 pb-4">
                <div className="flex items-center justify-between">
                  <p className="text-[12px] font-semibold">Channels</p>
                  <span className="text-[10px] text-[#7d879e]">1–6 of 248</span>
                </div>
                <div className="mt-3 grid gap-1">
                  {channels.map(([num, name, genre], i) => (
                    <div
                      key={num}
                      className={cn(
                        'flex items-center gap-2.5 rounded-sm px-2 py-2',
                        i === 0 ? 'bg-[#f24e1e]/15 ring-1 ring-[#f24e1e]/40' : 'hover:bg-white/5',
                      )}
                    >
                      <span className="w-6 text-[11px] text-[#7d879e] tabular-nums">{num}</span>
                      <span className="min-w-0">
                        <span className={cn('block truncate text-[12px]', i === 0 ? 'font-semibold text-white' : 'text-[#d6dae6]')}>
                          {name}
                        </span>
                        <span className="block text-[10px] text-[#7d879e]">{genre}</span>
                      </span>
                    </div>
                  ))}
                </div>
                <p className="mt-auto text-[10px] leading-4 text-[#7d879e]">M3U and XMLTV ready for VLC, Kodi and TiviMate.</p>
              </div>
            </div>
          </WindowFrame>
        </motion.div>
      </motion.div>
    </div>
  )
}
