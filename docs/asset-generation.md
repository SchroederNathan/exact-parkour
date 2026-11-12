# Generating assets

This is the current practice for making 3D models for Rooftop Rumble with AI tools. It was written on 2026-11-11. The tools are installed and tested, but no generated model is in the game yet. The last section lists what is not proven.

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
5. **Download the GLB** and import it into Blender.
6. **Clean it in Blender.** See the checks below.
7. **Export one `.glb` into `art/`.** The bake makes `<stem>.model`.
8. **Run `bun exact.mjs test-rust`.** The bake names anything that it refuses.

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
| `pose_mode` | `a-pose` for a body. Leave it out for an object |

Smart Topology gives triangles only. Do not ask for quads with it. Do not make a Meshy 7.1 model and then reduce it to a face count.

Rig a body with `meshy_rig`. The model must be a textured humanoid, must face +Z, and can have 380,000 triangles at most.

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

## Character customizer

The plan is one bare body and one model for each item in a slot.

| Type | Slots | Attachment |
|---|---|---|
| Rigid | Hat, mask, shoes | `SocketFollow` on the `head` or `foot` joint |
| Bends with the body | Shirt, pants | The item uses the body's skeleton. Blender copies the weights from the body |

Order of work:

1. Generate and rig the body. Decide how its skeleton maps to the rig in `logic/src/stickman.rs`.
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

- No generated model has gone through the bake. The texture format and the material extensions of a Meshy GLB are not checked.
- The engine has not been tested with two skinned models on one skeleton. Test one shirt before you make more.
- The player is a model that `logic/src/stickman.rs` builds from a code rig, with clips made in code. A generated body must use that skeleton, or the animation code must change. This is not decided.
- The prompts above have not been run. Adjust the style line on the first five images, then keep it fixed.

Update this document when one of these points is proven or changes.
