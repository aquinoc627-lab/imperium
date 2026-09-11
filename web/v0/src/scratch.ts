export const SCRATCH_PREFIX = "scratch";

export function resolveScratchPath(
  raw: string,
): { ok: true; path: string } | { ok: false; error: string } {
  const trimmed = raw.trim();
  if (!trimmed) return { ok: false, error: "Path is empty." };
  if (trimmed.includes("\0")) return { ok: false, error: "Path contains NUL." };

  const norm = trimmed.replace(/\\/g, "/");
  if (norm.startsWith("/")) {
    return { ok: false, error: "Absolute paths are denied." };
  }
  if (/^[a-zA-Z]:/.test(norm)) {
    return { ok: false, error: "Drive-letter paths are denied." };
  }

  const parts = norm.split("/").filter((p) => p.length > 0);
  if (parts.some((p) => p === ".." || p === ".")) {
    return { ok: false, error: "Path escape (..) is denied." };
  }
  const rest = parts[0] === SCRATCH_PREFIX ? parts.slice(1) : parts;
  if (rest.length === 0) {
    return { ok: false, error: "Path must name a file under scratch/." };
  }
  if (rest.some((p) => p === ".." || p.includes(".."))) {
    return { ok: false, error: "Path escape is denied." };
  }

  return { ok: true, path: `${SCRATCH_PREFIX}/${rest.join("/")}` };
}
