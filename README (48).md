# Gradient: a picture generator (template)

`example.gradient` makes a picture from nothing: a smooth blend from a start
colour (top) to an end colour (bottom). With the switch `horizontal` on, it
blends from left to right instead. The two colours can be animated with
keyframes; a switch cannot.

## The files

- `block.json`: the description (kind `video.source`, the settings, the
  stage `source`).
- `block.wgsl`: one WGSL function `source` that returns the colour of the
  pixel at a place in the frame.
- `README.md`: this file, shown by the block manager.

## Make it yours

1. In `block.json`, change `id` (your own name before the dot, never
   `chadfx.` or `core.`), `name`, `description`, `author` and `license`.
2. Change `params`. The code reads each one as `p.<id>`: a colour is a
   vec4<f32>, a switch a u32 that is 0 or 1.
3. Change the body of `source` in `block.wgsl`. Return the colour as footage
   carries it (sRGB values from 0 to 1) with straight alpha: the editor turns
   it into linear light like any picture.

## Check, pack and install

With the editor's command-line tool, `vep`, from this folder:

```text
vep blocks validate .
vep blocks pack . example.gradient.chadfx-block
vep blocks install example.gradient.chadfx-block
vep blocks remove example.gradient
```

In the editor: Blocks in the toolbar, then "Install from file..." or
"Install a folder...". A generator is the source of a clip, shown instead of
a media file; the guide to blocks (`docs/BLOCKS.md`) says how to put one on a
track.
