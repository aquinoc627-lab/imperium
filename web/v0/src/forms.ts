/**
 * Forms + shadow verification — TS reference mirror of
 * `crates/imperium-core/src/forms.rs`. Both validated against the shared
 * fixtures in `tests/contract/v0_kernel.json`.
 */

import type { EffectPreview, IntentIR } from "./types.ts";

export const SLOT_PLACEHOLDER = "{{slot}}";
export const SLOT_NAME = "description";

export interface FormTemplate {
  template: string;
  slot: string;
}

/** Structural template building (mirror of build_template). */
export function buildTemplate(ir: IntentIR): { ok: true; template: FormTemplate } | { ok: false; error: string } {
  const task = ir.tasks[0];
  if (!task) return { ok: false, error: "intent has no tasks" };
  const cap = task.capabilities[0] ?? "";
  if (cap === "cap.http") return { ok: false, error: "fetch intents cannot become forms" };
  const relFor = (target: string) =>
    target.startsWith("scratch/") ? target.slice("scratch/".length) : target;
  let template: string;
  switch (cap) {
    case "cap.echo":
      template = `Echo this message: ${SLOT_PLACEHOLDER}`;
      break;
    case "cap.write": {
      if (!task.target_path) return { ok: false, error: "intent has no target path" };
      template = `Write file ${relFor(task.target_path)} with contents ${SLOT_PLACEHOLDER}`;
      break;
    }
    case "cap.read":
      template = `Read file ${SLOT_PLACEHOLDER}`;
      break;
    case "cap.append": {
      if (!task.target_path) return { ok: false, error: "intent has no target path" };
      template = `Append file ${relFor(task.target_path)} with contents ${SLOT_PLACEHOLDER}`;
      break;
    }
    case "cap.list":
      template = `List files under ${SLOT_PLACEHOLDER}`;
      break;
    default:
      return { ok: false, error: `unknown capability: ${cap}` };
  }
  return { ok: true, template: { template, slot: SLOT_NAME } };
}

export function renderTemplate(template: string, value: string): { ok: true; canonical: string } | { ok: false; error: string } {
  if (!template.includes(SLOT_PLACEHOLDER)) {
    return { ok: false, error: "template has no slot" };
  }
  return { ok: true, canonical: template.replaceAll(SLOT_PLACEHOLDER, value) };
}

function normalizedPath(p: string): string {
  if (p.startsWith("scratch/shadow/")) return p.slice("scratch/shadow/".length);
  if (p.startsWith("scratch/")) return p.slice("scratch/".length);
  return p;
}

function previewMatches(a: EffectPreview, b: EffectPreview): boolean {
  if (a.kind !== b.kind) return false;
  switch (a.kind) {
    case "echo":
      return a.text === (b as typeof a).text;
    case "write":
    case "append": {
      const bb = b as typeof a;
      return normalizedPath(a.path) === normalizedPath(bb.path) && a.bytes === bb.bytes;
    }
    case "read":
    case "list":
      return normalizedPath(a.path) === normalizedPath((b as typeof a).path);
    case "fetch":
      return a.url === (b as typeof a).url;
    default:
      return false;
  }
}

export interface ShadowDiff {
  predicted: EffectPreview[];
  actual: EffectPreview[];
  match: boolean;
}

export function verifyShadow(predicted: EffectPreview[], actual: EffectPreview[]): ShadowDiff {
  const match =
    predicted.length === actual.length &&
    predicted.every((p, i) => previewMatches(p, actual[i]));
  return { predicted, actual, match };
}
