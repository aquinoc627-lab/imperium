# Phase 19 — Voice input (normative)

Status: implementing
Gate: `just v0` plus tests listed below

## Goal

A user can dictate a natural-language sentence into the **browser workbench**
and have that text land in the same Source box that Compile / Propose already
read. Voice is only another way to type. The gauntlet is unchanged.

## Normative behavior

1. Workbench exposes a **Hold to talk** (or click-to-toggle) control next to
   Compile / Propose.
2. On a user gesture, the workbench starts the browser **SpeechRecognition**
   API (or `webkitSpeechRecognition`). Continuous mode is off. Interim
   results may update the Source box live; the **final** result replaces the
   box contents with the normalized transcript.
3. Recognition stops on end-of-utterance, a second click, or an error.
4. The resulting string is ordinary Source text. The user must still press
   **Compile** or **Propose**. Voice never calls simulate, approve, or
   execute.
5. If SpeechRecognition is unavailable, or the user denies the mic, the
   control is disabled or shows a fail-closed message in the Gauntlet error
   strip. There is no silent fallback that invents text.
6. Pure helpers `normalizeTranscript` and `isSpeechRecognitionAvailable` live
   in `web/v0/src/voice_transcript.ts` and are covered by node tests. The
   workbench may inline the same normalize rules; behavior must match the
   tests.

## Fail-closed cases

- No SpeechRecognition constructor in the environment → control disabled,
  clear message.
- `not-allowed` / permission denied → stop listening, surface error, do not
  retry without a new gesture.
- Empty final transcript → no Source mutation; toast/error, not a blank
  compile.
- Network / service-not-allowed from the browser STT engine → surface the
  error string; do not auto-retry.

## Audit events

None in the kernel or ledger. Voice does not create intents. After Compile,
existing `IntentCompiled` / `IntentProposed` events apply as today.

## Tests

- `web/v0/src/voice_transcript.test.ts` (node:test):
  - `normalizeTranscript` trims and collapses whitespace
  - empty / whitespace-only remains empty after normalize
  - `isSpeechRecognitionAvailable` is true when either constructor exists,
    false on a bare object
- Manual smoke (not CI): mic → transcript in Source → Compile still works

## Non-goals

- Text-to-speech / spoken replies
- Always-on wake word or background listening
- Auto-compile or auto-execute after speech
- CLI microphone capture (`imperium voice …`)
- On-device-only STT guarantee (see honesty notes)
- Expanding the Rust kernel or shared contract fixtures
- Reviving `crates/imperium-voice` scaffolding

## Honesty notes

Browser **SpeechRecognition** is typically implemented by the browser vendor
and may send audio to that vendor's servers (e.g. Chromium → Google). This
phase does **not** claim air-gapped or on-device speech. Operators who need
offline STT must wait for a separate named spec (local model or CLI capture).

Phase 19 is **input only**. It does not change approval, tokens, policy, or
network grants.
