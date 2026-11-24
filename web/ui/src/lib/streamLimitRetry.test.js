import test from "node:test";
import assert from "node:assert/strict";
import {
  STREAM_LIMIT_MAX_ATTEMPTS,
  STREAM_LIMIT_RETRY_MS,
  streamLimitExhaustedDetail,
  streamLimitWaitingDetail,
} from "./streamLimitRetry.js";

test("stream-limit waiting detail counts down and keeps the attempt number", () => {
  assert.equal(STREAM_LIMIT_RETRY_MS, 5000);
  assert.equal(STREAM_LIMIT_MAX_ATTEMPTS, 24);
  assert.equal(
    streamLimitWaitingDetail(1, 5000),
    "This account allows one live stream. Retrying in 5s (1/24).",
  );
  assert.equal(
    streamLimitWaitingDetail(12, 0),
    "This account allows one live stream. Retrying in 0s (12/24).",
  );
  assert.match(streamLimitExhaustedDetail(), /Stop playback on any other device/);
});
