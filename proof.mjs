#!/usr/bin/env bun
import {resolve} from 'node:path';
import {readFileSync} from 'node:fs';
import {proof, completedWorldFrame, worldPixels} from '../../../../private/tmp/exact-framework-20261109/game/proof.mjs';

const play = async session => {
  await session.tap('play');
  return session.world('world');
};

if (import.meta.main) await proof(import.meta, async ({open, check, out, host, look, say}) => {
  const session = await open();
  const world = await play(session);
  const title = await session.tree();
  check('the title screen is replaced by the world', title.nodes.every(node => node.props?.testId !== 'play'));
  await world.hold('KeyD', 900);
  await world.run(250);
  const position = await world.global_position('player');
  check('the player moves on the platform', position[0] > 0.5, position);
  await world.tap('Space');
  await world.run(100);
  check('the player jumps from the platform', (await world.global_position('player'))[1] > 0.0);

  if (host !== 'linux') {
    const screenshot = await session.screenshot(resolve(out, 'game.png'));
    const pixels = worldPixels(readFileSync(resolve(out, 'game.png')), screenshot, await session.layout(), await session.tree());
    const frame = completedWorldFrame(pixels, (await session.state()).world?.[0]);
    check('the GPU host presents a completed world frame', frame.ok, frame);
  }

  await world.save(resolve(out, 'checkpoint.world'));
  const checkpoint = await world.snapshot();
  await world.run(250);
  const continued = await world.snapshot();
  await session.close();

  const restored = await open({fresh: true, world: resolve(out, 'checkpoint.world')});
  const restoredWorld = await play(restored);
  check('a fresh process restores the player', JSON.stringify(await restoredWorld.snapshot()) === JSON.stringify(checkpoint));
  await restoredWorld.run(250);
  check('a fresh process continues identically', JSON.stringify(await restoredWorld.snapshot()) === JSON.stringify(continued));

  const plan = await look(restored, {top: 30}, 'plan').catch(error => ({unavailable: error.message}));
  say(plan.picture ? `LOOK plan ${plan.picture}` : `look: ${plan.unavailable}`);
  await restored.close();
});
