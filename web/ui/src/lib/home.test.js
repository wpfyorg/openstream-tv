import test from "node:test";
import assert from "node:assert/strict";
import { HOME_CATEGORIES, categoryRails, homeSelection } from "./home.js";

const channels = [
  { id: "s1", name: "Sport One", category: "Sports", playable: true },
  { id: "s2", name: "Sport Two", category: "Sports", playable: true },
  { id: "s3", name: "Sport Three", category: "Sports", playable: true },
  { id: "n1", name: "News One", category: "News", playable: true },
  { id: "n2", name: "News Two", category: "News", playable: true },
  { id: "m1", name: "Movie One", category: "Movies", playable: true },
  { id: "m2", name: "Movie Two", category: "Movies", playable: true },
  { id: "a1", name: "Any One", category: "All Categories", playable: true },
];

test("category rails rank the most-watched category first", () => {
  const history = [
    { id: "n1", at: 3 },
    { id: "n2", at: 2 },
    { id: "s1", at: 1 },
  ];
  const rails = categoryRails(channels, "", history);
  assert.deepEqual(rails.map((r) => r.title), ["News", "Sports", "Movies"]);
});

test("category rails skip single-channel and catch-all categories", () => {
  const rails = categoryRails(channels, "", []);
  assert.ok(!rails.some((r) => r.title === "All Categories"));
  assert.ok(rails.every((r) => r.items.length > 1));
});

test("the hero channel never repeats in a rail or the on-now strip", () => {
  const { onNow, rails } = homeSelection(channels, "s1", []);
  assert.ok(!onNow.some((c) => c.id === "s1"));
  assert.ok(rails.every((r) => r.items.every((c) => c.id !== "s1")));
});

test("on-now lists at most six and rails at most six each", () => {
  const many = Array.from({ length: 20 }, (_, i) => ({
    id: `c${i}`,
    name: `Channel ${i}`,
    category: "Sports",
    playable: true,
  }));
  const { onNow, rails, ids } = homeSelection(many, "c0", []);
  assert.equal(onNow.length, 6);
  assert.equal(rails[0].items.length, 6);
  assert.equal(new Set(ids).size, ids.length);
  assert.ok(ids.length <= 60);
});

test("every named home category gets a rail, unused ones are skipped", () => {
  const catalogue = [];
  for (const category of ["Sports", "News", "Kids", "Music"]) {
    for (let i = 0; i < 3; i++) {
      catalogue.push({ id: `${category}${i}`, name: `${category} ${i}`, category, playable: true });
    }
  }
  const rails = categoryRails(catalogue, "", []);
  assert.deepEqual(rails.map((r) => r.title), ["Kids", "Music", "News", "Sports"]);
});

test("named categories lead a catalogue that also has unnamed ones", () => {
  const catalogue = [
    { id: "b1", name: "Biz 1", category: "Business", playable: true },
    { id: "b2", name: "Biz 2", category: "Business", playable: true },
    { id: "b3", name: "Biz 3", category: "Business", playable: true },
    { id: "s1", name: "Sport 1", category: "Sports", playable: true },
    { id: "s2", name: "Sport 2", category: "Sports", playable: true },
  ];
  const rails = categoryRails(catalogue, "", []);
  assert.deepEqual(rails.map((r) => r.title), ["Sports"]);
  assert.ok(HOME_CATEGORIES.includes("Sports"));
});

test("a catalogue without the named categories still gets rails", () => {
  const catalogue = [
    { id: "a1", name: "Alpha 1", category: "Alpha", playable: true },
    { id: "a2", name: "Alpha 2", category: "Alpha", playable: true },
  ];
  assert.deepEqual(categoryRails(catalogue).map((r) => r.title), ["Alpha"]);
});

test("the guide ids cover everything the home screen renders", () => {
  const { onNow, rails, ids } = homeSelection(channels, "s1", []);
  const rendered = new Set(["s1", ...onNow.map((c) => c.id), ...rails.flatMap((r) => r.items.map((c) => c.id))]);
  for (const id of rendered) assert.ok(ids.includes(id), `missing guide id ${id}`);
});
