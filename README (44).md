# Warm tint in one file: a picture block as one .wgsl file (template)

`warm-tint.wgsl` is a whole block in one file: the very filter of
`../picture-filter`, with its description (the manifest) in the
`/* chadfx-block` comment at the top of the shader instead of in a
`block.json`. After that comment the code is the same, byte for byte. Its id,
`example.warm-one-file`, differs only so that both templates can be installed
at once.

## The rules of the form

- The `/* chadfx-block` comment comes first in the file (only blank lines may
  come before it), with the marker word on the same line as the `/*`.
- Inside it is the same JSON as a `block.json`, without `"source"` in the
  `video` section: the file is its own shader.
- The JSON may contain neither `/*` nor `*/` (they would open a comment or
  end the header early).
- Picture blocks only. A sound block needs a WebAssembly module, which a text
  file cannot hold.
- No icon, read-me or licence file can come with it (this README is for you,
  not part of the block). For those, make a package folder like
  `../picture-filter`.
- At most 64 KiB, header included.

## Make it yours

Change the same lines as in `../picture-filter/README.md`, in the header
here and in the `pixel` function below it. Name the file as you like; only
its extension, `.wgsl`, matters.

## Check, pack and install

With the editor's command-line tool, `vep`, from this folder:

```text
vep blocks validate warm-tint.wgsl
vep blocks pack warm-tint.wgsl
vep blocks install warm-tint.wgsl
vep blocks remove example.warm-one-file
```

`pack` writes `warm-tint.chadfx-block` beside the file: exactly the package
installing the `.wgsl` file keeps, so installing either gives the same block
with the same identity. In the editor, "Install from file..." in Blocks
offers `.wgsl` files, or drop the file on the window. ChadFX does not take
`.wgsl` files over: a double-click still opens the program you chose for
them, and "Open with" ChadFX asks whether to install the block.
