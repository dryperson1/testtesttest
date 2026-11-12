# Iris: a picture transition (template)

`example.iris` turns one clip into the next at a cut: the incoming clip
appears in a circle that opens from a point (`center`, the middle by
default) and grows until it covers the frame, with a soft edge (`softness`,
a share of the frame's longer side; 0 is a hard edge). At its first moment
the picture is exactly the outgoing clip, at its last exactly the incoming
one. With no clip on one side, it opens onto what is beneath, or onto black.

## Why the "mix" style

A transition has one of two styles (`"transition": {"style": ...}`):

- **fade**: no code at all. The manifest gives curves of each clip's
  opacity over the transition's progress, and the editor draws the clips
  with them. Every pixel of a clip gets the same opacity, so a fade can
  dissolve, or dip through what is beneath, but cannot draw a shape.
- **mix**: the editor draws both clips, and a `transition` function chooses
  each pixel from the two.

An iris shows one clip inside a circle and the other outside it, a choice
per pixel, so it is a mix. For a fade, write `block.json` alone, without a
`video` section, for example a dip through what is beneath:

```json
"transition": { "style": "fade",
  "outgoing": [[0, 1], [0.5, 0], [1, 0]], "incoming": [[0, 0], [0.5, 0], [1, 1]],
  "outgoing_alone": [[0, 1], [1, 0]], "incoming_alone": [[0, 0], [1, 1]] }
```

## Make it yours

1. In `block.json`, change `id` (your own name before the dot), `name`,
   `description`, `author` and `license`, and the `params`. The settings of
   a transition cannot be animated: a transition keeps one value for each.
2. Change the body of `transition` in `block.wgsl`. `a` is the outgoing
   clip's pixel and `b` the incoming one's, premultiplied; `uv` is the place
   in the frame; `ctx.progress` runs from 0 to 1. Return `a` at progress 0
   and `b` at 1, so the cut lands exactly.

## Check, pack and install

With the editor's command-line tool, `vep`, from this folder:

```text
vep blocks validate .
vep blocks pack . example.iris.chadfx-block
vep blocks install example.iris.chadfx-block
vep blocks remove example.iris
```

In the editor: Blocks in the toolbar, then "Install from file..." or
"Install a folder...". The iris is then offered among the transitions of a
picture clip in the inspector.
