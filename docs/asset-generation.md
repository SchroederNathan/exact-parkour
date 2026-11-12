# Generating assets

This is the current practice for making 3D models for Rooftop Rumble with AI tools. It was written on 2026-11-11. Two generated bodies are in the game: `art/body_a.glb` and `art/body_b.glb`. The player chooses one on the start screen. No clothing item or prop is generated yet. The last section lists what is not proven.

## The rule

Generate one object at a time from a reference image, clean it in Blender, and export one `.glb` into `art/`.

Do not build a character from primitives with a Blender script. That method was tried in `blender_variants/` and the result was rejected. Do not ask a generator for a dressed character or for a set of parts. Meshy's documentation says to use one object for each image.

## Tools

| Tool | Use |
|---|---|
| `meshy` MCP server (`@meshy-ai/meshy-mcp-server`) | Reference images, models with a face budget, rigging, download |
| `blender` MCP server (`mcp-for-blender`) | Import, fit, check, look at, and export a model in the open Blender |
| Blender 5.2 | Must be open. The add-on starts its server on `localhost:9876` |

The Meshy key is in the MCP configuration and in the Blender add-on preferences. It never goes into this repository, which is public.

The Blender server has a `generate_3d` tool. It has no setting for a face count or a texture model, so use it for a fast draft only. For a game asset, generate with the `meshy` server and import the downloaded file into Blender. `generate_3d` with Meshy also needs "Use Meshy" on in the scene, and that setting is off in a new file.

## Pipeline

1. **Write the prompt.** Use the pattern below. End every prompt with the same style line.
2. **Make a reference image.** Use `meshy_text_to_image`, another image tool, or a drawing. Meshy accepts any image.
3. **Approve the image.** Put the images of a set side by side. Reject an image that does not match the set. An image costs less than a model.
4. **Make the model.** Use `meshy_image_to_3d` with the settings below.
5. **Compare Meshy's preview image with the reference.** Do this before the rig. See "Pose mode" below.
6. **Download the GLB** and import it into Blender. For a body, rig it with `meshy_rig` first and download the rigged GLB.
7. **Clean it in Blender.** See the checks below. For a body, `tools/fit_body.py` does this step.
8. **Export one `.glb` into `art/`.** The bake makes `<stem>.model`.
9. **Run `bun exact.mjs test-rust`.** The bake names anything that it refuses.

## Prompt pattern

A prompt can have 800 characters at most. Describe one object, the view, and nothing else. Then add the style line.

Style line, the same for every asset:

```text
chunky stylized game art, flat matte colours, no text, no logos, plain white background, even light, no shadow
```

| Asset | Prompt before the style line |
|---|---|
| Body | `Full-body human mannequin, A-pose, bald, barefoot, plain grey fitted bodysuit, neutral face, front view` |
| Hat | `A single red baseball cap, empty, no head, no mannequin, three-quarter view` |
| Mask | `A single white hockey mask, empty, no head, front view` |
| Shoe | `A single black high-top sneaker, left foot, side view` |
| Shirt | `A single blue hoodie, hollow, no body inside, sleeves in A-pose, front view` |
| Pants | `A single pair of grey cargo pants, hollow, no body inside, legs slightly apart, front view` |
| Prop | `A single <object>, three-quarter view` |

Choose the view that shows the most of the object. A Smart Topology model takes one image only.

To make all the clothing images in an image chat tool, use the prompt in [Prompt for clothing reference images](clothing-image-prompt.md).

## Meshy settings

Meshy's documentation gives these settings for a game character and for a low-poly game asset:

| Setting | Value |
|---|---|
| `model_type` | `smart-topology` |
| `ai_model` | `meshy-t2.5` (100 to 30,000 faces). `meshy-t2` is faster (100 to 15,000 faces) |
| `texture_model` | `meshy-n1`. It makes a 2K base colour from the reference image and takes no text prompt |
| `target_polycount` | Our budget: 10,000 to 15,000 for a body, 1,000 to 3,000 for an item or a small prop |
| `pose_mode` | Leave it out when the reference image is already in a T-pose or an A-pose. Leave it out for an object |

Smart Topology gives triangles only. Do not ask for quads with it. Do not make a Meshy 7.1 model and then reduce it to a face count.

Rig a body with `meshy_rig`. The model must be a textured humanoid, must face +Z, and can have 380,000 triangles at most. Give it `height_meters`. The rig has 24 joints, and the model stays in the pose of the image.

### Pose mode

`pose_mode` makes Meshy draw the person again in the pose before it makes the model. On 2026-11-11 `pose_mode: "t-pose"` changed a bearded man into a woman with a ponytail. The same call without `pose_mode` kept the person. The other body kept its person with `pose_mode`. The changed model and its rig cost 20 credits. Always compare the preview image of the finished task with the reference before you rig.

