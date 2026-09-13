# Walkthrough of the portal changes

The old `texture_test` showed that one RenderView could render into a texture and another could display it. This change builds the portal behaviour around that existing mechanism.

The result is a pair of linked doorways. Each doorway displays another scene from a camera whose position and orientation follow your view through the opening.

## 1. Fixed the custom camera constructors

Files: [graphics.rs](D:/Documents/Hyggekode/Rust/sparmos-engine-working/sparmos-engine/src/application/graphics.rs:847) and [render_view.rs](D:/Documents/Hyggekode/Rust/sparmos-engine-working/sparmos-engine/src/core/render/render_view.rs:314).

There were two places where the supplied camera was being discarded. `Graphics::new_render_view_with_camera()` called the default constructor, and `RenderViewHandler::new_view_with_camera()` created a new default camera internally.

The Graphics function now forwards the camera:

```rust
self.render_views
    .new_view_with_camera(target, scene, name, role, camera)
```

The handler resizes that camera to match the target, then stores it:

```rust
let size = render_target.size();
camera.resize(PhysicalSize::new(size.width as f32, size.height as f32));
```

`resize()` adjusts the screen dimensions and aspect ratio. The supplied eye position, orientation, FOV and projection settings survive creation.

## 2. Moved camera movement earlier in the frame

Previously, `CameraSystem::run()` handled movement and animation during rendering. Website update happened before that. A portal camera calculated during website update would therefore use the main camera's position from before the current movement step.

[State::begin_frame()](D:/Documents/Hyggekode/Rust/sparmos-engine-working/sparmos-engine/src/application/state.rs:348) now advances cameras:

```rust
pub(crate) fn begin_frame(&mut self, dt: std::time::Duration) {
    self.graphics.engine.engine_time.update_time(dt, true);
    self.graphics.update_view_cameras(dt);
}
```

That calls this loop in `RenderViewHandler`:

```rust
for (_, view) in self.views.iter_mut() {
    if !view.enabled || !view.simulate_camera {
        continue;
    }
    view.camera.update_camera(dt);
    if let Some(animator) = &mut view.camera_animator {
        animator.update(dt.as_secs_f32(), &mut view.camera);
    }
}
```

I added `simulate_camera` to RenderView. It defaults to `true`; portal views set it to `false` because their complete camera pose is calculated by the playground.

`CameraSystem::run()` now only updates and uploads the camera uniform. The frame order becomes:

```text
Move the main camera
    ↓
Website update derives the portal cameras
    ↓
Update scene systems and instance buffers
    ↓
Upload the final camera uniforms
    ↓
Render portal textures
    ↓
Render the main view using those textures
```

The existing auxiliary-before-main rendering order was retained. There is still one renderer and the same command encoder.

## 3. Put the experiment in its own playground

File: [gameloop.rs](D:/Documents/Hyggekode/Rust/sparmos-engine-working/sparmos-website/src/gameloop.rs:76).

I removed the inline `texture_test` setup and its per-frame camera copy. The new state keeps the handles needed by the update loop:

```rust
struct PortalEndpoint {
    scene: SceneHandle,
    transform: Transform,
    view: RenderViewHandle,
    surface: RenderableHandle,
}

pub struct PortalPlayground {
    main_view: RenderViewHandle,
    portal_a: PortalEndpoint,
    portal_b: PortalEndpoint,
}
```

`Website` holds `Option<PortalPlayground>`, which defaults to `None`. Setup enables everything with this one call:

```rust
self.portal_playground = Some(Self::portal_playground(gfx, main_scene, main_view));
```

Commenting it out skips the portal scene and GPU resource creation. Shader bytes remain in the asset preload list, but the playground does not create its shader modules or materials.

`portal_playground()` creates the destination scene, both views and their targets, and the doorway geometry. The blue endpoint lives in the main scene; the orange endpoint lives in the destination scene.

The endpoint's `view` means the view whose image appears on that endpoint:

