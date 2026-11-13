import assert from "node:assert/strict";
import test from "node:test";
import {
  channelLanguage,
  formatRange,
  languageLabels,
  matchesLanguage,
  nextProgram,
  programAt,
  posterUrl,
  programProgress,
  UNSPECIFIED,
} from "./guide.js";

// Two programmes: 20:00–21:00 and 21:00–22:00 on a fixed day.
const day = new Date(2026, 0, 15).getTime();
const guide = [
  { showname: "Matchday", startEpoch: day + 20 * 3600e3, endEpoch: day + 21 * 3600e3 },
  { showname: "Bulletin", startEpoch: day + 21 * 3600e3, endEpoch: day + 22 * 3600e3 },
];

test("programAt picks the airing slot and rejects gaps", () => {
  assert.equal(programAt(guide, day + 20.5 * 3600e3).showname, "Matchday");
  assert.equal(programAt(guide, day + 21.5 * 3600e3).showname, "Bulletin");
  assert.equal(programAt(guide, day + 18 * 3600e3), null);
  // A programme's end is exclusive, so the next one owns that instant.
  assert.equal(programAt(guide, day + 21 * 3600e3).showname, "Bulletin");
});

test("nextProgram finds the soonest upcoming slot, not the first in the list", () => {
  const shuffled = [guide[1], guide[0]];
  assert.equal(nextProgram(shuffled, day + 19 * 3600e3).showname, "Matchday");
  assert.equal(nextProgram(guide, day + 23 * 3600e3), null);
});

test("programProgress is clamped and survives empty or zero-length slots", () => {
  assert.equal(programProgress(guide[0], day + 20.5 * 3600e3), 50);
  assert.equal(programProgress(guide[0], day + 18 * 3600e3), 0);
  assert.equal(programProgress(guide[0], day + 23 * 3600e3), 100);
  assert.equal(programProgress({ startEpoch: 5, endEpoch: 5 }, 5), 0);
  assert.equal(programProgress(null, 0), 0);
});

test("formatRange only formats programmes that have both times", () => {
  assert.match(formatRange(guide[0]), /\d{2}:\d{2}.*–.*\d{2}:\d{2}/);
  assert.equal(formatRange({ showname: "No times" }), "");
  assert.equal(formatRange(null), "");
});

test("matchesLanguage treats an empty selection as every language", () => {
  assert.equal(matchesLanguage({ language: "Hindi" }, []), true);
  assert.equal(matchesLanguage({ language: "" }, []), true);
  assert.equal(matchesLanguage({ language: "Hindi" }, undefined), true);
});

test("matchesLanguage accepts every selected language, not just the first", () => {
  const hindi = { language: "Hindi" };
  const english = { language: "English" };
  const tamil = { language: "Tamil" };
  assert.equal(matchesLanguage(hindi, ["Hindi", "English"]), true);
  assert.equal(matchesLanguage(english, ["Hindi", "English"]), true);
  assert.equal(matchesLanguage(tamil, ["Hindi", "English"]), false);
});

test("the catalogue's \"All Languages\" label is treated as no language", () => {
  const pseudo = { language: "All Languages" };
  assert.equal(channelLanguage(pseudo), "");
  assert.equal(channelLanguage({ language: " Hindi " }), "Hindi");
  assert.equal(matchesLanguage(pseudo, ["Hindi"]), false);
  assert.equal(matchesLanguage(pseudo, [UNSPECIFIED]), true);
  assert.deepEqual(
    languageLabels([pseudo, { language: "Tamil" }, { language: "Hindi" }, { language: "" }]),
    ["Hindi", "Tamil"],
  );
});

test("channels without a language are matched only by the Unspecified value", () => {
  const unknown = { language: "" };
  assert.equal(matchesLanguage(unknown, [UNSPECIFIED]), true);
  assert.equal(matchesLanguage(unknown, ["Hindi"]), false);
  assert.equal(matchesLanguage({ language: "Hindi" }, [UNSPECIFIED]), false);
  assert.equal(matchesLanguage(unknown, [UNSPECIFIED, "Hindi"]), true);
});

test("poster URLs route JioTV's relative guide paths through the proxy", () => {
  assert.equal(posterUrl("epgdata/abc.jpg"), "/jtvposter/epgdata/abc.jpg");
  assert.equal(posterUrl("https://cdn.example/p.jpg"), "https://cdn.example/p.jpg");
  assert.equal(posterUrl("/jtvposter/x/y.jpg"), "/jtvposter/x/y.jpg");
  assert.equal(posterUrl(""), "");
  assert.equal(posterUrl("some/other/path.jpg"), "");
});
