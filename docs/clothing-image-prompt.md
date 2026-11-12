# Prompt for clothing reference images

This prompt makes the reference images for the character customizer in an image chat tool such as ChatGPT. Each image becomes one 3D model. See [Generating assets](asset-generation.md) for the full pipeline.

The prompt has not been run yet. The 20 items are a first suggestion. Change the list before you send the prompt.

## The prompt

Paste this as the first message.

```text
You will make reference images for a 3D game asset pipeline. Each image becomes a 3D model, so the rules below are strict. Apply them to every image.

RULES
1. One image shows exactly one clothing item. No sets, no pairs, no sheets, no variations.
2. The item is empty and hollow. No person, no mannequin, no body parts, no hanger, no stand.
3. The item is centred. The full item is in the frame, with a margin on all sides.
4. Background: plain pure white. No floor, no shadow, no gradient, no props.
5. Light: soft and even from the front. No strong highlights.
6. Style: chunky stylized 3D game art, simple thick shapes, flat matte colours, large clean colour areas, no fine pattern, no photo realism.
7. No text, no logos, no brand marks, no letters, no numbers.
8. Square image, 1024 x 1024.
9. Keep the same style, the same light and the same level of detail in every image, so all the items look like one set.

VIEWS
- Hat: three-quarter view from the front, a little above.
- Mask: straight front view.
- Shirt: straight front view, laid out as if worn, sleeves angled 45 degrees down from the shoulders.
- Pants: straight front view, laid out as if worn, legs slightly apart.
- Shoe: one left shoe only, side view, toe pointing left.

HOW TO WORK
Make one image for each message, in the order of the list. After each image, write only the item number and name, then stop. When I write "next", make the next item. When I write "redo" and a note, make the same item again with that change.

ITEMS
Hats
1. Red baseball cap, curved brim
2. Black beanie with a folded cuff
3. Yellow construction hard hat
4. Dark blue bucket hat

Masks
5. White hockey goalie mask with round holes
6. Black cloth bandana face mask, tied at the back
7. Grey gas mask with two round filters
8. Orange ski goggles with a wide strap

Shirts
9. Blue pullover hoodie with a front pocket
10. White sleeveless tank top
11. Red and black bomber jacket, zipped closed
12. Green short-sleeve t-shirt

Pants
13. Grey cargo pants with side pockets
14. Blue jeans, straight leg
15. Black jogger pants with a white side stripe
16. Tan knee-length shorts

Shoes
17. Black high-top sneaker with a white sole
18. White low-top running shoe
19. Brown lace-up work boot
20. Red slip-on skate shoe

Start with item 1.
```

## How to use it

1. Check the first image of each slot before you make the others: items 1, 5, 9, 13 and 17. If the style is wrong, correct rule 6 and start again.
2. Use "redo" for a bad image. Common faults are a body inside the item, a shadow, two shoes, or a cut-off edge.
3. Save each image as a PNG with a clear name, for example `hat-01-red-cap.png`.
4. Keep the images out of `art/`. The bake treats a PNG in that folder as a sprite.

The body is not in this list. It needs its own image, with the body prompt in [Generating assets](asset-generation.md).