| Endpoint | Its auxiliary view renders | Its surface displays |
| --- | --- | --- |
| A, blue | Scene B | Texture from `portal_0` |
| B, orange | Scene A | Texture from `portal_1` |

Each view has its own color target and private depth target. The visible surface is the existing two-sided quad mesh with a dedicated portal material. The unrelated sprite playground remains in place.

I added colored frames and rows of columns to make parallax visible. B has a 15 degree tilt to demonstrate camera roll. The magenta bar behind each exit is a clipping probe.

## 4. Calculate the camera through the portal

The main maths lives in [camera_through_portal()](D:/Documents/Hyggekode/Rust/sparmos-engine-working/sparmos-website/src/gameloop.rs:94).

Each portal has a local coordinate frame. Local +Z points out of its front face. To transfer the camera, we express it relative to A, turn it 180 degrees around local Y, and place it relative to B:

```text
destination camera = B × half_turn × inverse(A) × source camera
```

The half turn puts the derived eye behind the destination doorway, looking out through it.

Because the endpoint transforms have unit scale, the code can do this directly with quaternions:

```rust
let rotation = to.rotation
    * Quaternion::from_angle_y(Deg(180.0))
    * from.rotation.conjugate();
```

These products apply from right to left. For the unit quaternions used here, `conjugate()` gives the inverse rotation.

The eye position becomes:

```rust
camera.eye = Point3::from_vec(
    to.position + rotation.rotate_vector(source.eye.to_vec() - from.position)
);
```

In compact maths, with source portal position `a`, destination position `b`, source eye `e`, and combined rotation `R`:

$$
e' = b + R(e-a)
$$

Subtracting `a` gives the eye's offset from the source portal. Rotating transfers that offset into the destination orientation. Adding `b` places it in the destination world.

For example, suppose A is at the origin and B is at `(10, 0, 0)`, both with identity rotation:

```text
Source eye:              (1, 2,  6)
After the half turn:    (-1, 2, -6)
After adding B:         (9, 2, -6)
```

If the source eye moves sideways, the derived eye moves too:

$$
\Delta e' = R\Delta e
$$

That is where the parallax comes from. The destination is viewed from a changing position as you move around the entrance.

Forward and up are directions, so they only receive the rotation:

```rust
camera.forward = rotation.rotate_vector(forward).normalize();
camera.up = rotation.rotate_vector(up).normalize();
```

Transforming `up` preserves roll. Copying only yaw and pitch would lose that part of the orientation.

The helper then synchronises the camera's other orientation fields:

```rust
camera.target = camera.eye + camera.forward;
camera.yaw = camera.forward.z.atan2(camera.forward.x).to_degrees();
camera.pitch = camera.forward.y.clamp(-1.0, 1.0).asin().to_degrees();
camera.camera_mode = CameraMode::Fixed;
```

Fixed mode builds the view using `eye`, `target` and `up`. The separate `simulate_camera = false` setting prevents ordinary movement from advancing this derived camera.

The helper starts by copying the source Camera, retaining FOV, near/far settings and projection type. It resizes the copy to the destination target. I also made fixed orthographic cameras honour their target and up vectors; the other orthographic modes retain their existing orientation convention.

Endpoint scale is asserted to be `(1, 1, 1)`. The doorway's width and height are instead set by its surface instance, currently `(6, 8, 1)`.

## 5. Sample the texture using screen position

File: [portal.wgsl](D:/Documents/Hyggekode/Rust/sparmos-engine-working/sparmos-website/assets/shaders/portal.wgsl).

The vertex shader places the doorway quad in the world using its instance transform and the active camera. The fragment shader determines which destination pixel belongs at each visible doorway pixel:

```wgsl
@fragment
fn fs_main(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
    let uv = position.xy / camera.viewport.xy;
    return vec4<f32>(textureSample(portal_texture, portal_sampler, uv).rgb, 1.0);
}
```

The texture contains the complete destination camera image. A doorway fragment at 25% of the screen width samples 25% across that image. Only the portion covered by the doorway is displayed.

