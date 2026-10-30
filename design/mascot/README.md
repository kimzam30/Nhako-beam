# The voxel mascot

The bolt from the logo in 3D, for the app's personality: the splash, Home, and every state (Phase 22). It's made from the logo's own pixels, not redrawn: `build.mjs` reads `docs/brand/logo/svg/nhako-beam-bolt.svg` (48 x 48) and turns each colored pixel into a column of cubes.

```bash
node design/mascot/build.mjs     # or `npm run mascot` in desktop/
```

It writes `build/`, which is committed:

| File | What |
|---|---|
| `mascot.glb` | One glTF 2.0 model (about 216 KB), a node per moving part, vertex colors, one material. The desktop app loads it with three.js, Android with Filament |
| `mascot.json` | The parts, which ones make each face, and the states with their timings |

## How it's built

- **The face** (the yellow bolt and its dark outline) is 4 voxels deep. **The orange side** the logo draws as the bolt's edge stands one voxel back, so turning the bolt shows real depth.
- **Eyes, mouth and cheeks** are their own parts, standing a little proud of the face, with the face's yellow behind them, so a blink shows yellow, not a hole.
- **The three sparkles** are separate parts, so they twinkle apart.
- **Faces for other states** are hidden parts made here: x eyes (failed), happy ^ eyes and an open smile (happy, done), a crack (failed), and three Zs (asleep).
- Faces of one color next to each other merge into one rectangle, which keeps the file small. Colors are the logo's own, flat, with no textures.
- The model stands on y = 0 and is 3 units high. Each part's node sits at its own middle, so it scales and turns about itself.

## States

From `mascot.json`. Each has full motion and a still pose for reduced motion.

| State | When | Full motion | Reduced motion |
|---|---|---|---|
| idle | Home with nothing going on | Breathes, blinks every 2.6 to 5.2 s, follows the pointer | Still |
| looking | The pointer is over a device, or one was just tapped | Turns to the device, eyes shift | Turned toward it |
| happy | A device was found, or pairing worked | Two hops with squash, sparkles twinkle, happy face | Happy face, sparkles |
| sending | Files are going out | Leans in, sparks fly to the device | Leaning in |
| receiving | Files are coming in | Leans back, sparks fly in | Leaning back |
| done | A transfer finished | Jumps and spins once, sparks burst, happy face | Happy face, sparkles |
| failed | A transfer failed, or a device was lost mid-transfer | Cracks and wobbles, x eyes | Crack, x eyes, tilted |
| asleep | Nothing nearby, or history is empty | Eyes closed, slow breath, Zs drift up | Eyes closed, Zs, tilted |
| offline | No network | Dims, half-closed eyes, droops | Dim, half-closed eyes |
| loading | Starting up, or connecting | Spins on its axis | Still |

The approval mockup (Phase 21) is a published page with every state and a reduced-motion switch.
