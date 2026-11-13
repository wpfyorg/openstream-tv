// Motion tokens shared by components and documented on /styleguide.
export const ease = [0.22, 1, 0.36, 1] as const
export const spring = { type: 'spring', stiffness: 520, damping: 34, mass: 0.8 } as const

export const duration = {
  fast: 0.15,
  base: 0.25,
  slow: 0.5,
  reveal: 0.8,
} as const

export const motionTokens = [
  { name: 'fast', value: `${duration.fast * 1000}ms`, use: 'Hover, press, colour changes' },
  { name: 'base', value: `${duration.base * 1000}ms`, use: 'Dialogs, toasts, popovers' },
  { name: 'slow', value: `${duration.slow * 1000}ms`, use: 'Panels and page sections' },
  { name: 'reveal', value: `${duration.reveal * 1000}ms`, use: 'Scroll reveal, the hero mock' },
  { name: 'ease-out', value: 'cubic-bezier(0.22, 1, 0.36, 1)', use: 'Everything that enters' },
  { name: 'spring', value: 'stiffness 520 / damping 34', use: 'Toggles, the tab indicator, check-in' },
]
