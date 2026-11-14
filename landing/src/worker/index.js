import { Hono } from 'hono'
import { bodyLimit } from 'hono/body-limit'

// The Worker in front of the static site. There is no backend: the landing
// page is static and every route falls through to the single-page app.
const app = new Hono()

app.use('/api/*', bodyLimit({ maxSize: 16 * 1024, onError: (c) => c.json({ error: 'Request is too large.' }, 413) }))

app.get('/api/health', (c) => c.json({ ok: true }))

app.all('/api/*', (c) => c.json({ error: 'Not found' }, 404))

app.all('*', (c) => c.env.ASSETS.fetch(c.req.raw))

export default app
