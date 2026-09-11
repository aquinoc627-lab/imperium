import assert from "node:assert/strict";
import test from "node:test";
import {
  normalizeTranscript,
  isSpeechRecognitionAvailable,
} from "./voice_transcript.ts";

test("normalizeTranscript trims and collapses whitespace", () => {
  assert.equal(
    normalizeTranscript("  Echo   this message: ping  "),
    "Echo this message: ping",
  );
});

test("normalizeTranscript leaves empty input empty", () => {
  assert.equal(normalizeTranscript(""), "");
  assert.equal(normalizeTranscript("   \n\t  "), "");
});

test("isSpeechRecognitionAvailable detects either constructor", () => {
  assert.equal(isSpeechRecognitionAvailable({}), false);
  assert.equal(isSpeechRecognitionAvailable({ SpeechRecognition: class {} }), true);
  assert.equal(
    isSpeechRecognitionAvailable({ webkitSpeechRecognition: class {} }),
    true,
  );
});
