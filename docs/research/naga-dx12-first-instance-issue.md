# Draft: naga HLSL backend adds `first_instance` twice to a named `instance_index`

> **Duplicate found. Comment on the existing issue; don't file a new one.**
>
> [gfx-rs/wgpu#4564](https://github.com/gfx-rs/wgpu/issues/4564), "[hlsl-out] `instance_index` workaround doesn't always get applied", is open. It was transferred from `gfx-rs/naga#2416`, the issue Bevy cites in `mesh2d/vertex_input.wesl`.
>
> - Its 2023 body already shows this exact `let idx = v.instance_index` doubling.
> - A 2026-10-29 comment from Firefox / PlayCanvas measured bases 0, 1, 5 and 37 coming out as 0, 2, 10 and 74.
> - No fix PR is linked. Trunk at `dcf2190` (2026-11-04) has the same code.
>
> A new issue would most likely be closed as a duplicate. **Part A** is a short comment for #4564 that adds only what the thread lacks. **Part B** is the full standalone issue, in case you'd rather file one anyway.
>
> Not posted anywhere. You post it.

## What was checked, and how

| Claim | Evidence | Status |
|---|---|---|
| naga 30.0.1 writes `(_NagaConstants.first_instance + idx)` where `idx` already includes the offset | WGSL → HLSL through `naga::back::hlsl::Writer`, with `special_constants_binding` set, the way the DX12 backend sets it | Confirmed (output below) |
| Same for `vertex_index` / `first_vertex` | Same translator | Confirmed |
| Wrong pixels on DX12 only | Headless wgpu 30.0.1 program on an RTX 2080 Ti: DX12 vs Vulkan | Confirmed |
| Moving the `named_expressions` check first fixes the doubling | Patched copy of naga 30.0.1: HLSL re-translated, GPU repro re-run | Confirmed (see Part B, "Suggested fix") |
| Trunk unchanged | Read `naga/src/back/hlsl/writer.rs` at `dcf21900` | Read only, not built |

Repro files are outside the repo, in `%TEMP%\naga-first-instance\` (`C:\Users\flynn\AppData\Local\Temp\naga-first-instance\`):

- `shaders\` holds the WGSL variants.
- `hlsl\` holds the generated HLSL, both stock (`*.hlsl`) and patched (`*.patched.hlsl`).
- `translate\` is the WGSL → HLSL tool. `translate-patched\` is the same tool built against the patched naga.
- `repro\` is the headless DX12 vs Vulkan program. `repro-patched\` is the same program built against the patched naga.
- `naga-patched\` is a copy of naga 30.0.1 with the fix applied.
- `trunk-writer.rs` is trunk's `writer.rs` at `dcf2190`.

Nothing new was downloaded from crates.io. Every crate (naga, wgpu, wgpu-hal and their dependencies) was already in `~/.cargo/registry`, and both programs built with `cargo build --offline`. The only network reads were GitHub, through `gh`: issue searches, #4564, and trunk's `writer.rs` / `back/mod.rs`.

---

## Part A: comment for gfx-rs/wgpu#4564 (recommended)

````markdown
Still present on trunk (`dcf2190`) and in naga/wgpu 30.0.1. Here is where it comes from, plus two details not yet in this thread: `vertex_index` doubles the same way, and Bevy 0.20's default sprite shader hits it.

**Root cause.** In `Writer::write_expr` (`naga/src/back/hlsl/writer.rs`, trunk ~L3797–3827; 30.0.1 L3731–3762), `is_fixed_function_input` runs *before* the `named_expressions` lookup. For an expression that resolves to `BuiltIn::InstanceIndex` or `BuiltIn::VertexIndex`, the writer has already emitted `(_NagaConstants.first_instance + ` when it reaches

```rust
if let Some(name) = self.named_expressions.get(&expr) {
    write!(self.out, "{name}{closing_bracket}")?;
    return Ok(());
}
```

The name was declared by `write_named_expr`, which calls `write_expr` on the same handle *before* inserting the name. So the name already contains the offset, and every use adds it again. Only struct members (`AccessIndex` on the entry-point argument) are affected. With `@builtin(instance_index) i: u32` as a direct parameter, `let idx = i;` is never baked, and the output is correct.

**Minimal WGSL** (translated with `special_constants_binding: Some(..)`, SM 6.0):

```wgsl
struct VertexIn { @builtin(instance_index) instance_index: u32 }
@group(0) @binding(0) var<storage, read> offsets: array<vec4<f32>>;
@vertex
fn vs_main(in: VertexIn) -> @builtin(position) vec4<f32> {
    let idx = in.instance_index;
    return offsets[idx];
}
```

```hlsl
uint idx = (_NagaConstants.first_instance + in_.instance_index);
float4 _e4 = asfloat(offsets.Load4((_NagaConstants.first_instance + idx)*16));
```

`vertex_index` gives `uint idx = (_NagaConstants.first_vertex + in_.vertex_index);` followed by `Load4((_NagaConstants.first_vertex + idx)*16)`.

On an RTX 2080 Ti (Windows 11), a headless wgpu 30.0.1 program issues `draw(0..6, 0..1)` then `draw(0..6, 1..2)`. Instance *i* fills pixel column `cells[i].column`. With the `let` form, DX12 drew the second draw from cell 2 instead of cell 1. Vulkan, and the inline form on DX12, were correct:

```
Dx12   inline -> ["red", "green", "-", "-"]
Dx12   named  -> ["red", "-", "blue", "-"]
Vulkan inline -> ["red", "green", "-", "-"]
Vulkan named  -> ["red", "green", "-", "-"]
```

**Minimal fix for the doubling.** Do the `named_expressions` lookup first and write `name` with no prefix. I tried this on a local copy of 30.0.1. `let` and `vertex_index` then emit the offset once, the inline, direct-parameter and `var` forms are unchanged, and the GPU repro above is correct on DX12. This does *not* fix the other half of this issue, where the struct is passed to a helper function or copied and the offset is lost. Adding the base once in the entry-point prologue, as suggested above, or `SV_StartInstanceLocation` (#6589), would fix both.

**How it surfaces.** Bevy 0.20.1's `bevy_sprite_render/src/sprite_mesh/sprite_material.wesl` starts its vertex shader with `let instance_index = vertex.instance_index;`. On DX12, when sprites aren't bindless, every `Sprite` draw after the first reads the wrong per-instance data or reads past the end of the buffer. Bevy's own `mesh2d.wesl` reads `vertex.instance_index` inline and is fine. https://github.com/milocosmopolitan/little-forge/pull/16 hit the same Bevy trigger independently.
````

---

## Part B: standalone issue (fallback)

### Title

```
[hlsl-out] Named `instance_index` / `vertex_index` gets `_NagaConstants.first_*` added twice (DX12)
```

### Body

````markdown
> Possibly a duplicate of #4564, which reports the same `let` doubling alongside a separate "offset lost when the struct is passed to a helper" case. Filed separately because this half has a small, local fix, and its cause is pinned to one function below. Feel free to close it in favour of #4564.

### Summary

On the DX12 backend, naga's HLSL writer emulates `first_instance` / `first_vertex` by adding `_NagaConstants.first_instance` (or `.first_vertex`) wherever it writes the `instance_index` / `vertex_index` builtin. When the builtin is read from the entry-point input struct and bound to a name (`let idx = in.instance_index;`), the offset goes into the name's initializer *and* into every use of the name. The shader therefore sees `local + 2 × first_instance`.

### Affected

- naga 30.0.1 / wgpu 30.0.1: confirmed, both from the generated HLSL and on a GPU.
- Trunk `dcf21900e592c3c0be10c9888d23d443c058eea2` (2026-11-04): the same code in `naga/src/back/hlsl/writer.rs`. I read it there but didn't build it.
- Backend: DX12 (HLSL out with `special_constants_binding` set). Vulkan and Metal are unaffected. I tested Vulkan only.
- Tested on an NVIDIA RTX 2080 Ti, Windows 11, driver 32.0.16.1088 (Vulkan reports 610.88).

### When it happens

All of these must hold:

1. An `@builtin(instance_index)` (or `@builtin(vertex_index)`) is a member of the vertex entry point's input struct.
2. The entry point binds that member to a name with `let`, e.g. `let idx = in.instance_index;`.
3. The draw has a non-zero `first_instance` (or `first_vertex` / `base_vertex`).

These are not affected:

- Reading `in.instance_index` inline, even several times.
- `var idx = in.instance_index;`.
- A direct `@builtin(instance_index) i: u32` parameter bound with `let`.

### Minimal WGSL

```wgsl
struct VertexIn {
    @builtin(instance_index) instance_index: u32,
}

@group(0) @binding(0) var<storage, read> offsets: array<vec4<f32>>;

@vertex
fn vs_main(in: VertexIn) -> @builtin(position) vec4<f32> {
    let idx = in.instance_index;
    return offsets[idx];
}
```

Translated with `naga::back::hlsl::Writer`, using `Options { shader_model: V6_0, special_constants_binding: Some(BindTarget { space: 0, register: 0, .. }), ..Default::default() }`:

**Actual**

```hlsl
float4 vs_main(VertexIn in_) : SV_Position
{
    uint idx = (_NagaConstants.first_instance + in_.instance_index);
    float4 _e4 = asfloat(offsets.Load4((_NagaConstants.first_instance + idx)*16));
    return _e4;
}
```

**Expected** (the offset applied once):

```hlsl
    uint idx = (_NagaConstants.first_instance + in_.instance_index);
    float4 _e4 = asfloat(offsets.Load4(idx*16));
```

For comparison, the inline form `return offsets[in.instance_index];` is correct today:

```hlsl
    float4 _e4 = asfloat(offsets.Load4((_NagaConstants.first_instance + in_.instance_index)*16));
```

`vertex_index` behaves the same way:

```hlsl
    uint idx = (_NagaConstants.first_vertex + in_.vertex_index);
    float4 _e4 = asfloat(offsets.Load4((_NagaConstants.first_vertex + idx)*16));
```

### End-to-end repro

This is a headless wgpu 30.0.1 program: no window, render to a 4×1 `Rgba8Unorm` texture, read it back. A storage buffer holds four cells, `{color, column}` = red@0, green@1, blue@2, yellow@3. The vertex shader draws a quad over pixel column `cells[i].column` in `cells[i].color`. One pass issues `draw(0..6, 0..1)` then `draw(0..6, 1..2)`. Expected: `[red, green, -, -]`.

```
Dx12 NVIDIA GeForce RTX 2080 Ti: inline -> columns ["red", "green", "-", "-"]
Dx12 NVIDIA GeForce RTX 2080 Ti: named  -> columns ["red", "-", "blue", "-"]
Vulkan NVIDIA GeForce RTX 2080 Ti (610.88): inline -> columns ["red", "green", "-", "-"]
Vulkan NVIDIA GeForce RTX 2080 Ti (610.88): named  -> columns ["red", "green", "-", "-"]
```

In the second draw, `SV_InstanceID = 0` and `first_instance = 1`, so the shader reads cell `0 + 2×1 = 2`. With `first_instance = 2` it would read cell 4, past the end of the buffer. Full program: <link to gist, if you want one>.

### Root cause

`naga/src/back/hlsl/writer.rs`, `Writer::write_expr` (30.0.1: L3731–3762; trunk: ~L3797–3827):

```rust
// Handle the special semantics of vertex_index/instance_index
let ff_input = if self.options.special_constants_binding.is_some() {
    func_ctx.is_fixed_function_input(expr, module)
} else {
    None
};
let closing_bracket = match ff_input {
    Some(crate::BuiltIn::VertexIndex) => {
        write!(self.out, "({SPECIAL_CBUF_VAR}.{SPECIAL_FIRST_VERTEX} + ")?;
        ")"
    }
    Some(crate::BuiltIn::InstanceIndex) => {
        write!(self.out, "({SPECIAL_CBUF_VAR}.{SPECIAL_FIRST_INSTANCE} + ",)?;
        ")"
    }
    // ...
    _ => "",
};

if let Some(name) = self.named_expressions.get(&expr) {
    write!(self.out, "{name}{closing_bracket}")?;
    return Ok(());
}
```

1. `FunctionCtx::is_fixed_function_input` (`naga/src/back/mod.rs`) follows `AccessIndex` back to the entry-point `FunctionArgument`. For `in.instance_index`, it returns `Some(BuiltIn::InstanceIndex)`, so the `(_NagaConstants.first_instance + ` prefix is written.
2. When the WGSL `let` is emitted, `Writer::write_named_expr` writes `uint idx = `, calls `write_expr(handle)` (the prefix is applied, which is correct), and only then runs `self.named_expressions.insert(expr, name)`.
3. On every later use, `write_expr` is called with the same `AccessIndex` handle. `is_fixed_function_input` still says `InstanceIndex`, so the prefix is written again. Then the `named_expressions` early return writes `idx)`. `idx` already contains the offset, so the result is `(first_instance + idx)`.

The condition is: `ff_input` is `VertexIndex` / `InstanceIndex` **and** `named_expressions` contains `expr`. The `NumWorkGroups` arm returns before the lookup, so it is unaffected. A direct builtin parameter is a bare `FunctionArgument`, which is never baked into a named expression, so it is unaffected too.

### Suggested fix

Do the `named_expressions` lookup before the fixed-function check, and write the name with no prefix. Any named expression was initialised through `write_expr`, so it already carries the offset:

```diff
     ) -> BackendResult {
         use crate::Expression;

+        if let Some(name) = self.named_expressions.get(&expr) {
+            write!(self.out, "{name}")?;
+            return Ok(());
+        }
+
         // Handle the special semantics of vertex_index/instance_index
         let ff_input = if self.options.special_constants_binding.is_some() {
@@
-        if let Some(name) = self.named_expressions.get(&expr) {
-            write!(self.out, "{name}{closing_bracket}")?;
-            return Ok(());
-        }
```

I applied this to a local copy of naga 30.0.1 and checked it two ways:

- **HLSL output.** The `let` case now emits `uint idx = (_NagaConstants.first_instance + in_.instance_index);` followed by `Load4(idx*16)`, and `vertex_index` emits the same with `first_vertex`. The inline, inline-used-twice, direct-parameter and `var` forms are byte-for-byte unchanged.
- **GPU repro.** The repro above, built against the patched naga, gives `[red, green, -, -]` on DX12.

I didn't run naga's snapshot tests.

This fixes the doubling only. It does not fix the other case in #4564, where the input struct is passed to a helper function or copied and the offset is lost entirely. A more complete fix is the one proposed in #4564: add the base to the builtin once, in the entry-point prologue, and drop the per-use rewrite. On SM 6.8 or later, `SV_StartInstanceLocation` / `SV_StartVertexLocation` would also work (#6589).

### How it surfaced

Bevy 0.20.1's `bevy_sprite_render/src/sprite_mesh/sprite_material.wesl` begins its vertex shader with `let instance_index = vertex.instance_index;` and uses the name to index mesh data. On DX12, when sprites aren't bindless (each sprite is its own draw, with `first_instance` = its slot), the first sprite draws correctly. The second draws at the third sprite's transform, and the third reads past the end of the instance buffer and disappears. Bevy's `mesh2d.wesl` reads `vertex.instance_index` inline and draws correctly. Bevy already works around this bug elsewhere: `mesh2d/vertex_input.wesl` cites gfx-rs/naga#2416, which is now this repo's #4564.
````

---

## Separate Bevy issue (worth filing too)

No existing Bevy issue turned up. I searched `bevyengine/bevy` for "sprite dx12", "sprite_material.wesl", "SpriteMesh DX12", "instance_index naga" and "first_instance". Even if naga is fixed, Bevy 0.20.x ships the trigger:

- `sprite_material.wesl:22` has `let instance_index = vertex.instance_index;`.
- Lines 25, 36 and 41 use it to index mesh data.
- Line 65, `out.instance_index = instance_index;`, passes the doubled value to the fragment stage, which reads `mesh.instance_index`.

Bevy's own `mesh2d.wesl` avoids the bug by reading `vertex.instance_index` inline, as its comment on naga#2416 says. The fix is a one-line change in `sprite_material.wesl`: read `vertex.instance_index` inline, or use `var instance_index`. The `var` form generates `idx = (first_instance + in_.instance_index)` once. [milocosmopolitan/little-forge#16](https://github.com/milocosmopolitan/little-forge/pull/16) shipped exactly that `var` patch as a workaround in their game.
