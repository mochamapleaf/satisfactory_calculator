// node docs/design/shots.mjs outdir prefix [games] [tabs] [schemes] [widths]
// Screenshots the planner (localhost:4173) via Chrome DevTools Protocol.
import { spawn } from 'node:child_process'
import { mkdtempSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

const [outdir, prefix = 'shot', gamesA = 'satisfactory,dsp,factorio', tabsA = 'Plan,Item flow,Machine graph,Item values', schemesA = 'light,dark', widthsA = '1400,390'] = process.argv.slice(2)
const CHROME = 'C:/Program Files/Google/Chrome/Application/chrome.exe'
const port = 9333
const prof = mkdtempSync(join(tmpdir(), 'cdp-'))
const proc = spawn(CHROME, ['--headless=new', `--remote-debugging-port=${port}`, `--user-data-dir=${prof}`, '--hide-scrollbars', 'about:blank'], { stdio: 'ignore' })
const sleep = ms => new Promise(r => setTimeout(r, ms))
let targets
for (let i = 0; i < 50; i++) {
  try { targets = await (await fetch(`http://127.0.0.1:${port}/json`)).json(); if (targets.find(t => t.type === 'page')) break } catch {}
  await sleep(200)
}
const page = targets.find(t => t.type === 'page')
const ws = new WebSocket(page.webSocketDebuggerUrl)
await new Promise(r => ws.addEventListener('open', r))
let id = 0; const pending = new Map()
ws.addEventListener('message', e => { const m = JSON.parse(e.data); if (m.id && pending.has(m.id)) { pending.get(m.id)(m); pending.delete(m.id) } })
const send = (method, params = {}) => new Promise(r => { const i = ++id; pending.set(i, r); ws.send(JSON.stringify({ id: i, method, params })) })
const evalJs = async expr => (await send('Runtime.evaluate', { expression: expr, awaitPromise: true, returnByValue: true })).result?.result?.value

const hashFor = g => Buffer.from(encodeURIComponent(JSON.stringify({ game: g }))).toString('base64')
const report = []
for (const width of widthsA.split(',').map(Number)) {
  const mobile = width < 700
  await send('Emulation.setDeviceMetricsOverride', { width, height: mobile ? 844 : 900, deviceScaleFactor: 1, mobile })
  for (const scheme of schemesA.split(',')) {
    await send('Emulation.setEmulatedMedia', { features: [{ name: 'prefers-color-scheme', value: scheme }] })
    for (const g of gamesA.split(',')) {
      await send('Page.navigate', { url: `http://localhost:4173/?r=${Math.random()}#${hashFor(g)}` })
      await sleep(1500)
      for (const tab of tabsA.split(',')) {
        await evalJs(`[...document.querySelectorAll('.tabs button')].find(b => b.textContent.trim() === ${JSON.stringify(tab)})?.click()`)
        // wait for solve
        for (let i = 0; i < 40; i++) { const busy = await evalJs(`!!document.querySelector('.solving') || /Solving/.test(document.querySelector('main')?.innerText.slice(0,200) ?? '')`); if (!busy) break; await sleep(500) }
        await sleep(700)
        if (mobile) await evalJs(`document.querySelector('main')?.scrollIntoView()`)
        await sleep(150)
        const over = await evalJs(`document.documentElement.scrollWidth - innerWidth`)
        report.push(`${width} ${scheme} ${g} ${tab}: hscroll=${over}`)
        const shot = await send('Page.captureScreenshot', { format: 'png', captureBeyondViewport: false })
        const name = `${prefix}-${g}-${tab.replace(/ /g, '')}-${scheme}-${width}.png`
        writeFileSync(join(outdir, name), Buffer.from(shot.result.data, 'base64'))
      }
    }
  }
}
console.log(report.join('\n'))
ws.close(); proc.kill()
