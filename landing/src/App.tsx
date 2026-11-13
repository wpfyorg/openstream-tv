import { MotionConfig } from 'motion/react'
import { useEffect, useMemo } from 'react'

import { brand } from './content'
import { usePath } from './lib/router'
import { Landing } from './pages/Landing'
import { NotFound } from './pages/NotFound'

const title = `${brand.name}: ${brand.tagline}`

// Routes: / (landing) and a friendly 404. Everything lives on one page.
export function App() {
  const path = usePath()
  const route = useMemo(() => (path === '/' ? 'landing' : '404'), [path])
  useEffect(() => {
    document.title = route === '404' ? `Page not found · ${brand.name}` : title
  }, [route])
  return (
    <MotionConfig reducedMotion="user">
      {route === 'landing' ? <Landing /> : <NotFound />}
    </MotionConfig>
  )
}
