export interface Permissions {
  fs: string[];
  net: string[];
  env: string[];
}

export interface TokenClaims {
  id: string;
  capability: string;
  subject: string;
  intent_id: string;
  permissions: Permissions;
  nonce: string;
  issued_at: number;
  expires_at: number;
}

export interface CapabilityToken extends TokenClaims {
  signature: string;
}

export type VerifyReason =
  | "ok"
  | "empty signature"
  | "invalid signature"
  | "expired"
  | "nonce reused"
  | "revoked"
  | "unknown capability"
  | "network denied"
  | "permission not subset"
  | "subject mismatch"
  | "intent mismatch";

export interface VerifyResult {
  ok: boolean;
  reason: VerifyReason;
}

export const ALLOWED_CAPABILITIES = ["cap.echo", "cap.write"] as const;

export const V0_GRANT: Permissions = {
  fs: [],
  net: [],
  env: [],
};

export const WRITE_GRANT: Permissions = {
  fs: ["scratch"],
  net: [],
  env: [],
};

export function grantForCapability(capability: string): Permissions {
  if (capability === "cap.write") return WRITE_GRANT;
  return V0_GRANT;
}

export function canonicalPayload(claims: TokenClaims): string {
  return JSON.stringify({
    capability: claims.capability,
    expires_at: claims.expires_at,
    id: claims.id,
    intent_id: claims.intent_id,
    issued_at: claims.issued_at,
    nonce: claims.nonce,
    permissions: {
      env: [...claims.permissions.env].sort(),
      fs: [...claims.permissions.fs].sort(),
      net: [...claims.permissions.net].sort(),
    },
    subject: claims.subject,
  });
}

function toHex(buf: ArrayBuffer): string {
  return [...new Uint8Array(buf)]
    .map((b) => b.toString(16).padStart(2, "0"))
    .join("");
}

export async function signClaims(
  claims: TokenClaims,
  secret: string,
): Promise<string> {
  const key = await crypto.subtle.importKey(
    "raw",
    new TextEncoder().encode(secret),
    { name: "HMAC", hash: "SHA-256" },
    false,
    ["sign"],
  );
  const sig = await crypto.subtle.sign(
    "HMAC",
    key,
    new TextEncoder().encode(canonicalPayload(claims)),
  );
  return toHex(sig);
}

export async function issueToken(
  input: Omit<TokenClaims, "id" | "nonce" | "issued_at"> & {
    issued_at?: number;
    id?: string;
    nonce?: string;
  },
  secret: string,
  now = Date.now(),
): Promise<CapabilityToken> {
  const claims: TokenClaims = {
    id: input.id ?? crypto.randomUUID(),
    capability: input.capability,
    subject: input.subject,
    intent_id: input.intent_id,
    permissions: {
      fs: [...input.permissions.fs],
      net: [...input.permissions.net],
      env: [...input.permissions.env],
    },
    nonce: input.nonce ?? crypto.randomUUID(),
    issued_at: input.issued_at ?? now,
    expires_at: input.expires_at,
  };
  const signature = await signClaims(claims, secret);
  return { ...claims, signature };
}

export function pathAllowed(requested: string, grantedPrefixes: string[]): boolean {
  if (grantedPrefixes.length === 0) return false;
  const norm = requested.replace(/\\/g, "/");
  return grantedPrefixes.some((prefix) => {
    const p = prefix.replace(/\\/g, "/");
    return norm === p || norm.startsWith(p.endsWith("/") ? p : `${p}/`);
  });
}

export function isPermissionSubset(
  requested: Permissions,
  granted: Permissions,
): boolean {
  const fsOk = requested.fs.every((p) => pathAllowed(p, granted.fs));
  const netOk = requested.net.every((h) => granted.net.includes(h));
  const envOk = requested.env.every((k) => granted.env.includes(k));
  return fsOk && netOk && envOk;
}

export interface VerifyContext {
  secret: string;
  now?: number;
  seenNonces?: Set<string>;
  revokedIds?: Set<string>;
  expectedSubject?: string;
  expectedIntentId?: string;
  grant?: Permissions;
}

export async function verifyToken(
  token: CapabilityToken,
  ctx: VerifyContext,
): Promise<VerifyResult> {
  if (!token.signature) {
    return { ok: false, reason: "empty signature" };
  }
  const expected = await signClaims(token, ctx.secret);
  if (expected !== token.signature) {
    return { ok: false, reason: "invalid signature" };
  }
  const now = ctx.now ?? Date.now();
  if (now >= token.expires_at) {
    return { ok: false, reason: "expired" };
  }
  if (ctx.revokedIds?.has(token.id)) {
    return { ok: false, reason: "revoked" };
  }
  if (ctx.seenNonces?.has(token.nonce)) {
    return { ok: false, reason: "nonce reused" };
  }
  if (
    !(ALLOWED_CAPABILITIES as readonly string[]).includes(token.capability)
  ) {
    return { ok: false, reason: "unknown capability" };
  }
  if (token.permissions.net.length > 0) {
    return { ok: false, reason: "network denied" };
  }
  const grant = ctx.grant ?? V0_GRANT;
  if (!isPermissionSubset(token.permissions, grant)) {
    return { ok: false, reason: "permission not subset" };
  }
  if (ctx.expectedSubject && token.subject !== ctx.expectedSubject) {
    return { ok: false, reason: "subject mismatch" };
  }
  if (ctx.expectedIntentId && token.intent_id !== ctx.expectedIntentId) {
    return { ok: false, reason: "intent mismatch" };
  }
  return { ok: true, reason: "ok" };
}

export function fingerprint(signature: string): string {
  return signature.slice(0, 12);
}