This works because the camera transfer preserves the correspondence between rays through the source and destination openings. Ordinary mesh UVs would attach the image coordinates to the quad's corners, stretching the whole image across the quad.

Both fragment positions and render textures use the same top-left convention here, so the shader does not flip Y.

I appended `viewport: [f32; 4]` to CameraUniform. Its first two values contain the actual render-pass width and height. The existing `screen` field remains available for screen sprites.

The distinction matters when post processing is active: the scene is rendered into an enlarged target before cropping to the window. Portal sampling uses those enlarged viewport dimensions. Auxiliary target dimensions still match the main view's window dimensions, preserving the camera aspect ratio exactly.

## 6. Keep depth testing and prevent recursion

Files: [render_view.rs](D:/Documents/Hyggekode/Rust/sparmos-engine-working/sparmos-engine/src/core/render/render_view.rs:225) and [render.rs](D:/Documents/Hyggekode/Rust/sparmos-engine-working/sparmos-engine/src/core/render/render.rs:98).

The destination scene uses the auxiliary view's private depth texture. That resolves which destination objects cover each other. The doorway surface also depth-tests normally in the main scene, so a nearby source object can obscure it.

Only destination color is sampled. The main depth buffer contains the entrance surface's depth; destination depth is not copied into it.

For recursion safety, I added generic `RenderLayer(u32)` and `RenderMask(u32)` types. The visibility rule is:

```rust
self.0 & layer.copied().unwrap_or_default().0 != 0
```

Untagged draw registrations use layer `1`. The default view mask includes every bit. Portal surfaces use layer `2`, and auxiliary views exclude that bit:

```rust
const PORTAL_SURFACE_LAYER: RenderLayer = RenderLayer(1 << 1);

render_view.render_mask = RenderMask(!PORTAL_SURFACE_LAYER.0);
```

The draw traversal checks the mask before drawing:

```rust
for (renderable_handle, layer) in query.iter() {
    if !render_mask.includes(layer) {
        continue;
    }
    // Existing draw code follows.
}
```

I applied the same filtering to Model, RenderableHandle, ComputeRenderable and the selected skybox registration. It acts on complete draw registrations, including their batches of instances.

Consequently, an auxiliary view never draws either portal surface. The frames remain ordinary geometry. Untagged screen sprites also remain eligible for auxiliary rendering.

## 7. Clip geometry behind the exit

Files: [portal_exit_plane()](D:/Documents/Hyggekode/Rust/sparmos-engine-working/sparmos-website/src/gameloop.rs:143) and [camera.rs](D:/Documents/Hyggekode/Rust/sparmos-engine-working/sparmos-engine/src/systems/camera.rs:298).

The derived camera sits behind the destination opening. We need the rendered world to begin at that opening, so objects between the derived eye and the exit are clipped.

A `ClipPlane` stores a normal `n` and a constant `d`. Its plane equation is:

$$
h(x)=n\cdot x+d
$$

Points where `h(x) >= 0` survive. For a point `p` on the plane, `d = -n · p`.

The playground starts with the endpoint's rotated +Z normal and flips it if needed to put the eye on the rejected side:

```rust
let mut normal = transform.rotation.rotate_vector(Vector3::unit_z());
if normal.dot(eye.to_vec() - transform.position) > 0.0 {
    normal = -normal;
}
ClipPlane::from_point_normal(
    Point3::from_vec(transform.position + normal * 0.01),
    normal,
)
```

The `0.01` offset pushes the boundary slightly into the destination world.

I implemented this as an optional oblique near plane on RenderView. The engine adjusts the camera projection, so materials using that projection get the clipping automatically.

The relevant projection code is:

```rust
let row = plane * (2.0 / denominator) - projection.row(3);
projection.x.z = row.x;
projection.y.z = row.y;
projection.z.z = row.z;
projection.w.z = row.w;
```

Here `plane` has already been transformed into camera space using the inverse transpose of the view matrix. `denominator` is the plane dotted with the selected far corner transformed back through the original projection.

The assignments replace the projection's third row. `cgmath` stores columns, which is why the code writes the `.z` component of each column.

