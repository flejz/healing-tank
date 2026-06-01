# Enclosure Models

3D-printable enclosure for the BIO-TANK MK.VII prop. Designed in **Fusion 360**.

> **Scale note:** Fusion 360 wasn't exporting the STLs at the intended size (the design was authored in cm but exported as mm), so the files were manually upscaled **1000×**. Check dimensions in your slicer before printing.

## Parts

| Part | Preview |
|------|---------|
| **`tank-base.stl`** — tiered cylindrical body with embossed `C_CORP` branding, serial markings, twin instrument ports and recessed side panels. | <img src="tank-base_front-panel.jpeg" width="360"> |
| **`front-panel.stl`** — protruding control bezel that mounts on the base; houses the OLED display window. *(Shown attached to the base in the render above.)* | |
| **`tank-top.stl`** — domed lid with a radial ventilation grille and mounting slots around the rim. | <img src="tank-top.jpeg" width="360"> |

## Printing

- Orientation, supports and material are up to you — no print profiles are committed.
- Verify the model scale in your slicer first (see the scale note above).
- The vent grille on `tank-top.stl` has thin features; a 0.4 mm nozzle handles them, but check your slicer's preview.
