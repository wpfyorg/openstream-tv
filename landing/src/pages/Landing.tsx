import { Cta } from '../sections/Cta'
import { Faq } from '../sections/Faq'
import { Features } from '../sections/Features'
import { Footer } from '../sections/Footer'
import { Hero } from '../sections/Hero'
import { Install } from '../sections/Install'
import { Nav } from '../sections/Nav'
import { Proof } from '../sections/Proof'
import { Providers } from '../sections/Providers'
import { Steps } from '../sections/Steps'

// The marketing page: a stack of sections. Copy lives in src/content.ts.
export function Landing() {
  return (
    <>
      <Nav />
      <main id="main" tabIndex={-1} className="outline-none">
        <Hero />
        <Proof />
        <Features />
        <Steps />
        <Providers />
        <Install />
        <Faq />
        <Cta />
      </main>
      <Footer />
    </>
  )
}