The reason this works is that OpenGL's near clipping condition is `z + w >= 0`. Replacing the third row with a scaled plane minus the fourth row makes `z + w` evaluate that plane equation. The projection's X, Y and W rows stay unchanged, so screen alignment survives.

The engine then applies its existing OpenGL-to-WGPU conversion:

$$
z_{wgpu}=(z_{gl}+w_{gl})/2
$$

The new near plane therefore lands at WGPU depth `0`, with the far boundary at depth `1` after perspective division. The far clipping plane also changes when this oblique projection is applied.

The implementation follows [Eric Lengyel's oblique clipping method](https://terathon.com/blog/oblique-clipping.html). Views without a clip plane skip this calculation. Degenerate cases fall back to the ordinary projection.

## 8. Update and resize the portal resources

[update_portal_playground()](D:/Documents/Hyggekode/Rust/sparmos-engine-working/sparmos-website/src/gameloop.rs:331) runs at the end of Website update. It reads the main camera once and processes A → B and B → A using the stored handles.

For each endpoint it derives the camera, assigns the exit plane, and updates the visible surface's position and rotation.

On resize, it also performs:

```rust
if target.resize_texture(gfx, size) {
    let texture = gfx.get_texture(target.color_texture().unwrap()).clone();
    let material = gfx.get_renderable(source.surface).material_handle;
    gfx.set_material_texture(material, &texture, 1, 0);
}
```

`RenderTarget::resize_texture()` replaces the color and depth textures in their existing storage slots. Their handles stay the same.

The material must then be rebound because a GPU bind group still references the old texture view. `Graphics::set_material_texture()` rebuilds those bindings while retaining the material handle and pipeline, and updates its lookup key.

This avoids accumulating new targets and materials every time the window changes size. The current targets use full window resolution.

The decorative frames and room geometry are created during setup. Their transforms are not refreshed alongside the surface if an endpoint is moved dynamically later.

## 9. Controls and the startup fix

Keys **1** and **2** position the main camera in front of A or B for inspection. The shared scene-selection helper restores the main presenting view and active gameplay scene, including when returning from the alternate view selected by **4**. Key **3** still selects its existing scene.

There is no player teleportation when walking through a doorway.

During the native check, an existing egui texture-update bug caused a startup panic. I fixed [gui.rs](D:/Documents/Hyggekode/Rust/sparmos-engine-working/sparmos-engine/src/application/gui.rs:96) to consume every pending image update:

```rust
for (id, image_deltas) in full_output.textures_delta.set.drain() {
    for image_delta in image_deltas {
        self.renderer
            .update_texture(&rc.device, &rc.queue, id, &image_delta);
    }
}
```

Texture frees are drained after GUI rendering too. Previously, only the first image delta was applied and the collections were left populated, triggering the dependency's unapplied-delta assertion.

## 10. Checks from the implementation

The earlier implementation run passed eight focused unit tests and a Windows GPU smoke test. It also passed the native build and WASM checks for both crates.

The GPU test captured both directions and changes in camera position and rotation. It checked exit clipping using the magenta bar, verified that camera upload did not apply movement again, and checked resource counts after resizing.

The captures are still available locally:

- [Initial view through A](D:/Documents/Hyggekode/Rust/sparmos-engine-working/sparmos-website/target/portal-smoke/a.png)
- [After strafing](D:/Documents/Hyggekode/Rust/sparmos-engine-working/sparmos-website/target/portal-smoke/strafe.png)
- [Return view through tilted B](D:/Documents/Hyggekode/Rust/sparmos-engine-working/sparmos-website/target/portal-smoke/b.png)
- [Destination with clipping disabled](D:/Documents/Hyggekode/Rust/sparmos-engine-working/sparmos-website/target/portal-smoke/unclipped-destination.png)

The test source modules are no longer in the current working tree. These are the results from the implementation run; this walkthrough does not restore those tests or change the engine code. The `pollster` and `image` development dependencies added for the capture test remain in Cargo.toml/Cargo.lock.
