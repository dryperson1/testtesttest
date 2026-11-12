# Warm tint: a picture filter (template)

`example.warm` tints the picture towards a colour. It has a number setting,
`amount` (0 leaves the picture exactly as it is, 1 multiplies every colour by
the tint), and a colour setting, `tint` (warm orange by default). Both can be
animated with keyframes. With a white tint, or at amount 0, nothing changes.

## The files

- `block.json`: the block's description (its manifest), which the editor
  reads without running any code: id, version, name, the description the
  AI reads, the kind (`video.filter`), the settings, and which stage the
  code provides (`pixel`).
- `block.wgsl`: the code, one WGSL function `pixel` that gets one pixel in
  linear light and returns it changed.
- `README.md`: this file. The block manager shows it as plain text.

## Make it yours

1. In `block.json`, change `id`: put your own name before the dot, such as
   `yourname.warm`. Ids starting with `chadfx.` or `core.` are refused.
2. Change `name`, `description`, `category`, `author` and `license`.
3. Change `params`: one entry per setting. The code reads each one as
   `p.<id>` (`p.amount` is an f32, `p.tint` a vec4<f32>).
4. Change the body of `pixel` in `block.wgsl`. Keep its first line (its
   name and the types of what it takes and returns).
5. The `"$schema"` line lets a code editor check `block.json` against
   `blocks/block-manifest.schema.json`; point it at your copy or delete it.

Once people use your block, never rename or remove a setting without raising
`format` and adding a migration (see the guide to blocks, `docs/BLOCKS.md`),
because projects store the settings by their ids. Raise `version` with
every change.

## Check, pack and install

With the editor's command-line tool, `vep`, from this folder:

```text
vep blocks validate .
vep blocks pack . example.warm.chadfx-block
vep blocks install example.warm.chadfx-block
vep blocks info example.warm
vep blocks remove example.warm
```

`validate` explains every problem it finds and exits with 1 when there is
one. `pack` writes the one file to share; `install` also takes this folder
itself. In the editor: Blocks in the toolbar, then "Install from file..."
for the `.chadfx-block` file or "Install a folder..." for this folder, or
drop the file on the window. The filter then appears in the inspector's
list of effects.
