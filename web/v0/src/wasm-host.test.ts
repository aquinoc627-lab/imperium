import assert from "node:assert/strict";
import test from "node:test";
import { guestWasmBytes } from "./guest-bytes.ts";
import { pathAllowed } from "./token.ts";

async function run(
  op: { kind: "echo"; text: string } | { kind: "write"; path: string; contents: string },
  rights: { echo: boolean; write: boolean; fsPrefixes: string[] },
) {
  let echoOut = "";
  let written: { path: string; contents: string } | null = null;
  let memory: WebAssembly.Memory;
  const { instance } = (await WebAssembly.instantiate(guestWasmBytes(), {
    host: {
      echo: (ptr: number, len: number) => {
        if (!rights.echo) throw new Error("host.echo denied");
        echoOut = new TextDecoder().decode(
          new Uint8Array(memory.buffer).subarray(ptr, ptr + len),
        );
      },
      write: (pp: number, pl: number, bp: number, bl: number) => {
        if (!rights.write) throw new Error("host.write denied");
        const mem = new Uint8Array(memory.buffer);
        const path = new TextDecoder().decode(mem.subarray(pp, pp + pl));
        const contents = new TextDecoder().decode(mem.subarray(bp, bp + bl));
        if (!pathAllowed(path, rights.fsPrefixes)) throw new Error("host.write path denied");
        written = { path, contents };
        return contents.length;
      },
    },
  })) as WebAssembly.WebAssemblyInstantiatedSource;
  memory = instance.exports.memory as WebAssembly.Memory;
  const enc = new TextEncoder();
  if (op.kind === "echo") {
    if (!rights.echo) throw new Error("host.echo denied");
    const bytes = enc.encode(op.text);
    new Uint8Array(memory.buffer).set(bytes, 64);
    (instance.exports.run_echo as (a: number, b: number) => void)(64, bytes.length);
    return { echoOut, written };
  }
  if (!rights.write) throw new Error("host.write denied");
  const p = enc.encode(op.path);
  const c = enc.encode(op.contents);
  const mem = new Uint8Array(memory.buffer);
  mem.set(p, 64);
  mem.set(c, 64 + p.length + 8);
  (instance.exports.run_write as (a: number, b: number, c: number, d: number) => number)(
    64,
    p.length,
    64 + p.length + 8,
    c.length,
  );
  return { echoOut, written };
}

test("echo goes through the wasm guest", async () => {
  const r = await run({ kind: "echo", text: "ping" }, {
    echo: true,
    write: false,
    fsPrefixes: [],
  });
  assert.equal(r.echoOut, "ping");
});

test("write goes through the wasm guest", async () => {
  const r = await run(
    { kind: "write", path: "scratch/a.txt", contents: "hi" },
    { echo: false, write: true, fsPrefixes: ["scratch"] },
  );
  assert.deepEqual(r.written, { path: "scratch/a.txt", contents: "hi" });
});

test("write without rights is denied", async () => {
  await assert.rejects(
    () =>
      run(
        { kind: "write", path: "scratch/a.txt", contents: "hi" },
        { echo: true, write: false, fsPrefixes: [] },
      ),
    /denied/,
  );
});

test("write path outside grant is denied", async () => {
  await assert.rejects(
    () =>
      run(
        { kind: "write", path: "etc/passwd", contents: "x" },
        { echo: false, write: true, fsPrefixes: ["scratch"] },
      ),
    /path denied/,
  );
});
