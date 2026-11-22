// Number formatting shared by every view (own file so node tests can load it without the WASM solver).
export const fmt = (x: number, d = 2) =>
  // trailing zeros go only after a decimal point: fmt(9.7, 0) is "10", not "1"
  Math.abs(x - Math.round(x)) < 1e-6 ? String(Math.round(x)) : d > 0 ? x.toFixed(d).replace(/0+$/, '').replace(/\.$/, '') : x.toFixed(0)
