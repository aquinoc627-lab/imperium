/**
 * Pure helpers for Phase 19 voice input.
 * Browser SpeechRecognition stays in the workbench; these are testable without a mic.
 */

export function normalizeTranscript(raw: string): string {
  return raw.trim().replace(/\s+/g, " ");
}

/** Feature-detect constructors without assuming a Window type. */
export function isSpeechRecognitionAvailable(g: {
  SpeechRecognition?: unknown;
  webkitSpeechRecognition?: unknown;
}): boolean {
  return (
    typeof g.SpeechRecognition === "function" ||
    typeof g.webkitSpeechRecognition === "function"
  );
}
