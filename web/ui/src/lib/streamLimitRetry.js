// The provider allows one live stream. A closed session's licence stream can
// take a few minutes to disappear, so the player retries with a visible
// countdown instead of making the viewer press Try again.
export const STREAM_LIMIT_RETRY_MS = 5000;
export const STREAM_LIMIT_MAX_ATTEMPTS = 24;

export function streamLimitWaitingDetail(attempt, remainingMs) {
  const seconds = Math.max(0, Math.ceil(remainingMs / 1000));
  return `This account allows one live stream. Retrying in ${seconds}s (${attempt}/${STREAM_LIMIT_MAX_ATTEMPTS}).`;
}

export function streamLimitExhaustedDetail() {
  return "This account allows one live stream. Stop playback on any other device, then try again.";
}
