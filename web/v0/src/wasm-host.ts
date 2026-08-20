import { pathAllowed } from "./token.ts";
import { guestWasmBytes } from "./guest-bytes.ts";

export interface GuestRights {
  echo: boolean;
  write: boolean;
  fsPrefixes: string[];
}

export interface GuestEffects {
  onEcho: (text: string) => void;
  onWrite: (path: string, contents: string) => void;
}

export type GuestOp =
  | { kind: "echo"; text: string }
  | { kind: "write"; path: string; contents: string };

function writeUtf8(mem: Uint8Array, ptr: number, text: string): number {
  const bytes = new TextEncoder().encode(text);
  if (ptr + bytes.length > mem.length) {
    throw new Error("guest memory overflow");
  }
  mem.set(bytes, ptr);
  return bytes.length;
}

function readUtf8(mem: Uint8Array, ptr: number, len: number): string {
  if (ptr < 0 || len < 0 || ptr + len > mem.length) {
    throw new Error("guest memory out of bounds");
  }
  return new TextDecoder().decode(mem.subarray(ptr, ptr + len));
}

export async function runGuest(
  op: GuestOp,
  rights: GuestRights,
  effects: GuestEffects,
  wasm: Uint8Array = guestWasmBytes(),
): Promise<string> {
  let echoOut = "";
  let memory!: WebAssembly.Memory;
  const imports = {
    host: {
      echo: (ptr: number, len: number) => {
        if (!rights.echo) throw new Error("host.echo denied");
        const mem = new Uint8Array(memory.buffer);
        echoOut = readUtf8(mem, ptr, len);
        effects.onEcho(echoOut);
      },
      write: (pp: number, pl: number, bp: number, bl: number) => {
        if (!rights.write) throw new Error("host.write denied");
        const mem = new Uint8Array(memory.buffer);
        const path = readUtf8(mem, pp, pl);
        const contents = readUtf8(mem, bp, bl);
        if (!pathAllowed(path, rights.fsPrefixes)) {
          throw new Error("host.write path denied");
        }
        effects.onWrite(path, contents);
        return contents.length;
      },
    },
  };

  const instantiated = (await WebAssembly.instantiate(
    wasm,
    imports,
  )) as WebAssembly.Instance | WebAssembly.WebAssemblyInstantiatedSource;
  const instance =
    instantiated instanceof WebAssembly.Instance
      ? instantiated
      : instantiated.instance;
  memory = instance.exports.memory as WebAssembly.Memory;
  const mem = new Uint8Array(memory.buffer);

  if (op.kind === "echo") {
    if (!rights.echo) throw new Error("host.echo denied");
    const run = instance.exports.run_echo as (p: number, n: number) => void;
    const len = writeUtf8(mem, 64, op.text);
    run(64, len);
    return echoOut;
  }

  if (!rights.write) throw new Error("host.write denied");
  const run = instance.exports.run_write as (
    a: number,
    b: number,
    c: number,
    d: number,
  ) => number;
  const pathLen = writeUtf8(mem, 64, op.path);
  const bodyPtr = 64 + pathLen + 8;
  const bodyLen = writeUtf8(mem, bodyPtr, op.contents);
  const n = run(64, pathLen, bodyPtr, bodyLen);
  return `wrote ${op.path} (${n} bytes)`;
}

export function rightsFromToken(capability: string, fsPrefixes: string[]): GuestRights {
  return {
    echo: capability === "cap.echo",
    write: capability === "cap.write",
    fsPrefixes: capability === "cap.write" ? fsPrefixes : [],
  };
}
