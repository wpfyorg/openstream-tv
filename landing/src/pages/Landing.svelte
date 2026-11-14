<script>
  import Cta from '../sections/Cta.svelte'
  import Faq from '../sections/Faq.svelte'
  import Features from '../sections/Features.svelte'
  import Footer from '../sections/Footer.svelte'
  import Hero from '../sections/Hero.svelte'
  import Install from '../sections/Install.svelte'
  import Nav from '../sections/Nav.svelte'
  import Proof from '../sections/Proof.svelte'
  import Providers from '../sections/Providers.svelte'
  import Steps from '../sections/Steps.svelte'
</script>

<!-- The marketing page: a stack of sections. Copy lives in src/content.js. -->
<Nav />
<main id="main" tabindex="-1" class="outline-none">
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