A Smart Topology model takes one image. Crop the front view out of a sheet of views and give Meshy that crop. The two bodies came from crops of 580 by 615 pixels.

## Checks in Blender

Do these for every model before the export:

- Scale the model to metres. glTF is in metres.
- Put the origin where the game attaches the model: at the feet for a body, at the joint for a worn item.
- Tick Backface Culling on every material. A double-sided material hides what is behind it.
- Fill a vertex colour attribute with white, or export without vertex colours. A joined part with no colour draws black.
- Export the active scene only. The bake takes exactly one scene.
- Mirror one shoe to make the pair.

The bake accepts triangles, skins, skeletal animation, one UV set, and 8-bit PNG or JPEG textures. It refuses Draco and meshopt compression, morph targets, a second UV or colour set, and the material extensions for specular, IOR, transmission and variants. The full list is in the framework's `game/recipes/blender-export.md`.

Keep reference images and `.blend` files out of `art/`. The bake treats a PNG in `art/` as a sprite.

## Bodies

The game has one skeleton. `logic/src/stickman.rs` builds it in code, with all the clips. Its arms hang at rest, and each joint rests with no rotation. A body from Meshy is in a T-pose on Meshy's own 24 joints. `tools/fit_body.py` fits such a body to the game's skeleton inside Blender:

```python
import runpy
fit = runpy.run_path("/path/to/exact-beat-em-up/tools/fit_body.py")["fit"]
fit("/path/to/rigged.glb", "/path/to/exact-beat-em-up/art/body_c.glb", "body_c")
```

Run it with `execute_blender_code`. It makes a scene with the name of the body and leaves the fitted body there, so you can pose it and look at it. The script does these things:

1. It scales the body to 1.8 m, puts the soles on the ground and puts the feet where the mannequin's feet are.
2. It makes each arm longer along its own line, until the elbow and the wrist are as far from the shoulder as the game's. The two bodies needed 8 to 31 percent.
3. It swings the arms down to the rest pose. The blend at the shoulder is a dual quaternion blend, which keeps the volume.
4. It adds the weights of each Meshy joint to the game joint that moves that part. A vertex keeps four joints at most.
5. It makes the game's joints as bones that point up with no roll. Blender exports such a bone with no rotation.
6. It ticks Backface Culling, removes vertex colours, makes the texture 1024 pixels and exports the GLB.

The trunk and the legs keep their shape. The game's hip joint is 6 cm above Meshy's, so a thigh turns about a point that is a little too high.

In the game, `Game::ASSETS` names the bodies of `stickman::BODIES`, so setup waits for them. `stickman::dressed` puts the mesh, the material and the texture of a body on the code skeleton. `Options::character` selects the body: 0 is the mannequin, 1 is `body_a`, 2 is `body_b`. `logic/tests/bodies.rs` checks each body in `art/`: the joints and their places, the weights, the size, and that the player moves the same in every body.

To add a body, export `art/body_<letter>.glb`, add its name to `stickman::BODIES`, and add a radio button to the start screen in `app.contract`.

Because the game now has `art/`, a hostless test makes its world with `exact_game::sim_with_art!`, not `Sim::new`.

## Character customizer

The plan is one bare body and one model for each item in a slot. The two bodies wear a dark top and shorts, which are part of the mesh and the texture.

| Type | Slots | Attachment |
|---|---|---|
| Rigid | Hat, mask, shoes | `SocketFollow` on the `head` or `foot` joint |
| Bends with the body | Shirt, pants | The item uses the body's skeleton. Blender copies the weights from the body |

Order of work:

1. Generate and rig the body, and fit it to the rig in `logic/src/stickman.rs`. This is done for two bodies.
2. Make one hat and attach it to the `head` joint.
3. Make one shirt and copy the body weights to it.
4. Make the other items only after steps 2 and 3 work in the game.

## Cost

| Task | Credits |
|---|---|
| Reference image | 3 to 12, by image model |
| Smart Topology model with a 2K texture | 15 |
| Meshy 7.1 model with a texture | 30 |
| Rig | 5 |

Meshy refunds only a task that fails. A model that completes but looks bad still costs its credits. One body and 20 items cost about 450 credits with no retries. `meshy_check_balance` is free.

## Not proven yet

- The engine has not been tested with two skinned models on one skeleton. Test one shirt before you make more.
- The prompts above have not been run. The two bodies came from reference images that were made with another tool. Adjust the style line on the first five images, then keep it fixed.
- The two bodies were looked at in a stand and a run only. The hang, the climb, the vault, the slide and the roll were not looked at. The hands are open and have no finger joints.
- The texture has detail from the front view only. The back comes from Meshy's guess.
- The body's shoulders are square after the swing from the T-pose. An A-pose reference image needs a smaller swing.

Update this document when one of these points is proven or changes.
