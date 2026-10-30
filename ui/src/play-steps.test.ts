import { expect, test } from "bun:test";
import { stepsAfterError } from "./play-steps";

test("a failed press lists the run's own step lines, then the reason, once", () => {
  const lines = ["Map: Smashcraft 0.3-dev.470", "Starting Warcraft III…", "Warcraft III couldn't start: wine: No such file or directory (os error 2)"];
  expect(stepsAfterError(lines, "Play couldn't start Warcraft III.")).toEqual([...lines, "Play couldn't start Warcraft III."]);
  expect(stepsAfterError([...lines, "Play couldn't start Warcraft III."], "Play couldn't start Warcraft III.")).toHaveLength(4);
  expect(stepsAfterError([], new Error("boom"))).toEqual(["Error: boom"]);
});
