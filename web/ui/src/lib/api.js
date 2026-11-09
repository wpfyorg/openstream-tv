// Small fetch wrapper for the OpenStream TV API.
import { signsOutOn401 } from "./authPolicy.js";

export class ApiError extends Error {
  constructor(status, message) {
    super(message);
    this.status = status;
  }
}

// The /k/<key>/ prefix when the page was opened through a key link.
export const keyBase = location.pathname.match(/^\/k\/[0-9a-f]{32}\//)?.[0] ?? null;

export async function api(path, { method = "GET", body } = {}) {
  const res = await fetch(path, {
    method,
    credentials: "same-origin",
    headers: body === undefined ? {} : { "Content-Type": "application/json" },
    body: body === undefined ? undefined : JSON.stringify(body),
  });
  const text = await res.text();
  let data = null;
  try {
    data = text ? JSON.parse(text) : null;
  } catch {
    data = { message: text };
  }
  if (!res.ok) {
    if (res.status === 401 && signsOutOn401(path, data?.message)) {
      window.dispatchEvent(new CustomEvent("openstream:signed-out"));
    }
    throw new ApiError(res.status, data?.message || res.statusText);
  }
  return data;
}

let channelsPromise = null;

// Channels are fetched once per page load.
export function loadChannels(force = false) {
  if (force || !channelsPromise) {
    channelsPromise = api("/api/channels").then((d) => d.channels);
    channelsPromise.catch(() => (channelsPromise = null));
  }
  return channelsPromise;
}

// Drops the cached catalogue so the next load refetches it. Used when the
// active provider changes while the Channels view is unmounted.
export function invalidateChannels() {
  channelsPromise = null;
}

// Shape of the extras unlock code (see docs/config.md and src/unlock.rs).
// Matched before ever sending a search-box query to the server, so an
// ordinary channel search never leaves the browser.
export function looksLikeUnlockCode(s) {
  return /^\d{1,3}[a-z]{3,9}\d{1,2}a\d{1,3}k\d{1,3}n\d{1,3}$/i.test(s.trim());
}

// Optional OTT provider logins. The server owns rate limiting and account
// state; these helpers only name the routes used by the login components.
export function airtelSendOtp(number) {
  return api("/api/airtel/login/sendOTP", { method: "POST", body: { number } });
}

export function airtelVerifyOtp(number, otp) {
  return api("/api/airtel/login/verifyOTP", { method: "POST", body: { number, otp } });
}

// Tata Play answers send-OTP with `accounts` (login references) that must be
// chosen before the OTP can be verified.
export function tataplaySendOtp(number) {
  return api("/api/tataplay/login/sendOTP", { method: "POST", body: { number } });
}

export function tataplayVerifyOtp({ otp, refId, refIdMode = false }) {
  return api("/api/tataplay/login/verifyOTP", {
    method: "POST",
    body: { otp, refId, refIdMode },
  });
}

export function formatTime(ms) {
  return new Date(ms).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
}
