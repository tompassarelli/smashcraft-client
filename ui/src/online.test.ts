import { expect, test } from "bun:test";
import { DID_NOT_START, looksLikeRelayCode, standaloneView, WAITING_FOR_OPPONENT } from "./online";

test("a standalone run's view reads the host's relay code and the region's round trip from the lines wisp play --standalone prints, and only a host shows the code (lines recorded in the gex45 two-client run of 30 October 2026)", () => {
  const lines = [
    "net: hosting through sgp, round trip to it 183 ms",
    "Smashcraft: join code sgp-7K2M-QX9D-R3TB-4 (relay room; the joiner's map must match 4abc9ceb)",
    "Smashcraft: standalone window opened. Connect a controller or use the keyboard.",
  ];
  const waiting = standaloneView({ available: true, map: null, running: true, mode: "host", lines: lines.slice(0, 2), finished: null });
  expect(waiting.steps).toEqual([WAITING_FOR_OPPONENT]);
  const host = standaloneView({ available: true, map: null, running: true, mode: "host", lines, finished: null });
  expect(host.code).toBe("sgp-7K2M-QX9D-R3TB-4");
  expect(host.region).toEqual({ name: "sgp", rttMs: 183 });
  expect(host.steps).toEqual(["standalone window opened. Connect a controller or use the keyboard."]);
  expect(standaloneView({ available: true, map: null, running: false, mode: "join", lines: [], finished: false }).steps).toEqual([DID_NOT_START]);
  expect(host.busy).toBe(true);
  const joiner = standaloneView({ available: true, map: null, running: true, mode: "join", lines: ["net: joining through sgp, round trip to it 40 ms", lines[2]!], finished: false });
  expect(joiner.code).toBeUndefined();
  expect(joiner.region).toEqual({ name: "sgp", rttMs: 40 });
  expect(joiner.failed).toBe(true);
  expect(standaloneView({ available: true, map: null, running: true, mode: "join", lines: lines.slice(0, 2), finished: null }).code).toBeUndefined();
});

test("a typed relay code is sent when it names a region and thirteen letters or digits, or a relay's address and port with them (external reference: wisp:host/net/joinCode.ts relayCode)", () => {
  for (const good of ["sgp-7K2M-QX9D-R3TB-4", " sgp-7k2mqx9dr3tb4 ", "gex-0000-0000-0000-1", "159.89.205.60:7457/7K2M-QX9D-R3TB-4"]) expect(looksLikeRelayCode(good)).toBe(true);
  for (const bad of ["", "ABCD-EFGH", "sgp-7K2M-QX9D-R3TB", "sgp-7K2M-QX9D-R3TB-45", "7K2M-QX9D-R3TB-4", "R2M6-WDYR-7MQC-S0ZA", "sgp 7K2M QX9D R3TB 4"]) expect(looksLikeRelayCode(bad)).toBe(false);
});
