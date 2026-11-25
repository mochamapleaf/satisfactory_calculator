// Parses a Satisfactory .sav off the main thread. The parser (@etothepii/satisfactory-file-parser 4.1.2, MIT) is
// imported here only, so it is downloaded the first time a save is imported, not with the app.
//   in:  {buf: ArrayBuffer, name, recipes: the game's recipe ids}   out: {progress} … then {summary} | {error}
import { summarizeSave, type ParsedSave } from './saveImport.ts'
import type { GeoNode, SitesDoc } from './model.ts'
import nodesJson from '../../../data/satisfactory-geo/nodes.json'
import sitesJson from '../../../data/satisfactory-geo/sites.json'

self.onmessage = async (e: MessageEvent<{ buf: ArrayBuffer; name: string; recipes: string[] }>) => {
  try {
    const { Parser } = await import('@etothepii/satisfactory-file-parser')
    let last = -1
    const save = Parser.ParseSave(e.data.name, e.data.buf, {
      onProgressCallback: (p: number) => { const q = Math.floor(p * 20); if (q !== last) { last = q; postMessage({ progress: p }) } },
    }) as unknown as ParsedSave
    postMessage({ summary: summarizeSave(save, nodesJson.nodes as GeoNode[], sitesJson as unknown as SitesDoc, new Set(e.data.recipes)) })
  } catch (err) {
    postMessage({ error: String((err as Error)?.message ?? err) })
  }
}
