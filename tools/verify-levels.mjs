import { readFileSync } from "node:fs";
import { spawnSync } from "node:child_process";

const contract = readFileSync("app.contract", "utf8");
const rust = readFileSync("logic/src/lib.rs", "utf8");
const required = ["chooseRace0", "chooseRace1", "chooseRace2", "chooseRace3", "chooseArena", "chooseArenaFromPause", "mode=mode", "race=race"];
const missing = required.filter((needle) => !contract.includes(needle));
if (missing.length) {
  console.error(`missing level controls: ${missing.join(", ")}`);
  process.exit(1);
}
for (const needle of ["RACE_MODE", "ARENA_MODE", "race::tick", "arena::tick"]) {
  if (!rust.includes(needle)) {
    console.error(`missing mode seam: ${needle}`);
    process.exit(1);
  }
}
for (const args of [["exact.mjs", "contract", "build", "app.contract", "--json"], ["exact.mjs", "test-rust"]]) {
  const result = spawnSync("bun", args, { stdio: "inherit" });
  if (result.status !== 0) process.exit(result.status ?? 1);
}
