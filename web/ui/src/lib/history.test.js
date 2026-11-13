import test from "node:test";
import assert from "node:assert/strict";
import { orderByHistory } from "./history.js";

const channels = [
  { id: "a", name: "A" },
  { id: "b", name: "B" },
  { id: "c", name: "C" },
  { id: "d", name: "D" },
];

const ids = (list) => list.map((c) => c.id);

test("history ordering keeps catalogue order when nothing was watched", () => {
  assert.deepEqual(ids(orderByHistory(channels, [])), ["a", "b", "c", "d"]);
});

test("history ordering leads with the most recently watched channel", () => {
  const history = [
    { id: "c", at: 3 },
    { id: "a", at: 2 },
  ];
  assert.deepEqual(ids(orderByHistory(channels, history)), ["c", "a", "b", "d"]);
});

test("history ordering ignores channels that are no longer in the catalogue", () => {
  const history = [
    { id: "gone", at: 9 },
    { id: "d", at: 8 },
  ];
  assert.deepEqual(ids(orderByHistory(channels, history)), ["d", "a", "b", "c"]);
});

test("history ordering keeps the first position for a duplicated entry", () => {
  const history = [
    { id: "b", at: 5 },
    { id: "b", at: 4 },
    { id: "a", at: 3 },
  ];
  assert.deepEqual(ids(orderByHistory(channels, history)), ["b", "a", "c", "d"]);
});
