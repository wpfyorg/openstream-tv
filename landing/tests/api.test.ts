import { expect, test } from 'bun:test'
import app from '../src/worker/index'

test('health check answers with JSON', async () => {
  const response = await app.request('https://example.test/api/health')
  expect(response.status).toBe(200)
  expect(await response.json()).toEqual({ ok: true })
})

test('oversized requests and unknown API routes never fall through to HTML', async () => {
  const oversized = await app.request('https://example.test/api/health', {
    method: 'POST',
    body: 'x'.repeat(20_000),
  })
  expect(oversized.status).toBe(413)
  const missing = await app.request('https://example.test/api/missing')
  expect(missing.status).toBe(404)
  expect((await missing.json()).error).toBeString()
})
