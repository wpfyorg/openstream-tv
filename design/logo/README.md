# openstream tv — logo

The ring **is** the `o`. It is cut to the width of the word's own letters and centred on the
x-height band, so it sits in *openstream* as a letter: one open ring, an 80° opening at 9 o'clock
facing away from the word, and the on-air dot held in that opening, one stroke thick.

## Files

| File | Use |
| --- | --- |
| `openstream-lockup.svg` | Primary lockup, ink on transparent. Websites, decks, print on light grounds. |
| `openstream-lockup-inverse.svg` | Same lockup reversed white for dark grounds, video lower-thirds, dark UI. |
| `openstream-mark.svg` | The ring alone, ink — app icons, avatars, favicons, watermarks. |
| `openstream-mark-inverse.svg` | The ring alone, reversed white. |
| `exports/openstream-lockup-1020.png` | Raster lockup, 1020 × 154, transparent. |
| `exports/openstream-lockup-inverse-1020.png` | Raster lockup, inverted, 1020 × 154, transparent. |
| `exports/openstream-mark-1024.png` | Raster ring, 1024 × 1024, transparent. |
| `exports/openstream-mark-inverse-1024.png` | Raster ring, inverted, 1024 × 1024, transparent. |
| `exports/openstream-mark-256.png` | Raster ring for app and avatar slots. |

All artwork is outlined vector — no font dependency inside the files.

The opening is drawn as explicit cubic Bézier segments with round caps, never as an elliptical-arc
(`A`) command. An `A` command whose start and end points nearly meet can be resolved to the wrong
centre by some rasterisers, which is what split the older file into a fragment at small sizes.

## Colour

| Token | Value | Role |
| --- | --- | --- |
| Ink | `#0B0F1A` | Wordmark, ring |
| On-air red | `#FF4438` | The dot only |
| Paper white | `#FFFFFF` | Reversed artwork |

The dot is the only place the red appears. Do not recolour it as a secondary accent, and do not
tint the ring.

## Construction

Drawn from the Outfit skeleton, so ring and letters share one weight.

- **Lockup** — 570.25 × 85.97 units on a 4-unit pad. Ring outer diameter 49, i.e. slightly larger
  than the 44.35 x-height, because the opening removes part of the ring's width: the ring's own
  visible width is 44.23 units against the 45.52-unit ink width of the letter `o` at em 88, so the
  ring reads as wide as the letters beside it. Stroke 8.23 (0.168 × the diameter), centreline radius
  20.38, counter radius 16.27, opening 80° centred at 9 o'clock with round caps. The dot — radius
  8.23, exactly one stroke thick and twice the earlier dot — sits on the ring's centreline in the
  opening, where cap-to-cap clearance is 17.97 units against the 16.46-unit dot, 0.76 units of air
  on each side. Because the dot is wider than half the stroke, it bulges 4.12 units past the ring's
  silhouette at 9 o'clock; that bulge sets the left edge of the lockup. The ring is centred on the
  x-height band (top −46.68, bottom +2.32), the word baseline is 0, and the `t` ascender sets the
  top of the file at −60.15. The ring ends 6.5 units before the `p` ink. Wordmark Outfit 600, em 88,
  tracking −0.8 %; `TV` tag Outfit 500, em 48.4, tracking +8.5 %, raised 2.3 units, 15 units after
  the word.
- **Ring alone** — 96-unit square, ring outer diameter 76, stroke 12.77, dot radius 12.77, same
  80° opening. Same proportions as the lockup ring, so the two are one drawing at two scales: stroke
  0.168 × the outer diameter, dot radius 1.0 × the stroke. The dot stops 3.62 units inside the box
  edge.

## Clear space and minimum size

- Clear space: a quarter of the ring's height on all four sides, measured from the ink — 12.25 units
  in the lockup, 19 units on the 96 grid. Nothing, including rules, buttons or other logos, enters it.
- Minimum sizes: mark 32 px screen / 10 mm print; lockup 320 px wide screen / 70 mm wide print.
  Below that the dot closes up against the ring's caps — use the mark instead.
- Keep the lockup horizontal. The opening stays at 9 o'clock, facing away from the word, with the
  dot in it. Do not close the ring, move or recolour the dot, set the ring beside a typed `o`, stack
  the ring above the wordmark, stretch, rotate, outline, add shadows or gradients. On mid-tone
  photography use the inverse artwork or a solid ground.

## Typeface

Wordmark and suggested UI/display typeface: **Outfit** by Smartsheet Inc. / Rodrigo Fuenzalida,
SIL Open Font License 1.1 — free for commercial use, embeddable, and the origin of the outlined
letterforms in these files. Keep Outfit as the brand display face so future lockups stay drawn from
the same skeleton.

## Rebuilding

The artwork is generated from `.build/make_logo_v2.py`, which outlines the wordmark from
`.build/fonts/Outfit.ttf` and writes the four SVGs. Re-run it after any geometry change, then run
`.build/render_exports.py` to re-cut the PNGs at their exact pixel sizes with a transparent
background.
