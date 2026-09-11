// Generates web/v0/src/guest-bytes.ts from guest.wat's declared surface.
//
// The guest is a fixed trampoline module (imports + memory + exported call
// thunks), so we encode the WASM binary directly — no external toolchain.
// Safety: the encoder must first reproduce the historical echo+write module
// byte-for-byte; if the baseline changes, this script refuses to emit.
//
// Usage: node scripts/assemble-guest.mjs   (from repo root)
import { readFileSync, writeFileSync } from "node:fs";

/** unsigned LEB128 */
function uleb(n) {
  const out = [];
  do {
    let b = n & 0x7f;
    n >>>= 7;
    if (n !== 0) b |= 0x80;
    out.push(b);
  } while (n !== 0);
  return out;
}

function section(id, payload) {
  return [id, ...uleb(payload.length), ...payload];
}

function vec(items) {
  return [...uleb(items.length), ...items.flat()];
}

function name(s) {
  const bytes = [...new TextEncoder().encode(s)];
  return [...uleb(bytes.length), ...bytes];
}

function functype(params, results) {
  return [0x60, ...uleb(params.length), ...params, ...uleb(results.length), ...results];
}

const I32 = 0x7f;
// Signatures: (i32,i32) -> (), (i32,i32,i32,i32) -> i32, (i32,i32) -> i32
const T_II_V = 0;
const T_IIII_I = 1;
const T_II_I = 2;
const TYPE_DEFS = [
  functype([I32, I32], []),
  functype([I32, I32, I32, I32], [I32]),
  functype([I32, I32], [I32]),
];

/**
 * Build the guest module. Type section contains only the signatures used.
 * imports: [{name, type}]  — host.<name> with signature type index
 * thunks:  [{name, type}]  — exported `run_<name>` funcs calling import i,
 *                            passing through (ptr,len) or (pp,pl,bp,bl).
 */
function buildModule({ imports, thunks }) {
  const used = [...imports, ...thunks].map((e) => e.type);
  const typeIdx = new Map();
  for (const t of used) {
    if (!typeIdx.has(t)) typeIdx.set(t, typeIdx.size);
  }
  const types = [...typeIdx.keys()].sort((a, b) => typeIdx.get(a) - typeIdx.get(b));
  const typeSection = section(1, vec(types.map((t) => TYPE_DEFS[t])));
  const mapType = (t) => typeIdx.get(t);
  const importSection = section(
    2,
    vec(imports.map((imp) => [...name("host"), ...name(imp.name), 0x00, ...uleb(mapType(imp.type))])),
  );
  const functionSection = section(3, vec(thunks.map((t) => uleb(mapType(t.type)))));
  const memorySection = section(5, vec([[0x00, 0x01]]));
  const exportSection = section(
    7,
    vec([
      [...name("memory"), 0x02, ...uleb(0)],
      ...thunks.map((t, i) => [...name(`run_${t.name}`), 0x00, ...uleb(imports.length + i)]),
    ]),
  );
  const bodies = thunks.map((t) => {
    const callIdx = imports.findIndex((i) => i.name === t.name);
    const code =
      t.type === T_IIII_I
        ? [0x20, 0x00, 0x20, 0x01, 0x20, 0x02, 0x20, 0x03, 0x10, ...uleb(callIdx), 0x0b]
        : [0x20, 0x00, 0x20, 0x01, 0x10, ...uleb(callIdx), 0x0b];
    const body = [...uleb(0), ...code]; // no locals
    return [...uleb(body.length), ...body];
  });
  const codeSection = section(10, vec(bodies));
  const bytes = [
    0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00,
    ...typeSection, ...importSection, ...functionSection, ...memorySection,
    ...exportSection, ...codeSection,
  ];
  return new Uint8Array(bytes);
}

// Historical baseline (echo + write only) — must be reproduced exactly.
const BASELINE_BASE64 =
  "AGFzbQEAAAABDgJgAn9/AGAEf39/fwF/AhoCBGhvc3QEZWNobwAABGhvc3QFd3JpdGUAAQMDAgABBQMBAAEHIQMGbWVtb3J5AgAIcnVuX2VjaG8AAglydW5fd3JpdGUAAwoXAggAIAAgARAACwwAIAAgASACIAMQAQs=";

const baseline = buildModule({
  imports: [
    { name: "echo", type: T_II_V },
    { name: "write", type: T_IIII_I },
  ],
  thunks: [
    { name: "echo", type: T_II_V },
    { name: "write", type: T_IIII_I },
  ],
});
const baselineB64 = Buffer.from(baseline).toString("base64");
if (baselineB64 !== BASELINE_BASE64) {
  console.error("encoder does not reproduce the historical guest bytes; refusing to emit");
  console.error("  expected:", BASELINE_BASE64);
  console.error("  actual:  ", baselineB64);
  process.exit(1);
}
console.log("baseline guest reproduced byte-for-byte");

// Phase 8 module: read / append / list host imports + thunks.
const guest = buildModule({
  imports: [
    { name: "echo", type: T_II_V },
    { name: "write", type: T_IIII_I },
    { name: "read", type: T_II_V },
    { name: "append", type: T_IIII_I },
    { name: "list", type: T_II_I },
  ],
  thunks: [
    { name: "echo", type: T_II_V },
    { name: "write", type: T_IIII_I },
    { name: "read", type: T_II_V },
    { name: "append", type: T_IIII_I },
    { name: "list", type: T_II_I },
  ],
});

// Cross-check against guest.wat: every declared import/export must exist.
const wat = readFileSync(new URL("../web/v0/src/guest.wat", import.meta.url), "utf8");
for (const imp of ["echo", "write", "read", "append", "list"]) {
  if (!wat.includes(`(import "host" "${imp}"`)) {
    console.error(`guest.wat is missing import: ${imp}`);
    process.exit(1);
  }
  if (!wat.includes(`(export "run_${imp}"`)) {
    console.error(`guest.wat is missing export: run_${imp}`);
    process.exit(1);
  }
}

const out = `/* generated by scripts/assemble-guest.mjs — do not edit */
export const GUEST_WASM_BASE64 =
  "${Buffer.from(guest).toString("base64")}";

export function guestWasmBytes(): Uint8Array {
  const bin = atob(GUEST_WASM_BASE64);
  const out = new Uint8Array(bin.length);
  for (let i = 0; i < bin.length; i++) out[i] = bin.charCodeAt(i);
  return out;
}
`;
writeFileSync(new URL("../web/v0/src/guest-bytes.ts", import.meta.url), out);
console.log(`guest-bytes.ts written (${guest.length} bytes wasm)`);
