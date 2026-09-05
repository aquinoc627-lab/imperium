import { isSensitivePath } from "./simulate.ts";
import { pathAllowed } from "./token.ts";
import { guestWasmBytes } from "./guest-bytes.ts";

export interface GuestRights {
  echo: boolean;
  write: boolean;
  read: boolean;
  append: boolean;
  list: boolean;
  fsPrefixes: string[];
}

/** In-process filesystem the guest's host imports act on. */
export interface GuestFs {
  write(path: string, contents: string): number;
  read(path: string): string | null;
  append(path: string, contents: string): number;
  list(path: string): string[];
}

export type GuestOp =
  | { kind: "echo"; text: string }
  | { kind: "write"; path: string; contents: string }
  | { kind: "read"; path: string }
  | { kind: "append"; path: string; contents: string }
  | { kind: "list"; path: string };

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

function checkFs(rights: GuestRights, path: string, op: string): void {
  if (!pathAllowed(path, rights.fsPrefixes)) {
    throw new Error(`host.${op} path denied`);
  }
  if (isSensitivePath(path)) {
    throw new Error(`host.${op} sensitive path denied`);
  }
}

export async function runGuest(
  op: GuestOp,
  rights: GuestRights,
  fs: GuestFs,
  wasm: Uint8Array = guestWasmBytes(),
): Promise<string> {
  let echoOut = "";
  let readOut = "";
  let listOut: string[] = [];
  let memory!: WebAssembly.Memory;
  const imports = {
    host: {
      echo: (ptr: number, len: number) => {
        if (!rights.echo) throw new Error("host.echo denied");
        const mem = new Uint8Array(memory.buffer);
        echoOut = readUtf8(mem, ptr, len);
      },
      write: (pp: number, pl: number, bp: number, bl: number) => {
        if (!rights.write) throw new Error("host.write denied");
        const mem = new Uint8Array(memory.buffer);
        const path = readUtf8(mem, pp, pl);
        const contents = readUtf8(mem, bp, bl);
        checkFs(rights, path, "write");
        return fs.write(path, contents);
      },
      read: (ptr: number, len: number) => {
        if (!rights.read) throw new Error("host.read denied");
        const mem = new Uint8Array(memory.buffer);
        const path = readUtf8(mem, ptr, len);
        checkFs(rights, path, "read");
        const contents = fs.read(path);
        if (contents == null) throw new Error(`host.read not found: ${path}`);
        readOut = contents;
      },
      append: (pp: number, pl: number, bp: number, bl: number) => {
        if (!rights.append) throw new Error("host.append denied");
        const mem = new Uint8Array(memory.buffer);
        const path = readUtf8(mem, pp, pl);
        const contents = readUtf8(mem, bp, bl);
        checkFs(rights, path, "append");
        return fs.append(path, contents);
      },
      list: (ptr: number, len: number) => {
        if (!rights.list) throw new Error("host.list denied");
        const mem = new Uint8Array(memory.buffer);
        const path = readUtf8(mem, ptr, len);
        checkFs(rights, path, "list");
        listOut = fs.list(path);
        return listOut.length;
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

  if (op.kind === "read") {
    if (!rights.read) throw new Error("host.read denied");
    const run = instance.exports.run_read as (p: number, n: number) => void;
    const len = writeUtf8(mem, 64, op.path);
    run(64, len);
    return readOut;
  }

  if (op.kind === "list") {
    if (!rights.list) throw new Error("host.list denied");
    const run = instance.exports.run_list as (p: number, n: number) => number;
    const len = writeUtf8(mem, 64, op.path);
    const n = run(64, len);
    void n;
    return listOut.join("\n");
  }

  if (!rights[op.kind]) throw new Error(`host.${op.kind} denied`);
  const run = instance.exports[`run_${op.kind}`] as (
    a: number,
    b: number,
    c: number,
    d: number,
  ) => number;
  const pathLen = writeUtf8(mem, 64, op.path);
  const bodyPtr = 64 + pathLen + 8;
  const bodyLen = writeUtf8(mem, bodyPtr, op.contents);
  const n = run(64, pathLen, bodyPtr, bodyLen);
  return op.kind === "append"
    ? `appended ${n} bytes to ${op.path}`
    : `wrote ${op.path} (${n} bytes)`;
}

export function rightsFromToken(
  capability: string,
  fsPrefixes: string[],
): GuestRights {
  return {
    echo: capability === "cap.echo",
    write: capability === "cap.write",
    read: capability === "cap.read",
    append: capability === "cap.append",
    list: capability === "cap.list",
    fsPrefixes: fsPrefixes,
  };
}
