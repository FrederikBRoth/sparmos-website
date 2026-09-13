# Portal playground

Run `cargo run` from `sparmos-website`. The blue portal in the main scene looks into the orange room. Press **2** to inspect the orange portal looking back into the main scene; **1** returns to the blue portal. **3** and **4** retain the existing scene experiments.

Use **WASD** to move, the **arrow keys** to turn, and **left Shift / left Ctrl** to move vertically. Strafe across the doorway or move closer to see the destination perspective change. The orange endpoint has a 15 degree tilt, so its return view also demonstrates camera roll.

The single `self.portal_playground = Some(Self::portal_playground(...));` statement in `Website::setup()` enables the experiment. Comment it out to skip all portal scene, view, material and target creation. The optional state then stays `None`, the update path does nothing, and key 2 does nothing. Shader files remain in the asset preload manifest, but no portal GPU resources are allocated. Other playgrounds remain available.

Endpoint transforms and demo geometry live in `gameloop.rs`. Local +Z points out of each portal. Transform scale must stay `(1, 1, 1)`; the aperture size is stored separately in its mesh instance. Player teleportation and recursive portals are outside this version.

The engine advances view cameras during `begin_frame`, before game logic derives the portal cameras. View systems only upload the final camera data. Both portal views render before the presenting view using the existing encoder and private depth targets. Auxiliary masks exclude the portal surface layer from every draw path; untagged draw registrations use world layer 1. Filtering applies to entire Model or RenderableHandle registrations, including their instances.

Targets match the main view resolution exactly and resize with it. Reallocation keeps texture handles and rebinds the existing portal materials, so resizing does not accumulate materials or targets. The portal shader samples fragment screen coordinates using the actual per-view viewport dimensions, including the presenting view's post-process overscan.

Each exit plane keeps the half-space beyond the destination portal. Oblique projection changes the OpenGL near plane before the engine converts depth to WGPU's 0..1 range. The calculation follows [Eric Lengyel's oblique clipping derivation](https://terathon.com/blog/oblique-clipping.html). Ordinary views skip this calculation. Degenerate planes that cannot define a usable frustum retain the ordinary projection.

## Checks

From each crate, run `cargo test --lib` and `cargo check --target wasm32-unknown-unknown`. From the website, also run `cargo build`.

The explicit Windows GPU test uses a hidden window:

```powershell
cargo test --lib portal_gpu_smoke -- --ignored --nocapture
```

It renders both directions, changes camera position and rotation, compares clipping enabled and disabled, resizes to an odd aspect ratio, and exercises post processing. It checks that the magenta forbidden-side bar is clipped, camera upload does not advance movement again, and resizing retains resource counts. PNG captures are written to `target/portal-smoke/` for inspection. Post processing is exercised on the presenting window; the saved captures are auxiliary scene views.

The native startup check also exposed an existing egui texture-delta panic. The integration now consumes all pending image updates and texture frees after applying them.
