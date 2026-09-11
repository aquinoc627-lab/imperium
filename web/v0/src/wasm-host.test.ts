import assert from "node:assert/strict";
import test from "node:test";
import { runGuest, rightsFromToken, type GuestFs } from "./wasm-host.ts";

function memFs(): GuestFs & { written: Map<string, string> } {
  const files = new Map<string, string>();
  const dirs = new Set<string>();
  const dirOf = (path: string) => {
    const i = path.lastIndexOf("/");
    if (i > 0) dirs.add(path.slice(0, i));
  };
  return {
    written: files,
    write(path, contents) {
      files.set(path, contents);
      dirOf(path);
      return contents.length;
    },
    read(path) {
      return files.get(path) ?? null;
    },
    append(path, contents) {
      files.set(path, (files.get(path) ?? "") + contents);
      dirOf(path);
      return contents.length;
    },
    list(path) {
      const out: string[] = [];
      for (const [p] of files) {
        if (p.startsWith(`${path}/`)) out.push(p.slice(path.length + 1));
      }
      for (const d of dirs) {
        if (d.startsWith(`${path}/`)) out.push(`${d.slice(path.length + 1)}/`);
      }
      return out;
    },
  };
}

const SCRATCH = ["scratch"];

test("echo goes through the wasm guest", async () => {
  const out = await runGuest(
    { kind: "echo", text: "ping" },
    rightsFromToken("cap.echo", []),
    memFs(),
  );
  assert.equal(out, "ping");
});

test("write goes through the wasm guest", async () => {
  const fs = memFs();
  const out = await runGuest(
    { kind: "write", path: "scratch/a.txt", contents: "hi" },
    rightsFromToken("cap.write", SCRATCH),
    fs,
  );
  assert.equal(out, "wrote scratch/a.txt (2 bytes)");
  assert.equal(fs.written.get("scratch/a.txt"), "hi");
});

test("write without rights is denied", async () => {
  await assert.rejects(
    () =>
      runGuest(
        { kind: "write", path: "scratch/a.txt", contents: "hi" },
        rightsFromToken("cap.echo", []),
        memFs(),
      ),
    /host\.write denied/,
  );
});

test("write path outside grant is denied", async () => {
  await assert.rejects(
    () =>
      runGuest(
        { kind: "write", path: "etc/passwd", contents: "x" },
        rightsFromToken("cap.write", SCRATCH),
        memFs(),
      ),
    /host\.write path denied/,
  );
});

test("read returns file contents", async () => {
  const fs = memFs();
  fs.write("scratch/notes.txt", "hello");
  const out = await runGuest(
    { kind: "read", path: "scratch/notes.txt" },
    rightsFromToken("cap.read", SCRATCH),
    fs,
  );
  assert.equal(out, "hello");
});

test("read without rights is denied", async () => {
  await assert.rejects(
    () =>
      runGuest(
        { kind: "read", path: "scratch/notes.txt" },
        rightsFromToken("cap.write", SCRATCH),
        memFs(),
      ),
    /host\.read denied/,
  );
});

test("read of a sensitive path is denied", async () => {
  const fs = memFs();
  fs.write("scratch/.env", "TOP_SECRET=1");
  await assert.rejects(
    () =>
      runGuest(
        { kind: "read", path: "scratch/.env" },
        rightsFromToken("cap.read", SCRATCH),
        fs,
      ),
    /host\.read sensitive path denied/,
  );
  await assert.rejects(
    () =>
      runGuest(
        { kind: "read", path: "scratch/server.key" },
        rightsFromToken("cap.read", SCRATCH),
        fs,
      ),
    /host\.read sensitive path denied/,
  );
});

test("read of a missing file fails", async () => {
  await assert.rejects(
    () =>
      runGuest(
        { kind: "read", path: "scratch/absent.txt" },
        rightsFromToken("cap.read", SCRATCH),
        memFs(),
      ),
    /host\.read not found/,
  );
});

test("append grows the file without truncating", async () => {
  const fs = memFs();
  await runGuest(
    { kind: "append", path: "scratch/log.txt", contents: "one" },
    rightsFromToken("cap.append", SCRATCH),
    fs,
  );
  const out = await runGuest(
    { kind: "append", path: "scratch/log.txt", contents: "two" },
    rightsFromToken("cap.append", SCRATCH),
    fs,
  );
  assert.equal(out, "appended 3 bytes to scratch/log.txt");
  assert.equal(fs.written.get("scratch/log.txt"), "onetwo");
});

test("append without rights is denied", async () => {
  await assert.rejects(
    () =>
      runGuest(
        { kind: "append", path: "scratch/log.txt", contents: "x" },
        rightsFromToken("cap.write", SCRATCH),
        memFs(),
      ),
    /host\.append denied/,
  );
});

test("list returns entries under the prefix", async () => {
  const fs = memFs();
  fs.write("scratch/notes_dir/alpha.txt", "a");
  fs.write("scratch/notes_dir/beta.txt", "b");
  const out = await runGuest(
    { kind: "list", path: "scratch/notes_dir" },
    rightsFromToken("cap.list", SCRATCH),
    fs,
  );
  assert.deepEqual(out.split("\n").sort(), ["alpha.txt", "beta.txt"]);
});

test("list without rights is denied", async () => {
  await assert.rejects(
    () =>
      runGuest(
        { kind: "list", path: "scratch/notes_dir" },
        rightsFromToken("cap.read", SCRATCH),
        memFs(),
      ),
    /host\.list denied/,
  );
});

test("append to a sensitive path is denied", async () => {
  await assert.rejects(
    () =>
      runGuest(
        { kind: "append", path: "scratch/creds.secret", contents: "x" },
        rightsFromToken("cap.append", SCRATCH),
        memFs(),
      ),
    /host\.append sensitive path denied/,
  );
});
