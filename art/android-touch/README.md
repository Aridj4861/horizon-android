# Android touch artwork

Original editable SVG frames for contributors. The SVG files are templates;
export your finished artwork as transparent RGBA PNGs. They are not Mojang assets.

The game reads optional textures from an enabled Java resource pack:

`assets/minecraft/textures/gui/horizon/touch/`

| File stem | Use |
| --- | --- |
| `chat` | Chat button |
| `pause` | Pause button |
| `settings` | Settings button |
| `debug` | F3 button |
| `perspective` | F5 button |
| `button` | Empty frame behind custom-button labels and auxiliary keys |

Export `chat_normal.png`, `chat_pressed.png`, and so on. Use the same square
canvas for every top button, typically 32 x 32 or 64 x 64. Include the frame and
icon in each image. Keep a transparent margin if wanted; the complete image is
mapped to the button without cropping. A missing pressed image uses the normal
image. A missing normal image keeps the existing client artwork.

The empty `button` frame uses standard `gui.scaling` metadata. Copy
`button_normal.png.mcmeta` beside both normal/pressed PNGs (rename for each
image). This preserves its corners when a label makes a button wider. Letters
are drawn by the client, so leave the centre empty. SVG `selected` is a design
reference; the current skin loader uses normal and pressed states only.

Test the pack through the game's Resource Packs menu. Keep only your original
artwork in Git; downloaded control textures remain private under the prepared
resource tree, `assets/minecraft/textures/gui/bedrock/`.
